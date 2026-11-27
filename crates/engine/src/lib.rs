//! Real-time host for the chip engine.
//!
//! Threads and data ownership (see ENGINE.md for the diagram):
//! - **UI thread** owns `Engine`: the command producer and the garbage
//!   consumer. It builds song snapshots, sends them, and frees the old ones.
//! - **Audio thread** (cpal's) runs `Core::process`: it owns the chip
//!   `Player` and the current snapshot; it publishes to `Shared` with atomics.
//! - **Host thread** owns the cpal `Stream` and rebuilds it on device errors.

pub mod alloc_check;
pub mod command;
pub mod env;
pub mod core;
pub mod cpu;
mod host;
pub mod mixdown;
pub mod shared;
pub mod song;
pub mod studio;
pub mod testkit;

pub use chip;
pub use chip::driver::{ParamId, Params, LANES};
pub use chip::filter::FilterMode;
pub use command::Command;
pub use host::EngineConfig;
pub use shared::{device_error, PositionSnapshot, Shared};
pub use song::{ClipInput, DpcmBank, NoteInput, SongSnapshot, StudioLane};
pub use mixdown::Mixdown;
pub use studio::{Rack, StudioTrack, TrackCtl};

use host::CoreCell;
use rtrb::{Consumer, Producer, RingBuffer};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;
use std::time::Instant;

// Tests run every `Core::process` call under the allocation checker: any
// allocation or free inside it aborts the test binary.
#[cfg(test)]
#[global_allocator]
static TEST_ALLOC: alloc_check::CheckingAllocator = alloc_check::CheckingAllocator;

const COMMAND_QUEUE: usize = 1024;
const GARBAGE_QUEUE: usize = 64;

pub struct Engine {
    tx: Producer<Command>,
    reconfig: Arc<host::Reconfig>,
    cfg: EngineConfig,
    garbage: Consumer<core::Garbage>,
    shared: Arc<Shared>,
    epoch: Instant,
    stop: Arc<AtomicBool>,
    host: Option<JoinHandle<()>>,
    info: Arc<Mutex<String>>,
    sent: u64,
    dropped: u64,
    sample_rate: u32,
}

impl Engine {
    pub fn start(cfg: EngineConfig) -> Self {
        let (tx, rx) = RingBuffer::new(COMMAND_QUEUE);
        let (gtx, grx) = RingBuffer::new(GARBAGE_QUEUE);
        let shared = Arc::new(Shared::default());
        let epoch = Instant::now();
        let core = core::Core::new(cfg.sample_rate, rx, gtx, shared.clone(), epoch);
        let cell = Arc::new(CoreCell::new(core));
        let stop = Arc::new(AtomicBool::new(false));
        let info = Arc::new(Mutex::new(String::from("starting")));
        let reconfig = Arc::new(host::Reconfig {
            cfg: Mutex::new(None),
            err: Arc::new(std::sync::atomic::AtomicU32::new(0)),
        });
        let sample_rate = cfg.sample_rate;
        let host = {
            let (shared, stop, info, reconfig, cfg) = (shared.clone(), stop.clone(), info.clone(), reconfig.clone(), cfg.clone());
            std::thread::Builder::new()
                .name("cartwave-audio-host".into())
                .spawn(move || host::run(cell, shared, cfg, stop, info, reconfig))
                .expect("spawn audio host thread")
        };
        Self {
            tx,
            reconfig,
            cfg,
            garbage: grx,
            shared,
            epoch,
            stop,
            host: Some(host),
            info,
            sent: 0,
            dropped: 0,
            sample_rate,
        }
    }

    /// Switches the output device, buffer size or sample rate while running
    /// (Settings > Audio). The host drops the stream and rebuilds it through
    /// the same path as a device change; the song, transport and chip state
    /// carry over. Returns false when nothing changed.
    pub fn reconfigure(&mut self, cfg: EngineConfig) -> bool {
        if cfg == self.cfg {
            return false;
        }
        self.sample_rate = cfg.sample_rate;
        self.cfg = cfg.clone();
        if let Ok(mut c) = self.reconfig.cfg.lock() {
            *c = Some(cfg);
        }
        self.reconfig.err.store(host::ERR_RECONFIG, Ordering::Release);
        if let Some(h) = &self.host {
            h.thread().unpark();
        }
        true
    }

    /// The configuration last asked for.
    pub fn config(&self) -> &EngineConfig {
        &self.cfg
    }

    /// The names of the output devices (Settings > Audio).
    pub fn output_devices() -> Vec<String> {
        host::output_devices()
    }

    pub fn now_ns(&self) -> u64 {
        self.epoch.elapsed().as_nanos() as u64
    }

    /// Queues a command. Returns false (and drops it here, on the UI thread)
    /// if the queue is full.
    pub fn send(&mut self, cmd: Command) -> bool {
        match self.tx.push(cmd) {
            Ok(()) => {
                self.sent += 1;
                true
            }
            Err(_) => {
                self.dropped += 1;
                false
            }
        }
    }

    /// Sends a new song snapshot (boxed here, on the UI thread).
    pub fn send_song(&mut self, mut song: SongSnapshot) -> bool {
        song.sent_ns = self.now_ns();
        self.send(Command::Song(Box::new(song)))
    }

    /// Frees snapshots the audio thread has finished with. Call regularly.
    pub fn collect_garbage(&mut self) -> usize {
        let mut n = 0;
        while let Ok(s) = self.garbage.pop() {
            drop(s);
            n += 1;
        }
        n
    }

    pub fn shared(&self) -> &Shared {
        &self.shared
    }

    /// Commands sent but not yet applied by the audio thread.
    pub fn pending(&self) -> u64 {
        self.sent
            .saturating_sub(self.shared.commands.load(Ordering::Acquire))
    }

    pub fn dropped_commands(&self) -> u64 {
        self.dropped
    }

    /// Device description of the running stream (or the last error).
    pub fn device_info(&self) -> String {
        self.info.lock().map(|s| s.clone()).unwrap_or_default()
    }

    pub fn sample_rate(&self) -> u32 {
        self.sample_rate
    }

    /// The song position being heard right now; see `audible_position_at`.
    pub fn audible_position(&self) -> Option<(f64, bool)> {
        self.audible_position_at(Instant::now())
    }

    /// The song position (ticks) that will be heard at `at`, compensated for
    /// the output latency the driver reports, plus whether it is playing.
    /// `None` before the first callback or while commands are in flight
    /// (the caller should keep showing what it requested). The UI asks for
    /// the time its next frame will be presented, not "now".
    pub fn audible_position_at(&self, at: Instant) -> Option<(f64, bool)> {
        if self.pending() > 0 {
            return None;
        }
        let p = self.shared.position()?;
        if !p.playing {
            return Some((p.ticks, false));
        }
        let now = at.saturating_duration_since(self.epoch).as_nanos() as f64;
        let sr = p.sample_rate.max(1) as f64;
        // Frames since the first frame of the last buffer reached the speaker.
        let mut frames = (now - p.callback_ns as f64 - p.latency_ns as f64) * sr / 1e9;
        // Never extrapolate past what has been rendered (audio stalled).
        let rendered = self.shared.cb_frames_last.load(Ordering::Relaxed) as f64;
        frames = frames.min(rendered);
        let t = p.ticks + frames * p.ticks_per_frame;
        Some((chip::player::wrap_loop(t, p.loop_start, p.loop_end), true))
    }
}

impl Drop for Engine {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Release);
        if let Some(h) = self.host.take() {
            h.thread().unpark();
            let _ = h.join();
        }
        self.collect_garbage();
    }
}
