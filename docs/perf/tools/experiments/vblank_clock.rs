//! Windows vblank clock for the standalone host.
//!
//! Sleeps (high-resolution waitable timer, no spinning) until just after the
//! next DWM vblank and then tells the UI to produce a frame. Used with the swap
//! interval set to 0, so `SwapBuffers` returns immediately instead of blocking
//! in the GL driver's vsync wait, which on NVIDIA costs measurable CPU.
//! DWM composes the frame at the following vblank, so presentation stays
//! tear-free and evenly paced.

#![cfg(windows)]

use std::sync::atomic::Ordering;
use std::time::Duration;

use vizia::prelude::*;

/// How long after a vblank to wake: leaves the UI a full frame to render and
/// present before DWM's next composition.
const WAKE_AFTER_VBLANK: Duration = Duration::from_micros(1000);

#[repr(C, packed)]
#[derive(Default, Clone, Copy)]
struct UnsignedRatio {
    num: u32,
    den: u32,
}

/// `DWM_TIMING_INFO` (packed to 1 byte in the Windows headers).
#[repr(C, packed)]
#[derive(Clone, Copy)]
struct DwmTimingInfo {
    cb_size: u32,
    rate_refresh: UnsignedRatio,
    qpc_refresh_period: u64,
    rate_compose: UnsignedRatio,
    qpc_vblank: u64,
    /// Remaining fields, unused here (292-byte struct in total).
    rest: [u8; 256],
}

const _: () = assert!(std::mem::size_of::<DwmTimingInfo>() == 292);

#[link(name = "dwmapi")]
unsafe extern "system" {
    fn DwmGetCompositionTimingInfo(hwnd: isize, info: *mut DwmTimingInfo) -> i32;
}

#[link(name = "kernel32")]
unsafe extern "system" {
    fn QueryPerformanceCounter(count: *mut i64) -> i32;
    fn QueryPerformanceFrequency(freq: *mut i64) -> i32;
    fn CreateWaitableTimerExW(attrs: *const u8, name: *const u16, flags: u32, access: u32)
        -> isize;
    fn SetWaitableTimer(
        timer: isize,
        due: *const i64,
        period: i32,
        routine: *const u8,
        arg: *const u8,
        resume: i32,
    ) -> i32;
    fn WaitForSingleObject(handle: isize, ms: u32) -> u32;
}

const CREATE_WAITABLE_TIMER_HIGH_RESOLUTION: u32 = 0x2;
const TIMER_ALL_ACCESS: u32 = 0x1F0003;

fn timing() -> Option<(i64, i64)> {
    // SAFETY: plain FFI with a correctly sized, zeroed out-struct.
    unsafe {
        let mut info: DwmTimingInfo = std::mem::zeroed();
        info.cb_size = std::mem::size_of::<DwmTimingInfo>() as u32;
        if DwmGetCompositionTimingInfo(0, &mut info) != 0 {
            return None;
        }
        let (vblank, period) = (info.qpc_vblank as i64, info.qpc_refresh_period as i64);
        (period > 0).then_some((vblank, period))
    }
}

/// Starts the vblank thread. Returns false (and the UI keeps its own pump and
/// vsync) if DWM timing is unavailable.
pub fn start(proxy: ContextProxy) -> bool {
    if timing().is_none() {
        return false;
    }
    std::thread::Builder::new()
        .name("vblank".into())
        .spawn(move || run(proxy))
        .is_ok()
}

fn run(mut proxy: ContextProxy) {
    let mut freq = 0i64;
    // SAFETY: plain FFI.
    let timer = unsafe {
        QueryPerformanceFrequency(&mut freq);
        CreateWaitableTimerExW(
            std::ptr::null(),
            std::ptr::null(),
            CREATE_WAITABLE_TIMER_HIGH_RESOLUTION,
            TIMER_ALL_ACCESS,
        )
    };
    if timer == 0 || freq <= 0 {
        return;
    }
    let wake_qpc = (WAKE_AFTER_VBLANK.as_secs_f64() * freq as f64) as i64;
    let mut last_target = 0i64;
    loop {
        if !ui::WANTS_FRAMES.load(Ordering::Relaxed) {
            std::thread::sleep(Duration::from_millis(10));
            continue;
        }
        let Some((vblank, period)) = timing() else {
            std::thread::sleep(Duration::from_millis(16));
            continue;
        };
        let mut now = 0i64;
        // SAFETY: plain FFI.
        unsafe { QueryPerformanceCounter(&mut now) };
        // Next wake-up: first vblank (+offset) after now and after the last one we used.
        let mut target = vblank + wake_qpc;
        while target <= now || target <= last_target {
            target += period;
        }
        // After an idle stretch the gap spans many refreshes; animate one frame's worth.
        let interval = if last_target == 0 || target - last_target > 2 * period {
            period
        } else {
            target - last_target
        };
        last_target = target;
        // Relative due time in 100 ns units (negative = relative).
        let due = -(((target - now) as f64 / freq as f64) * 1e7).max(1.0) as i64;
        // SAFETY: valid timer handle and due-time pointer.
        unsafe {
            SetWaitableTimer(timer, &due, 0, std::ptr::null(), std::ptr::null(), 0);
            WaitForSingleObject(timer, 100);
        }
        let dt = Duration::from_secs_f64(interval as f64 / freq as f64);
        if proxy.emit(ui::AppEvent::VBlank(dt)).is_err() {
            return;
        }
    }
}
