//! Per-thread CPU counters, to report UI-thread and audio-thread cost
//! separately (per frame / per buffer), not just process CPU %.
//!
//! - Windows: `QueryThreadCycleTime`, which counts TSC reference cycles the
//!   thread was scheduled for (not retired instructions: those need PMU access,
//!   i.e. an elevated ETW session; see ENGINE.md). Raw kernel32 FFI, no crate.
//! - macOS: `proc_pidinfo(PROC_PIDTHREADCOUNTS)`, which reports retired
//!   instructions, cycles, CPU time and energy per thread, split by the core
//!   type it ran on (P or E). No privileges are needed for our own process.
//!   The flavor and its structs are in XNU's `bsd/sys/proc_info_private.h`
//!   (not in the public SDK); the layout and the meaning of the thread id were
//!   validated against `CLOCK_THREAD_CPUTIME_ID` and a loop of known
//!   instruction count (ENGINE-macos.md §2). Raw libc FFI, no crate.
//! - Other platforms: not implemented (returns `None`).

/// Counter totals for one thread (or the process) since it started.
/// Fields a platform can't measure are `None`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Counts {
    /// Windows: TSC reference cycles. macOS: core clock cycles.
    pub cycles: u64,
    pub instructions: Option<u64>,
    pub energy_nj: Option<u64>,
    /// CPU time on performance and efficiency cores (ns).
    pub p_ns: Option<u64>,
    pub e_ns: Option<u64>,
}

impl Counts {
    /// `self - earlier`, field by field.
    pub fn since(&self, earlier: &Counts) -> Counts {
        let d = |a: Option<u64>, b: Option<u64>| Some(a?.saturating_sub(b?));
        Counts {
            cycles: self.cycles.saturating_sub(earlier.cycles),
            instructions: d(self.instructions, earlier.instructions),
            energy_nj: d(self.energy_nj, earlier.energy_nj),
            p_ns: d(self.p_ns, earlier.p_ns),
            e_ns: d(self.e_ns, earlier.e_ns),
        }
    }

    pub fn add(&mut self, o: &Counts) {
        let a = |x: &mut Option<u64>, y: Option<u64>| {
            if let (Some(x), Some(y)) = (x.as_mut(), y) {
                *x += y;
            }
        };
        self.cycles += o.cycles;
        a(&mut self.instructions, o.instructions);
        a(&mut self.energy_nj, o.energy_nj);
        a(&mut self.p_ns, o.p_ns);
        a(&mut self.e_ns, o.e_ns);
    }

    /// Share of CPU time spent on performance cores (0..1).
    pub fn p_share(&self) -> Option<f64> {
        let (p, e) = (self.p_ns?, self.e_ns?);
        (p + e > 0).then(|| p as f64 / (p + e) as f64)
    }
}

#[cfg(windows)]
mod imp {
    use std::ffi::c_void;

    type Handle = *mut c_void;
    const THREAD_QUERY_LIMITED_INFORMATION: u32 = 0x0800;

    #[link(name = "kernel32")]
    extern "system" {
        fn GetCurrentThreadId() -> u32;
        fn GetCurrentThread() -> Handle;
        fn OpenThread(access: u32, inherit: i32, tid: u32) -> Handle;
        fn CloseHandle(h: Handle) -> i32;
        fn QueryThreadCycleTime(h: Handle, cycles: *mut u64) -> i32;
    }

    /// A user-mode read of the thread environment block (no syscall).
    pub fn current_thread_id() -> u64 {
        unsafe { GetCurrentThreadId() as u64 }
    }

    pub fn current_thread_cycles() -> Option<u64> {
        let mut c = 0u64;
        // SAFETY: pseudo-handle for the calling thread; valid out-pointer.
        (unsafe { QueryThreadCycleTime(GetCurrentThread(), &mut c) } != 0).then_some(c)
    }

    pub fn current_thread_counts() -> Option<super::Counts> {
        current_thread_cycles().map(|cycles| super::Counts { cycles, ..Default::default() })
    }

    pub fn process_counts() -> Option<super::Counts> {
        None
    }

    /// Reads another thread's cycle counter by id.
    pub struct ThreadClock {
        h: Handle,
    }

    // SAFETY: a thread handle may be used from any thread.
    unsafe impl Send for ThreadClock {}

    impl ThreadClock {
        pub fn open(tid: u64) -> Option<Self> {
            // SAFETY: plain Win32 call; a null result means failure.
            let h = unsafe { OpenThread(THREAD_QUERY_LIMITED_INFORMATION, 0, tid as u32) };
            (!h.is_null()).then_some(Self { h })
        }

        pub fn cycles(&self) -> Option<u64> {
            let mut c = 0u64;
            // SAFETY: `h` is an open thread handle owned by self.
            (unsafe { QueryThreadCycleTime(self.h, &mut c) } != 0).then_some(c)
        }

        pub fn counts(&self) -> Option<super::Counts> {
            self.cycles().map(|cycles| super::Counts { cycles, ..Default::default() })
        }
    }

    impl Drop for ThreadClock {
        fn drop(&mut self) {
            // SAFETY: closing the handle we opened.
            unsafe { CloseHandle(self.h) };
        }
    }
}

#[cfg(target_os = "macos")]
mod imp {
    use super::Counts;
    use std::ffi::c_void;

    /// `PROC_PIDTHREADCOUNTS` (XNU `bsd/sys/proc_info_private.h`).
    const PROC_PIDTHREADCOUNTS: i32 = 34;
    /// `RUSAGE_INFO_V6` (`<sys/resource.h>`).
    const RUSAGE_INFO_V6: i32 = 6;
    /// Perf levels reported (M1: 0 = Performance, 1 = Efficiency; `hw.perflevelN.name`).
    const LEVELS: usize = 2;

    #[repr(C)]
    #[derive(Clone, Copy, Default)]
    struct ThreadCountsData {
        instructions: u64,
        cycles: u64,
        user_time_mach: u64,
        system_time_mach: u64,
        energy_nj: u64,
    }

    #[repr(C)]
    #[derive(Default)]
    struct ThreadCounts {
        len: u16,
        _r0: u16,
        _r1: u32,
        counts: [ThreadCountsData; LEVELS],
    }

    /// `struct rusage_info_v6`, `<sys/resource.h>`: 16 bytes of UUID then
    /// u64 fields; only the ones used here are named, by index.
    #[repr(C)]
    struct RusageV6 {
        uuid: [u8; 16],
        f: [u64; 56],
    }
    // Field indices into `f` (counted from ri_user_time = 0).
    const RI_USER_TIME: usize = 0;
    const RI_SYSTEM_TIME: usize = 1;
    const RI_INSTRUCTIONS: usize = 29;
    const RI_CYCLES: usize = 30;
    const RI_USER_PTIME: usize = 36;
    const RI_SYSTEM_PTIME: usize = 37;
    const RI_ENERGY_NJ: usize = 40;

    extern "C" {
        fn proc_pidinfo(pid: i32, flavor: i32, arg: u64, buf: *mut c_void, size: i32) -> i32;
        fn proc_pid_rusage(pid: i32, flavor: i32, buf: *mut c_void) -> i32;
        fn pthread_threadid_np(thread: *mut c_void, id: *mut u64) -> i32;
        fn getpid() -> i32;
        fn mach_timebase_info(info: *mut [u32; 2]) -> i32;
    }

    fn mach_to_ns(t: u64) -> u64 {
        let mut tb = [0u32; 2];
        // SAFETY: valid out-pointer.
        unsafe { mach_timebase_info(&mut tb) };
        if tb[1] == 0 {
            return t;
        }
        (t as u128 * tb[0] as u128 / tb[1] as u128) as u64
    }

    /// The system-wide 64-bit thread id. For the calling thread this reads
    /// the id cached in its pthread structure (no syscall).
    pub fn current_thread_id() -> u64 {
        let mut id = 0u64;
        // SAFETY: null = the calling thread; valid out-pointer.
        unsafe { pthread_threadid_np(std::ptr::null_mut(), &mut id) };
        id
    }

    fn thread_counts(tid: u64) -> Option<Counts> {
        let mut c = ThreadCounts::default();
        // SAFETY: the buffer is a ThreadCounts of the size passed.
        let n = unsafe {
            proc_pidinfo(
                getpid(),
                PROC_PIDTHREADCOUNTS,
                tid,
                &mut c as *mut ThreadCounts as *mut c_void,
                std::mem::size_of::<ThreadCounts>() as i32,
            )
        };
        if n <= 0 {
            return None;
        }
        let k = (c.len as usize).min(LEVELS);
        let mut r = Counts {
            instructions: Some(0),
            energy_nj: Some(0),
            p_ns: Some(0),
            e_ns: Some(0),
            ..Default::default()
        };
        for (lvl, d) in c.counts[..k].iter().enumerate() {
            r.cycles += d.cycles;
            *r.instructions.as_mut()? += d.instructions;
            *r.energy_nj.as_mut()? += d.energy_nj;
            let t = mach_to_ns(d.user_time_mach + d.system_time_mach);
            if lvl == 0 {
                r.p_ns = Some(t);
            } else {
                *r.e_ns.as_mut()? += t;
            }
        }
        Some(r)
    }

    pub fn current_thread_cycles() -> Option<u64> {
        current_thread_counts().map(|c| c.cycles)
    }

    pub fn current_thread_counts() -> Option<Counts> {
        thread_counts(current_thread_id())
    }

    /// Whole-process totals (`proc_pid_rusage`, as in PERF-macos.md).
    pub fn process_counts() -> Option<Counts> {
        let mut r = RusageV6 { uuid: [0; 16], f: [0; 56] };
        // SAFETY: the buffer is a rusage_info_v6-sized struct.
        let st = unsafe { proc_pid_rusage(getpid(), RUSAGE_INFO_V6, &mut r as *mut RusageV6 as *mut c_void) };
        if st != 0 {
            return None;
        }
        let _ = r.uuid;
        let total = r.f[RI_USER_TIME] + r.f[RI_SYSTEM_TIME];
        let p = r.f[RI_USER_PTIME] + r.f[RI_SYSTEM_PTIME];
        Some(Counts {
            cycles: r.f[RI_CYCLES],
            instructions: Some(r.f[RI_INSTRUCTIONS]),
            energy_nj: Some(r.f[RI_ENERGY_NJ]),
            p_ns: Some(mach_to_ns(p)),
            e_ns: Some(mach_to_ns(total.saturating_sub(p))),
        })
    }

    /// Reads another thread of this process by id.
    pub struct ThreadClock {
        tid: u64,
    }

    impl ThreadClock {
        pub fn open(tid: u64) -> Option<Self> {
            thread_counts(tid).map(|_| Self { tid })
        }

        pub fn cycles(&self) -> Option<u64> {
            self.counts().map(|c| c.cycles)
        }

        pub fn counts(&self) -> Option<Counts> {
            thread_counts(self.tid)
        }
    }
}

#[cfg(not(any(windows, target_os = "macos")))]
mod imp {
    pub fn current_thread_id() -> u64 {
        0
    }

    pub fn current_thread_counts() -> Option<super::Counts> {
        None
    }

    pub fn process_counts() -> Option<super::Counts> {
        None
    }

    pub fn current_thread_cycles() -> Option<u64> {
        None
    }

    pub struct ThreadClock;

    impl ThreadClock {
        pub fn open(_tid: u64) -> Option<Self> {
            None
        }

        pub fn cycles(&self) -> Option<u64> {
            None
        }

        pub fn counts(&self) -> Option<super::Counts> {
            None
        }
    }
}

pub use imp::*;

#[cfg(all(test, target_os = "macos"))]
mod mac_tests {
    use super::*;

    #[test]
    fn counts_track_known_work() {
        let a = current_thread_counts().unwrap();
        let mut x = 0u64;
        for i in 0..50_000_000u64 {
            x = x.wrapping_add(std::hint::black_box(i));
        }
        std::hint::black_box(x);
        let d = current_thread_counts().unwrap().since(&a);
        assert!(d.instructions.unwrap() > 50_000_000, "{d:?}");
        assert!(d.cycles > 0 && d.p_ns.unwrap() + d.e_ns.unwrap() > 0);
        let c = ThreadClock::open(current_thread_id()).unwrap();
        assert!(c.counts().unwrap().instructions >= current_thread_counts().unwrap().instructions.map(|i| i / 2));
        let p = process_counts().unwrap();
        assert!(p.instructions.unwrap() >= d.instructions.unwrap());
        assert!(ThreadClock::open(1).is_none());
    }
}

#[cfg(all(test, windows))]
mod tests {
    use super::*;

    #[test]
    fn cycles_advance() {
        let a = current_thread_cycles().unwrap();
        let mut x = 0u64;
        for i in 0..1_000_000u64 {
            x = x.wrapping_add(std::hint::black_box(i));
        }
        std::hint::black_box(x);
        let b = current_thread_cycles().unwrap();
        assert!(b > a);
        let c = ThreadClock::open(current_thread_id()).unwrap();
        assert!(c.cycles().unwrap() >= b);
    }
}
