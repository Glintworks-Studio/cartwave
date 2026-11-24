//! The host thread: owns the cpal stream, rebuilds it when the device goes
//! away or the default output changes, and hands the engine core from one
//! stream to the next so the song, transport and chip state survive.
//!
//! The core lives in a `CoreCell`. The audio callback takes it with an atomic
//! swap (leaving null), uses it, and puts it back: wait-free, no lock. The
//! host only takes it out after dropping the stream, which joins the audio
//! thread (cpal's WASAPI `Stream::drop`), so they never overlap.

use crate::core::Core;
use crate::shared::{device_error, Shared};
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use std::ptr;
use std::sync::atomic::{AtomicBool, AtomicPtr, AtomicU32, Ordering};
use std::sync::Arc;
use std::thread::{self, Thread};
use std::time::Duration;

pub(crate) struct CoreCell(AtomicPtr<Core>);

impl CoreCell {
    pub fn new(core: Box<Core>) -> Self {
        Self(AtomicPtr::new(Box::into_raw(core)))
    }

    /// Host only, with no stream running.
    fn with<R>(&self, f: impl FnOnce(&mut Core) -> R) -> Option<R> {
        let p = self.0.swap(ptr::null_mut(), Ordering::Acquire);
        if p.is_null() {
            return None;
        }
        // SAFETY: `p` came from `Box::into_raw`, and the swap above made this
        // the only holder until it is stored back.
        let r = f(unsafe { &mut *p });
        self.0.store(p, Ordering::Release);
        Some(r)
    }
}

impl Drop for CoreCell {
    fn drop(&mut self) {
        let p = self.0.swap(ptr::null_mut(), Ordering::Acquire);
        if !p.is_null() {
            // SAFETY: as above; no stream exists any more (the host dropped it).
            drop(unsafe { Box::from_raw(p) });
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub struct EngineConfig {
    pub sample_rate: u32,
    /// Requested buffer size in frames (`None` = device default). WASAPI
    /// shared mode treats it as a hint; see ENGINE.md.
    pub buffer_frames: Option<u32>,
    /// Use this output device name instead of following the default.
    pub device_name: Option<&'static str>,
}

impl Default for EngineConfig {
    fn default() -> Self {
        Self {
            sample_rate: 48_000,
            buffer_frames: None,
            device_name: None,
        }
    }
}

/// Error from the stream's error callback (1 = device gone, 2 = invalidated).
const ERR_NONE: u32 = 0;
const ERR_DEVICE_GONE: u32 = 1;
const ERR_INVALIDATED: u32 = 2;

pub(crate) fn run(
    cell: Arc<CoreCell>,
    shared: Arc<Shared>,
    cfg: EngineConfig,
    stop: Arc<AtomicBool>,
    info: Arc<std::sync::Mutex<String>>,
) {
    let me = thread::current();
    let err = Arc::new(AtomicU32::new(ERR_NONE));
    while !stop.load(Ordering::Acquire) {
        match build(&cell, &shared, &cfg, &err, &me) {
            Ok((stream, desc)) => {
                if let Ok(mut i) = info.lock() {
                    *i = desc;
                }
                shared.device_error.store(device_error::NONE, Ordering::Relaxed);
                shared.stream_running.store(true, Ordering::Release);
                shared.generation.fetch_add(1, Ordering::Relaxed);
                let kind = loop {
                    thread::park();
                    if stop.load(Ordering::Acquire) {
                        break ERR_NONE;
                    }
                    let e = err.swap(ERR_NONE, Ordering::AcqRel);
                    if e != ERR_NONE {
                        break e;
                    }
                };
                // Joins the audio thread: the core is ours again afterwards.
                drop(stream);
                shared.stream_running.store(false, Ordering::Release);
                if kind != ERR_NONE {
                    shared.device_error.store(
                        if kind == ERR_DEVICE_GONE {
                            device_error::NO_DEVICE
                        } else {
                            device_error::REBUILDING
                        },
                        Ordering::Relaxed,
                    );
                    // Let Windows settle the new default before rebuilding.
                    thread::park_timeout(Duration::from_millis(250));
                }
            }
            Err(e) => {
                if let Ok(mut i) = info.lock() {
                    *i = format!("no output: {e}");
                }
                shared.device_error.store(
                    if e.contains("no output device") {
                        device_error::NO_DEVICE
                    } else {
                        device_error::BUILD_FAILED
                    },
                    Ordering::Relaxed,
                );
                thread::park_timeout(Duration::from_secs(1));
            }
        }
    }
}

fn build(
    cell: &Arc<CoreCell>,
    shared: &Arc<Shared>,
    cfg: &EngineConfig,
    err: &Arc<AtomicU32>,
    host_thread: &Thread,
) -> Result<(cpal::Stream, String), String> {
    let host = cpal::default_host();
    let dev = match cfg.device_name {
        Some(name) => host
            .output_devices()
            .map_err(|e| e.to_string())?
            .find(|d| d.description().map(|x| x.name() == name).unwrap_or(false))
            .ok_or_else(|| format!("no output device named {name}"))?,
        None => host
            .default_output_device()
            .ok_or_else(|| "no output device".to_string())?,
    };
    let name = dev
        .description()
        .map(|d| d.name().to_string())
        .unwrap_or_else(|_| "?".into());
    let def = dev.default_output_config().map_err(|e| e.to_string())?;
    let channels = def.channels();
    let config = cpal::StreamConfig {
        channels,
        sample_rate: cfg.sample_rate,
        buffer_size: match cfg.buffer_frames {
            Some(n) => cpal::BufferSize::Fixed(n),
            None => cpal::BufferSize::Default,
        },
    };
    let sr = cell.with(|c| {
        c.stream_started();
        c.sample_rate()
    });
    if sr != Some(cfg.sample_rate) {
        return Err("engine core missing or sample rate mismatch".into());
    }
    let cb_cell = cell.clone();
    let cb_shared = shared.clone();
    let ch = channels as usize;
    let err_flag = err.clone();
    let wake = host_thread.clone();
    let err_shared = shared.clone();
    let stream = dev
        .build_output_stream(
            config,
            move |data: &mut [f32], info: &cpal::OutputCallbackInfo| {
                let ts = info.timestamp();
                let ahead = ts.playback.duration_since(ts.callback).as_nanos() as u64;
                let p = cb_cell.0.swap(ptr::null_mut(), Ordering::Acquire);
                if p.is_null() {
                    data.fill(0.0);
                    cb_shared.silent_callbacks.fetch_add(1, Ordering::Relaxed);
                    return;
                }
                // SAFETY: the swap gave this callback exclusive use until the store.
                unsafe { (*p).process(data, ch, ahead) };
                cb_cell.0.store(p, Ordering::Release);
            },
            move |e: cpal::Error| {
                let k = match e.kind() {
                    cpal::ErrorKind::DeviceNotAvailable => ERR_DEVICE_GONE,
                    // CoreAudio reroutes a default-device stream by itself and
                    // reports this; the stream keeps running (WASAPI can't
                    // rebind and reports StreamInvalidated instead).
                    cpal::ErrorKind::DeviceChanged => return,
                    // An overload (CoreAudio's kAudioDeviceProcessorOverload):
                    // one late cycle, already over. The stream is fine; count it.
                    // Rebuilding here turned every overload into a ~100 ms+
                    // gap (ENGINE-macos.md, bug 1).
                    cpal::ErrorKind::Xrun => {
                        err_shared.overloads.fetch_add(1, Ordering::Relaxed);
                        return;
                    }
                    _ => ERR_INVALIDATED,
                };
                err_flag.store(k, Ordering::Release);
                wake.unpark();
            },
            None,
        )
        .map_err(|e| e.to_string())?;
    stream.play().map_err(|e| e.to_string())?;
    let desc = format!(
        "{name} · {} Hz · {} ch · requested {}",
        cfg.sample_rate,
        channels,
        match cfg.buffer_frames {
            Some(n) => format!("{n} frames"),
            None => "default".into(),
        }
    );
    Ok((stream, desc))
}
