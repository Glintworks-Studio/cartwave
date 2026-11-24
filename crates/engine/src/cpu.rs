//! Per-thread CPU cycle counters, to report UI-thread and audio-thread cost
//! separately (cycles per frame / per buffer), not just process CPU %.
//!
//! Windows: `QueryThreadCycleTime`, which counts TSC reference cycles the
//! thread was scheduled for (not retired instructions: those need PMU access,
//! i.e. an elevated ETW session; see ENGINE.md). Raw kernel32 FFI, no crate.
//! Other platforms: not implemented here (returns `None`).

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
    pub fn current_thread_id() -> u32 {
        unsafe { GetCurrentThreadId() }
    }

    pub fn current_thread_cycles() -> Option<u64> {
        let mut c = 0u64;
        // SAFETY: pseudo-handle for the calling thread; valid out-pointer.
        (unsafe { QueryThreadCycleTime(GetCurrentThread(), &mut c) } != 0).then_some(c)
    }

    /// Reads another thread's cycle counter by id.
    pub struct ThreadClock {
        h: Handle,
    }

    // SAFETY: a thread handle may be used from any thread.
    unsafe impl Send for ThreadClock {}

    impl ThreadClock {
        pub fn open(tid: u32) -> Option<Self> {
            // SAFETY: plain Win32 call; a null result means failure.
            let h = unsafe { OpenThread(THREAD_QUERY_LIMITED_INFORMATION, 0, tid) };
            (!h.is_null()).then_some(Self { h })
        }

        pub fn cycles(&self) -> Option<u64> {
            let mut c = 0u64;
            // SAFETY: `h` is an open thread handle owned by self.
            (unsafe { QueryThreadCycleTime(self.h, &mut c) } != 0).then_some(c)
        }
    }

    impl Drop for ThreadClock {
        fn drop(&mut self) {
            // SAFETY: closing the handle we opened.
            unsafe { CloseHandle(self.h) };
        }
    }
}

#[cfg(not(windows))]
mod imp {
    pub fn current_thread_id() -> u32 {
        0
    }

    pub fn current_thread_cycles() -> Option<u64> {
        None
    }

    pub struct ThreadClock;

    impl ThreadClock {
        pub fn open(_tid: u32) -> Option<Self> {
            None
        }

        pub fn cycles(&self) -> Option<u64> {
            None
        }
    }
}

pub use imp::*;

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
