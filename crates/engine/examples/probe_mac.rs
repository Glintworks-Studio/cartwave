//! macOS probe: what CoreAudio grants through cpal, and what the audio thread is.
//!
//!   cargo run --release -p engine --example probe_mac -- [--buffers 64,128,256,512,default]
//!       [--rate 48000] [--secs 3] [--hold SECS] [--device NAME]
//!
//! For each requested buffer size it opens the default output (or `--device`) (cpal, the same
//! call the engine makes), plays a quiet tone, and reports:
//! - frames per callback (min / max / mode) and the gap between callbacks;
//! - cpal's playback lead (`playback - callback`) and how far cpal's
//!   `callback` timestamp is from "now" inside the callback (the A/V zero point);
//! - cpal's `Stream::buffer_size()` (the device property, read back);
//! - the audio thread: its 64-bit id, scheduling policy and priorities
//!   (`proc_pidinfo(PROC_PIDTHREADID64INFO)`), its Mach time-constraint
//!   parameters (`thread_policy_get`), and its P/E-core time and instructions
//!   (`engine::cpu`).
//!
//! `--hold SECS` keeps each stream open that long after measuring, so another
//! process (`coreaudio list`) can read the device's buffer size meanwhile.

#[cfg(not(target_os = "macos"))]
fn main() {
    eprintln!("probe_mac is macOS-only");
}

#[cfg(target_os = "macos")]
fn main() {
    mac::main()
}

#[cfg(target_os = "macos")]
mod mac {
    use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
    use std::ffi::c_void;
    use std::sync::atomic::{AtomicU64, Ordering};
    use std::sync::{Arc, Mutex};
    use std::time::Duration;

    #[derive(Clone, Copy)]
    struct Rec {
        now_ns: u64,
        frames: u32,
        ahead_ns: u64,
        /// cpal `callback` timestamp minus "now" (ns, signed).
        cb_minus_now_ns: i64,
    }

    extern "C" {
        fn mach_absolute_time() -> u64;
        fn mach_timebase_info(info: *mut [u32; 2]) -> i32;
        fn proc_pidinfo(pid: i32, flavor: i32, arg: u64, buf: *mut c_void, size: i32) -> i32;
        fn getpid() -> i32;
        static mach_task_self_: u32;
        fn task_threads(task: u32, list: *mut *mut u32, count: *mut u32) -> i32;
        fn thread_info(thread: u32, flavor: u32, info: *mut i32, count: *mut u32) -> i32;
        fn thread_policy_get(thread: u32, flavor: u32, info: *mut u32, count: *mut u32, get_default: *mut u32) -> i32;
        fn mach_port_deallocate(task: u32, name: u32) -> i32;
        fn vm_deallocate(task: u32, addr: usize, size: usize) -> i32;
    }

    /// `PROC_PIDTHREADID64INFO` and `struct proc_threadinfo` (`<sys/proc_info.h>`).
    const PROC_PIDTHREADID64INFO: i32 = 15;
    #[repr(C)]
    struct ProcThreadInfo {
        user_time: u64,
        system_time: u64,
        cpu_usage: i32,
        policy: i32,
        run_state: i32,
        flags: i32,
        sleep_time: i32,
        curpri: i32,
        priority: i32,
        maxpriority: i32,
        name: [u8; 64],
    }
    /// `THREAD_IDENTIFIER_INFO` (`<mach/thread_info.h>`): thread_id, thread_handle, dispatch_qaddr.
    const THREAD_IDENTIFIER_INFO: u32 = 4;
    /// `THREAD_TIME_CONSTRAINT_POLICY` (`<mach/thread_policy.h>`): period, computation,
    /// constraint (u32, Mach time units), preemptible (boolean_t).
    const THREAD_TIME_CONSTRAINT_POLICY: u32 = 2;

    fn mach_ns(t: u64) -> u64 {
        let mut tb = [0u32; 2];
        unsafe { mach_timebase_info(&mut tb) };
        (t as u128 * tb[0] as u128 / tb[1].max(1) as u128) as u64
    }

    fn now_ns() -> u64 {
        mach_ns(unsafe { mach_absolute_time() })
    }

    fn policy_name(p: i32) -> &'static str {
        match p {
            1 => "TIMESHARE",
            2 => "RR",
            4 => "FIFO",
            _ => "?",
        }
    }

    /// Time-constraint parameters of the thread with this 64-bit id, if it has them.
    fn time_constraint(tid: u64) -> Option<(f64, f64, f64, bool)> {
        unsafe {
            let task = mach_task_self_;
            let mut list: *mut u32 = std::ptr::null_mut();
            let mut n = 0u32;
            if task_threads(task, &mut list, &mut n) != 0 {
                return None;
            }
            let threads = std::slice::from_raw_parts(list, n as usize);
            let mut out = None;
            for &th in threads {
                let mut id = [0u64; 3];
                let mut cnt = 6u32;
                if thread_info(th, THREAD_IDENTIFIER_INFO, id.as_mut_ptr() as *mut i32, &mut cnt) == 0 && id[0] == tid {
                    let mut p = [0u32; 4];
                    let mut c = 4u32;
                    let mut def = 0u32;
                    if thread_policy_get(th, THREAD_TIME_CONSTRAINT_POLICY, p.as_mut_ptr(), &mut c, &mut def) == 0 {
                        let ms = |v: u32| mach_ns(v as u64) as f64 / 1e6;
                        // get_default comes back true if the thread has no such policy.
                        out = (def == 0).then(|| (ms(p[0]), ms(p[1]), ms(p[2]), p[3] != 0));
                    }
                }
                mach_port_deallocate(task, th);
            }
            vm_deallocate(task, list as usize, n as usize * 4);
            out
        }
    }

    fn thread_sched(tid: u64) -> Option<String> {
        let mut ti: ProcThreadInfo = unsafe { std::mem::zeroed() };
        let n = unsafe {
            proc_pidinfo(getpid(), PROC_PIDTHREADID64INFO, tid, &mut ti as *mut _ as *mut c_void, std::mem::size_of::<ProcThreadInfo>() as i32)
        };
        if n <= 0 {
            return None;
        }
        let name = String::from_utf8_lossy(&ti.name).trim_end_matches('\0').to_string();
        Some(format!(
            "name {:?}, policy {} ({}), priority {}, current {}, max {}",
            name,
            ti.policy,
            policy_name(ti.policy),
            ti.priority,
            ti.curpri,
            ti.maxpriority
        ))
    }

    pub fn main() {
        let args: Vec<String> = std::env::args().collect();
        let get = |k: &str| args.iter().position(|a| a == k).and_then(|i| args.get(i + 1)).cloned();
        let buffers: Vec<Option<u32>> = get("--buffers")
            .unwrap_or_else(|| "64,128,256,512,default".into())
            .split(',')
            .map(|s| s.parse().ok())
            .collect();
        let rate: u32 = get("--rate").and_then(|s| s.parse().ok()).unwrap_or(48_000);
        let secs: f64 = get("--secs").and_then(|s| s.parse().ok()).unwrap_or(3.0);
        let hold: f64 = get("--hold").and_then(|s| s.parse().ok()).unwrap_or(0.0);

        let host = cpal::default_host();
        let dev = match get("--device") {
            Some(name) => host
                .output_devices()
                .unwrap()
                .find(|d| d.description().map(|x| x.name() == name).unwrap_or(false))
                .expect("no output device with that name"),
            None => host.default_output_device().expect("no output device"),
        };
        println!("host {:?}, device {:?}", host.id(), dev.description().map(|d| d.name().to_string()));
        println!("default config: {:?}", dev.default_output_config());
        for c in dev.supported_output_configs().unwrap() {
            println!("  supported: {c:?}");
        }
        for req in buffers {
            run(&dev, req, rate, secs, hold);
        }
    }

    fn run(dev: &cpal::Device, req: Option<u32>, rate: u32, secs: f64, hold: f64) {
        let config = cpal::StreamConfig {
            channels: 2,
            sample_rate: rate,
            buffer_size: match req {
                Some(n) => cpal::BufferSize::Fixed(n),
                None => cpal::BufferSize::Default,
            },
        };
        let recs = Arc::new(Mutex::new(Vec::<Rec>::with_capacity(20_000)));
        let tid = Arc::new(AtomicU64::new(0));
        let errors = Arc::new(Mutex::new(Vec::<String>::new()));
        let (r2, t2, e2) = (recs.clone(), tid.clone(), errors.clone());
        let mut phase = 0f32;
        let stream = match dev.build_output_stream(
            config,
            move |data: &mut [f32], info: &cpal::OutputCallbackInfo| {
                let now = now_ns();
                if t2.load(Ordering::Relaxed) == 0 {
                    t2.store(engine::cpu::current_thread_id(), Ordering::Relaxed);
                }
                let ts = info.timestamp();
                for f in data.chunks_mut(2) {
                    phase = (phase + 440.0 / rate as f32).fract();
                    let v = (phase * std::f32::consts::TAU).sin() * 0.02;
                    f.fill(v);
                }
                if let Ok(mut r) = r2.try_lock() {
                    if r.len() < r.capacity() {
                        r.push(Rec {
                            now_ns: now,
                            frames: (data.len() / 2) as u32,
                            ahead_ns: ts.playback.duration_since(ts.callback).as_nanos() as u64,
                            cb_minus_now_ns: ts.callback.as_nanos() as i64 - now as i64,
                        });
                    }
                }
            },
            move |e| e2.lock().unwrap().push(format!("{:?}: {e}", e.kind())),
            None,
        ) {
            Ok(s) => s,
            Err(e) => {
                println!("\nrequested {req:?}: build failed: {e}");
                return;
            }
        };
        stream.play().unwrap();
        std::thread::sleep(Duration::from_millis(300));
        let t = tid.load(Ordering::Relaxed);
        let c0 = engine::cpu::ThreadClock::open(t).and_then(|c| c.counts());
        std::thread::sleep(Duration::from_secs_f64(secs));
        let c1 = engine::cpu::ThreadClock::open(t).and_then(|c| c.counts());
        let sched = thread_sched(t);
        let tc = time_constraint(t);
        let granted = stream.buffer_size();
        if hold > 0.0 {
            println!("\n(holding requested {req:?} open for {hold} s)");
            std::thread::sleep(Duration::from_secs_f64(hold));
        }
        drop(stream);

        let r = recs.lock().unwrap();
        let r = &r[r.len().min(10)..];
        if r.len() < 3 {
            println!("\nrequested {req:?}: only {} callbacks", r.len());
            return;
        }
        let mut frames: Vec<u32> = r.iter().map(|x| x.frames).collect();
        let gaps: Vec<f64> = r.windows(2).map(|w| (w[1].now_ns - w[0].now_ns) as f64 / 1e6).collect();
        let ahead: Vec<f64> = r.iter().map(|x| x.ahead_ns as f64 / 1e6).collect();
        let cbn: Vec<f64> = r.iter().map(|x| x.cb_minus_now_ns as f64 / 1e6).collect();
        let mn = |v: &[f64]| v.iter().cloned().fold(f64::MAX, f64::min);
        let mx = |v: &[f64]| v.iter().cloned().fold(f64::MIN, f64::max);
        let mean = |v: &[f64]| v.iter().sum::<f64>() / v.len() as f64;
        frames.sort_unstable();
        let mode = {
            let mut best = (0, 0);
            let mut i = 0;
            while i < frames.len() {
                let j = frames[i..].iter().take_while(|&&f| f == frames[i]).count();
                if j > best.1 {
                    best = (frames[i], j);
                }
                i += j;
            }
            best.0
        };
        let period = mode as f64 / rate as f64 * 1e3;
        println!(
            "\nrequested {req:?} @ {rate} Hz: stream.buffer_size() = {granted:?}; {} callbacks; frames/callback min {} max {} mode {} ({period:.3} ms)",
            r.len(),
            frames[0],
            frames[frames.len() - 1],
            mode
        );
        println!(
            "  callback gap ms: min {:.3} mean {:.3} max {:.3}; playback lead (playback - callback) ms: min {:.3} max {:.3}",
            mn(&gaps),
            mean(&gaps),
            mx(&gaps),
            mn(&ahead),
            mx(&ahead)
        );
        println!(
            "  cpal callback timestamp - now (ms): min {:+.3} mean {:+.3} max {:+.3}",
            mn(&cbn),
            mean(&cbn),
            mx(&cbn)
        );
        println!("  audio thread {t}: {}", sched.unwrap_or_else(|| "?".into()));
        match tc {
            Some((p, c, k, pre)) => println!(
                "  time-constraint policy: period {p:.3} ms, computation {c:.3} ms, constraint {k:.3} ms, preemptible {pre}"
            ),
            None => println!("  time-constraint policy: none"),
        }
        if let (Some(a), Some(b)) = (c0, c1) {
            let d = b.since(&a);
            let n = (secs * rate as f64 / mode.max(1) as f64).max(1.0);
            println!(
                "  over {secs} s: P-core share {:.3}, P {:.1} ms / E {:.1} ms CPU, {:.1} k instr/callback, {:.2} uJ/callback",
                d.p_share().unwrap_or(f64::NAN),
                d.p_ns.unwrap_or(0) as f64 / 1e6,
                d.e_ns.unwrap_or(0) as f64 / 1e6,
                d.instructions.unwrap_or(0) as f64 / n / 1e3,
                d.energy_nj.unwrap_or(0) as f64 / n / 1e3,
            );
        }
        let e = errors.lock().unwrap();
        if !e.is_empty() {
            println!("  stream errors: {:?}", *e);
        }
    }
}
