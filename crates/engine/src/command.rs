//! UI -> engine commands, carried by a lock-free SPSC queue.

use crate::song::SongSnapshot;
use crate::studio::Rack;
use chip::driver::ParamId;
use chip::filter::FilterMode;

pub enum Command {
    /// Replace the song. The old snapshot is returned to the UI thread to be
    /// freed there.
    Song(Box<SongSnapshot>),
    Play,
    /// Stop in place.
    Stop,
    Seek(f64),
    Bpm(f64),
    LoopEnd(f64),
    /// Loop start (ticks); playback wraps from the loop end to here.
    LoopStart(f64),
    /// Audition: play `pitch` (with an optional volume) on `lane` while the
    /// transport is stopped, until the next `Preview` for that lane; `None`
    /// releases it. Ignored while playing; `Play` clears it.
    Preview { lane: u8, note: Option<(u8, Option<u8>)> },
    /// Bit per lane.
    Mute(u8),
    Solo(u8),
    Param { lane: u8, id: ParamId, value: i8 },
    Vrc6(bool),
    Filter(FilterMode),
    /// Output gain (linear).
    Gain(f32),
    /// Per-track gains (linear, lane order): the track volume faders.
    LaneGains([f32; 8]),
    /// Replace the studio rack (a track added, removed, reordered, or a pool
    /// grown). The sounding voices carry over; the old rack goes back to the
    /// UI thread to be freed.
    Rack(Box<Rack>),
    /// The Main track's gain (linear), after the whole mix.
    MasterGain(f32),
    /// Test only: busy-wait this many microseconds inside the next callback,
    /// to validate the underrun detectors.
    TestStall(u32),
    /// Test only: mix a quiet pilot tone (Hz, 0 = off) for loopback glitch
    /// detection.
    TestPilot(f32),
    /// Test only: a short full-scale click at the next frame that crosses this
    /// song position (ticks), for A/V sync measurement. Negative = off.
    TestClickAt(f64),
}
