//! The arrangement model: lanes, clips (with their notes), selection and view.
//! Notes and their editing operations live in `notes.rs`.

use crate::instruments::{default_instruments, LaneInstrument};
use crate::notes::{self, Note};
use crate::theme::tokens::{self, Rgba};
use engine::ParamId;
use std::hash::{Hash, Hasher};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

/// Ticks per quarter note. All timing and snapping is done in ticks.
pub const PPQ: u32 = 960;
/// 4/4 time.
pub const BEATS_PER_BAR: u32 = 4;
pub const TICKS_PER_BAR: u32 = PPQ * BEATS_PER_BAR;
/// The longest song: 9999 bars. Files and imports are checked against it, so
/// tick arithmetic (start + length, ids + 1) can never overflow.
pub const MAX_SONG_BARS: u32 = 9999;
pub const MAX_SONG_TICKS: u32 = MAX_SONG_BARS * TICKS_PER_BAR;
/// Largest clip / note id a file may use (leaves room to count up).
pub const MAX_ID: u32 = 1 << 30;
/// Minimum clip length when resizing.
pub const MIN_CLIP_TICKS: u32 = PPQ / 4;

/// A clip's revision stamp: a process-wide counter, set fresh every time the
/// clip is changed (`Arrangement::clip_mut`, `Clip::new`, splits). `(id, rev)`
/// identifies a clip's content without hashing its notes, and is never reused
/// (unlike an `Arc` address). Not part of the content: equal clips compare
/// equal whatever their stamps.
#[derive(Clone, Copy, Debug)]
pub struct Rev(pub u64);

impl Rev {
    pub fn next() -> Self {
        static NEXT: AtomicU64 = AtomicU64::new(1);
        Rev(NEXT.fetch_add(1, Ordering::Relaxed))
    }
}

impl Default for Rev {
    fn default() -> Self {
        Self::next()
    }
}

impl PartialEq for Rev {
    fn eq(&self, _: &Self) -> bool {
        true
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Clip {
    pub id: u32,
    pub lane: usize,
    pub start_ticks: u32,
    pub len_ticks: u32,
    pub name: String,
    /// Sorted by start, never overlapping (the monophonic rule, `notes.rs`).
    /// Times are relative to the clip start. Notes past the clip end are kept
    /// (a clip can be lengthened again) but not played or drawn.
    pub notes: Vec<Note>,
    /// Next note id in this clip (ids are per clip).
    pub next_note_id: u32,
    /// The clip's own instrument (an id in the project's table); `None`:
    /// its lane's default. Copies of a clip share it (by reference).
    pub instrument: Option<crate::instruments::InstId>,
    /// The clip's own colour (A18 Colour ▸); `None`: its track's.
    pub color: Option<Rgba>,
    pub rev: Rev,
}

impl Clip {
    pub fn new(id: u32, lane: usize, start_ticks: u32, len_ticks: u32, name: String, notes: Vec<Note>) -> Self {
        let next_note_id = notes.iter().map(|n| n.id.saturating_add(1)).max().unwrap_or(1);
        Self { id, lane, start_ticks, len_ticks, name, notes, next_note_id, instrument: None, color: None, rev: Rev::next() }
    }

    pub fn end_ticks(&self) -> u32 {
        self.start_ticks.saturating_add(self.len_ticks)
    }

    /// The notes that sound: those starting inside the clip (cut at its end
    /// by the snapshot builder).
    pub fn audible_notes(&self) -> impl Iterator<Item = &Note> {
        let len = self.len_ticks;
        self.notes.iter().filter(move |n| n.start < len)
    }
}

/// The channel controls a track keeps (the driver's lane row: Duty,
/// Volume, Envelope… whichever its voice type has).
pub type TrackParams = [i8; chip::driver::ParamId::ALL.len()];

/// A track (Batch 4a: **a track is an instrument** of one voice type). A
/// *studio* track (`lock: None`) plays through its own pool of voices; a
/// *locked* track is bound to one real channel of the project's hardware
/// profile and plays through the full chip emulator (`tracks.rs`).
#[derive(Clone, Debug, PartialEq)]
pub struct Lane {
    /// Stable id (never reused in a project): the engine's studio rack, the
    /// mute/solo sets and the file refer to it.
    pub id: u32,
    /// The voice type (Pulse, Triangle, Noise, DPCM, VRC6 Pulse, Saw).
    pub voice: chip::voice::VoiceKind,
    /// The console channel (0..8, `chip::driver::LANE_*`) a locked track is
    /// bound to; `None`: a studio track.
    pub lock: Option<usize>,
    /// The Voices setting: most voices at once (0 = unlimited; studio tracks).
    pub voices: u16,
    /// The fader and pan before locking (locking pins them at 0 dB and
    /// centre; unlocking restores them).
    pub before_lock: Option<(f32, i8)>,
    /// The channel controls (the inspector's dials).
    pub params: TrackParams,
    /// The track's name: the hardware channel's by default, renamable
    /// (Cmd+R, T2; saved from format v4).
    pub name: String,
    pub short: &'static str,
    /// The track's colour (T3 Colour ▸; saved from format v4).
    pub color: Rgba,
    /// The row's height in logical px without its 1 px hairline:
    /// `ROW_H_MIN`..=`ROW_H_MAX` in `ROW_H_STEP`s (polish 2 §4; format v6).
    pub height: f32,
    /// Pan, −`PAN_MAX` (left) … 0 (centre) … +`PAN_MAX` (right). Saved and
    /// editable (format v6); heard on studio tracks (Batch 4b, the pan law).
    pub pan: i8,
    /// The track volume in dB, −inf … +`VOL_DB_MAX`: a gain after the chip
    /// (polish 2 §3; format v6). 0 dB = unity.
    pub vol_db: f32,
    /// Unison, Detune and Drift (Batch 4b §5; format v8): studio tracks only
    /// (a locked track ignores and hides them).
    pub unison: Unison,
}

/// A studio track's unison voices per note (1–4), their detune spread
/// (0–50 cents) and per-voice drift (0–10 cents). The default is one voice,
/// no detune, no drift: exactly the sound from before Batch 4b.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Unison {
    pub voices: u8,
    pub detune: u8,
    pub drift: u8,
}

impl Default for Unison {
    fn default() -> Self {
        Self { voices: 1, detune: 0, drift: 0 }
    }
}

impl Unison {
    /// Clamped to the engine's ranges.
    pub fn clamped(self) -> Self {
        Self {
            voices: self.voices.clamp(1, engine::studio::UNISON_MAX),
            detune: self.detune.min(engine::studio::DETUNE_MAX),
            drift: self.drift.min(engine::studio::DRIFT_MAX),
        }
    }
}

impl Lane {
    /// A track locked to console channel `chan` (the legacy layout's lanes).
    pub fn new(name: &str, short: &'static str, color: Rgba) -> Self {
        let chan = CHANNEL_SHORT.iter().position(|s| *s == short).unwrap_or(0);
        Self {
            id: chan as u32 + 1,
            voice: chip::voice::VoiceKind::of_lane(chan),
            lock: Some(chan),
            voices: 0,
            before_lock: None,
            params: engine::Params::default().values[chan],
            name: name.into(),
            short,
            color,
            height: ROW_H_MIN,
            pan: 0,
            vol_db: 0.0,
            unison: Unison::default(),
        }
    }

    /// A new studio track of `voice` (Insert Track).
    pub fn studio(id: u32, voice: chip::voice::VoiceKind, name: String, color: Rgba) -> Self {
        Self {
            id,
            voice,
            lock: None,
            voices: 0,
            before_lock: None,
            params: engine::Params::default().values[voice.lane()],
            name,
            short: crate::tracks::kind_short(voice),
            color,
            height: ROW_H_MIN,
            pan: 0,
            vol_db: 0.0,
            unison: Unison::default(),
        }
    }

    /// The console lane whose controls and register layout this track uses:
    /// its channel when locked, else its type's first channel. The dials,
    /// `ParamId::for_lane` and `range` take this.
    pub fn chan(&self) -> usize {
        self.lock.unwrap_or(self.voice.lane())
    }

    pub fn locked(&self) -> bool {
        self.lock.is_some()
    }

    /// The row's pitch: its height plus the hairline under it.
    pub fn pitch(&self) -> f32 {
        self.height + ROW_HAIRLINE
    }
}

/// Row heights (logical px, without the hairline): 72 is the minimum and
/// today's height, 300 the maximum, in 4 px steps (polish 2 §4).
pub const ROW_H_MIN: f32 = 72.0;
pub const ROW_H_MAX: f32 = 300.0;
pub const ROW_H_STEP: f32 = 4.0;
/// The hairline under each row.
pub const ROW_HAIRLINE: f32 = 1.0;

/// A height clamped to the range, on its 4 px steps.
pub fn clamp_row_h(h: f32) -> f32 {
    ((h / ROW_H_STEP).round() * ROW_H_STEP).clamp(ROW_H_MIN, ROW_H_MAX)
}

/// The volume fader's scale (polish 2 §3): position `(dB + 48) / 54`,
/// −inf at the bottom, +6 dB of headroom at the top.
pub const VOL_DB_MAX: f32 = 6.0;
pub const VOL_DB_FLOOR: f32 = -48.0;
/// Typed values are clamped to this at the bottom (−inf is typed as "-inf").
pub const VOL_DB_MIN_TYPED: f32 = -70.0;
/// Pan range: 50L … C … 50R.
pub const PAN_MAX: i8 = 50;

/// A dB value's fader position 0..1.
pub fn db_to_pos(db: f32) -> f32 {
    if db == f32::NEG_INFINITY {
        return 0.0;
    }
    ((db - VOL_DB_FLOOR) / (VOL_DB_MAX - VOL_DB_FLOOR)).clamp(0.0, 1.0)
}

/// A fader position's dB value: the very bottom is −inf.
pub fn pos_to_db(pos: f32) -> f32 {
    if pos <= 0.0 {
        f32::NEG_INFINITY
    } else {
        (VOL_DB_FLOOR + pos.min(1.0) * (VOL_DB_MAX - VOL_DB_FLOOR)).min(VOL_DB_MAX)
    }
}

/// dB → linear gain (−inf → 0, 0 dB → exactly 1); at most ×2 (+6 dB).
pub fn db_to_gain(db: f32) -> f32 {
    if db == f32::NEG_INFINITY {
        0.0
    } else if db == 0.0 {
        1.0
    } else {
        10f32.powf(db.min(VOL_DB_MAX) / 20.0)
    }
}

/// A linear level → dB (0 → −inf).
pub fn gain_to_db(g: f32) -> f32 {
    if g <= 0.0 {
        f32::NEG_INFINITY
    } else {
        20.0 * g.log10()
    }
}

/// "−2.2 dB" (a real minus sign, one decimal), "0.0 dB", "2.5 dB", "−inf dB".
pub fn format_db(db: f32) -> String {
    if db == f32::NEG_INFINITY {
        return "\u{2212}inf dB".into();
    }
    let r = (db * 10.0).round() / 10.0;
    if r < 0.0 {
        format!("\u{2212}{:.1} dB", -r)
    } else {
        format!("{:.1} dB", r.abs())
    }
}

/// Typed dB: `-3`, `-3 dB`, `−3`, `+2.5`, `-inf`; clamped to −70 … +6.
pub fn parse_db(text: &str) -> Option<f32> {
    let t = text.trim().replace('\u{2212}', "-").to_ascii_lowercase();
    let t = t.strip_suffix("db").unwrap_or(&t).trim();
    if matches!(t, "-inf" | "-infinity" | "inf" | "-\u{221E}") {
        return Some(f32::NEG_INFINITY);
    }
    let v: f32 = t.parse().ok().filter(|v: &f32| v.is_finite())?;
    Some(v.clamp(VOL_DB_MIN_TYPED, VOL_DB_MAX))
}

/// "C", "25L", "30R".
pub fn format_pan(p: i8) -> String {
    match p {
        0 => "C".into(),
        p if p < 0 => format!("{}L", -(p as i32)),
        p => format!("{p}R"),
    }
}

/// Typed pan: `C`, `0`, `25L`, `30R`, `-25` (left), `+25` / `25` (right); clamped.
pub fn parse_pan(text: &str) -> Option<i8> {
    let t = text.trim().replace('\u{2212}', "-").to_ascii_uppercase();
    let t = t.trim();
    if t == "C" || t == "CENTER" || t == "CENTRE" {
        return Some(0);
    }
    let (num, sign) = if let Some(n) = t.strip_suffix('L') {
        (n.trim(), -1)
    } else if let Some(n) = t.strip_suffix('R') {
        (n.trim(), 1)
    } else {
        (t, 1)
    };
    if num.is_empty() {
        return None;
    }
    let v: i32 = num.parse().ok()?;
    Some((v * sign).clamp(-(PAN_MAX as i32), PAN_MAX as i32) as i8)
}

/// The palette offered by the Colour ▸ menus (clips and tracks): the lane
/// colours first, then extra hues of the same lightness.
pub const PALETTE: [(&str, u32); 12] = [
    ("Sage", 0xB4C08A),
    ("Periwinkle", 0x7F86E0),
    ("Sand", 0xB5AC98),
    ("Amber", 0xE39A42),
    ("Rose", 0xD9579A),
    ("Teal", 0x3CC4A6),
    ("Sky", 0x4FA8E0),
    ("Coral", 0xE0705A),
    ("Violet", 0xA77BE0),
    ("Mint", 0x7FD08A),
    ("Gold", 0xD8B83C),
    ("Slate", 0x8C98A8),
];

/// A colour as 0xRRGGBB (files; the palette's colours round-trip exactly).
pub fn rgb_hex(c: Rgba) -> u32 {
    let ch = |v: f32| ((v.clamp(0.0, 1.0) * 255.0).round() as u32) & 0xFF;
    (ch(c.r) << 16) | (ch(c.g) << 8) | ch(c.b)
}

/// The console channels' short names, in lane order.
pub const CHANNEL_SHORT: [&str; 8] = ["P1", "P2", "TRI", "NOI", "DMC", "V1", "V2", "SAW"];

/// The legacy layout: the eight console channels, each a track locked to its
/// channel (projects from before format 7 open this way).
pub fn default_lanes() -> Vec<Lane> {
    vec![
        Lane::new("Pulse 1", "P1", tokens::LANE_PULSE),
        Lane::new("Pulse 2", "P2", tokens::LANE_PULSE),
        Lane::new("Triangle", "TRI", tokens::LANE_TRI),
        Lane::new("Noise", "NOI", tokens::LANE_NOISE),
        Lane::new("DPCM", "DMC", tokens::LANE_DPCM),
        Lane::new("VRC6 Pulse 1", "V1", tokens::LANE_VRC6_PULSE),
        Lane::new("VRC6 Pulse 2", "V2", tokens::LANE_VRC6_PULSE),
        Lane::new("VRC6 Saw", "SAW", tokens::LANE_VRC6_SAW),
    ]
}

/// The arrangement's fixed grid sizes, finest first (R56: Cmd+1 / Cmd+2
/// step through them).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Snap {
    ThirtySecond,
    Sixteenth,
    Eighth,
    Quarter,
    Half,
    Bar,
}

impl Snap {
    pub fn ticks(self) -> u32 {
        match self {
            Snap::ThirtySecond => PPQ / 8,
            Snap::Sixteenth => PPQ / 4,
            Snap::Eighth => PPQ / 2,
            Snap::Quarter => PPQ,
            Snap::Half => PPQ * 2,
            Snap::Bar => TICKS_PER_BAR,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Snap::ThirtySecond => "1/32",
            Snap::Sixteenth => "1/16",
            Snap::Eighth => "1/8",
            Snap::Quarter => "1/4",
            Snap::Half => "1/2",
            Snap::Bar => "Bar",
        }
    }

    pub const ALL: [Snap; 6] = [Snap::ThirtySecond, Snap::Sixteenth, Snap::Eighth, Snap::Quarter, Snap::Half, Snap::Bar];

    /// One step finer (`-1`) or coarser (`+1`), clamped.
    pub fn step(self, d: i32) -> Self {
        let i = Self::ALL.iter().position(|g| *g == self).unwrap_or(1) as i32;
        Self::ALL[(i + d).clamp(0, Self::ALL.len() as i32 - 1) as usize]
    }

    /// The grid a label names (settings.json); unknown labels give 1/16.
    pub fn from_label(s: &str) -> Self {
        Self::ALL.into_iter().find(|g| g.label() == s).unwrap_or(Snap::Sixteenth)
    }

    /// Snap a tick position to the nearest grid line.
    pub fn round(self, ticks: i64) -> i64 {
        let g = self.ticks() as i64;
        (ticks + g / 2).div_euclid(g) * g
    }
}

/// The arrangement grid's switches (R57–R59): triplet spacing, snapping on,
/// and a zoom-adaptive size (`density` is how Cmd+1 / Cmd+2 move it).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct GridOpts {
    pub triplet: bool,
    pub snap: bool,
    pub adaptive: bool,
    pub density: i8,
}

impl Default for GridOpts {
    fn default() -> Self {
        Self { triplet: false, snap: true, adaptive: false, density: 0 }
    }
}

/// An adaptive grid's lines are at least this far apart (logical px) at
/// density 0; each density step halves (finer) or doubles it.
pub const ADAPTIVE_MIN_PX: f64 = 16.0;

/// The grid in effect: its spacing and whether positions snap to it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Step {
    pub ticks: u32,
    pub on: bool,
}

impl Step {
    /// Nearest grid line (or `t` unchanged when snapping is off).
    pub fn round(self, t: i64) -> i64 {
        if !self.on {
            return t;
        }
        let g = self.ticks.max(1) as i64;
        (t + g / 2).div_euclid(g) * g
    }

    /// Grid line at or before `t` (or `t` when snapping is off).
    pub fn floor(self, t: i64) -> i64 {
        if !self.on {
            return t;
        }
        let g = self.ticks.max(1) as i64;
        t.div_euclid(g) * g
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

/// An arrangement time selection (drag across empty lane space):
/// a tick range over a run of lanes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TimeSel {
    pub start: u32,
    pub end: u32,
    pub lane0: usize,
    pub lane1: usize,
}

impl TimeSel {
    pub fn contains(&self, tick: f64, lane: usize) -> bool {
        tick >= self.start as f64 && tick < self.end as f64 && (self.lane0..=self.lane1).contains(&lane)
    }
}

/// Context-menu commands.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum MenuAction {
    /// A command from the shortcut table (its label and key come from there).
    Cmd(crate::shortcuts::Action),
    LoopClips,
    /// The clip gets its own copy of its instrument.
    MakeUnique,
    /// The clip goes back to its track's default instrument.
    LaneInstrument,
    /// Save the clip's instrument as a user preset.
    SavePreset,
    /// A fixed arrangement grid size.
    Grid(Snap),
    /// The selected clips' colour (`None`: the track's).
    ClipColor(Option<Rgba>),
    TrackColor(usize, Rgba),
    RenameTrack(usize),
    /// Insert Track ▸ a type: a new studio track below the selected one.
    InsertTrack(chip::voice::VoiceKind),
    DuplicateTrack(usize),
    DeleteTrack(usize),
    /// The badge's lock menu: lock track to a channel, or `None`: studio.
    LockTrack(usize, Option<usize>),
    /// A studio track's Voices setting (0 = unlimited).
    SetVoices(usize, u16),
    /// A user preset in the browser (its browser index).
    PresetRename(usize),
    PresetDelete(usize),
    PresetReveal(usize),
    /// The Voice selector: a studio track's voice.
    SetVoice(usize, chip::voice::VoiceKind),
    /// Add a browser item to collection n, or take it out.
    Collect(crate::bank::Item, u8),
    /// The spectrum's style.
    SpectrumStyle(crate::views::chrome::SpectrumStyle),
    /// Settings ▸ Editing: the default track colour (auto colours off).
    DefaultTrackColour(Rgba),
    /// Opens a submenu.
    Sub(SubMenu),
    Separator,
    /// A line of information (the status bar's problem details).
    Info,
}

/// The submenus.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SubMenu {
    ClipColor,
    TrackColor(usize),
    Grid,
    /// Insert Track ▸ (the voice types).
    InsertTrack,
    /// Lock ▸ (the badge's menu) for a track.
    Lock(usize),
    /// Voices ▸ (a studio track's polyphony cap).
    Voices(usize),
}

/// One row of a context menu.
#[derive(Clone, Debug, PartialEq)]
pub struct MenuItem {
    pub action: MenuAction,
    pub label: String,
    /// The key, from the shortcut table ("" if none).
    pub shortcut: String,
    pub enabled: bool,
    /// Why it's disabled, shown on hover (F-24, principle 3).
    pub reason: String,
    pub checked: bool,
    /// A colour swatch (Colour ▸ items).
    pub swatch: Option<Rgba>,
    /// A cell of the colour grid (swatch only, laid out 14 to a row).
    pub grid: bool,
}

impl MenuItem {
    pub fn new(action: MenuAction, label: impl Into<String>) -> Self {
        Self { action, label: label.into(), shortcut: String::new(), enabled: true, reason: String::new(), checked: false, swatch: None, grid: false }
    }

    /// A shortcut-table command: its name and key.
    pub fn cmd(a: crate::shortcuts::Action) -> Self {
        let mut m = Self::new(MenuAction::Cmd(a), crate::shortcuts::get(a).name);
        m.shortcut = crate::shortcuts::label(a);
        m
    }

    pub fn sep() -> Self {
        Self::new(MenuAction::Separator, "")
    }

    /// Disabled, with the reason shown on hover.
    pub fn off_if(mut self, off: bool, reason: &str) -> Self {
        if off {
            self.enabled = false;
            self.reason = reason.into();
        }
        self
    }

    pub fn checked(mut self, on: bool) -> Self {
        self.checked = on;
        self
    }

    pub fn key(mut self, k: impl Into<String>) -> Self {
        self.shortcut = k.into();
        self
    }
}

/// An open context menu: where (window logical px), its rows, and an open
/// submenu.
#[derive(Clone, Debug, PartialEq)]
pub struct ContextMenuSpec {
    pub x: f32,
    pub y: f32,
    pub items: Vec<MenuItem>,
    pub sub: Option<Box<ContextMenuSpec>>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Arrangement {
    pub lanes: Vec<Lane>,
    /// Shared so an undo snapshot costs one pointer per clip; changed only
    /// through `clip_mut` (copy on write + a fresh `Rev`).
    pub clips: Vec<Arc<Clip>>,
    /// Selected clip ids.
    pub selection: Vec<u32>,
    /// The fixed grid size and the grid switches.
    pub snap: Snap,
    pub grid_opts: GridOpts,
    pub view: Viewport,
    /// Selected tracks (T1; not part of the document).
    pub track_sel: Vec<usize>,
    /// The insert marker (A1): tick and track. Paste, Split and Insert
    /// Silence act here; while stopped the playhead sits on it.
    pub insert: (u32, usize),
    /// The arrangement view's width (logical px), written by the view
    /// (zoom to selection / fit need it).
    pub view_w: f32,
    pub next_id: u32,
    /// Clips being dragged/resized right now. They are the "newer" clips: the
    /// engine hears them trimming whatever they overlap (without changing the
    /// other clips until the drop), and on release the trim is applied.
    pub editing: Vec<u32>,
    /// Notes being dragged in the piano roll: (clip, note ids). Like
    /// `editing`, the engine hears them winning while the drag lasts.
    pub editing_notes: Option<(u32, Vec<u32>)>,
    /// The project's instruments: lane defaults and clip overrides (part of
    /// the document: undo, files).
    pub instruments: crate::instruments::InstrumentTable,
    /// A preset being auditioned from the browser (not part of the document).
    pub audition: Option<LaneInstrument>,
    /// Imported DPCM samples (user slots, played from DPCM rows 5..8).
    /// Index = user slot (DPCM row 5 + i); `None` = empty slot.
    pub samples: Vec<Option<crate::dpcm::UserSample>>,
}

impl Arrangement {
    /// An empty legacy project: the 8 lanes (each locked to its channel, on
    /// the default band's channel controls) and no clips.
    pub fn empty() -> Self {
        let mut lanes = default_lanes();
        let params = crate::instruments::default_params();
        for (i, l) in lanes.iter_mut().enumerate() {
            l.params = params.values[i];
        }
        Self {
            lanes,
            clips: Vec::new(),
            selection: Vec::new(),
            snap: Snap::Sixteenth,
            grid_opts: GridOpts::default(),
            view: Viewport::default(),
            track_sel: Vec::new(),
            insert: (0, 0),
            view_w: 1000.0,
            next_id: 1,
            editing: Vec::new(),
            editing_notes: None,
            instruments: default_instruments(),
            audition: None,
            samples: Vec::new(),
        }
    }

    /// The top of row `i` in lane space (logical px from the first row's top);
    /// rows have their own heights (polish 2 §4). `i == lanes.len()` is the
    /// bottom of the last row.
    pub fn lane_top(&self, i: usize) -> f32 {
        self.lanes.iter().take(i).map(Lane::pitch).sum()
    }

    /// Every row's pitch together.
    pub fn lanes_height(&self) -> f32 {
        self.lane_top(self.lanes.len())
    }

    /// The row at lane-space `y`, if any (a row owns its hairline).
    pub fn lane_at(&self, y: f32) -> Option<usize> {
        if y < 0.0 {
            return None;
        }
        let mut top = 0.0;
        for (i, l) in self.lanes.iter().enumerate() {
            top += l.pitch();
            if y < top {
                return Some(i);
            }
        }
        None
    }

    /// The row at lane-space `y`, clamped to the first and last.
    pub fn lane_at_clamped(&self, y: f32) -> usize {
        if y < 0.0 {
            0
        } else {
            self.lane_at(y).unwrap_or(self.lanes.len().saturating_sub(1))
        }
    }

    pub fn is_selected(&self, id: u32) -> bool {
        self.selection.contains(&id)
    }

    pub fn clip(&self, id: u32) -> Option<&Clip> {
        self.clips.iter().find(|c| c.id == id).map(|c| &**c)
    }

    /// Mutable access for an edit: copies the clip if an undo snapshot (or
    /// anything else) shares it, and stamps a new revision.
    pub fn clip_mut(&mut self, id: u32) -> Option<&mut Clip> {
        let c = Arc::make_mut(self.clips.iter_mut().find(|c| c.id == id)?);
        c.rev = Rev::next();
        Some(c)
    }

    pub fn push_clip(&mut self, c: Clip) {
        self.clips.push(Arc::new(c));
    }

    pub fn delete_selected(&mut self) {
        let sel = std::mem::take(&mut self.selection);
        self.clips.retain(|c| !sel.contains(&c.id));
    }

    /// Tick span covered by the selected clips.
    pub fn selection_span(&self) -> Option<(u32, u32)> {
        self.selection
            .iter()
            .filter_map(|id| self.clip(*id))
            .map(|c| (c.start_ticks, c.end_ticks()))
            .reduce(|(a0, a1), (b0, b1)| (a0.min(b0), a1.max(b1)))
    }

    /// Duplicate (Ctrl+D): copies of the selected clips placed
    /// right after the selection's span; the copies become the selection.
    pub fn duplicate_selected(&mut self) {
        let Some((start, end)) = self.selection_span() else { return };
        let offset = end - start;
        let originals: Vec<Clip> = self
            .selection
            .iter()
            .filter_map(|id| self.clip(*id).cloned())
            .collect();
        let mut new_ids = Vec::new();
        for mut c in originals {
            c.id = self.next_id;
            self.next_id += 1;
            c.start_ticks += offset;
            c.rev = Rev::next();
            new_ids.push(c.id);
            self.push_clip(c);
        }
        self.resolve_overlaps(&new_ids);
        self.selection = new_ids;
    }

    /// Brings the given clip ids to the end of the list (drawn on top).
    pub fn raise(&mut self, ids: &[u32]) {
        let (mut top, rest): (Vec<_>, Vec<_>) =
            self.clips.drain(..).partition(|c| ids.contains(&c.id));
        self.clips = rest;
        self.clips.append(&mut top);
    }

    pub fn first_selected(&self) -> Option<&Clip> {
        self.selection.first().and_then(|id| self.clip(*id))
    }

    /// The instrument the inspector edits for `lane`: the selected clip's
    /// (its own or its lane's default), else the lane's default.
    pub fn current_instrument(&self, lane: usize) -> crate::instruments::InstId {
        match self.first_selected().filter(|c| c.lane == lane) {
            Some(c) => self.instruments.id_for(c),
            None => self.instruments.lane_id(lane),
        }
    }

    /// The instrument a clip plays.
    pub fn clip_instrument(&self, c: &Clip) -> Option<&LaneInstrument> {
        self.instruments.get(self.instruments.id_for(c))
    }

    /// A new empty clip of `len` ticks on `lane` at `start` (the newest clip:
    /// it wins overlaps). Returns its id.
    pub fn add_clip(&mut self, lane: usize, start: u32, len: u32) -> u32 {
        let id = self.next_id;
        self.next_id += 1;
        let name = format!("{} {id}", self.lanes.get(lane).map_or("?", |l| l.short));
        self.push_clip(Clip::new(id, lane, start, len, name, Vec::new()));
        self.resolve_overlaps(&[id]);
        id
    }

    /// Monophonic overlap rule: each clip in `winners` trims, splits or
    /// removes every other clip it overlaps in its lane. Split-off right parts
    /// get new ids. Winners don't trim each other.
    pub fn resolve_overlaps(&mut self, winners: &[u32]) {
        for &w in winners {
            let Some(wc) = self.clip(w) else { continue };
            let (lane, ws, we) = (wc.lane, wc.start_ticks, wc.end_ticks());
            let mut out = Vec::with_capacity(self.clips.len() + 1);
            for c in std::mem::take(&mut self.clips) {
                if winners.contains(&c.id)
                    || c.lane != lane
                    || c.end_ticks() <= ws
                    || c.start_ticks >= we
                {
                    out.push(c);
                    continue;
                }
                let left = (c.start_ticks < ws).then(|| c.trimmed(c.start_ticks, ws));
                let right = (c.end_ticks() > we).then(|| c.trimmed(we, c.end_ticks()));
                match (left, right) {
                    (Some(l), Some(mut r)) => {
                        r.id = self.next_id;
                        self.next_id += 1;
                        out.push(Arc::new(l));
                        out.push(Arc::new(r));
                    }
                    (Some(l), None) => out.push(Arc::new(l)),
                    (None, Some(r)) => out.push(Arc::new(r)),
                    (None, None) => {}
                }
            }
            self.clips = out;
        }
        let ids: Vec<u32> = self.clips.iter().map(|c| c.id).collect();
        self.selection.retain(|id| ids.contains(id));
    }

    /// The clips as the engine should hear them right now (with `editing`
    /// clips and `editing_notes` winning over everything they overlap).
    pub fn resolved_clips(&self) -> std::borrow::Cow<'_, [Arc<Clip>]> {
        if self.editing.is_empty() && self.editing_notes.is_none() {
            return std::borrow::Cow::Borrowed(&self.clips);
        }
        let mut a = self.clone();
        let w = a.editing.clone();
        a.resolve_overlaps(&w);
        if let Some((cid, ids)) = a.editing_notes.clone() {
            if let Some(c) = a.clip_mut(cid) {
                let mut next = c.next_note_id;
                notes::resolve(true, &mut c.notes, &ids, &mut next);
            }
        }
        std::borrow::Cow::Owned(a.clips)
    }

    /// Changes whenever anything audible changes (not selection or view):
    /// every clip edit stamps a new `Rev`.
    pub fn content_key(&self) -> u64 {
        let mut h = std::collections::hash_map::DefaultHasher::new();
        for c in &self.clips {
            (c.id, c.rev.0).hash(&mut h);
        }
        // The track layout decides where every clip plays (console channel or
        // studio voices).
        for l in &self.lanes {
            (l.id, l.lock, l.voice, l.voices, l.unison.voices).hash(&mut h);
        }
        self.editing.hash(&mut h);
        self.editing_notes.hash(&mut h);
        self.instruments.hash(&mut h);
        self.audition.hash(&mut h);
        self.samples.hash(&mut h);
        h.finish()
    }

}

impl Clip {
    /// The part of this clip between absolute ticks `from..to`; notes keep
    /// their song positions (notes starting before `from` are dropped, notes
    /// crossing `to` are shortened).
    pub fn trimmed(&self, from: u32, to: u32) -> Clip {
        let shift = from - self.start_ticks;
        let len = to - from;
        let notes = self
            .notes
            .iter()
            .filter(|n| n.start >= shift && n.start < shift + len)
            .map(|n| Note {
                start: n.start - shift,
                len: n.len.min(shift + len - n.start),
                ..*n
            })
            .collect();
        // The parts keep the clip's own instrument and colour (before Batch 3
        // an overlap split dropped the instrument override).
        Clip {
            next_note_id: self.next_note_id,
            instrument: self.instrument,
            color: self.color,
            ..Clip::new(self.id, self.lane, from, len, self.name.clone(), notes)
        }
    }
}

// ───────────────────────── Arrangement editing (Batch 3) ─────────────────────────

/// Copied clips (Cmd+C / Copy Time): each with its position relative to the
/// copy's start and its track relative to the copy's top track.
#[derive(Clone, Debug, PartialEq)]
pub struct ClipCopy {
    pub lane_off: usize,
    pub start: u32,
    pub clip: Clip,
}

/// The arrangement clipboard. `len` is the copied time span (a time
/// selection's length, or the clips' span); `all_lanes` marks a Copy Time
/// (track offsets are absolute: it pastes from the first track).
#[derive(Clone, Debug, PartialEq)]
pub struct ClipBoard {
    pub clips: Vec<ClipCopy>,
    pub len: u32,
    pub all_lanes: bool,
}

/// Why a Consolidate left a track alone.
pub const CONSOLIDATE_MIXED: &str = "Consolidate needs the clips on a track to share one instrument: use Use Track Instrument or Make Instrument Unique first.";

impl Arrangement {
    /// The grid in effect (fixed or zoom-adaptive, straight or triplet).
    pub fn grid(&self) -> Step {
        let o = self.grid_opts;
        let base = if o.adaptive {
            // The finest fixed size whose lines are at least the adaptive
            // spacing apart at this zoom.
            let min_px = ADAPTIVE_MIN_PX * 2f64.powi(o.density as i32);
            Snap::ALL
                .into_iter()
                .find(|g| g.ticks() as f64 * self.view.px_per_tick >= min_px)
                .unwrap_or(Snap::Bar)
                .ticks()
        } else {
            self.snap.ticks()
        };
        let ticks = if o.triplet { base * 2 / 3 } else { base };
        Step { ticks: ticks.max(1), on: o.snap }
    }

    /// The grid as the context menu and the hint show it ("1/16", "1/8T",
    /// "Adaptive 1/8", "1/16 · snap off").
    pub fn grid_label(&self) -> String {
        let g = self.grid();
        let name = Snap::ALL
            .into_iter()
            .find(|s| s.ticks() == if self.grid_opts.triplet { g.ticks * 3 / 2 } else { g.ticks })
            .map_or("?", |s| s.label());
        let t = if self.grid_opts.triplet { "T" } else { "" };
        let a = if self.grid_opts.adaptive { "Adaptive " } else { "" };
        let off = if g.on { "" } else { " · snap off" };
        format!("{a}{name}{t}{off}")
    }

    fn take_id(&mut self) -> u32 {
        let id = self.next_id;
        self.next_id += 1;
        id
    }

    /// Splits clip `id` at absolute `tick` (strictly inside it). The left
    /// part keeps the id; returns the right part's new id.
    pub fn split_clip(&mut self, id: u32, tick: u32) -> Option<u32> {
        let c = self.clip(id)?;
        if tick <= c.start_ticks || tick >= c.end_ticks() {
            return None;
        }
        let left = c.trimmed(c.start_ticks, tick);
        let mut right = c.trimmed(tick, c.end_ticks());
        right.id = self.take_id();
        let rid = right.id;
        let i = self.clips.iter().position(|c| c.id == id)?;
        self.clips[i] = Arc::new(left);
        self.clips.insert(i + 1, Arc::new(right));
        Some(rid)
    }

    /// Splits the clips on `lanes` at `start` and `end`, so that the range
    /// is made of whole clips; returns their ids.
    pub fn isolate(&mut self, start: u32, end: u32, lanes: std::ops::RangeInclusive<usize>) -> Vec<u32> {
        let on = |c: &Clip| lanes.contains(&c.lane) && c.start_ticks < end && c.end_ticks() > start;
        let ids: Vec<u32> = self.clips.iter().filter(|c| on(c)).map(|c| c.id).collect();
        for id in ids {
            let right = self.split_clip(id, start);
            self.split_clip(right.unwrap_or(id), end);
        }
        self.clips.iter().filter(|c| on(c) && c.start_ticks >= start && c.end_ticks() <= end).map(|c| c.id).collect()
    }

    /// Delete with a time selection (A11, F-22): the range's contents on
    /// `lanes` go; clips crossing its edges are split there. No gap closes.
    pub fn clear_range(&mut self, start: u32, end: u32, lanes: std::ops::RangeInclusive<usize>) -> bool {
        let ids = self.isolate(start, end, lanes);
        self.clips.retain(|c| !ids.contains(&c.id));
        self.selection.retain(|id| !ids.contains(id));
        !ids.is_empty()
    }

    /// The clips of `ids`, for the clipboard.
    pub fn copy_clips(&self, ids: &[u32]) -> Option<ClipBoard> {
        let sel: Vec<&Clip> = self.clips.iter().filter(|c| ids.contains(&c.id)).map(|c| &**c).collect();
        let start = sel.iter().map(|c| c.start_ticks).min()?;
        let end = sel.iter().map(|c| c.end_ticks()).max()?;
        let top = sel.iter().map(|c| c.lane).min()?;
        Some(ClipBoard {
            clips: sel.iter().map(|c| ClipCopy { lane_off: c.lane - top, start: c.start_ticks - start, clip: (*c).clone() }).collect(),
            len: end - start,
            all_lanes: false,
        })
    }

    /// The material of `start..end` on `lanes`, for the clipboard (clips cut
    /// at the edges; the arrangement is unchanged).
    pub fn copy_range(&self, start: u32, end: u32, lanes: std::ops::RangeInclusive<usize>, all_lanes: bool) -> ClipBoard {
        let top = if all_lanes { 0 } else { *lanes.start() };
        let clips = self
            .clips
            .iter()
            .filter(|c| lanes.contains(&c.lane) && c.start_ticks < end && c.end_ticks() > start)
            .map(|c| {
                let part = c.trimmed(c.start_ticks.max(start), c.end_ticks().min(end));
                ClipCopy { lane_off: c.lane - top, start: part.start_ticks - start, clip: part }
            })
            .collect();
        ClipBoard { clips, len: end.saturating_sub(start), all_lanes }
    }

    /// Pastes `cb` at tick `at`, its top track on `lane` (a Copy Time pastes
    /// from the first track). The pasted clips are new clips that win
    /// overlaps; they become the selection. Tracks past the last, or that
    /// `open` refuses (dimmed VRC6 lanes, F-04), are skipped.
    pub fn paste(&mut self, cb: &ClipBoard, at: u32, lane: usize, open: &dyn Fn(usize) -> bool) -> Vec<u32> {
        let top = if cb.all_lanes { 0 } else { lane };
        let mut ids = Vec::new();
        for cc in &cb.clips {
            let l = top + cc.lane_off;
            let start = at as u64 + cc.start as u64;
            if l >= self.lanes.len() || !open(l) || start + cc.clip.len_ticks as u64 > MAX_SONG_TICKS as u64 {
                continue;
            }
            let mut c = cc.clip.clone();
            c.id = self.take_id();
            c.lane = l;
            c.start_ticks = start as u32;
            c.rev = Rev::next();
            ids.push(c.id);
            self.push_clip(c);
        }
        self.resolve_overlaps(&ids);
        let alive: Vec<u32> = self.clips.iter().map(|c| c.id).collect();
        ids.retain(|id| alive.contains(id));
        self.selection = ids.clone();
        ids
    }

    /// Insert Silence / Paste Time (R46, R47): `len` empty ticks at `at` on
    /// every track; a clip crossing `at` is split there, later ones move.
    pub fn insert_time(&mut self, at: u32, len: u32) {
        if len == 0 {
            return;
        }
        let ids: Vec<u32> = self.clips.iter().filter(|c| c.start_ticks < at && c.end_ticks() > at).map(|c| c.id).collect();
        for id in ids {
            self.split_clip(id, at);
        }
        let later: Vec<u32> = self.clips.iter().filter(|c| c.start_ticks >= at).map(|c| c.id).collect();
        for id in later {
            if let Some(c) = self.clip_mut(id) {
                c.start_ticks = c.start_ticks.saturating_add(len).min(MAX_SONG_TICKS - c.len_ticks.min(MAX_SONG_TICKS));
            }
        }
    }

    /// Cut / Delete Time (R46): `start..end` goes on every track and the gap
    /// closes.
    pub fn delete_time(&mut self, start: u32, end: u32) {
        if end <= start {
            return;
        }
        let all = 0..=self.lanes.len().saturating_sub(1);
        self.clear_range(start, end, all);
        let later: Vec<u32> = self.clips.iter().filter(|c| c.start_ticks >= end).map(|c| c.id).collect();
        for id in later {
            if let Some(c) = self.clip_mut(id) {
                c.start_ticks -= end - start;
            }
        }
    }

    /// Consolidate (R39): each track's clips among `ids` become one clip
    /// from the first start to the last end (the gaps stay silent). With
    /// `range`, the clips are first cut to it and the new clip spans it
    /// exactly. Returns the new clips and the tracks left alone (their clips
    /// play different instruments).
    pub fn consolidate(&mut self, ids: &[u32], range: Option<(u32, u32)>) -> (Vec<u32>, Vec<usize>) {
        let mut lanes: Vec<usize> = self.clips.iter().filter(|c| ids.contains(&c.id)).map(|c| c.lane).collect();
        lanes.sort_unstable();
        lanes.dedup();
        let (mut made, mut mixed) = (Vec::new(), Vec::new());
        for lane in lanes {
            let mut group: Vec<Clip> = self.clips.iter().filter(|c| c.lane == lane && ids.contains(&c.id)).map(|c| (**c).clone()).collect();
            group.sort_by_key(|c| c.start_ticks);
            if group.windows(2).any(|w| w[0].instrument != w[1].instrument) {
                mixed.push(lane);
                continue;
            }
            let (start, end) = match range {
                Some(r) => r,
                None => (group[0].start_ticks, group.iter().map(|c| c.end_ticks()).max().unwrap_or(0)),
            };
            if group.len() < 2 && range.is_none_or(|r| (group[0].start_ticks, group[0].end_ticks()) == r) {
                continue;
            }
            let mut notes = Vec::new();
            let mut next = 1;
            for c in &group {
                let off = c.start_ticks - start;
                for n in c.audible_notes() {
                    notes.push(Note { id: next, start: n.start + off, len: n.len.min(c.len_ticks - n.start), ..*n });
                    next += 1;
                }
            }
            notes.sort_by_key(|n| n.start);
            notes::resolve(true, &mut notes, &[], &mut next);
            let first = &group[0];
            let mut c = Clip::new(first.id, lane, start, (end - start).max(1), first.name.clone(), notes);
            c.instrument = first.instrument;
            c.color = first.color;
            let gone: Vec<u32> = group.iter().map(|g| g.id).collect();
            let i = self.clips.iter().position(|x| x.id == first.id).unwrap_or(self.clips.len());
            self.clips.retain(|x| !gone.contains(&x.id));
            self.clips.insert(i.min(self.clips.len()), Arc::new(c));
            made.push(first.id);
        }
        if !made.is_empty() {
            self.selection = made.clone();
        }
        (made, mixed)
    }

    /// ← → with clips selected (R51): moves them by `dt` ticks (never
    /// before the song start); they win overlaps.
    pub fn nudge_selected(&mut self, dt: i64) -> bool {
        let sel = self.selection.clone();
        let Some(min) = sel.iter().filter_map(|id| self.clip(*id)).map(|c| c.start_ticks).min() else { return false };
        let dt = dt.max(-(min as i64));
        if dt == 0 {
            return false;
        }
        for id in &sel {
            if let Some(c) = self.clip_mut(*id) {
                c.start_ticks = (c.start_ticks as i64 + dt) as u32;
            }
        }
        self.resolve_overlaps(&sel);
        true
    }

    /// The song's end: the last clip end (0 when there are none).
    pub fn song_end(&self) -> u32 {
        self.clips.iter().map(|c| c.end_ticks()).max().unwrap_or(0)
    }
}

/// Zoom that shows ticks `start..end` in `width` px with a margin, and the
/// scroll that puts `start` (minus the margin) at the left edge.
pub fn view_for(start: u32, end: u32, width: f32) -> Viewport {
    const MARGIN: f64 = 0.04;
    let span = (end.max(start + 1) - start) as f64;
    let ppt = width.max(1.0) as f64 / (span * (1.0 + 2.0 * MARGIN));
    let ppb = (ppt * TICKS_PER_BAR as f64).clamp(Viewport::MIN_PX_PER_BAR, Viewport::MAX_PX_PER_BAR);
    let px_per_tick = ppb / TICKS_PER_BAR as f64;
    let shown = width as f64 / px_per_tick;
    let scroll = (start as f64 + span * 0.5 - shown * 0.5).max(0.0);
    Viewport { px_per_tick, scroll_ticks: scroll }
}

#[cfg(test)]
mod edit_tests {
    use super::*;

    fn clip(id: u32, lane: usize, start: u32, len: u32) -> Clip {
        // One note per quarter.
        let notes = (0..len / PPQ).map(|q| Note::new(q + 1, q * PPQ, PPQ / 2, 60 + q as u8)).collect();
        Clip::new(id, lane, start, len, format!("c{id}"), notes)
    }

    fn arr(clips: Vec<Clip>) -> Arrangement {
        let next_id = clips.iter().map(|c| c.id).max().unwrap_or(0) + 1;
        Arrangement { clips: clips.into_iter().map(Arc::new).collect(), next_id, ..Arrangement::empty() }
    }

    fn spans(a: &Arrangement, lane: usize) -> Vec<(u32, u32)> {
        let mut v: Vec<_> = a.clips.iter().filter(|c| c.lane == lane).map(|c| (c.start_ticks, c.end_ticks())).collect();
        v.sort();
        v
    }

    fn song_notes(a: &Arrangement, lane: usize) -> Vec<(u32, u8)> {
        let mut v: Vec<_> = a
            .clips
            .iter()
            .filter(|c| c.lane == lane)
            .flat_map(|c| c.audible_notes().map(move |n| (c.start_ticks + n.start, n.pitch)))
            .collect();
        v.sort();
        v
    }

    const B: u32 = TICKS_PER_BAR;

    /// AT-11 [unit]: bars 1–4 split at bar 3, then consolidated again.
    #[test]
    fn split_then_consolidate_keeps_every_note() {
        let mut a = arr(vec![clip(1, 0, 0, 4 * B)]);
        let before = song_notes(&a, 0);
        let r = a.split_clip(1, 2 * B).unwrap();
        assert_eq!(spans(&a, 0), [(0, 2 * B), (2 * B, 4 * B)]);
        assert_eq!(song_notes(&a, 0), before);
        let (made, mixed) = a.consolidate(&[1, r], None);
        assert!(mixed.is_empty());
        assert_eq!(made, [1]);
        assert_eq!(spans(&a, 0), [(0, 4 * B)]);
        assert_eq!(song_notes(&a, 0), before);
        assert!(notes::is_mono(&a.clip(1).unwrap().notes));
    }

    #[test]
    fn split_keeps_the_instrument_and_colour() {
        let mut c = clip(1, 0, 0, 4 * B);
        c.instrument = Some(7);
        c.color = Some(Rgba::hex(0x4FA8E0));
        let mut a = arr(vec![c]);
        let r = a.split_clip(1, B).unwrap();
        assert_eq!(a.clip(r).unwrap().instrument, Some(7));
        assert_eq!(a.clip(r).unwrap().color, Some(Rgba::hex(0x4FA8E0)));
    }

    #[test]
    fn consolidate_refuses_mixed_instruments() {
        let mut b = clip(2, 0, 4 * B, B);
        b.instrument = Some(9);
        let mut a = arr(vec![clip(1, 0, 0, B), b]);
        let (made, mixed) = a.consolidate(&[1, 2], None);
        assert!(made.is_empty());
        assert_eq!(mixed, [0]);
        assert_eq!(a.clips.len(), 2);
    }

    /// AT-22 [unit]: Delete on a time selection across two tracks.
    #[test]
    fn clear_range_splits_at_the_edges() {
        let mut a = arr(vec![clip(1, 0, 0, 4 * B), clip(2, 1, 0, 4 * B), clip(3, 2, 0, 4 * B)]);
        assert!(a.clear_range(B, 3 * B, 0..=1));
        for lane in [0, 1] {
            assert_eq!(spans(&a, lane), [(0, B), (3 * B, 4 * B)]);
            assert!(song_notes(&a, lane).iter().all(|(t, _)| *t < B || *t >= 3 * B));
        }
        assert_eq!(spans(&a, 2), [(0, 4 * B)], "track 3 was outside the selection");
    }

    /// AT-07 [unit]: copy two clips, paste at bar 9.
    #[test]
    fn copy_paste_clips_at_the_insert_marker() {
        let mut a = arr(vec![clip(1, 0, 0, B), clip(2, 1, B, B)]);
        let cb = a.copy_clips(&[1, 2]).unwrap();
        let new = a.paste(&cb, 8 * B, 0, &|_| true);
        assert_eq!(new.len(), 2);
        assert_eq!(a.clips.len(), 4);
        let c1 = a.clip(new[0]).unwrap();
        assert_eq!((c1.lane, c1.start_ticks), (0, 8 * B));
        assert_eq!(c1.notes, a.clip(1).unwrap().notes);
        let c2 = a.clip(new[1]).unwrap();
        assert_eq!((c2.lane, c2.start_ticks), (1, 9 * B));
        assert_eq!(a.selection, new);
        // A refused track is skipped.
        let more = a.paste(&cb, 12 * B, 0, &|l| l == 0);
        assert_eq!(more.len(), 1);
    }

    #[test]
    fn time_commands_move_every_track() {
        let mut a = arr(vec![clip(1, 0, 0, 4 * B), clip(2, 3, 2 * B, B)]);
        a.insert_time(B, 2 * B);
        assert_eq!(spans(&a, 0), [(0, B), (3 * B, 6 * B)]);
        assert_eq!(spans(&a, 3), [(4 * B, 5 * B)]);
        a.delete_time(B, 3 * B);
        assert_eq!(spans(&a, 0), [(0, B), (B, 4 * B)]);
        assert_eq!(spans(&a, 3), [(2 * B, 3 * B)]);
        // Copy Time + Paste Time = Duplicate Time.
        let cb = a.copy_range(0, B, 0..=7, true);
        a.insert_time(B, cb.len);
        a.paste(&cb, B, 5, &|_| true);
        assert_eq!(spans(&a, 0), [(0, B), (B, 2 * B), (2 * B, 5 * B)]);
    }

    #[test]
    fn nudge_and_adaptive_grid() {
        let mut a = arr(vec![clip(1, 0, PPQ, B)]);
        a.selection = vec![1];
        assert!(a.nudge_selected(-(4 * PPQ as i64)));
        assert_eq!(a.clip(1).unwrap().start_ticks, 0, "clamped at the song start");
        assert_eq!(a.grid().ticks, PPQ / 4);
        a.grid_opts.triplet = true;
        assert_eq!(a.grid().ticks, PPQ / 6);
        a.grid_opts = GridOpts { adaptive: true, ..GridOpts::default() };
        // 92 px per bar: 1/16 = 5.75 px, 1/8 = 11.5, 1/4 = 23 ≥ 16.
        assert_eq!(a.grid().ticks, PPQ);
        a.grid_opts.snap = false;
        assert_eq!(a.grid().round(1234), 1234);
    }

    #[test]
    fn zoom_to_a_range() {
        let v = view_for(4 * B, 6 * B, 1000.0);
        assert!(v.tick_to_x(4.0 * B as f64) > 0.0 && v.tick_to_x(6.0 * B as f64) < 1000.0);
        assert!(v.tick_to_x(6.0 * B as f64) - v.tick_to_x(4.0 * B as f64) > 900.0);
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
    /// Loop start in ticks (0 unless "loop this clip" is on).
    pub loop_start_ticks: f64,
}

impl Transport {
    pub const DEFAULT_BPM: f32 = 150.0;
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
#[derive(Debug, PartialEq, Default)]
pub struct Meters {
    /// Per track (by index), resized as tracks come and go (`fit`).
    pub level: Vec<f32>,
    pub peak: Vec<f32>,
    pub peak_hold: Vec<f32>,
    /// 0..1 brightness of the track's light (pulses on note-on).
    pub light: Vec<f32>,
    /// The Main track's meter: the output level before the master gain
    /// (the fader's meter applies the gain, like a track's), 0..1.
    pub main: f32,
    /// Each track's held output peak (track id, linear: 1.0 = 0 dBFS) since
    /// its last reset (Batch 4b G8): the clip mark lights above 1.0 and holds
    /// until clicked; the mixer's PEAK readout shows it. Keyed by id, so it
    /// follows a track that moves.
    pub held: Vec<(u32, f32)>,
    /// Main's stereo meter (Batch 4b §7): left and right before the master
    /// gain, 0..1, and their peak holds.
    pub main_lr: (f32, f32),
    pub main_peak_lr: (f32, f32),
    /// Voices sounding per studio track (id, count): the mixer's VOICES n.
    pub voices: Vec<(u32, u16)>,
    /// Per track (by index): its clip mark is lit (`clipped`), computed once
    /// per tick by `mark_clips` so the views don't search `held` per frame.
    pub clip: Vec<bool>,
}

/// `clone_from` reuses the vectors (the meter tick builds its next state in
/// a kept buffer every frame; the derived one would allocate them anew).
impl Clone for Meters {
    fn clone(&self) -> Self {
        let mut m = Self::default();
        m.clone_from(self);
        m
    }

    fn clone_from(&mut self, o: &Self) {
        self.level.clone_from(&o.level);
        self.peak.clone_from(&o.peak);
        self.peak_hold.clone_from(&o.peak_hold);
        self.light.clone_from(&o.light);
        self.main = o.main;
        self.held.clone_from(&o.held);
        self.main_lr = o.main_lr;
        self.main_peak_lr = o.main_peak_lr;
        self.voices.clone_from(&o.voices);
        self.clip.clone_from(&o.clip);
    }
}

impl Meters {
    /// One entry per track.
    pub fn fit(&mut self, n: usize) {
        for v in [&mut self.level, &mut self.peak, &mut self.peak_hold, &mut self.light] {
            v.resize(n, 0.0);
        }
    }

    pub fn level_of(&self, i: usize) -> f32 {
        self.level.get(i).copied().unwrap_or(0.0)
    }

    /// Voices sounding on track `id` now.
    pub fn voices_of(&self, id: u32) -> u16 {
        self.voices.iter().find(|v| v.0 == id).map_or(0, |v| v.1)
    }

    /// Track `id`'s held peak (1.0 = 0 dBFS; 0 when reset or silent).
    pub fn held_of(&self, id: u32) -> f32 {
        // `held` is kept sorted by id.
        self.held.binary_search_by_key(&id, |h| h.0).map_or(0.0, |i| self.held[i].1)
    }

    /// Track `i`'s clip mark (as of the last `mark_clips`).
    pub fn clip_at(&self, i: usize) -> bool {
        self.clip.get(i).copied().unwrap_or(false)
    }

    /// Recomputes every track's clip mark, tracks in order by id.
    pub fn mark_clips(&mut self, ids: impl Iterator<Item = u32>) {
        let mut n = 0;
        for id in ids {
            let lit = self.clipped(id);
            if n < self.clip.len() {
                self.clip[n] = lit;
            } else {
                self.clip.push(lit);
            }
            n += 1;
        }
        self.clip.truncate(n);
    }

    /// Track `i`'s clip mark off now (its held peak was just reset).
    pub fn unmark_clip(&mut self, i: usize) {
        if let Some(c) = self.clip.get_mut(i) {
            *c = false;
        }
    }

    /// Whether track `id`'s clip mark is lit: it went over 0 dBFS since its
    /// last reset.
    pub fn clipped(&self, id: u32) -> bool {
        self.held_of(id) > 1.0
    }

    /// Raises track `id`'s held peak to `v` (it only goes up until a reset).
    pub fn hold(&mut self, id: u32, v: f32) {
        debug_assert!(self.held.windows(2).all(|w| w[0].0 < w[1].0), "held stays sorted by id");
        match self.held.binary_search_by_key(&id, |h| h.0) {
            Ok(i) => self.held[i].1 = self.held[i].1.max(v),
            Err(i) if v > 0.0 => self.held.insert(i, (id, v)),
            Err(_) => {}
        }
    }

    /// Clears track `id`'s held peak (a click on its clip mark or PEAK).
    pub fn reset_held(&mut self, id: u32) {
        self.held.retain(|h| h.0 != id);
    }

    pub fn light_of(&self, i: usize) -> f32 {
        self.light.get(i).copied().unwrap_or(0.0)
    }
}

/// Most dials any channel shows (pulse: duty, volume, envelope, sweep, rate).
pub const MAX_DIALS: usize = 5;

/// What a dial can do right now. The rule: no control may look like it does
/// something when it doesn't (STABILIZE_PROMPT.md §A4.1).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct DialState {
    /// A label that replaces the spec's (Volume reads "Decay" in the
    /// hardware decay modes, where V is the decay speed).
    pub label: Option<&'static str>,
    /// Inert, and why ("set by instrument"): the value is kept and saved, and
    /// applies again when the instrument stops overriding it.
    pub locked: Option<&'static str>,
}

/// The dial states for `lane`, given its controls and instrument.
pub fn dial_states(lane: usize, params: &engine::Params, inst: Option<&crate::instruments::LaneInstrument>) -> [DialState; MAX_DIALS] {
    use chip::instrument::{Instrument, Seq};
    // A macro overrides a control if the instrument has it; for a kit, only
    // if every drum has it (otherwise the control still shapes some drums).
    let overrides = |which: fn(&Instrument) -> &Seq| -> Option<&'static str> {
        let i = inst?;
        if i.is_kit() {
            (!i.kit.is_empty() && i.kit.iter().all(|k| !which(&k.inst).is_empty())).then_some("set by kit")
        } else {
            (!which(&i.inst).is_empty()).then_some("set by instrument")
        }
    };
    let vol_macro = overrides(|i| &i.volume);
    let duty_macro = overrides(|i| &i.duty);
    let mut out = [DialState::default(); MAX_DIALS];
    for (slot, (pid, _)) in lane_dials(lane).into_iter().enumerate() {
        out[slot] = match pid {
            ParamId::Duty | ParamId::NoiseMode => DialState { label: None, locked: duty_macro },
            ParamId::Envelope => DialState { label: None, locked: vol_macro },
            ParamId::Volume if vol_macro.is_none() && params.get(lane, ParamId::Envelope) != 0 => {
                DialState { label: Some("Decay"), locked: None }
            }
            _ => DialState::default(),
        };
    }
    out
}

/// How a dial shows its value.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Fmt {
    Int,
    PulseDuty,
    Vrc6Duty,
    Envelope,
    Sweep,
    NoiseMode,
    NoisePeriod,
    Sample,
    OnOff,
    SawRate,
    /// Unison voices ("1 voice", "3 voices").
    Voices,
    /// Cents ("0 ct", "15 ct").
    Cents,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct DialSpec {
    pub label: &'static str,
    pub min: f32,
    pub max: f32,
    pub default: f32,
    /// Discrete steps (0 = continuous).
    pub steps: u32,
    pub fmt: Fmt,
}

impl DialSpec {
    pub fn format(&self, v: f32) -> String {
        let i = v.round() as i32;
        match self.fmt {
            Fmt::Int => format!("{i}"),
            Fmt::Voices if i == 1 => "1 voice".into(),
            Fmt::Voices => format!("{i} voices"),
            Fmt::Cents => format!("{i} ct"),
            Fmt::PulseDuty => ["12.5%", "25%", "50%", "75%"][i.clamp(0, 3) as usize].into(),
            Fmt::Vrc6Duty if i >= 8 => "digi".into(),
            Fmt::Vrc6Duty => format!("{}/16", i + 1),
            Fmt::Envelope => ["const", "decay", "loop"][i.clamp(0, 2) as usize].into(),
            Fmt::Sweep if i == 0 => "off".into(),
            Fmt::Sweep if i < 0 => format!("up {}", -i),
            Fmt::Sweep => format!("down {i}"),
            Fmt::NoiseMode => ["long", "short"][i.clamp(0, 1) as usize].into(),
            Fmt::NoisePeriod if i < 0 => "note".into(),
            Fmt::NoisePeriod => format!("{i}"),
            Fmt::Sample if i < 0 => "note".into(),
            Fmt::Sample => sample_name(i as usize).into(),
            Fmt::OnOff => if i != 0 { "on" } else { "off" }.into(),
            Fmt::SawRate if i > 42 => format!("{i} wrap"),
            Fmt::SawRate => format!("{i}"),
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

fn sample_name(i: usize) -> &'static str {
    ["Kick", "Snare", "Tom", "Crash"].get(i).copied().unwrap_or("?")
}

/// The dials a lane really has (hardware controls only), in display order.
pub fn lane_dials(lane: usize) -> Vec<(ParamId, DialSpec)> {
    ParamId::for_lane(lane)
        .iter()
        .map(|&p| {
            let (lo, hi, def) = p.range(lane);
            let (label, fmt) = match p {
                ParamId::Duty if lane >= 5 => ("Duty", Fmt::Vrc6Duty),
                ParamId::Duty => ("Duty", Fmt::PulseDuty),
                ParamId::Volume => ("Volume", Fmt::Int),
                ParamId::Envelope => ("Envelope", Fmt::Envelope),
                ParamId::Sweep => ("Sweep", Fmt::Sweep),
                ParamId::SweepRate => ("Sweep rate", Fmt::Int),
                ParamId::NoiseMode => ("Mode", Fmt::NoiseMode),
                ParamId::NoisePeriod => ("Period", Fmt::NoisePeriod),
                ParamId::Sample => ("Sample", Fmt::Sample),
                ParamId::Rate if lane == 7 => ("Rate", Fmt::SawRate),
                ParamId::Rate => ("Rate", Fmt::Int),
                ParamId::Loop => ("Loop", Fmt::OnOff),
            };
            (
                p,
                DialSpec {
                    label,
                    min: lo as f32,
                    max: hi as f32,
                    default: def as f32,
                    steps: (hi - lo) as u32,
                    fmt,
                },
            )
        })
        .collect()
}

/// Engine status for the F3 overlay and the error line.
#[derive(Clone, Debug, PartialEq, Default)]
pub struct AudioStats {
    pub device: String,
    pub running: bool,
    pub filter: &'static str,
    /// The filter follows the chip (false once the user picks one with F7).
    pub filter_auto: bool,
    pub frames_per_cb: u32,
    pub latency_ms: f32,
    pub cb_mean: f32,
    pub cb_p99: f32,
    pub cb_max: f32,
    pub underruns: u64,
    pub overruns: u64,
    pub edits: u64,
    pub edit_tick_ms: f32,
}

/// The project's **hardware profile** (Batch 4a §5): which real channels
/// locked tracks can use. (The type keeps its old name: before the model
/// change it was the chip setting, and the two older values are the NES and
/// Famicom + VRC6 profiles.)
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum SoundChip {
    /// None (studio only): no real channels; every track is a studio track.
    StudioOnly,
    /// NES (2A03): Pulse 1, Pulse 2, Triangle, Noise, DPCM.
    Base2A03,
    /// Famicom + VRC6: the 2A03 plus VRC6 Pulse 1, Pulse 2 and Saw.
    WithVrc6,
}

/// The hardware profile (the same type).
pub type Profile = SoundChip;

/// F-04 / Batch 4a: a track locked to a channel the profile doesn't have is
/// silent, so it is drawn dimmed and can't be edited ("nothing pretends to
/// work"); the status bar lists it as a problem. Studio tracks are never off.
pub fn lane_off(chip: SoundChip, a: &Arrangement, lane: usize) -> bool {
    a.lanes.get(lane).and_then(|l| l.lock).is_some_and(|c| !chip.has_channel(c))
}

/// What a dimmed track says on hover.
pub const VRC6_OFF_HINT: &str = "This track is locked to a channel the project's hardware profile doesn't have. Change the profile in File > Project Settings…, or unlock the track (its badge).";

impl SoundChip {
    pub const ALL: [SoundChip; 3] = [SoundChip::StudioOnly, SoundChip::Base2A03, SoundChip::WithVrc6];

    pub fn label(self) -> &'static str {
        match self {
            SoundChip::StudioOnly => "None (studio only)",
            SoundChip::Base2A03 => "NES (2A03)",
            SoundChip::WithVrc6 => "Famicom + VRC6",
        }
    }

    /// Whether the profile has console channel `c` (0..8).
    pub fn has_channel(self, c: usize) -> bool {
        match self {
            SoundChip::StudioOnly => false,
            SoundChip::Base2A03 => c < chip::driver::LANE_V1,
            SoundChip::WithVrc6 => c < chip::driver::LANES,
        }
    }

    /// The console's VRC6 is plugged in.
    pub fn vrc6(self) -> bool {
        self == SoundChip::WithVrc6
    }
}

/// Debug overlay stats (F3).
#[derive(Clone, Copy, Debug, PartialEq, Default)]
pub struct FrameStats {
    pub visible: bool,
    /// Active frame-rate cap and the display's refresh rate, for the header line.
    pub cap: FrameCap,
    pub display_hz: Option<f64>,
    /// Smoothed interval between presented frames, ms.
    pub frame_ms: f32,
    /// Worst presented-frame interval in the last second, ms.
    pub worst_ms: f32,
    /// Smoothed per-phase UI-thread time, ms, in `PHASE_NAMES` order.
    pub phases_ms: [f32; 7],
    /// Smoothed per-view custom draw time, ms, in `perf::SPANS` order.
    pub spans_ms: [f32; crate::perf::SPANS.len()],
    /// Process CPU, % of one core (all threads, incl. the GL driver's).
    pub cpu_percent: f32,
    pub clip_count: usize,
    pub frames: u64,
    /// Playing frames in the last second whose playhead came from the audio
    /// clock, and from the UI's own fallback clock (no stream, or a command
    /// in flight; ENGINE.md §5).
    pub clock_audio: u32,
    pub clock_local: u32,
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

/// Frame-rate cap (F6 cycles 60 → 120 → off; start value from
/// `CARTWAVE_FPS_CAP=60|120|off`).
///
/// The default is 60 fps (decided after PERF-macos.md §6): half the CPU and
/// energy of anything else on a 320 Hz display, with the most even cadence.
/// 120 is the opt-in "smooth motion" setting. `Off` presents as often as vsync
/// allows. The former "display rate" setting measured strictly worse than 120
/// and was removed; `display` is accepted as an alias for `off`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum FrameCap {
    Off,
    #[default]
    Fps60,
    Fps120,
}

impl FrameCap {
    pub fn from_env() -> Self {
        Self::parse(crate::dev::var("CARTWAVE_FPS_CAP").as_deref())
    }

    /// From Settings > Display (60, 120; 0 = off, dev mode only). In dev
    /// mode `CARTWAVE_FPS_CAP` overrides it (the benchmarks set it).
    pub fn from_setting(fps: u32) -> Self {
        if crate::dev::var("CARTWAVE_FPS_CAP").is_some() {
            return Self::from_env();
        }
        match fps {
            120 => Self::Fps120,
            0 if crate::dev::enabled() => Self::Off,
            _ => Self::Fps60,
        }
    }

    fn parse(v: Option<&str>) -> Self {
        match v {
            Some("off" | "display") => Self::Off,
            Some("120") => Self::Fps120,
            _ => Self::Fps60,
        }
    }

    pub fn next(self) -> Self {
        match self {
            Self::Fps60 => Self::Fps120,
            Self::Fps120 => Self::Off,
            Self::Off => Self::Fps60,
        }
    }

    /// The frame-rate limit to request from the backend.
    pub fn max_fps(self) -> Option<f64> {
        match self {
            Self::Off => None,
            Self::Fps60 => Some(60.0),
            Self::Fps120 => Some(120.0),
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Off => "off",
            Self::Fps60 => "60",
            Self::Fps120 => "120",
        }
    }
}

#[cfg(test)]
mod meters_tests {
    use super::*;

    /// Batch 4c §2: held peaks stay keyed by id (in id order, for the binary
    /// search) whatever order tracks report in; the clip marks are per track
    /// index and a reset turns one off at once.
    #[test]
    fn held_peaks_and_clip_marks() {
        let mut m = Meters::default();
        for (id, v) in [(9, 0.5), (3, 1.2), (7, 0.0), (5, 0.9), (3, 0.4), (9, 1.5)] {
            m.hold(id, v);
        }
        assert_eq!(m.held, vec![(3, 1.2), (5, 0.9), (9, 1.5)], "sorted, maxima kept, silence not stored");
        m.mark_clips([5, 3, 9, 7].into_iter());
        assert_eq!(m.clip, vec![false, true, true, false]);
        assert_eq!((m.clip_at(1), m.clip_at(9)), (true, false));
        m.reset_held(3);
        m.unmark_clip(1);
        assert_eq!((m.held_of(3), m.clip_at(1)), (0.0, false));
        m.mark_clips([9].into_iter());
        assert_eq!(m.clip, vec![true], "fewer tracks: the marks follow");
        let mut c = Meters::default();
        c.clone_from(&m);
        assert_eq!(c, m);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dials_say_when_the_instrument_overrides_them() {
        use crate::instruments::LaneInstrument;
        let p = engine::Params::default();
        let slot = |lane: usize, id: ParamId| lane_dials(lane).iter().position(|d| d.0 == id).unwrap();
        // Raw channel: everything live; pulse Volume is a level (constant envelope).
        let raw = LaneInstrument::plain();
        let st = dial_states(0, &p, Some(&raw));
        assert!(st.iter().all(|d| d.locked.is_none()));
        assert_eq!(st[slot(0, ParamId::Volume)].label, None);
        // A preset with a duty macro and a volume macro locks Duty and Envelope.
        let cello = LaneInstrument::named("Cello");
        assert!(!cello.inst.duty.is_empty() && !cello.inst.volume.is_empty());
        let st = dial_states(5, &p, Some(&cello));
        assert_eq!(st[slot(5, ParamId::Duty)].locked, Some("set by instrument"));
        assert_eq!(st[slot(5, ParamId::Volume)].locked, None, "the channel volume always works");
        let st = dial_states(0, &p, Some(&cello));
        assert_eq!(st[slot(0, ParamId::Envelope)].locked, Some("set by instrument"));
        // Decay envelope without a volume macro: the Volume dial is the decay speed.
        let mut q = p;
        q.set(0, ParamId::Envelope, 1);
        assert_eq!(dial_states(0, &q, Some(&raw))[slot(0, ParamId::Volume)].label, Some("Decay"));
        // A kit locks only what every drum overrides.
        let kit = LaneInstrument::named("Standard Kit");
        assert!(kit.is_kit());
        let all_vol = kit.kit.iter().all(|k| !k.inst.volume.is_empty());
        assert_eq!(dial_states(3, &p, Some(&kit))[slot(3, ParamId::Envelope)].locked.is_some(), all_vol);
    }


    #[test]
    fn frame_cap_default_and_aliases() {
        assert_eq!(FrameCap::default(), FrameCap::Fps60);
        assert_eq!(FrameCap::parse(None), FrameCap::Fps60);
        assert_eq!(FrameCap::parse(Some("120")), FrameCap::Fps120);
        assert_eq!(FrameCap::parse(Some("off")), FrameCap::Off);
        assert_eq!(FrameCap::parse(Some("display")), FrameCap::Off);
        let mut c = FrameCap::default();
        let cycle: Vec<_> = (0..3).map(|_| { c = c.next(); c }).collect();
        assert_eq!(cycle, [FrameCap::Fps120, FrameCap::Off, FrameCap::Fps60]);
    }

    fn clip(id: u32, lane: usize, start: u32, len: u32) -> Clip {
        // One note per quarter, absolute pitch = 60 + quarter index.
        let notes = (0..len / PPQ)
            .map(|q| Note::new(q + 1, q * PPQ, PPQ / 2, 60 + q as u8))
            .collect();
        Clip::new(id, lane, start, len, format!("c{id}"), notes)
    }

    fn arr(clips: Vec<Clip>) -> Arrangement {
        let next_id = clips.iter().map(|c| c.id).max().unwrap_or(0) + 1;
        Arrangement {
            clips: clips.into_iter().map(Arc::new).collect(),
            next_id,
            ..Arrangement::empty()
        }
    }

    fn spans(a: &Arrangement, lane: usize) -> Vec<(u32, u32, u32)> {
        let mut v: Vec<_> = a
            .clips
            .iter()
            .filter(|c| c.lane == lane)
            .map(|c| (c.id, c.start_ticks, c.end_ticks()))
            .collect();
        v.sort_by_key(|x| x.1);
        v
    }

    #[test]
    fn newer_clip_trims_end_and_start() {
        let q = PPQ;
        let mut a = arr(vec![clip(1, 0, 0, 8 * q), clip(2, 0, 12 * q, 8 * q), clip(3, 0, 6 * q, 8 * q)]);
        a.resolve_overlaps(&[3]);
        assert_eq!(spans(&a, 0), [(1, 0, 6 * q), (3, 6 * q, 14 * q), (2, 14 * q, 20 * q)]);
        // Notes keep their song positions: clip 2 lost its first two quarters.
        let c2 = a.clip(2).unwrap();
        assert_eq!(c2.notes.first().map(|n| (n.start, n.pitch)), Some((0, 62)));
    }

    #[test]
    fn dropped_in_the_middle_splits() {
        let q = PPQ;
        let mut a = arr(vec![clip(1, 0, 0, 16 * q), clip(2, 0, 4 * q, 4 * q)]);
        a.resolve_overlaps(&[2]);
        let s = spans(&a, 0);
        assert_eq!(s.len(), 3);
        assert_eq!((s[0].1, s[0].2), (0, 4 * q));
        assert_eq!(s[1], (2, 4 * q, 8 * q));
        assert_eq!((s[2].1, s[2].2), (8 * q, 16 * q));
        assert!(s[2].0 > 2, "the right part is a new clip");
        let right = a.clip(s[2].0).unwrap();
        assert_eq!(right.notes.first().map(|n| (n.start, n.pitch)), Some((0, 68)));
    }

    #[test]
    fn covered_clip_is_removed_other_lanes_untouched() {
        let q = PPQ;
        let mut a = arr(vec![clip(1, 0, 2 * q, 2 * q), clip(2, 0, 0, 8 * q), clip(3, 1, 0, 8 * q)]);
        a.selection = vec![1];
        a.resolve_overlaps(&[2]);
        assert_eq!(spans(&a, 0), [(2, 0, 8 * q)]);
        assert_eq!(spans(&a, 1), [(3, 0, 8 * q)]);
        assert!(a.selection.is_empty(), "removed clips leave the selection");
    }

    #[test]
    fn snapshot_never_overlaps_while_dragging() {
        let q = PPQ;
        let mut a = arr(vec![clip(1, 0, 0, 8 * q), clip(2, 0, 4 * q, 8 * q)]);
        // Mid-drag: clip 2 overlaps clip 1, the model isn't trimmed yet...
        a.editing = vec![2];
        let snap = a.song_snapshot(1);
        assert_eq!(a.clips.len(), 2);
        assert_eq!(a.clip(1).unwrap().len_ticks, 8 * q);
        // ...but the engine hears clip 2 winning.
        for w in snap.lanes[0].windows(2) {
            assert!(w[0].end <= w[1].start, "{w:?}");
        }
        let last_of_1 = snap.lanes[0].iter().rfind(|n| n.start < 4 * q).unwrap();
        assert!(last_of_1.end <= 4 * q);
    }

    #[test]
    fn content_key_ignores_selection_and_view() {
        let mut a = arr(vec![clip(1, 0, 0, 8 * PPQ)]);
        let k = a.content_key();
        a.selection = vec![1];
        a.view.scroll_ticks = 100.0;
        assert_eq!(a.content_key(), k);
        a.clip_mut(1).unwrap().start_ticks += 240;
        assert_ne!(a.content_key(), k);
    }
}

/// The piano roll's state: which clip is open, its view, and the note
/// selection. Not part of undo or the file.
#[derive(Clone, Debug, PartialEq)]
pub struct PianoState {
    /// The open clip (`None`: the device panel is shown).
    pub clip: Option<u32>,
    pub grid: crate::notes::Grid,
    /// Page the view along with the playhead while playing.
    pub follow: bool,
    /// Playback loops over the open clip.
    pub loop_clip: bool,
    /// Selected note ids (in the open clip).
    pub sel: Vec<u32>,
    /// Logical px per tick.
    pub px_per_tick: f64,
    /// Tick (relative to the clip start) at the grid's left edge.
    pub scroll_ticks: f64,
    /// Row height, logical px.
    pub row_h: f32,
    /// Rows scrolled down from the top row.
    pub top: f32,
    /// Grid width in logical px (written by the view; used to follow).
    pub width_px: f32,
    /// Hover read-out: the row's note, period and tuning error.
    pub hover: String,
    /// The lane's Volume control, drawn for notes without their own volume.
    pub lane_volume: u8,
    /// Draw Mode (B, R5): a click draws a note; off, a click selects, a drag
    /// selects several and a double-click adds or deletes a note (P1–P4).
    pub draw: bool,
    /// Earlier zooms (Z / X in the piano roll): (px per tick, scroll).
    pub zoom_back: Vec<(f64, f64)>,
}

impl Default for PianoState {
    fn default() -> Self {
        Self {
            clip: None,
            grid: crate::notes::Grid::Sixteenth,
            follow: true,
            loop_clip: false,
            sel: Vec::new(),
            // One bar = 256 logical px.
            px_per_tick: 256.0 / TICKS_PER_BAR as f64,
            scroll_ticks: 0.0,
            row_h: 12.0,
            top: 0.0,
            width_px: 800.0,
            hover: String::new(),
            lane_volume: 12,
            draw: false,
            zoom_back: Vec::new(),
        }
    }
}

impl PianoState {
    pub const MIN_PX_PER_BAR: f64 = 32.0;
    pub const MAX_PX_PER_BAR: f64 = 4096.0;
}

/// Lane height in the arrangement: normal, and compact while the piano roll
/// is open (so the piano roll gets the room; PIANOROLL.md §2).
/// A track row's pitch: 72 px plus its 1 px hairline (Option D).
pub const LANE_H_NORMAL: f32 = 73.0;
/// Lane viewport height (below the ruler) while the piano roll is open: the
/// default (four lanes) and the smallest the split handle allows (one lane).
/// Lanes keep their full height and scroll vertically.
pub const PIANO_PANE_DEFAULT: f32 = 4.0 * LANE_H_NORMAL;
pub const PIANO_PANE_MIN: f32 = LANE_H_NORMAL;

impl Arrangement {
    /// New clips' colours (Settings ▸ Editing ▸ New clip colour): with
    /// `random`, each gets a palette colour distinct from the clips next to
    /// it on its track and from the track's colour; else the track's
    /// (no colour of its own).
    pub fn colour_new_clips(&mut self, ids: &[u32], random: bool) {
        if !random {
            return;
        }
        for id in ids {
            let Some((lane, start, end)) = self.clip(*id).map(|c| (c.lane, c.start_ticks, c.end_ticks())) else { continue };
            let track = self.lanes[lane].color;
            // The nearest clip before and after on the track.
            let mut before: Option<(u32, Rgba)> = None;
            let mut after: Option<(u32, Rgba)> = None;
            for c in self.clips.iter().filter(|c| c.lane == lane && c.id != *id) {
                let col = c.color.unwrap_or(track);
                if c.end_ticks() <= start && before.is_none_or(|(e, _)| c.end_ticks() > e) {
                    before = Some((c.end_ticks(), col));
                }
                if c.start_ticks >= end && after.is_none_or(|(s, _)| c.start_ticks < s) {
                    after = Some((c.start_ticks, col));
                }
            }
            let near: Vec<Rgba> = before.into_iter().chain(after).map(|(_, c)| c).collect();
            let pick = crate::theme::palette::next_clip_color(&near, track, false);
            if let Some(c) = self.clip_mut(*id) {
                c.color = Some(pick);
            }
        }
    }
}

/// The volume fader's thumb centre, px from the well's top, for `db` on a
/// well `len` px long (polish 2 §3/§4: `round((1 − pos) · len)`).
pub fn fader_y(db: f32, len: f32) -> f32 {
    ((1.0 - db_to_pos(db)) * len).round()
}

/// The post-fader meter's fill height (px from the well's bottom) for a
/// pre-fader `level` (0..1 of full scale) and the track's `db`, on the
/// fader's own dB scale, never past the thumb's centre.
pub fn meter_fill(level: f32, db: f32, len: f32) -> f32 {
    let post = gain_to_db(level.clamp(0.0, 1.0) * db_to_gain(db));
    let fill = (db_to_pos(post) * len).round();
    fill.min(len - fader_y(db, len)).max(0.0)
}

/// The dB scale marks beside a tall fader (polish 2 §4: rows ≥ 96 px):
/// 0, −6, −12 and −24 dB, as (tick y from the well's top, label, shown). A
/// label closer than 12 px to the last shown one is skipped; ticks stay.
pub const SCALE_MIN_ROW_H: f32 = 96.0;
pub const SCALE_MARKS: [f32; 4] = [0.0, -6.0, -12.0, -24.0];
pub fn scale_marks(len: f32) -> Vec<(f32, &'static str, bool)> {
    let mut out = Vec::new();
    let mut last: Option<f32> = None;
    for (db, label) in SCALE_MARKS.iter().zip(["0", "6", "12", "24"]) {
        let y = fader_y(*db, len);
        let show = last.is_none_or(|l| (y - l).abs() >= 12.0);
        if show {
            last = Some(y);
        }
        out.push((y, label, show));
    }
    out
}

#[cfg(test)]
mod fader_tests {
    use super::*;

    /// Polish 2 §3: the dB scale, its round trip and the meter rule.
    #[test]
    fn db_scale_round_trips() {
        assert_eq!(db_to_pos(f32::NEG_INFINITY), 0.0);
        assert_eq!(db_to_pos(-48.0), 0.0);
        assert!((db_to_pos(0.0) - 48.0 / 54.0).abs() < 1e-6);
        assert_eq!(db_to_pos(6.0), 1.0);
        assert_eq!(db_to_pos(12.0), 1.0, "clamped");
        assert_eq!(pos_to_db(0.0), f32::NEG_INFINITY);
        for db in [-47.5f32, -24.0, -6.0, -2.2, 0.0, 2.5, 6.0] {
            assert!((pos_to_db(db_to_pos(db)) - db).abs() < 1e-4, "{db}");
        }
        assert_eq!(db_to_gain(0.0), 1.0);
        assert_eq!(db_to_gain(f32::NEG_INFINITY), 0.0);
        assert!((db_to_gain(6.0) - 1.9953).abs() < 1e-3 && db_to_gain(9.0) <= 2.0);
        assert!((gain_to_db(db_to_gain(-3.0)) + 3.0).abs() < 1e-4);
        // The mockup's 144 / 240 px rows: −1.3 dB → thumb centre 15, −3.1 → 35.
        assert_eq!(fader_y(-1.3, 112.0), 15.0);
        assert_eq!(fader_y(-3.1, 208.0), 35.0);
        assert_eq!(fader_y(-2.2, 40.0), 6.0);
    }

    #[test]
    fn meter_never_passes_the_thumb() {
        for db in [f32::NEG_INFINITY, -30.0, -6.0, 0.0, 3.0, 6.0] {
            for level in [0.0, 0.2, 0.5, 0.9, 1.0, 1.5] {
                for len in [40.0, 64.0, 112.0, 268.0] {
                    let f = meter_fill(level, db, len);
                    assert!(f >= 0.0 && f <= len - fader_y(db, len), "{db} {level} {len}: {f}");
                }
            }
        }
        // Full level at 0 dB reaches the 0 dB mark, where the gradient turns hot.
        assert_eq!(meter_fill(1.0, 0.0, 54.0), 48.0);
    }

    #[test]
    fn value_parsing() {
        assert_eq!(parse_db("-3"), Some(-3.0));
        assert_eq!(parse_db("-3 dB"), Some(-3.0));
        assert_eq!(parse_db("\u{2212}3"), Some(-3.0));
        assert_eq!(parse_db("+2.5"), Some(2.5));
        assert_eq!(parse_db("-inf"), Some(f32::NEG_INFINITY));
        assert_eq!(parse_db("\u{2212}inf dB"), Some(f32::NEG_INFINITY));
        assert_eq!(parse_db("12"), Some(6.0), "clamped");
        assert_eq!(parse_db("-200"), Some(VOL_DB_MIN_TYPED));
        assert_eq!(parse_db("loud"), None);
        assert_eq!(parse_db(""), None);
        assert_eq!(format_db(-2.2), "\u{2212}2.2 dB");
        assert_eq!(format_db(-0.04), "0.0 dB");
        assert_eq!(format_db(f32::NEG_INFINITY), "\u{2212}inf dB");
        assert_eq!(format_db(2.5), "2.5 dB");
        assert_eq!(parse_pan("C"), Some(0));
        assert_eq!(parse_pan("0"), Some(0));
        assert_eq!(parse_pan("25L"), Some(-25));
        assert_eq!(parse_pan("30r"), Some(30));
        assert_eq!(parse_pan("-25"), Some(-25));
        assert_eq!(parse_pan("+25"), Some(25));
        assert_eq!(parse_pan("80R"), Some(50), "clamped");
        assert_eq!(parse_pan("L"), None);
        assert_eq!(parse_pan("x"), None);
        for v in [-50i8, -13, 0, 15, 50] {
            assert_eq!(parse_pan(&format_pan(v)), Some(v));
        }
        assert_eq!(clamp_row_h(10.0), 72.0);
        assert_eq!(clamp_row_h(145.0), 144.0);
        assert_eq!(clamp_row_h(999.0), 300.0);
    }

    /// Polish 2 §4: marks at 0/6/12/24; a label under 12 px from the last
    /// shown one is skipped (the mockup's 144 and 240 px rows show all four).
    #[test]
    fn scale_marks_skip_crowded_labels() {
        let m = scale_marks(112.0);
        assert_eq!(m.iter().map(|m| m.0).collect::<Vec<_>>(), vec![12.0, 25.0, 37.0, 62.0]);
        assert!(m.iter().all(|m| m.2));
        let m = scale_marks(64.0); // a 96 px row
        assert_eq!(m.iter().map(|m| m.2).collect::<Vec<_>>(), vec![true, false, true, true]);
    }

    #[test]
    fn rows_have_their_own_heights() {
        let mut a = Arrangement::empty();
        a.lanes[1].height = 144.0;
        assert_eq!(a.lane_top(0), 0.0);
        assert_eq!(a.lane_top(1), 73.0);
        assert_eq!(a.lane_top(2), 73.0 + 145.0);
        assert_eq!(a.lane_at(72.9), Some(0));
        assert_eq!(a.lane_at(73.0), Some(1));
        assert_eq!(a.lane_at(217.9), Some(1));
        assert_eq!(a.lane_at(218.0), Some(2));
        assert_eq!(a.lane_at(-1.0), None);
        assert_eq!(a.lane_at(1.0e6), None);
        assert_eq!(a.lane_at_clamped(1.0e6), 7);
        assert_eq!(a.lanes_height(), 7.0 * 73.0 + 145.0);
    }
}
