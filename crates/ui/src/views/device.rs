//! The device panel: the selected track's instrument, its one home
//! (PRESETS_PROMPT.md §3, §4; VISION.md "Instruments"; the
//! `workflow-chip-synth` board, locked for layout).
//!
//! **Chip Synth** (Pulse, Triangle, Noise, VRC6 Pulse, VRC6 Saw) and the
//! **DPCM Sampler** (the DPCM voice). One custom-drawn view per device:
//!
//! - **Header** (40 px): the track-colour tab, the name, a STUDIO or LOCKED
//!   badge, and centred the preset field between ◀ ▶ (◀ ▶ step through the
//!   preset's folder and load it at once; the field opens the picker).
//! - **OSCILLATOR**: the Voice selector in its title row (the board), a
//!   display of the voice's real waveform at its current duty / mode / rate
//!   in the warm display colour, and its controls: Duty (pulse), Mode
//!   (noise), Rate (saw), then Unison, Detune, Drift (studio only); a second
//!   row of small knobs for the channel's other hardware controls (Sweep,
//!   Sweep rate, the noise Period).
//! - **ENVELOPES**: tabs VOLUME (the triangle's GATE) / ARPEGGIO / PITCH /
//!   DUTY (the noise's MODE) over one editor (the old inspector's macro
//!   graphs, moved), and Vibrato Depth, Rate, Delay and Arp Speed.
//! - **OUTPUT**: the large Volume knob (the chip's Volume register: the
//!   instrument's level, not the track fader, which is the mixer's), the
//!   hardware Envelope mode under it, and a small level meter.
//!
//! Controls a voice doesn't have are **hidden**, not disabled (the
//! parameter registry, `params.rs` / `ParamId::for_lane`, decides). On a
//! locked track Unison, Detune and Drift are gone, the voice is the
//! channel's (the lock glyph), the envelopes read "4-BIT · PER FRAME", and
//! displays and knobs are drawn at 40 % with labels at full ink (≥ 4.5 : 1);
//! the header stays at full brightness.
//!
//! **Performance.** Everything but the meter's lit segments is drawn into
//! one cached image, rebuilt only when what it shows changes (an edit, a
//! hover, a drag); a frame while playing blits it and draws 22 segments.

use std::cell::RefCell;

use chip::driver::ParamId;
use chip::instrument::{Seq, NONE};
use chip::voice::VoiceKind;
use vizia::prelude::*;
use vizia::vg;

use super::gfx::{self, Face};
use crate::instruments::{InstId, LaneInstrument};
use crate::state::{lane_dials, DialState, Unison};
use crate::theme::tokens::*;
use crate::{AppEvent, Signals};

/// Steps of a macro started by drawing on an unused one.
pub const NEW_SEQ_LEN: usize = 16;
/// The value a new macro starts at (full volume, no arpeggio, no pitch, 50 %).
pub const NEUTRAL: [i8; 4] = [15, 0, 0, 8];
/// The four macros: (tab label, value range shown).
pub const MACROS: [(&str, i32, i32); 4] = [("VOLUME", 0, 15), ("ARPEGGIO", -12, 12), ("PITCH", -32, 32), ("DUTY", 0, 16)];

/// Whether a voice's driver uses a macro (I2: the others are hidden). The
/// triangle's Volume works as a gate; its duty and every DPCM macro don't.
pub fn macro_used(voice: VoiceKind, m: usize) -> bool {
    !matches!((voice, m), (VoiceKind::Triangle, 3) | (VoiceKind::Dpcm, _))
}

/// A macro's tab label on a voice.
pub fn macro_label(voice: VoiceKind, m: usize) -> &'static str {
    match (voice, m) {
        (VoiceKind::Triangle, 0) => "GATE",
        (VoiceKind::Noise, 3) => "MODE",
        _ => MACROS[m].0,
    }
}

// ───────────────────────────── geometry (logical px, the device box) ─────────────────────────────

const HEADER_H: f32 = 40.0;
const KEY: f32 = 26.0;
const FIELD_W: f32 = 260.0;
const KEY_GAP: f32 = 6.0;
const SEC_Y: f32 = 52.0;
const SEC_H: f32 = 268.0;
/// The three sections: (x, width).
const OSC: (f32, f32) = (12.0, 256.0);
const ENV: (f32, f32) = (280.0, 400.0);
const OUT: (f32, f32) = (692.0, 176.0);
/// Inside a section: 12 px side padding, the title row 22 px from 10 px.
const PAD: f32 = 12.0;
const TITLE_Y: f32 = 10.0;
const TITLE_H: f32 = 22.0;
const DISPLAY_Y: f32 = 38.0;
const DISPLAY_H: f32 = 96.0;
const KNOB_ROW_Y: f32 = 146.0;
const KNOB_D: f32 = 40.0;
const KNOB_CELL: f32 = 64.0;
const SMALL_ROW_Y: f32 = 212.0;
const SMALL_D: f32 = 28.0;
const BIG_D: f32 = 64.0;
/// The OUTPUT row: knob column (64) + 18 + the meter (10), centred, from y 54.
const OUT_ROW_Y: f32 = 54.0;
const METER_W: f32 = 10.0;
const METER_H: f32 = 96.0;
const METER_SEGS: usize = 22;
/// Drag distance per step (logical px), and the finer one with Shift.
const DRAG_PX: f32 = 4.0;
const DRAG_PX_FINE: f32 = 12.0;
/// How close a press must be to a loop / release marker to grab it.
const MARKER_GRAB_PX: f32 = 5.0;

/// A control the device shows.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Ctl {
    /// A channel control (the chip's register).
    Param(ParamId),
    /// Unison (0), Detune (1), Drift (2): studio tracks only.
    Uni(usize),
    /// Vibrato depth (0), rate (1), delay (2).
    Vib(usize),
    ArpSpeed,
}

#[derive(Clone, Debug, PartialEq)]
struct Knob {
    ctl: Ctl,
    label: &'static str,
    /// Centre and diameter (device-local logical px).
    c: (f32, f32),
    d: f32,
    min: i32,
    max: i32,
    def: i32,
    value: i32,
    /// Inert, and why (an instrument macro drives it).
    inert: Option<&'static str>,
}

/// What the device shows for the selected track (read from the model).
#[derive(Clone, Debug, PartialEq)]
struct Dev {
    lane: usize,
    voice: VoiceKind,
    lock: Option<usize>,
    color: Rgba,
    chan: usize,
    params: crate::state::TrackParams,
    unison: Unison,
    inst_id: InstId,
    inst: LaneInstrument,
    states: [DialState; crate::state::MAX_DIALS],
}

impl Dev {
    fn of(s: &Signals) -> Option<Dev> {
        let lane = s.lane_sel.get_untracked();
        let states = s.dial_state.get_untracked();
        s.arrangement.with_untracked(|a| {
            let l = a.lanes.get(lane)?;
            let id = a.current_instrument(lane);
            let inst = a.instruments.get(id)?.clone();
            Some(Dev { lane, voice: l.voice, lock: l.lock, color: l.color, chan: l.chan(), params: l.params, unison: l.unison, inst_id: id, inst, states })
        })
    }

    fn locked(&self) -> bool {
        self.lock.is_some()
    }

    fn param(&self, id: ParamId) -> i32 {
        self.params[id as usize] as i32
    }

    /// The inert reason of a channel control (its dial slot's state).
    fn inert(&self, id: ParamId) -> Option<&'static str> {
        lane_dials(self.chan).iter().position(|(p, _)| *p == id).and_then(|slot| self.states.get(slot).and_then(|st| st.locked))
    }

    fn has(&self, id: ParamId) -> bool {
        ParamId::for_lane(self.chan).contains(&id)
    }
}

/// The knobs for a track (positions included), in the board's places.
fn knobs(d: &Dev) -> Vec<Knob> {
    let mut v = Vec::new();
    let studio = !d.locked();
    let param = |id: ParamId, label: &'static str| -> Knob {
        let (lo, hi, def) = id.range(d.chan);
        Knob { ctl: Ctl::Param(id), label, c: (0.0, 0.0), d: KNOB_D, min: lo, max: hi, def, value: d.param(id), inert: d.inert(id) }
    };
    if d.voice == VoiceKind::Dpcm {
        // The DPCM Sampler: Rate in its PLAYBACK row.
        let mut row = vec![param(ParamId::Rate, "RATE")];
        place_row(&mut row, ENV.0 + PAD, ENV.1 - 2.0 * PAD, SEC_Y + KNOB_ROW_Y, KNOB_D, false);
        v.extend(row);
        return v;
    }
    // OSCILLATOR, first row (the board's): the voice's own control, then
    // Unison and Detune (studio).
    let mut row = Vec::new();
    match d.voice {
        VoiceKind::Pulse | VoiceKind::Vrc6Pulse => row.push(param(ParamId::Duty, "DUTY")),
        VoiceKind::Noise => row.push(param(ParamId::NoiseMode, "MODE")),
        VoiceKind::Saw => row.push(param(ParamId::Rate, "RATE")),
        _ => {}
    }
    let u = d.unison;
    if studio {
        row.push(Knob { ctl: Ctl::Uni(0), label: "UNISON", c: (0.0, 0.0), d: KNOB_D, min: 1, max: engine::studio::UNISON_MAX as i32, def: 1, value: u.voices as i32, inert: None });
        row.push(Knob { ctl: Ctl::Uni(1), label: "DETUNE", c: (0.0, 0.0), d: KNOB_D, min: 0, max: engine::studio::DETUNE_MAX as i32, def: 0, value: u.detune as i32, inert: None });
    }
    place_row(&mut row, OSC.0 + PAD, OSC.1 - 2.0 * PAD, SEC_Y + KNOB_ROW_Y, KNOB_D, false);
    v.extend(row);
    // OSCILLATOR, second row (small): Drift (studio; the board leaves it
    // out, the prompt lists it), then the channel's other hardware controls.
    let mut row2 = Vec::new();
    if studio {
        row2.push(Knob { ctl: Ctl::Uni(2), label: "DRIFT", c: (0.0, 0.0), d: SMALL_D, min: 0, max: engine::studio::DRIFT_MAX as i32, def: 0, value: u.drift as i32, inert: None });
    }
    if d.has(ParamId::Sweep) {
        row2.push(Knob { d: SMALL_D, ..param(ParamId::Sweep, "SWEEP") });
        row2.push(Knob { d: SMALL_D, ..param(ParamId::SweepRate, "SWEEP RATE") });
    }
    if d.has(ParamId::NoisePeriod) {
        row2.push(Knob { d: SMALL_D, ..param(ParamId::NoisePeriod, "PERIOD") });
    }
    place_row(&mut row2, OSC.0 + PAD, OSC.1 - 2.0 * PAD, SEC_Y + SMALL_ROW_Y, SMALL_D, true);
    v.extend(row2);
    // ENVELOPES: the board's Vibrato Depth, Rate and Arp Speed; Vibrato
    // Delay (an existing control) under them.
    if !d.inst.is_kit() {
        let vib = d.inst.inst.vibrato;
        let mut row = vec![
            Knob { ctl: Ctl::Vib(0), label: "VIB DEPTH", c: (0.0, 0.0), d: KNOB_D, min: 0, max: 100, def: 0, value: vib.depth as i32, inert: None },
            Knob { ctl: Ctl::Vib(1), label: "VIB RATE", c: (0.0, 0.0), d: KNOB_D, min: 1, max: 30, def: 6, value: vib.period.max(1) as i32, inert: None },
            Knob { ctl: Ctl::ArpSpeed, label: "ARP SPEED", c: (0.0, 0.0), d: KNOB_D, min: 1, max: chip::instrument::ARP_SPEED_MAX as i32, def: 1, value: d.inst.inst.arp_speed.max(1) as i32, inert: None },
        ];
        place_row(&mut row, ENV.0 + PAD, ENV.1 - 2.0 * PAD, SEC_Y + KNOB_ROW_Y, KNOB_D, true);
        v.extend(row);
        let mut row2 = vec![Knob { ctl: Ctl::Vib(2), label: "VIB DELAY", c: (0.0, 0.0), d: SMALL_D, min: 0, max: 60, def: 0, value: vib.delay as i32, inert: None }];
        place_row(&mut row2, ENV.0 + PAD, ENV.1 - 2.0 * PAD, SEC_Y + SMALL_ROW_Y, SMALL_D, true);
        v.extend(row2);
    }
    // OUTPUT: the chip's level control, large; the hardware envelope under it.
    if d.has(ParamId::Volume) {
        let label = if d.states.iter().any(|s| s.label == Some("Decay")) && d.param(ParamId::Envelope) != 0 { "DECAY" } else { "VOLUME" };
        let x = OUT.0 + PAD + (OUT.1 - 2.0 * PAD - (BIG_D + 18.0 + METER_W)) * 0.5 + BIG_D * 0.5;
        let y = SEC_Y + OUT_ROW_Y + (METER_H - (BIG_D + 8.0 + 12.0)) + BIG_D * 0.5;
        v.push(Knob { c: (x, y), d: BIG_D, ..param(ParamId::Volume, label) });
    }
    if d.has(ParamId::Envelope) {
        let mut row = vec![Knob { d: SMALL_D, ..param(ParamId::Envelope, "ENVELOPE") }];
        place_row(&mut row, OUT.0 + PAD, OUT.1 - 2.0 * PAD, SEC_Y + SMALL_ROW_Y, SMALL_D, true);
        v.extend(row);
    }
    v
}

/// Lays a row of knobs out in a section's content box (x, w) at row top y:
/// 64 px cells, `space-between` (the OSCILLATOR row) or `space-around`
/// (the ENVELOPES row, and centred rows); narrower cells when they don't fit.
fn place_row(row: &mut [Knob], x: f32, w: f32, y: f32, d: f32, around: bool) {
    let n = row.len();
    if n == 0 {
        return;
    }
    let cell = KNOB_CELL.min(w / n as f32);
    let xs: Vec<f32> = if around || n == 1 {
        let gap = (w - cell * n as f32) / n as f32;
        (0..n).map(|i| x + gap * 0.5 + i as f32 * (cell + gap)).collect()
    } else {
        let gap = (w - cell * n as f32) / (n - 1) as f32;
        (0..n).map(|i| x + i as f32 * (cell + gap)).collect()
    };
    for (k, kx) in row.iter_mut().zip(xs) {
        k.c = (kx + cell * 0.5, y + d * 0.5);
    }
}

/// A value as the hover help and the drag readout show it.
fn value_text(d: &Dev, ctl: Ctl, v: i32) -> String {
    match ctl {
        Ctl::Param(id) => {
            let spec = lane_dials(d.chan).into_iter().find(|(p, _)| *p == id).map(|(_, s)| s);
            spec.map_or(v.to_string(), |s| s.format(v as f32))
        }
        Ctl::Uni(0) => if v == 1 { "1 voice".into() } else { format!("{v} voices") },
        Ctl::Uni(_) => format!("{v} ct"),
        Ctl::Vib(0) if v == 0 => "off".into(),
        Ctl::Vib(0) => format!("{v} ct"),
        Ctl::Vib(1) => format!("{:.1} Hz", 60.1 / v.max(1) as f32),
        Ctl::Vib(_) => format!("{} ms", (v as f32 * 1000.0 / 60.1).round() as u32),
        Ctl::ArpSpeed if v <= 1 => "every frame".into(),
        Ctl::ArpSpeed => format!("{v} frames"),
    }
}

fn ctl_help(d: &Dev, k: &Knob) -> String {
    let what = match k.ctl {
        Ctl::Param(ParamId::Volume) => "Volume: the chip's Volume register, the instrument's own level (the track fader is the mixer's)",
        Ctl::Param(ParamId::Duty) => "Duty: the pulse width",
        Ctl::Param(ParamId::NoiseMode) => "Mode: long (hiss) or short (metallic) noise",
        Ctl::Param(ParamId::NoisePeriod) => "Period: the noise pitch (note: the note picks it)",
        Ctl::Param(ParamId::Rate) if d.voice == VoiceKind::Saw => "Rate: the saw's accumulator rate, its level and edge (over 42 it wraps and distorts)",
        Ctl::Param(ParamId::Rate) => "Rate: the sample's playback rate (its pitch)",
        Ctl::Param(ParamId::Envelope) => "Envelope: the chip's own volume envelope (constant, decay, looping decay)",
        Ctl::Param(ParamId::Sweep) => "Sweep: the pulse channel's hardware pitch sweep",
        Ctl::Param(ParamId::SweepRate) => "Sweep rate: how fast the hardware sweep moves",
        Ctl::Param(_) => "A channel control",
        Ctl::Uni(0) => "Unison: voices per note (1–4), from the track's pool (studio only)",
        Ctl::Uni(1) => "Detune: spreads the unison voices over 0–50 cents (studio only)",
        Ctl::Uni(_) => "Drift: a slow pitch wander per voice, 0–10 cents (studio only)",
        Ctl::Vib(0) => "Vibrato depth",
        Ctl::Vib(1) => "Vibrato rate",
        Ctl::Vib(_) => "Vibrato delay: it starts after this long and fades in",
        Ctl::ArpSpeed => "Arp speed: frames per arpeggio step (1: every frame, the classic fast arpeggio)",
    };
    let v = value_text(d, k.ctl, k.value);
    match k.inert {
        Some(why) => format!("{what}: {v} · inert, {why}: clear that envelope to use it"),
        None => format!("{what}: {v} · drag or wheel (Shift: finer) · double-click: default"),
    }
}

// ───────────────────────────── the view ─────────────────────────────

/// A part of the device under the pointer.
#[derive(Clone, Copy, Debug, PartialEq)]
enum Part {
    None,
    Prev,
    Next,
    Field,
    Voice,
    Tab(usize),
    Knob(Ctl),
    Graph,
    /// DPCM Sampler: a sample row (-1: the note picks), the Loop key, Import.
    Sample(i32),
    Loop,
    Import,
}

#[derive(Clone, Copy, Debug, PartialEq)]
enum Drag {
    Knob { ctl: Ctl, y0: f32, v0: i32, fine: bool },
    Draw { m: usize, last: usize },
    Point { m: usize, release: bool },
}

pub struct DeviceView {
    s: Signals,
    tab: usize,
    hover: Part,
    drag: Option<Drag>,
    wheel: crate::input::WheelSteps,
    cache: RefCell<Option<(u64, vg::Image)>>,
    /// Dev mode: the control set last reported (the scenario checks).
    reported: RefCell<String>,
}

impl DeviceView {
    pub fn new(cx: &mut Context, s: Signals) -> Handle<'_, Self> {
        Self { s, tab: 0, hover: Part::None, drag: None, wheel: Default::default(), cache: RefCell::new(None), reported: RefCell::new(String::new()) }
            .build(cx, |_| {})
            .class("device-view")
            .width(Pixels(DEVICE_W))
            .height(Pixels(DEVICE_H))
            .bind(s.arrangement, |mut h| h.needs_redraw())
            .bind(s.lane_sel, |mut h| h.needs_redraw())
            .bind(s.dial_state, |mut h| h.needs_redraw())
            .bind(s.meters, |mut h| h.needs_redraw())
    }

    fn local(cx: &EventContext) -> (f32, f32) {
        let b = cx.bounds();
        let s = cx.scale_factor();
        ((cx.mouse().cursor_x - b.x) / s, (cx.mouse().cursor_y - b.y) / s)
    }

    /// The preset group's three rects: ◀, the field, ▶ (centred in the header).
    fn preset_rects() -> [(f32, f32, f32, f32); 3] {
        let total = KEY + KEY_GAP + FIELD_W + KEY_GAP + KEY;
        let x0 = (DEVICE_W - total) * 0.5;
        let y = (HEADER_H - KEY) * 0.5;
        [(x0, y, KEY, KEY), (x0 + KEY + KEY_GAP, y, FIELD_W, KEY), (x0 + KEY + KEY_GAP + FIELD_W + KEY_GAP, y, KEY, KEY)]
    }

    /// The Voice selector's rect in the OSCILLATOR title row (right-aligned).
    fn voice_rect(d: &Dev, s: f32) -> (f32, f32, f32, f32) {
        let f = gfx::font(Face::SemiBold, 11.0 * s);
        let w = 8.0 + gfx::text_width(&f, &voice_text(d)) / s + 6.0 + 8.0 + 8.0 + if d.locked() { 12.0 } else { 0.0 };
        (OSC.0 + OSC.1 - PAD - w, SEC_Y + TITLE_Y + 1.0, w, 20.0)
    }

    /// The envelope tabs (right-aligned in the ENVELOPES title row): (macro, rect).
    fn tab_rects(d: &Dev, s: f32) -> Vec<(usize, (f32, f32, f32, f32))> {
        let f = gfx::font(Face::SemiBold, 10.0 * s);
        let shown: Vec<usize> = (0..4).filter(|m| macro_used(d.voice, *m)).collect();
        let widths: Vec<f32> = shown.iter().map(|m| gfx::tracked_width(macro_label(d.voice, *m), &f, 0.6 * s) / s + 16.0).collect();
        let total: f32 = widths.iter().sum::<f32>() + 2.0 * (shown.len().saturating_sub(1)) as f32;
        let mut x = ENV.0 + ENV.1 - PAD - total;
        shown
            .into_iter()
            .zip(widths)
            .map(|(m, w)| {
                let r = (x, SEC_Y + TITLE_Y + 1.0, w, 20.0);
                x += w + 2.0;
                (m, r)
            })
            .collect()
    }

    fn graph_rect() -> (f32, f32, f32, f32) {
        (ENV.0 + PAD, SEC_Y + DISPLAY_Y, ENV.1 - 2.0 * PAD, DISPLAY_H)
    }

    /// DPCM Sampler rows: (sample, rect); -1 is "the note picks".
    fn sample_rows() -> Vec<(i32, (f32, f32, f32, f32))> {
        (-1..chip::samples::SAMPLE_COUNT as i32).enumerate().map(|(k, smp)| (smp, (OSC.0 + PAD, SEC_Y + DISPLAY_Y + k as f32 * 20.0, OSC.1 - 2.0 * PAD, 20.0))).collect()
    }

    fn import_rect() -> (f32, f32, f32, f32) {
        (OSC.0 + PAD, SEC_Y + SEC_H - 10.0 - 24.0, OSC.1 - 2.0 * PAD, 24.0)
    }

    fn loop_rect() -> (f32, f32, f32, f32) {
        (ENV.0 + ENV.1 * 0.5 + 40.0, SEC_Y + KNOB_ROW_Y + 8.0, 64.0, 24.0)
    }

    fn part_at(&self, d: &Dev, p: (f32, f32), s: f32) -> Part {
        let inside = |r: (f32, f32, f32, f32)| p.0 >= r.0 && p.0 < r.0 + r.2 && p.1 >= r.1 && p.1 < r.1 + r.3;
        let [pr, fl, nx] = Self::preset_rects();
        if inside(pr) {
            return Part::Prev;
        }
        if inside(nx) {
            return Part::Next;
        }
        if inside(fl) {
            return Part::Field;
        }
        if d.voice != VoiceKind::Dpcm && inside(Self::voice_rect(d, s)) {
            return Part::Voice;
        }
        if d.voice == VoiceKind::Dpcm {
            for (smp, r) in Self::sample_rows() {
                if inside(r) {
                    return Part::Sample(smp);
                }
            }
            if inside(Self::import_rect()) {
                return Part::Import;
            }
            if inside(Self::loop_rect()) {
                return Part::Loop;
            }
        } else if !d.inst.is_kit() {
            for (m, r) in Self::tab_rects(d, s) {
                if inside(r) {
                    return Part::Tab(m);
                }
            }
            if inside(Self::graph_rect()) {
                return Part::Graph;
            }
        }
        for k in knobs(d) {
            let r = k.d * 0.5 + 4.0;
            if (p.0 - k.c.0).abs() <= r && (p.1 - k.c.1).abs() <= r + 10.0 {
                return Part::Knob(k.ctl);
            }
        }
        Part::None
    }

    fn set(cx: &mut EventContext, d: &Dev, ctl: Ctl, v: i32) {
        match ctl {
            Ctl::Param(id) => {
                if let Some(slot) = lane_dials(d.chan).iter().position(|(p, _)| *p == id) {
                    cx.emit(AppEvent::SetDial(slot, v as f32));
                }
            }
            Ctl::Uni(f) => {
                let field = [super::dial::UniField::Voices, super::dial::UniField::Detune, super::dial::UniField::Drift][f.min(2)];
                cx.emit(AppEvent::SetUnison(field, v.clamp(0, 255) as u8));
            }
            Ctl::Vib(which) => cx.emit(AppEvent::EditVibrato { inst: d.inst_id, which, value: v.clamp(0, 255) as u8 }),
            Ctl::ArpSpeed => cx.emit(AppEvent::EditArpSpeed { inst: d.inst_id, value: v.clamp(1, chip::instrument::ARP_SPEED_MAX as i32) as u8 }),
        }
    }

    fn help(&self, d: &Dev, part: Part) -> String {
        match part {
            Part::Prev => "Previous preset in this folder: loads it now, even while playing (one undo step for a run of steps)".into(),
            Part::Next => "Next preset in this folder: loads it now, even while playing (one undo step for a run of steps)".into(),
            Part::Field => "Preset: click to choose (the browser's folders, Show in Browser, Save Preset As…)".into(),
            Part::Voice if d.locked() => format!("Voice: fixed to the {}, this locked track's channel · unlock the track (its badge) to change it", crate::tracks::channel_name(d.chan)),
            Part::Voice => "Voice: the sound source (Pulse, Triangle, Noise, VRC6 Pulse, VRC6 Saw) · one undo step".into(),
            Part::Tab(m) => format!("{} envelope: one value per frame (60 Hz)", macro_label(d.voice, m).to_lowercase()),
            Part::Graph => "Drag to draw the envelope · Shift-click: loop point · Alt-click: release point · double-click a marker to remove it".into(),
            Part::Knob(ctl) => knobs(d).into_iter().find(|k| k.ctl == ctl).map_or(String::new(), |k| ctl_help(d, &k)),
            Part::Sample(-1) => "The note picks the sample (C kick, C# snare, D tom, D# crash, then your samples)".into(),
            Part::Sample(i) => format!("Play {} on every note", chip::samples::NAMES.get(i as usize).copied().unwrap_or("?")),
            Part::Loop => "Loop: the sample repeats while the note is held".into(),
            Part::Import => "Import audio as a DPCM sample (a WAV file into a free slot)".into(),
            Part::None => String::new(),
        }
    }

    fn seq_of(inst: &chip::instrument::Instrument, m: usize) -> &Seq {
        match m {
            0 => &inst.volume,
            1 => &inst.arp,
            2 => &inst.pitch,
            _ => &inst.duty,
        }
    }

    /// The graph's bar pitch for `len` steps (the board: 15 px bars, narrower
    /// when they don't fit), its left edge and the value's y.
    fn bar_pitch(w: f32, len: usize) -> f32 {
        ((w - 20.0) / len.max(1) as f32).min(15.0)
    }

    fn step_at(x: f32, len: usize) -> usize {
        let (gx, _, gw, _) = Self::graph_rect();
        let bp = Self::bar_pitch(gw, len);
        (((x - gx - 10.0) / bp).floor().max(0.0) as usize).min(len.saturating_sub(1))
    }

    fn paint(&mut self, cx: &mut EventContext, d: &Dev, p: (f32, f32)) {
        let m = self.tab;
        let len = Self::seq_of(&d.inst.inst, m).len as usize;
        let len = if len == 0 { NEW_SEQ_LEN } else { len };
        let (_, gy, _, gh) = Self::graph_rect();
        let step = Self::step_at(p.0, len);
        let (_, lo, hi) = MACROS[m];
        let t = ((gy + gh - 8.0 - p.1) / (gh - 15.0)).clamp(0.0, 1.0);
        let v = (lo as f32 + t * (hi - lo) as f32).round() as i32;
        let from = match self.drag {
            Some(Drag::Draw { last, .. }) => last,
            _ => step,
        };
        let (a, b) = if from <= step { (from, step) } else { (step, from) };
        for st in a..=b {
            cx.emit(AppEvent::EditMacro { inst: d.inst_id, which: m, step: st, value: v as i8 });
        }
        self.drag = Some(Drag::Draw { m, last: step });
    }

    fn marker_at(q: &Seq, x: f32) -> Option<bool> {
        if q.len == 0 {
            return None;
        }
        let (gx, _, gw, _) = Self::graph_rect();
        let bp = Self::bar_pitch(gw, q.len as usize);
        let near = |i: u8| i != NONE && (x - (gx + 10.0 + bp * i as f32)).abs() <= MARKER_GRAB_PX;
        if near(q.release_at) {
            Some(true)
        } else if near(q.loop_at) {
            Some(false)
        } else {
            None
        }
    }
}

fn voice_text(d: &Dev) -> String {
    match d.lock {
        Some(c) => crate::tracks::channel_name(c).to_string(),
        None => crate::bank::voice_name(d.voice).to_string(),
    }
}

impl View for DeviceView {
    fn element(&self) -> Option<&'static str> {
        Some("device-view")
    }

    fn event(&mut self, cx: &mut EventContext, event: &mut Event) {
        let Some(d) = Dev::of(&self.s) else { return };
        if !macro_used(d.voice, self.tab) {
            self.tab = 0;
        }
        let s = cx.scale_factor();
        event.map(|we, meta| match we {
            WindowEvent::MouseMove(..) => {
                let p = Self::local(cx);
                match self.drag {
                    Some(Drag::Knob { ctl, y0, v0, fine }) => {
                        let step = if fine || cx.modifiers().shift() { DRAG_PX_FINE } else { DRAG_PX };
                        let k = knobs(&d).into_iter().find(|k| k.ctl == ctl);
                        if let Some(k) = k {
                            let n = ((y0 - p.1) / step).round() as i32;
                            let v = (v0 + n).clamp(k.min, k.max);
                            if v != k.value {
                                Self::set(cx, &d, ctl, v);
                            }
                            cx.emit(AppEvent::HoverHelp(ctl_help(&d, &Knob { value: v, ..k })));
                        }
                    }
                    Some(Drag::Draw { .. }) => self.paint(cx, &d, p),
                    Some(Drag::Point { m, release }) => {
                        let len = Self::seq_of(&d.inst.inst, m).len as usize;
                        if len > 0 {
                            let at = Self::step_at(p.0, len) as u8;
                            cx.emit(AppEvent::EditMacroPoint { inst: d.inst_id, which: m, release, at: Some(at) });
                        }
                    }
                    None => {
                        let h = self.part_at(&d, p, s);
                        if h != self.hover {
                            self.hover = h;
                            cx.emit(AppEvent::HoverHelp(self.help(&d, h)));
                            cx.needs_redraw();
                        }
                    }
                }
            }
            WindowEvent::MouseLeave => {
                if self.drag.is_none() && self.hover != Part::None {
                    self.hover = Part::None;
                    cx.emit(AppEvent::HoverHelp(String::new()));
                    cx.needs_redraw();
                }
            }
            WindowEvent::MouseDown(MouseButton::Left) => {
                let p = Self::local(cx);
                let part = self.part_at(&d, p, s);
                match part {
                    Part::Prev => cx.emit(AppEvent::StepPreset(-1)),
                    Part::Next => cx.emit(AppEvent::StepPreset(1)),
                    Part::Field => {
                        let b = cx.bounds();
                        let (fx, fy, fw, fh) = Self::preset_rects()[1];
                        cx.emit(AppEvent::OpenPicker(b.x / s + fx, b.y / s + fy + fh + 4.0, fw));
                    }
                    Part::Voice => {
                        if d.locked() {
                            cx.emit(AppEvent::HoverHelp(self.help(&d, part)));
                        } else {
                            let b = cx.bounds();
                            let (vx, vy, _, vh) = Self::voice_rect(&d, s);
                            cx.emit(AppEvent::ContextMenu(crate::edit::MenuSite::Voice(d.lane), b.x / s + vx, b.y / s + vy + vh + 2.0));
                        }
                    }
                    Part::Tab(m) => {
                        self.tab = m;
                        cx.needs_redraw();
                    }
                    Part::Knob(ctl) => {
                        if let Some(k) = knobs(&d).into_iter().find(|k| k.ctl == ctl) {
                            if k.inert.is_none() {
                                self.drag = Some(Drag::Knob { ctl, y0: p.1, v0: k.value, fine: false });
                                cx.capture();
                                cx.emit(AppEvent::GestureBegin);
                            }
                        }
                    }
                    Part::Graph => {
                        let m = self.tab;
                        let q = *Self::seq_of(&d.inst.inst, m);
                        let mods = cx.modifiers();
                        if (mods.shift() || mods.alt()) && q.len > 0 {
                            let at = Self::step_at(p.0, q.len as usize) as u8;
                            cx.emit(AppEvent::EditMacroPoint { inst: d.inst_id, which: m, release: mods.alt(), at: Some(at) });
                        } else if let Some(release) = Self::marker_at(&q, p.0) {
                            self.drag = Some(Drag::Point { m, release });
                            cx.capture();
                            cx.emit(AppEvent::GestureBegin);
                        } else {
                            cx.capture();
                            cx.emit(AppEvent::GestureBegin);
                            self.drag = None;
                            self.paint(cx, &d, p);
                        }
                    }
                    Part::Sample(smp) => {
                        if let Some(slot) = lane_dials(d.chan).iter().position(|(p, _)| *p == ParamId::Sample) {
                            cx.emit(AppEvent::SetDial(slot, smp as f32));
                            cx.emit(AppEvent::Commit);
                        }
                    }
                    Part::Loop => {
                        if let Some(slot) = lane_dials(d.chan).iter().position(|(p, _)| *p == ParamId::Loop) {
                            cx.emit(AppEvent::SetDial(slot, if d.param(ParamId::Loop) != 0 { 0.0 } else { 1.0 }));
                            cx.emit(AppEvent::Commit);
                        }
                    }
                    Part::Import => cx.emit(AppEvent::ImportAudio),
                    Part::None => {}
                }
                meta.consume();
            }
            WindowEvent::MouseDoubleClick(MouseButton::Left) => {
                let p = Self::local(cx);
                match self.part_at(&d, p, s) {
                    Part::Knob(ctl) => {
                        if let Some(k) = knobs(&d).into_iter().find(|k| k.ctl == ctl && k.inert.is_none()) {
                            Self::set(cx, &d, ctl, k.def);
                            cx.emit(AppEvent::Commit);
                        }
                    }
                    Part::Graph => {
                        let q = *Self::seq_of(&d.inst.inst, self.tab);
                        if let Some(release) = Self::marker_at(&q, p.0) {
                            cx.emit(AppEvent::EditMacroPoint { inst: d.inst_id, which: self.tab, release, at: None });
                        }
                    }
                    _ => {}
                }
                meta.consume();
            }
            WindowEvent::MouseUp(MouseButton::Left) if self.drag.is_some() => {
                self.drag = None;
                cx.release();
                cx.emit(AppEvent::GestureEnd);
                cx.needs_redraw();
                meta.consume();
            }
            WindowEvent::MouseScroll(_, sy) => {
                if let Part::Knob(ctl) = self.part_at(&d, Self::local(cx), s) {
                    let steps = self.wheel.steps(*sy);
                    if let Some(k) = knobs(&d).into_iter().find(|k| k.ctl == ctl && k.inert.is_none()) {
                        if steps != 0 {
                            let v = (k.value + steps).clamp(k.min, k.max);
                            Self::set(cx, &d, ctl, v);
                            cx.emit(AppEvent::Commit);
                            cx.emit(AppEvent::HoverHelp(ctl_help(&d, &Knob { value: v, ..k })));
                        }
                    }
                    meta.consume();
                }
            }
            _ => {}
        });
    }

    fn draw(&self, cx: &mut DrawContext, canvas: &Canvas) {
        let Some(d) = Dev::of(&self.s) else { return };
        let b = cx.bounds();
        let s = cx.scale_factor();
        let tab = if macro_used(d.voice, self.tab) { self.tab } else { 0 };
        let preset = preset_label(&d);
        // The cached image: everything except the meter's lit segments.
        let key = {
            use std::hash::{Hash, Hasher};
            let mut h = std::collections::hash_map::DefaultHasher::new();
            (d.lane, d.voice as u8, d.lock, crate::state::rgb_hex(d.color), d.params, d.unison.voices, d.unison.detune, d.unison.drift).hash(&mut h);
            d.inst.hash(&mut h);
            format!("{:?}{:?}{tab}{preset:?}", d.states, self.hover).hash(&mut h);
            (b.w as i32, b.h as i32, (s * 100.0) as i32).hash(&mut h);
            h.finish()
        };
        if crate::dev::enabled() {
            let tabs: Vec<&str> = (0..4).filter(|m| macro_used(d.voice, *m)).map(|m| macro_label(d.voice, m)).collect();
            let line = format!(
                "dev: device {} {:?} {} preset {:?}: knobs {} · tabs {}",
                if d.voice == VoiceKind::Dpcm { "DPCM Sampler" } else { "Chip Synth" },
                d.voice,
                if d.locked() { "locked" } else { "studio" },
                preset.0,
                knobs(&d).iter().map(|k| k.label).collect::<Vec<_>>().join("|"),
                tabs.join("|")
            );
            if *self.reported.borrow() != line {
                eprintln!("{line}");
                *self.reported.borrow_mut() = line;
            }
        }
        let mut cache = self.cache.borrow_mut();
        if cache.as_ref().map(|c| c.0) != Some(key) {
            *cache = render(&d, tab, &preset, self.hover, b.w, b.h, s).map(|img| (key, img));
        }
        if let Some((_, img)) = cache.as_ref() {
            // The image has room for the panel's shadow around the box.
            let pad = (16.0 * s).ceil();
            canvas.draw_image(img, (b.x - pad, b.y - pad), None);
        }
        drop(cache);
        // The meter: the track's level before its fader (the instrument's output).
        let level = self.s.meters.with_untracked(|m| m.level_of(d.lane));
        let lit = ((super::mixer::lit_segments(level) as f32) * METER_SEGS as f32 / 65.0).round() as usize;
        let (mx, my) = meter_origin(&d);
        // One save, one restore: a locked device's meter is a 40 % layer. (A save before the
        // layer left one level open, and every view drawn after it was clipped into the
        // meter: the window stopped showing updates; the regression gate found it.)
        if d.locked() {
            canvas.save_layer_alpha_f(vg::Rect::from_xywh(b.x + mx * s, b.y + my * s, METER_W * s, METER_H * s), 0.4);
        } else {
            canvas.save();
        }
        for k in 0..lit.min(METER_SEGS) {
            let r = vg::Rect::from_xywh(b.x + (mx + 2.0) * s, b.y + (my + METER_H - 2.0 - 3.0 - 4.0 * k as f32) * s, (METER_W - 4.0) * s, 3.0 * s);
            canvas.draw_rect(r, &gfx::fill(seg_colour(k)));
        }
        canvas.restore();
    }
}

/// The meter's top-left (device-local): beside the Volume knob, or centred
/// in OUTPUT on a voice without one.
fn meter_origin(d: &Dev) -> (f32, f32) {
    let inner = OUT.1 - 2.0 * PAD;
    if d.has(ParamId::Volume) {
        (OUT.0 + PAD + (inner - (BIG_D + 18.0 + METER_W)) * 0.5 + BIG_D + 18.0, SEC_Y + OUT_ROW_Y)
    } else {
        (OUT.0 + PAD + (inner - METER_W) * 0.5, SEC_Y + OUT_ROW_Y)
    }
}

/// A lit segment's colour (the board's warm white, then amber, then red).
fn seg_colour(k: usize) -> Rgba {
    match k {
        0..=13 => Rgba::hex(0xD6D0C4),
        14..=19 => Rgba::hex(0xF2B640),
        _ => Rgba::hex(0xFF6C63),
    }
}

/// The preset field's text and whether it's an authentic preset.
fn preset_label(d: &Dev) -> (String, bool) {
    let authentic = crate::bank::find(&d.inst.name).is_some_and(|p| p.authentic());
    (d.inst.label(), authentic)
}

pub(crate) fn lock_glyph(c: &vg::Canvas, o: (f32, f32), s: f32, ink: Rgba) {
    c.draw_path(&gfx::svg_path("M1 4H7V8.5H1Z", o, s), &gfx::fill(ink));
    c.draw_path(&gfx::svg_path("M2 4V2.8a2 2 0 0 1 4 0V4", o, s), &gfx::stroke(ink, 1.2 * s));
}

/// The board's key: `#0B0B0B → #262626`, a `#101010` border, the top sheen.
fn key_body(c: &vg::Canvas, r: vg::Rect, s: f32, hover: bool) {
    let rr = gfx::rrect(r, 4.0 * s);
    c.draw_rrect(rr, &gfx::vgrad(r, Rgba::hex(0x0B0B0B), Rgba::hex(if hover { 0x2E2E2E } else { 0x262626 })));
    gfx::ring(c, &rr, gfx::hairline(s), &gfx::fill(Rgba::hex(0x101010)));
    gfx::top_sheen(c, &rr.with_inset((gfx::hairline(s), gfx::hairline(s))), gfx::hairline(s), Rgba::hex(0x333333), Rgba::hex(0x7A7A7A));
}

/// A recessed field (`#0E0E0E`, inner shadow, a `#2E2E2E` ring).
fn field_body(c: &vg::Canvas, r: vg::Rect, s: f32, hover: bool) {
    let rr = gfx::rrect(r, 4.0 * s);
    c.draw_rrect(rr, &gfx::fill(Rgba::hex(if hover { 0x141414 } else { 0x0E0E0E })));
    gfx::inner_shadow(c, &rr, 0.0, s, s, Rgba::black(0.7));
    gfx::ring(c, &rr, gfx::hairline(s), &gfx::fill(Rgba::hex(if hover { 0x3A3A3A } else { 0x2E2E2E })));
}

fn chevron(c: &vg::Canvas, d: &str, o: (f32, f32), s: f32, ink: Rgba) {
    let mut p = gfx::stroke(ink, 1.4 * s);
    p.set_stroke_cap(vg::PaintCap::Butt);
    c.draw_path(&gfx::svg_path(d, o, s), &p);
}

/// The machined knob (option A) at any size: the 270° track ring with the
/// value arc from the minimum, the knurled skirt, the brushed cap and its
/// indicator (the board's 40 px knob, scaled).
fn knob(c: &vg::Canvas, o: (f32, f32), d: f32, norm: f32, s: f32, inert: bool) {
    let k = d / 40.0;
    let r = d * 0.5 * s;
    let ring = vg::Rect::from_xywh(o.0 - r + s, o.1 - r + s, 2.0 * r - 2.0 * s, 2.0 * r - 2.0 * s);
    let mut p = gfx::stroke(Rgba::hex(0x2A2A2A), 2.0 * s);
    p.set_stroke_cap(vg::PaintCap::Butt);
    c.draw_arc(ring, 135.0, 270.0, false, &p);
    if !inert && norm > 0.0 {
        let mut v = gfx::stroke(Rgba::hex(0xE6E6E6), 2.0 * s);
        v.set_stroke_cap(vg::PaintCap::Butt);
        c.draw_arc(ring, 135.0, 270.0 * norm.clamp(0.0, 1.0), false, &v);
    }
    // Skirt (32 px at 40): knurled, top light, shadows.
    let sk = 16.0 * k * s;
    let skirt = vg::Rect::from_xywh(o.0 - sk, o.1 - sk, 2.0 * sk, 2.0 * sk);
    gfx::drop_shadow(c, &vg::RRect::new_oval(skirt), 0.0, 2.0 * k * s, 5.0 * k * s, 0.0, Rgba::black(0.75));
    let wedges: Vec<Rgba> = (0..72).map(|i| if i % 2 == 0 { Rgba::hex(0x1C1C1C) } else { Rgba::hex(0x333333) }).collect();
    let pos: Vec<f32> = (0..72).map(|i| i as f32 / 71.0).collect();
    let mut kn = vg::Paint::default();
    kn.set_anti_alias(true);
    kn.set_shader(sweep(o, &wedges, &pos));
    c.draw_oval(skirt, &kn);
    c.draw_oval(skirt, &gfx::shaded(gfx::radial((o.0, o.1 - 0.4 * sk), sk * 1.2, &[Rgba::white(0.12), Rgba::white(0.0)], Some(&[0.0, 0.6]))));
    c.draw_arc(skirt.with_inset((0.5 * s, 0.5 * s)), 200.0, 140.0, false, &gfx::stroke(Rgba::white(0.18), s));
    // Cap (23 px at 40): brushed metal.
    let cr = 11.5 * k * s;
    let cap = vg::Rect::from_xywh(o.0 - cr, o.1 - cr, 2.0 * cr, 2.0 * cr);
    gfx::drop_shadow(c, &vg::RRect::new_oval(cap), 0.0, s, 2.0 * s, 0.0, Rgba::black(0.8));
    let brushed = [0x7E7E7E, 0xC9C9C9, 0x6A6A6A, 0xBEBEBE, 0x727272, 0xC6C6C6, 0x6C6C6C, 0xBCBCBC, 0x7E7E7E].map(Rgba::hex);
    let bpos = [0.0, 0.12, 0.25, 0.38, 0.5, 0.63, 0.75, 0.88, 1.0];
    let mut bp = vg::Paint::default();
    bp.set_anti_alias(true);
    bp.set_shader(sweep(o, &brushed, &bpos));
    c.draw_oval(cap, &bp);
    c.draw_oval(cap, &gfx::shaded(gfx::radial((o.0, o.1 - 0.5 * cr), cr * 1.1, &[Rgba::white(0.35), Rgba::white(0.0)], Some(&[0.0, 0.55]))));
    c.draw_arc(cap.with_inset((0.5 * s, 0.5 * s)), 200.0, 140.0, false, &gfx::stroke(Rgba::white(0.6), s));
    // Indicator: 2 × 7 at 40 px, from 2 px inside the cap's top.
    let ang = -135.0 + 270.0 * norm.clamp(0.0, 1.0);
    c.save();
    c.rotate(ang, Some(vg::Point::new(o.0, o.1)));
    let ind = vg::Rect::from_xywh(o.0 - s, o.1 - cr + 2.0 * k * s, 2.0 * s, 7.0 * k * s);
    c.draw_rrect(gfx::rrect(ind, s), &gfx::fill(Rgba::hex(0x151515)));
    c.restore();
}

fn sweep(o: (f32, f32), colours: &[Rgba], pos: &[f32]) -> Option<vg::Shader> {
    let c: Vec<vg::Color4f> = colours.iter().map(|c| c.c4f()).collect();
    let g = vg::gradient::Gradient::new(vg::gradient::Colors::new(&c, Some(pos), vg::TileMode::Clamp, None), vg::gradient::Interpolation::default());
    let m = vg::Matrix::rotate_deg_pivot(-90.0, vg::Point::new(o.0, o.1));
    vg::gradient::shaders::sweep_gradient(vg::Point::new(o.0, o.1), (0.0, 360.0), &g, Some(&m))
}

/// The device panel (the board's): gradient, top light, grain, border.
fn panel(c: &vg::Canvas, r: vg::Rect, s: f32) {
    let rr = gfx::rrect(r, 8.0 * s);
    gfx::drop_shadow(c, &rr, 0.0, 4.0 * s, 12.0 * s, 0.0, Rgba::black(0.6));
    c.draw_rrect(rr, &gfx::vgrad(r, Rgba::hex(0x1E1E1E), Rgba::hex(0x141414)));
    c.save();
    c.clip_rrect(rr, vg::ClipOp::Intersect, true);
    if let Some(noise) = vg::shaders::fractal_noise((0.9 / s, 0.9 / s), 3, 7.0, None) {
        let mut p = vg::Paint::default();
        p.set_shader(noise);
        p.set_color_filter(vg::color_filters::matrix(&vg::ColorMatrix::new(0.33, 0.33, 0.33, 0.0, 0.0, 0.33, 0.33, 0.33, 0.0, 0.0, 0.33, 0.33, 0.33, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.035), None));
        c.draw_rect(r, &p);
    }
    c.draw_rect(r, &gfx::shaded(gfx::radial((r.left + r.width() * 0.3, r.top), r.width() * 0.6, &[Rgba::white(0.06), Rgba::white(0.0)], None)));
    c.restore();
    gfx::ring(c, &rr, gfx::hairline(s), &gfx::fill(Rgba::hex(0x2A2A2A)));
    gfx::top_highlight(c, &rr.with_inset((gfx::hairline(s), gfx::hairline(s))), gfx::hairline(s), Rgba::white(0.08));
}

/// A section's recessed sub-panel and its title.
fn section(c: &vg::Canvas, at: &dyn Fn(f32, f32, f32, f32) -> vg::Rect, x: f32, w: f32, title: &str, s: f32) {
    let r = at(x, SEC_Y, w, SEC_H);
    let rr = gfx::rrect(r, 6.0 * s);
    c.draw_rrect(rr, &gfx::fill(Rgba::black(0.18)));
    gfx::ring(c, &rr, gfx::hairline(s), &gfx::fill(Rgba::hex(0x242424)));
    gfx::top_highlight(c, &rr.with_inset((gfx::hairline(s), gfx::hairline(s))), gfx::hairline(s), Rgba::white(0.04));
    let f = gfx::font(Face::SemiBold, 10.0 * s);
    let base = gfx::baseline_for_center(&f, r.top + (TITLE_Y + TITLE_H * 0.5) * s);
    gfx::draw_text_tracked(c, title, (r.left + PAD * s).round(), base.round(), &f, 1.0 * s, DEVICE_INK_LO);
}

/// A display well (`#0A0A0A`, inner shadow, a faint lip under it).
fn display_well(c: &vg::Canvas, r: vg::Rect, s: f32) {
    let rr = gfx::rrect(r, 4.0 * s);
    c.draw_rect(vg::Rect::from_xywh(r.left + 4.0 * s, r.bottom, r.width() - 8.0 * s, gfx::hairline(s)), &gfx::fill(Rgba::white(0.05)));
    c.draw_rrect(rr, &gfx::fill(Rgba::hex(0x0A0A0A)));
    gfx::inner_shadow(c, &rr, 0.0, s, 2.0 * s, Rgba::black(0.8));
}

/// The voice's waveform: three cycles, as the chip makes it now.
fn waveform(d: &Dev, w: f32, h: f32) -> Vec<(f32, f32)> {
    let mid = h * 0.5;
    let amp = h * 0.28;
    let cyc = w / 3.0;
    let mut pts = Vec::new();
    let duty_frac = |d: &Dev| -> f32 {
        let mac = (!d.inst.inst.duty.is_empty()).then(|| d.inst.inst.duty.at(0, 8));
        match d.voice {
            VoiceKind::Vrc6Pulse => {
                let i = match mac {
                    Some(m) => chip::instrument::duty_vrc6(m.clamp(0, 16) as u8) as i32,
                    None => d.param(ParamId::Duty),
                };
                if i >= 8 { 1.0 } else { (i + 1) as f32 / 16.0 }
            }
            _ => {
                let i = match mac {
                    Some(m) => chip::instrument::duty_2a03(m.clamp(0, 16) as u8) as i32,
                    None => d.param(ParamId::Duty),
                };
                [0.125, 0.25, 0.5, 0.75][i.clamp(0, 3) as usize]
            }
        }
    };
    match d.voice {
        VoiceKind::Pulse | VoiceKind::Vrc6Pulse => {
            let f = duty_frac(d);
            let (hi, lo) = (mid - amp, mid + amp);
            pts.push((0.0, lo));
            for k in 0..3 {
                let x0 = k as f32 * cyc;
                if f >= 1.0 {
                    pts.push((x0, hi));
                    pts.push((x0 + cyc, hi));
                    continue;
                }
                pts.push((x0, hi));
                pts.push((x0 + cyc * f, hi));
                pts.push((x0 + cyc * f, lo));
                pts.push((x0 + cyc, lo));
            }
        }
        VoiceKind::Triangle => {
            // The 2A03 triangle: 32 steps of a 4-bit staircase per cycle.
            for k in 0..3 {
                for st in 0..32 {
                    let lvl = if st < 16 { 15 - st } else { st - 16 } as f32;
                    let y = mid + amp - lvl / 15.0 * 2.0 * amp;
                    let x0 = k as f32 * cyc + st as f32 * cyc / 32.0;
                    pts.push((x0, y));
                    pts.push((x0 + cyc / 32.0, y));
                }
            }
        }
        VoiceKind::Saw => {
            // The VRC6 saw: the accumulator's 7 steps per cycle; past 42 it wraps.
            let rate = d.param(ParamId::Rate).max(0);
            for k in 0..3 {
                for st in 0..7 {
                    let acc = (rate * st) & 0xFF;
                    let lvl = (acc >> 3) as f32 / 31.0;
                    let y = mid + amp - lvl * 2.0 * amp;
                    let x0 = k as f32 * cyc + st as f32 * cyc / 7.0;
                    pts.push((x0, y));
                    pts.push((x0 + cyc / 7.0, y));
                }
            }
        }
        VoiceKind::Noise => {
            // The noise LFSR: long mode (bit 1 tap) or short (bit 6): 48 steps.
            let short = d.param(ParamId::NoiseMode) != 0 || d.inst.inst.duty.at(0, 0) != 0 && !d.inst.inst.duty.is_empty();
            let mut lfsr: u16 = 1;
            let n = 48;
            for st in 0..n {
                let tap = if short { 6 } else { 1 };
                let fb = (lfsr & 1) ^ ((lfsr >> tap) & 1);
                lfsr = (lfsr >> 1) | (fb << 14);
                let y = if lfsr & 1 == 0 { mid - amp } else { mid + amp };
                let x0 = st as f32 * w / n as f32;
                pts.push((x0, y));
                pts.push((x0 + w / n as f32, y));
            }
        }
        VoiceKind::Dpcm => {}
    }
    pts
}

/// Draws the whole device but the meter's lit segments into an image.
fn render(d: &Dev, tab: usize, preset: &(String, bool), hover: Part, bw: f32, bh: f32, s: f32) -> Option<vg::Image> {
    let pad = (16.0 * s).ceil();
    let mut surface = vg::surfaces::raster_n32_premul(((bw + 0.5) as i32 + 2 * pad as i32, (bh + 0.5) as i32 + 2 * pad as i32))?;
    let c = surface.canvas();
    c.clear(vg::Color::TRANSPARENT);
    c.translate((pad, pad));
    let at = |x: f32, y: f32, w: f32, h: f32| vg::Rect::from_xywh(x * s, y * s, w * s, h * s);
    let locked = d.locked();
    panel(c, at(0.0, 0.0, DEVICE_W, DEVICE_H), s);
    // ── Header (full brightness on a locked track too).
    c.draw_rect(at(0.0, HEADER_H - 1.0, DEVICE_W, 1.0), &gfx::fill(Rgba::hex(0x262626)));
    let tab_r = at(12.0, 10.0, 4.0, 20.0);
    c.draw_rrect(gfx::rrect(tab_r, 2.0 * s), &gfx::fill(d.color));
    let fname = gfx::font(Face::SemiBold, 13.0 * s);
    let name = if d.voice == VoiceKind::Dpcm { "DPCM Sampler" } else { "Chip Synth" };
    let base = gfx::baseline_for_center(&fname, HEADER_H * 0.5 * s);
    gfx::draw_text(c, name, (28.0 * s).round(), base.round(), &fname, TEXT_HI);
    let fb = gfx::font(Face::SemiBold, 10.0 * s);
    let mut bx = 28.0 * s + gfx::text_width(&fname, name) + 12.0 * s;
    let bbase = gfx::baseline_for_center(&fb, HEADER_H * 0.5 * s);
    if locked {
        lock_glyph(c, (bx.round(), (bbase - 8.0 * s).round()), s, Rgba::hex(0x9A9A9A));
        bx += 12.0 * s;
    }
    gfx::draw_text_tracked(c, if locked { "LOCKED" } else { "STUDIO" }, bx.round(), bbase.round(), &fb, 0.6 * s, DEVICE_INK_LO);
    // The preset group.
    let [pr, fl, nx] = DeviceView::preset_rects();
    key_body(c, at(pr.0, pr.1, pr.2, pr.3), s, hover == Part::Prev);
    chevron(c, "M6 1L2 4L6 7", ((pr.0 + 9.0) * s, (pr.1 + 9.0) * s), s, Rgba::hex(0x9A9A9A));
    key_body(c, at(nx.0, nx.1, nx.2, nx.3), s, hover == Part::Next);
    chevron(c, "M2 1L6 4L2 7", ((nx.0 + 9.0) * s, (nx.1 + 9.0) * s), s, Rgba::hex(0x9A9A9A));
    let fr = at(fl.0, fl.1, fl.2, fl.3);
    field_body(c, fr, s, hover == Part::Field);
    let ff = gfx::font(Face::SemiBold, 12.0 * s);
    let fbase = gfx::baseline_for_center(&ff, fr.center_y());
    let mut tx = fr.left + 10.0 * s;
    if preset.1 {
        lock_glyph(c, (tx.round(), (fbase - 8.0 * s).round()), s, Rgba::hex(0x9A9A9A));
        tx += 12.0 * s;
    }
    gfx::draw_text_clipped(c, &preset.0, tx.round(), fbase.round(), fr.right - 24.0 * s - tx, &ff, TEXT_HI);
    chevron(c, "M1 2L4 6L7 2", (fr.right - 18.0 * s, fr.top + 9.0 * s), s, Rgba::hex(0x9A9A9A));
    // ── Sections.
    let lbl = gfx::font(Face::SemiBold, 10.0 * s);
    let dim = |c: &vg::Canvas, r: vg::Rect| {
        if locked {
            c.save_layer_alpha_f(r.with_outset((8.0 * s, 8.0 * s)), 0.4);
        } else {
            c.save();
        }
    };
    if d.voice == VoiceKind::Dpcm {
        section(c, &at, OSC.0, OSC.1, "SAMPLE", s);
        section(c, &at, ENV.0, ENV.1, "PLAYBACK", s);
        section(c, &at, OUT.0, OUT.1, "OUTPUT", s);
        let sel = d.param(ParamId::Sample);
        let fr = gfx::font(Face::Medium, 12.0 * s);
        for (smp, r) in DeviceView::sample_rows() {
            let rr = at(r.0, r.1, r.2, r.3);
            if smp == sel {
                c.draw_rrect(gfx::rrect(rr.with_inset((0.0, s)), 4.0 * s), &gfx::fill(Rgba::hex(0x262626)));
            } else if hover == Part::Sample(smp) {
                c.draw_rrect(gfx::rrect(rr.with_inset((0.0, s)), 4.0 * s), &gfx::fill(Rgba::white(0.04)));
            }
            let t = if smp < 0 { "The note picks".to_string() } else { chip::samples::NAMES[smp as usize].to_string() };
            gfx::draw_text(c, &t, (rr.left + 8.0 * s).round(), gfx::baseline_for_center(&fr, rr.center_y()).round(), &fr, if smp == sel { TEXT_HI } else { TEXT_MID });
        }
        let ir = DeviceView::import_rect();
        key_body(c, at(ir.0, ir.1, ir.2, ir.3), s, hover == Part::Import);
        let t = "Import audio…";
        gfx::draw_text(c, t, ((ir.0 + ir.2 * 0.5) * s - gfx::text_width(&fr, t) * 0.5).round(), gfx::baseline_for_center(&fr, (ir.1 + ir.3 * 0.5) * s).round(), &fr, TEXT_MID);
        // PLAYBACK: the sample and its rate, in the display; Loop.
        let (gx, gy, gw, gh) = DeviceView::graph_rect();
        let gr = at(gx, gy, gw, gh);
        dim(c, gr);
        display_well(c, gr, s);
        let fd = gfx::font(Face::SemiBold, 13.0 * s);
        let rate = d.param(ParamId::Rate).clamp(0, 15) as usize;
        let hz = chip::tables::DMC_RATE_NTSC.get(rate).map_or(0.0, |p| chip::tables::CPU_HZ_NTSC / *p as f64);
        let line = if sel < 0 { "Each note picks its sample".to_string() } else { format!("{}  ·  {:.1} kHz", chip::samples::NAMES[sel as usize], hz / 1000.0) };
        gfx::draw_text(c, &line, (gr.left + 14.0 * s).round(), gfx::baseline_for_center(&fd, gr.center_y()).round(), &fd, DISPLAY_WARM);
        c.restore();
        let lr = DeviceView::loop_rect();
        let on = d.param(ParamId::Loop) != 0;
        key_body(c, at(lr.0, lr.1, lr.2, lr.3), s, hover == Part::Loop);
        let ft = gfx::font(Face::SemiBold, 11.0 * s);
        let tw = gfx::text_width(&ft, "LOOP");
        // Lime only for toggle on-states (VISUAL-SPEC §13).
        gfx::draw_text(c, "LOOP", ((lr.0 + lr.2 * 0.5) * s - tw * 0.5).round(), gfx::baseline_for_center(&ft, (lr.1 + lr.3 * 0.5) * s).round(), &ft, if on { ACCENT } else { TEXT_MID });
    } else {
        section(c, &at, OSC.0, OSC.1, "OSCILLATOR", s);
        section(c, &at, ENV.0, ENV.1, "ENVELOPES", s);
        section(c, &at, OUT.0, OUT.1, "OUTPUT", s);
        // The Voice selector (the lock and the channel on a locked track).
        let vr = DeviceView::voice_rect(d, s);
        let r = at(vr.0, vr.1, vr.2, vr.3);
        field_body(c, r, s, hover == Part::Voice && !locked);
        let fv = gfx::font(Face::SemiBold, 11.0 * s);
        let mut vx = r.left + 8.0 * s;
        let vbase = gfx::baseline_for_center(&fv, r.center_y());
        if locked {
            lock_glyph(c, (vx.round(), (vbase - 8.0 * s).round()), s, Rgba::hex(0x9A9A9A));
            vx += 12.0 * s;
        }
        let vt = voice_text(d);
        gfx::draw_text(c, &vt, vx.round(), vbase.round(), &fv, TEXT_HI);
        if !locked {
            chevron(c, "M1 2L4 6L7 2", (vx + gfx::text_width(&fv, &vt) + 6.0 * s, r.top + 6.0 * s), s, Rgba::hex(0x9A9A9A));
        }
        // The waveform display.
        let wr = at(OSC.0 + PAD, SEC_Y + DISPLAY_Y, OSC.1 - 2.0 * PAD, DISPLAY_H);
        dim(c, wr);
        display_well(c, wr, s);
        c.save();
        c.clip_rrect(gfx::rrect(wr, 4.0 * s), vg::ClipOp::Intersect, true);
        c.draw_rect(vg::Rect::from_xywh(wr.left, wr.top + DISPLAY_H * 0.5 * s - 0.5 * s, wr.width(), s), &gfx::fill(Rgba::hex(0x1E1E1E)));
        let pts = waveform(d, OSC.1 - 2.0 * PAD, DISPLAY_H);
        if let Some(first) = pts.first() {
            let mut path = vg::PathBuilder::new();
            path.move_to((wr.left + first.0 * s, wr.top + first.1 * s));
            for p in &pts[1..] {
                path.line_to((wr.left + p.0 * s, wr.top + p.1 * s));
            }
            let path = path.detach();
            let mut glow = gfx::stroke(DISPLAY_WARM.with_alpha(0.5), 2.0 * s);
            glow.set_mask_filter(vg::MaskFilter::blur(vg::BlurStyle::Normal, 2.0 * s, false));
            c.draw_path(&path, &glow);
            let mut line = gfx::stroke(DISPLAY_WARM, 2.0 * s);
            line.set_stroke_join(vg::PaintJoin::Miter);
            c.draw_path(&path, &line);
        }
        c.restore();
        c.restore();
        // The envelope tabs and editor.
        if d.inst.is_kit() {
            let gr = at(ENV.0 + PAD, SEC_Y + DISPLAY_Y, ENV.1 - 2.0 * PAD, DISPLAY_H);
            dim(c, gr);
            display_well(c, gr, s);
            let f = gfx::font(Face::Medium, 11.0 * s);
            let mut lo = 0u8;
            for (i, k) in d.inst.kit.iter().enumerate().take(5) {
                let y = gr.top + (14.0 + i as f32 * 16.0) * s;
                let range = format!("{} – {}", note_name(lo), note_name(k.top.min(127)));
                gfx::draw_text(c, &range, (gr.left + 10.0 * s).round(), y.round(), &f, DISPLAY_WARM.with_alpha(0.7));
                gfx::draw_text(c, &k.name, (gr.left + 110.0 * s).round(), y.round(), &f, DISPLAY_WARM);
                lo = k.top.saturating_add(1);
            }
            c.restore();
            let note = "A kit: the note picks the drum (each piece keeps its own envelopes)";
            gfx::draw_text_clipped(c, note, (gr.left).round(), (gr.bottom + 28.0 * s).round(), gr.width(), &lbl, DEVICE_INK_LO);
        } else {
            for (m, r) in DeviceView::tab_rects(d, s) {
                let rr = at(r.0, r.1, r.2, r.3);
                let sel = m == tab;
                if sel {
                    c.draw_rrect(gfx::rrect(rr, 4.0 * s), &gfx::fill(Rgba::hex(0x262626)));
                } else if hover == Part::Tab(m) {
                    c.draw_rrect(gfx::rrect(rr, 4.0 * s), &gfx::fill(Rgba::white(0.04)));
                }
                let f = gfx::font(Face::SemiBold, 10.0 * s);
                gfx::draw_text_tracked(c, macro_label(d.voice, m), (rr.left + 8.0 * s).round(), gfx::baseline_for_center(&f, rr.center_y()).round(), &f, 0.6 * s, if sel { TEXT_HI } else { DEVICE_INK_LO });
            }
            let (gx, gy, gw, gh) = DeviceView::graph_rect();
            let gr = at(gx, gy, gw, gh);
            dim(c, gr);
            display_well(c, gr, s);
            draw_graph(c, gr, DeviceView::seq_of(&d.inst.inst, tab), tab, s);
            c.restore();
            if locked {
                let f = gfx::font(Face::SemiBold, 9.0 * s);
                let t = "4-BIT · PER FRAME";
                let tw = gfx::tracked_width(t, &f, 0.9 * s);
                gfx::draw_text_tracked(c, t, (gr.right - 8.0 * s - tw).round(), (gr.top + 14.0 * s).round(), &f, 0.9 * s, DEVICE_INK_LO);
            }
        }
    }
    // ── Knobs (40 % on a locked track) and their labels (full ink).
    let ks = knobs(d);
    for k in &ks {
        let o = (k.c.0 * s, k.c.1 * s);
        let norm = if k.max > k.min { (k.value - k.min) as f32 / (k.max - k.min) as f32 } else { 0.0 };
        let r = vg::Rect::from_xywh(o.0 - k.d * 0.6 * s, o.1 - k.d * 0.6 * s, k.d * 1.2 * s, k.d * 1.2 * s);
        if locked || k.inert.is_some() {
            c.save_layer_alpha_f(r, 0.4);
        } else {
            c.save();
        }
        knob(c, o, k.d, norm, s, k.inert.is_some());
        c.restore();
        // The label, or the value while hovered.
        let hovered = hover == Part::Knob(k.ctl);
        let text = if hovered { value_text(d, k.ctl, k.value).to_uppercase() } else { k.label.to_string() };
        let gap = if k.d >= BIG_D { 8.0 } else if k.d <= SMALL_D { 4.0 } else { 6.0 };
        let top = k.c.1 + k.d * 0.5 + gap;
        let base = gfx::css_baseline(&lbl, top * s, 12.0 * s);
        let tw = gfx::tracked_width(&text, &lbl, 0.8 * s);
        gfx::draw_text_tracked(c, &text, (o.0 - tw * 0.5).round(), base.round(), &lbl, 0.8 * s, if hovered { TEXT_HI } else { DEVICE_INK_LO });
    }
    // The meter's well and unlit segments (its lit part is drawn per frame).
    let (mx, my) = meter_origin(d);
    let mr = at(mx, my, METER_W, METER_H);
    dim(c, mr);
    c.draw_rrect(gfx::rrect(mr, 2.0 * s), &gfx::fill(Rgba::hex(0x050505)));
    gfx::inner_shadow(c, &gfx::rrect(mr, 2.0 * s), 0.0, s, 2.0 * s, Rgba::black(1.0));
    for k in 0..METER_SEGS {
        let r = at(mx + 2.0, my + METER_H - 2.0 - 3.0 - 4.0 * k as f32, METER_W - 4.0, 3.0);
        c.draw_rect(r, &gfx::fill(Rgba::hex(0xD6D0C4).with_alpha(0.1)));
    }
    c.restore();
    Some(surface.image_snapshot())
}

/// The envelope editor's bars in the display colour, loop and release
/// markers; "not used · draw to add" when empty.
fn draw_graph(c: &vg::Canvas, r: vg::Rect, q: &Seq, m: usize, s: f32) {
    if q.is_empty() {
        let f = gfx::font(Face::Medium, 11.0 * s);
        let t = "not used · draw to add";
        let tw = gfx::text_width(&f, t);
        gfx::draw_text(c, t, (r.center_x() - tw * 0.5).round(), gfx::baseline_for_center(&f, r.center_y()).round(), &f, DISPLAY_WARM.with_alpha(0.55));
        return;
    }
    let (_, lo, hi) = MACROS[m];
    let span = (hi - lo) as f32;
    let top = r.top + 7.0 * s;
    let bottom = r.bottom - 8.0 * s;
    let h = bottom - top;
    let zero_y = bottom - h * ((0 - lo) as f32 / span);
    if lo < 0 {
        c.draw_rect(vg::Rect::from_xywh(r.left + 6.0 * s, zero_y.round(), r.width() - 12.0 * s, s), &gfx::fill(Rgba::hex(0x2A2A2A)));
    }
    let n = q.len as usize;
    let bp = DeviceView::bar_pitch(r.width() / s, n) * s;
    let bw = (bp - 4.0 * s).max(s);
    for (j, v) in q.values().iter().enumerate() {
        let v = (*v as i32).clamp(lo, hi);
        let y = bottom - h * ((v - lo) as f32 / span);
        let x0 = r.left + 10.0 * s + bp * j as f32;
        let (t, b) = if lo < 0 { (y.min(zero_y), y.max(zero_y)) } else { (y, bottom) };
        let bar = vg::Rect::from_ltrb(x0, t, x0 + bw, b.max(t + 1.5 * s));
        let after_rel = q.release_at != NONE && j as u8 > q.release_at;
        let a = if after_rel { 0.55 } else { 1.0 };
        c.draw_rect(bar, &gfx::vgrad(bar, Rgba::hex(0xEDE3CD).with_alpha(a), DISPLAY_WARM.with_alpha(0.55 * a)));
        c.draw_rect(vg::Rect::from_xywh(bar.left, bar.top, bar.width(), s), &gfx::fill(Rgba::hex(0xF4EEE0).with_alpha(a)));
    }
    let fl = gfx::font(Face::SemiBold, 10.0 * s);
    for (idx, txt) in [(q.loop_at, "↺"), (q.release_at, "R")] {
        if idx != NONE {
            let x = (r.left + 10.0 * s + bp * idx as f32 - 2.0 * s).round();
            c.draw_rect(vg::Rect::from_xywh(x, r.top + 2.0 * s, s, r.height() - 4.0 * s), &gfx::fill(TEXT_HI.with_alpha(0.7)));
            let tab = vg::Rect::from_xywh(x, r.top + 2.0 * s, 12.0 * s, 12.0 * s);
            c.draw_rrect(gfx::rrect(tab, 2.0 * s), &gfx::fill(TEXT_HI.with_alpha(0.22)));
            gfx::draw_text(c, txt, x + 3.0 * s, r.top + 11.5 * s, &fl, TEXT_HI);
        }
    }
}

fn note_name(p: u8) -> String {
    const N: [&str; 12] = ["C", "C#", "D", "D#", "E", "F", "F#", "G", "G#", "A", "A#", "B"];
    format!("{}{}", N[p as usize % 12], p as i32 / 12 - 1)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn dev(voice: VoiceKind, lock: Option<usize>) -> Dev {
        Dev {
            lane: 0,
            voice,
            lock,
            color: Rgba::hex(0x4FB8C9),
            chan: lock.unwrap_or(voice.lane()),
            params: engine::Params::default().values[lock.unwrap_or(voice.lane())],
            unison: Unison::default(),
            inst_id: 1,
            inst: LaneInstrument::plain(),
            states: Default::default(),
        }
    }

    fn labels(d: &Dev) -> Vec<&'static str> {
        knobs(d).iter().map(|k| k.label).collect()
    }

    /// Controls a voice doesn't have are hidden (the registry), and a locked
    /// track has no Unison, Detune or Drift.
    #[test]
    fn controls_follow_the_voice_and_the_lock() {
        let p = labels(&dev(VoiceKind::Pulse, None));
        // The board's first rows, then the small knobs.
        assert_eq!(&p[..6], ["DUTY", "UNISON", "DETUNE", "DRIFT", "SWEEP", "SWEEP RATE"]);
        assert_eq!(&p[6..10], ["VIB DEPTH", "VIB RATE", "ARP SPEED", "VIB DELAY"]);
        assert!(p.contains(&"SWEEP") && p.contains(&"VOLUME") && p.contains(&"ENVELOPE") && p.contains(&"ARP SPEED"));
        let t = labels(&dev(VoiceKind::Triangle, None));
        assert!(!t.contains(&"DUTY") && !t.contains(&"VOLUME") && t.contains(&"UNISON"));
        let n = labels(&dev(VoiceKind::Noise, None));
        assert!(n.contains(&"MODE") && n.contains(&"PERIOD") && !n.contains(&"DUTY"));
        let s = labels(&dev(VoiceKind::Saw, None));
        assert!(s.contains(&"RATE") && !s.contains(&"VOLUME"));
        let locked = labels(&dev(VoiceKind::Pulse, Some(1)));
        assert!(!locked.iter().any(|l| ["UNISON", "DETUNE", "DRIFT"].contains(l)));
        assert!(locked.contains(&"DUTY"));
        assert!(!macro_used(VoiceKind::Triangle, 3) && macro_used(VoiceKind::Pulse, 3));
        assert!((0..4).all(|m| !macro_used(VoiceKind::Dpcm, m)));
    }

    /// The board's places: the sections, the preset group centred, the
    /// OSCILLATOR row's first knob in its 64 px cell.
    #[test]
    fn geometry_follows_the_board() {
        let [p, f, n] = DeviceView::preset_rects();
        assert_eq!((p.0, f.0, n.0), (278.0, 310.0, 576.0));
        assert_eq!((OSC, ENV, OUT), ((12.0, 256.0), (280.0, 400.0), (692.0, 176.0)));
        let d = dev(VoiceKind::Pulse, Some(1));
        let k = knobs(&d);
        let duty = k.iter().find(|k| k.label == "DUTY").unwrap();
        // Alone in the locked row: centred (the board's locked Duty at x 365).
        assert_eq!(duty.c, (140.0, SEC_Y + KNOB_ROW_Y + 20.0));
        let vol = k.iter().find(|k| k.label == "VOLUME").unwrap();
        assert_eq!(vol.d, BIG_D);
        // The studio row: the board's three 64 px cells, space-between (x 0, 84, 168).
        let st = knobs(&dev(VoiceKind::Pulse, None));
        let xs: Vec<f32> = st.iter().filter(|k| ["DUTY", "UNISON", "DETUNE"].contains(&k.label)).map(|k| k.c.0).collect();
        assert_eq!(xs, [24.0 + 32.0, 24.0 + 84.0 + 32.0, 24.0 + 168.0 + 32.0]);
        // The envelope row: three cells, space-around (the board's).
        let ex: Vec<f32> = st.iter().filter(|k| ["VIB DEPTH", "VIB RATE", "ARP SPEED"].contains(&k.label)).map(|k| k.c.0).collect();
        let gap = (376.0 - 192.0) / 3.0;
        assert_eq!(ex, [292.0 + gap * 0.5 + 32.0, 292.0 + gap * 1.5 + 64.0 + 32.0, 292.0 + gap * 2.5 + 128.0 + 32.0]);
    }

    #[test]
    fn waveforms_are_drawn_for_every_synth_voice() {
        for v in crate::bank::SYNTH_VOICES {
            assert!(waveform(&dev(v, None), 232.0, 96.0).len() >= 6, "{v:?}");
        }
    }
}
