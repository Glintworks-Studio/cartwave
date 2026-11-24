//! Plain data model for the prototype. No audio; everything is mock data.

use crate::theme::tokens::{self, Rgba};

/// Ticks per quarter note. All timing and snapping is done in ticks.
pub const PPQ: u32 = 960;
/// 4/4 time.
pub const BEATS_PER_BAR: u32 = 4;
pub const TICKS_PER_BAR: u32 = PPQ * BEATS_PER_BAR;
/// Minimum clip length when resizing.
pub const MIN_CLIP_TICKS: u32 = PPQ / 4;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct MockNote {
    pub start: u32,
    pub len: u32,
    pub pitch: u8,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Clip {
    pub id: u32,
    pub lane: usize,
    pub start_ticks: u32,
    pub len_ticks: u32,
    pub name: String,
    /// Note times are relative to the clip start.
    pub notes: Vec<MockNote>,
}

impl Clip {
    pub fn end_ticks(&self) -> u32 {
        self.start_ticks + self.len_ticks
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Lane {
    pub name: &'static str,
    pub short: &'static str,
    pub color: Rgba,
}

pub fn default_lanes() -> Vec<Lane> {
    vec![
        Lane {
            name: "Pulse 1",
            short: "P1",
            color: tokens::LANE_PULSE,
        },
        Lane {
            name: "Pulse 2",
            short: "P2",
            color: tokens::LANE_PULSE,
        },
        Lane {
            name: "Triangle",
            short: "TRI",
            color: tokens::LANE_TRI,
        },
        Lane {
            name: "Noise",
            short: "NOI",
            color: tokens::LANE_NOISE,
        },
        Lane {
            name: "DPCM",
            short: "DMC",
            color: tokens::LANE_DPCM,
        },
        Lane {
            name: "VRC6 Pulse 1",
            short: "V1",
            color: tokens::LANE_VRC6_PULSE,
        },
        Lane {
            name: "VRC6 Pulse 2",
            short: "V2",
            color: tokens::LANE_VRC6_PULSE,
        },
        Lane {
            name: "VRC6 Saw",
            short: "SAW",
            color: tokens::LANE_VRC6_SAW,
        },
    ]
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Snap {
    Sixteenth,
    Quarter,
    Bar,
}

impl Snap {
    pub fn ticks(self) -> u32 {
        match self {
            Snap::Sixteenth => PPQ / 4,
            Snap::Quarter => PPQ,
            Snap::Bar => TICKS_PER_BAR,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Snap::Sixteenth => "1/16",
            Snap::Quarter => "1/4",
            Snap::Bar => "Bar",
        }
    }

    pub fn next(self) -> Self {
        match self {
            Snap::Sixteenth => Snap::Quarter,
            Snap::Quarter => Snap::Bar,
            Snap::Bar => Snap::Sixteenth,
        }
    }

    /// Snap a tick position to the nearest grid line.
    pub fn round(self, ticks: i64) -> i64 {
        let g = self.ticks() as i64;
        (ticks + g / 2).div_euclid(g) * g
    }
}

/// Horizontal view mapping. The ONLY place pixel <-> tick conversion happens.
/// Pixels here are logical (unscaled) and relative to the arrangement's left edge.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Viewport {
    /// Logical pixels per tick (zoom).
    pub px_per_tick: f64,
    /// Tick at the left edge of the arrangement.
    pub scroll_ticks: f64,
}

impl Default for Viewport {
    fn default() -> Self {
        // One bar = 92 logical px by default.
        Self {
            px_per_tick: 92.0 / TICKS_PER_BAR as f64,
            scroll_ticks: 0.0,
        }
    }
}

impl Viewport {
    pub const MIN_PX_PER_BAR: f64 = 24.0;
    pub const MAX_PX_PER_BAR: f64 = 640.0;

    pub fn tick_to_x(&self, ticks: f64) -> f64 {
        (ticks - self.scroll_ticks) * self.px_per_tick
    }

    pub fn x_to_tick(&self, x: f64) -> f64 {
        x / self.px_per_tick + self.scroll_ticks
    }

    pub fn px_to_ticks(&self, dx: f64) -> f64 {
        dx / self.px_per_tick
    }

    pub fn px_per_bar(&self) -> f64 {
        self.px_per_tick * TICKS_PER_BAR as f64
    }

    /// Zoom around a logical x anchor so the tick under the cursor stays put.
    pub fn zoom_at(&mut self, anchor_x: f64, factor: f64) {
        let anchor_tick = self.x_to_tick(anchor_x);
        let ppb = (self.px_per_bar() * factor).clamp(Self::MIN_PX_PER_BAR, Self::MAX_PX_PER_BAR);
        self.px_per_tick = ppb / TICKS_PER_BAR as f64;
        self.scroll_ticks = (anchor_tick - anchor_x / self.px_per_tick).max(0.0);
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Arrangement {
    pub lanes: Vec<Lane>,
    pub clips: Vec<Clip>,
    /// Selected clip ids.
    pub selection: Vec<u32>,
    pub snap: Snap,
    pub view: Viewport,
    pub next_id: u32,
}

impl Arrangement {
    pub fn is_selected(&self, id: u32) -> bool {
        self.selection.contains(&id)
    }

    pub fn clip(&self, id: u32) -> Option<&Clip> {
        self.clips.iter().find(|c| c.id == id)
    }

    pub fn clip_mut(&mut self, id: u32) -> Option<&mut Clip> {
        self.clips.iter_mut().find(|c| c.id == id)
    }

    pub fn delete_selected(&mut self) {
        let sel = std::mem::take(&mut self.selection);
        self.clips.retain(|c| !sel.contains(&c.id));
    }

    /// Brings the given clip ids to the end of the list (drawn on top).
    pub fn raise(&mut self, ids: &[u32]) {
        let (mut top, rest): (Vec<Clip>, Vec<Clip>) =
            self.clips.drain(..).partition(|c| ids.contains(&c.id));
        self.clips = rest;
        self.clips.append(&mut top);
    }

    pub fn first_selected(&self) -> Option<&Clip> {
        self.selection.first().and_then(|id| self.clip(*id))
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Transport {
    pub playing: bool,
    pub record_armed: bool,
    pub bpm: f32,
    pub playhead_ticks: f64,
    /// Loop end in ticks; playhead wraps here.
    pub loop_end_ticks: f64,
}

impl Transport {
    pub const MIN_BPM: f32 = 40.0;
    pub const MAX_BPM: f32 = 300.0;

    /// Bars.Beats.Sixteenths, 1-based, like "001.1.1".
    pub fn position_string(ticks: f64) -> String {
        let t = ticks.max(0.0) as u32;
        let bar = t / TICKS_PER_BAR + 1;
        let beat = (t % TICKS_PER_BAR) / PPQ + 1;
        let six = (t % PPQ) / (PPQ / 4) + 1;
        format!("{bar:03}.{beat}.{six}")
    }
}

/// Per-channel mock meter levels, written by the 60fps tick.
#[derive(Clone, Debug, PartialEq, Default)]
pub struct Meters {
    pub level: [f32; 8],
    pub peak: [f32; 8],
    pub peak_hold: [f32; 8],
    /// 0..1 brightness of the channel light (pulses on note-on).
    pub light: [f32; 8],
}

pub const DIAL_COUNT: usize = 6;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct DialSpec {
    pub label: &'static str,
    pub min: f32,
    pub max: f32,
    pub default: f32,
    /// Discrete steps (0 = continuous).
    pub steps: u32,
    pub unit: &'static str,
}

pub const DIALS: [DialSpec; DIAL_COUNT] = [
    DialSpec {
        label: "Duty",
        min: 0.0,
        max: 3.0,
        default: 2.0,
        steps: 3,
        unit: "",
    },
    DialSpec {
        label: "Volume",
        min: 0.0,
        max: 15.0,
        default: 12.0,
        steps: 15,
        unit: "",
    },
    DialSpec {
        label: "Env Attack",
        min: 0.0,
        max: 1000.0,
        default: 8.0,
        steps: 0,
        unit: "ms",
    },
    DialSpec {
        label: "Env Decay",
        min: 0.0,
        max: 2000.0,
        default: 420.0,
        steps: 0,
        unit: "ms",
    },
    DialSpec {
        label: "Sweep",
        min: -7.0,
        max: 7.0,
        default: 0.0,
        steps: 14,
        unit: "",
    },
    DialSpec {
        label: "Detune",
        min: -50.0,
        max: 50.0,
        default: 0.0,
        steps: 0,
        unit: "ct",
    },
];

impl DialSpec {
    pub fn format(&self, v: f32) -> String {
        match self.label {
            "Duty" => {
                ["12.5%", "25%", "50%", "75%"][v.round().clamp(0.0, 3.0) as usize].to_string()
            }
            _ if self.steps > 0 => format!("{}{}", v.round() as i32, self.unit),
            _ if self.min < 0.0 => format!("{:+.1}{}", v, self.unit),
            _ => format!("{:.0}{}", v, self.unit),
        }
    }

    pub fn to_norm(&self, v: f32) -> f32 {
        ((v - self.min) / (self.max - self.min)).clamp(0.0, 1.0)
    }

    pub fn from_norm(&self, n: f32) -> f32 {
        let v = self.min + n.clamp(0.0, 1.0) * (self.max - self.min);
        if self.steps > 0 {
            let step = (self.max - self.min) / self.steps as f32;
            self.min + ((v - self.min) / step).round() * step
        } else {
            v
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SoundChip {
    Base2A03,
    WithVrc6,
}

impl SoundChip {
    pub fn label(self) -> &'static str {
        match self {
            SoundChip::Base2A03 => "2A03",
            SoundChip::WithVrc6 => "2A03 + VRC6",
        }
    }
}

/// Debug overlay stats (F3).
#[derive(Clone, Copy, Debug, PartialEq, Default)]
pub struct FrameStats {
    pub visible: bool,
    /// Smoothed interval between presented frames, ms.
    pub frame_ms: f32,
    /// Worst presented-frame interval in the last second, ms.
    pub worst_ms: f32,
    /// Smoothed per-phase UI-thread time, ms, in `PHASE_NAMES` order.
    pub phases_ms: [f32; 7],
    /// Smoothed per-view custom draw time, ms, in `perf::SPANS` order.
    pub spans_ms: [f32; 9],
    /// Process CPU, % of one core (all threads, incl. the GL driver's).
    pub cpu_percent: f32,
    pub clip_count: usize,
    pub frames: u64,
}

pub const PHASE_NAMES: [&str; 7] = [
    "events",
    "style",
    "layout",
    "mk-current",
    "draw",
    "flush",
    "present",
];
