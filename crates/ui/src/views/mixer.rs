//! The mixer view (BATCH4B_PROMPT.md §7, VISION.md "The mixer view"; boards
//! `docs/design/meeting-2026-11-26/mixer.*`, `knobs.*` option A,
//! `meter-colours.*` option 2). It replaces the arrangement in place (Tab or
//! the view key, as the reference's view selector: Live 12 manual, ch. 42.1
//! "Showing and Hiding Views", R77).
//!
//! **One model.** The strips are another view of the tracks' controls: the
//! fader, pan, M and S emit exactly the header's events (`SetLaneVolume`,
//! `SetLanePan`, `ToggleMute`, `ToggleSolo`, `OpenRename`, gestures for one
//! undo step), with the header's hover help and lock rules. The mixer adds no
//! control of its own.
//!
//! **Geometry** (logical px, from the board's HTML): strips 152 × 552, 8 px
//! apart, 24 px from the edges, from y 60 under the channel-budget bar (y 16,
//! 32 tall); Main 176 px at the right. Inside a strip: the name plate (8, 8,
//! 136 × 40) with the track colour as a lit 4 px bar on top and a two-digit
//! number slot; the knob (34 px, centre 76, 73); M/S 24 × 24 at y 118; the clip
//! mark (102, 148, 16 × 4); the meter glass (97, 162, 22 × 268) with two
//! 7 px columns of 65 segments (3 px + 1 px gap) from y 167; the fader slot
//! (64, 160, 6 × 272), the cap 30 × 44 centred on the dB taper over 260 px
//! from y 166 (the headers' taper: (dB + 48) / 54); the scale (+6, 0, 6, 12,
//! 24, 36, ∞); PEAK at y 440; VOICES / the channel at y 472; the dB readout
//! (12, 514, 128 × 26).
//!
//! **Performance** (§7.7). Each strip keeps two cached images per content and
//! scale: everything static under the meters (panel and grain, plate, knob
//! body, key bodies, scale, slot, glass and unlit segments, labels and
//! wells), and the glass reflection over them; plus one pre-rendered lit
//! meter column per kind and scale. A frame draws those images, the lit part
//! of each meter (a clipped blit), the peak segment, the cap, the knob's
//! indicator and ring, the key glyphs and the readouts.

use std::cell::RefCell;

use vizia::prelude::*;
use vizia::vg;

use super::gfx::{self, Face};
use crate::theme::tokens::*;
use crate::{AppEvent, Signals};

pub const STRIP_W: f32 = 152.0;
pub const MAIN_W: f32 = 176.0;
pub const STRIP_H: f32 = 552.0;
pub const STRIP_GAP: f32 = 8.0;
pub const MARGIN: f32 = 24.0;
pub const BUDGET_TOP: f32 = 16.0;
pub const BUDGET_H: f32 = 32.0;
pub const STRIPS_TOP: f32 = 60.0;
/// The gap before Main.
pub const MAIN_GAP: f32 = 24.0;
const SCROLLBAR_H: f32 = 4.0;
const SCROLLBAR_GAP: f32 = 4.0;

/// The meter: 65 segments of 3 px with 1 px gaps over 260 px from y 166.
const METER_TOP: f32 = 166.0;
const METER_LEN: f32 = 260.0;
const SEGS: usize = 65;
const SEG_H: f32 = 3.0;
const SEG_STEP: f32 = 4.0;
/// Zone ends (segments from the bottom): warm white to 41 (63 %), amber to 52
/// (80 %), orange to 58 (0 dB, 89 %), red above (`meter-colours` option 2).
const ZONES: [usize; 3] = [41, 52, 58];
/// (lit edge, lit centre, glow) per zone: warm white, amber, orange, red.
const ZONE_LIT: [(u32, u32, u32); 4] = [(0xA09C93, 0xE0DCD3, 0xD6D0C4), (0xB68830, 0xF5C870, 0xF2B640), (0xBF6817, 0xFFA757, 0xFF8A1F), (0xBF2C23, 0xFF6C63, 0xFF3B2F)];
const SEG_GLOW_A: f32 = 0.45;
const UNLIT_A: f32 = 0.10;
const PEAK_SEG: Rgba = Rgba::hex(0xF2F2F2);

/// The knob (option A, machined).
/// Centred in the padding box (150 px wide on a 152 px strip: centre 75).
const KNOB_C: (f32, f32) = (75.0, 73.0);
const KNOB_R: f32 = 17.0;
const RING_W: f32 = 2.0;
/// Full pan swing of the indicator either side (degrees).
const KNOB_SWING: f32 = 135.0;
const PAN_TEXT_Y: f32 = 94.0;

const KEYS_Y: f32 = 118.0;
const KEY: f32 = 24.0;
const KEYS_X: [f32; 2] = [47.0, 79.0];
const CLIP_RECT: (f32, f32, f32, f32) = (102.0, 148.0, 16.0, 4.0);

/// A track strip's columns: glass x, width and the two segment columns.
const GLASS: (f32, f32) = (97.0, 22.0);
const SEG_COLS: [f32; 2] = [100.0, 109.0];
const SEG_W: f32 = 7.0;
const SLOT_X: f32 = 64.0;
const CAP_X: f32 = 52.0;
const TICK_X: f32 = 58.0;
const LABEL_R: f32 = 54.0;
/// Main's columns (wider meters).
const M_GLASS: (f32, f32) = (105.0, 30.0);
const M_SEG_COLS: [f32; 2] = [108.0, 121.0];
const M_SEG_W: f32 = 11.0;
const M_SLOT_X: f32 = 72.0;
const M_CAP_X: f32 = 60.0;
const M_TICK_X: f32 = 66.0;
const M_LABEL_R: f32 = 62.0;

const CAP_W: f32 = 30.0;
/// Room around the cap's image for its shadow (3σ).
const CAP_PAD: f32 = 16.0;
const CAP_H: f32 = 44.0;
const PEAK_Y: f32 = 440.0;
const VOICES_Y: f32 = 472.0;
/// The dB readout: `bottom: 12px` inside the 1 px border (550 − 12 − 26).
const READOUT: (f32, f32) = (512.0, 26.0);
/// The board's parts are placed from the strip's padding box, inside its
/// 1 px border: every position above is that far in (CSS absolute offsets).
const INSET: f32 = 1.0;

const INK_HI: Rgba = Rgba::hex(0xE6E6E6);
const INK_VAL: Rgba = Rgba::hex(0xC8C8C8);
const INK_MID: Rgba = Rgba::hex(0x9A9A9A);
// #8A8A8A, not the board's #808080: 4.83 : 1 on the panel's lightest top (#1E1E1E; #808080
// gave 4.22), gate 7 (B4B-D32).
const INK_LO: Rgba = Rgba::hex(0x8A8A8A);
const NUM_INK: Rgba = Rgba::hex(0x3A3A3A);
const RECESS: Rgba = Rgba::hex(0x0E0E0E);
/// Over budget (the record red, as the TRACKS cell).
const OVER_INK: Rgba = Rgba::hex(0xE0483B);

/// The dB scale marks: (dB, label).
const SCALE: [(f32, &str); 7] = [(6.0, "+6"), (0.0, "0"), (-6.0, "6"), (-12.0, "12"), (-24.0, "24"), (-36.0, "36"), (f32::NEG_INFINITY, "\u{221E}")];

thread_local! {
    /// Each strip's box (physical px) by lane, for typed values and renames
    /// (the rename field sits over the strip in the mixer).
    static STRIP_AT: RefCell<Vec<Option<BoundingBox>>> = const { RefCell::new(Vec::new()) };
}

fn register(lane: usize, b: BoundingBox) {
    STRIP_AT.with(|r| {
        let mut r = r.borrow_mut();
        if r.len() <= lane {
            r.resize(lane + 1, None);
        }
        r[lane] = Some(b);
    });
}

/// Where a typed value or rename opens over strip `lane` (logical px, window):
/// (x, y, w, h) of the name plate, the pan value or the dB readout.
pub fn field_rect(lane: usize, what: Field, scale: f32) -> Option<(f32, f32, f32, f32)> {
    let b = STRIP_AT.with(|r| r.borrow().get(lane).copied().flatten())?;
    let (x, y) = (b.x / scale + INSET, b.y / scale + INSET);
    Some(match what {
        Field::Name => (x + 8.0, y + 8.0, STRIP_W - 16.0, 40.0),
        Field::Pan => (x + 40.0, y + PAN_TEXT_Y - 2.0, 72.0, 16.0),
        Field::Volume => (x + 12.0, y + READOUT.0, STRIP_W - 24.0, READOUT.1),
    })
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Field {
    Name,
    Pan,
    Volume,
}

/// A meter column's lit segments for a post-fader level (linear, 1.0 = the
/// fader's 0 dB) on the dB taper.
pub fn lit_segments(level: f32) -> usize {
    if level <= 0.0 {
        return 0;
    }
    let pos = crate::state::db_to_pos(crate::state::gain_to_db(level));
    ((pos * SEGS as f32).floor() as usize).min(SEGS)
}

/// The zone of segment `k` (0 = the bottom).
pub fn zone(k: usize) -> usize {
    ZONES.iter().position(|z| k < *z).unwrap_or(3)
}

/// The cap's centre y (logical, strip-relative) at `db`.
pub fn cap_y(db: f32) -> f32 {
    METER_TOP + crate::state::fader_y(db, METER_LEN)
}

fn local(cx: &EventContext) -> (f32, f32) {
    let b = cx.bounds();
    let s = cx.scale_factor();
    ((cx.mouse().cursor_x - b.x) / s, (cx.mouse().cursor_y - b.y) / s)
}

fn inside(r: (f32, f32, f32, f32), p: (f32, f32)) -> bool {
    p.0 >= r.0 && p.0 < r.0 + r.2 && p.1 >= r.1 && p.1 < r.1 + r.3
}

fn rgba(h: u32) -> Rgba {
    Rgba::hex(h)
}

// ─────────────────────────────── drawing ───────────────────────────────

/// The strip panel: the board's gradient, two soft radial lights (the
/// top-light), a fine fractal-noise grain (no image files), a 1 px #2A2A2A
/// border, an inner top highlight and a soft drop shadow.
fn draw_panel(c: &vg::Canvas, r: vg::Rect, s: f32, top: Rgba, bottom: Rgba) {
    let rr = gfx::rrect(r, 8.0 * s);
    gfx::drop_shadow(c, &rr, 0.0, 4.0 * s, 12.0 * s, 0.0, Rgba::black(0.6));
    c.draw_rrect(rr, &gfx::vgrad(r, top, bottom));
    c.save();
    c.clip_rrect(rr, vg::ClipOp::Intersect, true);
    // Grain: Skia's fractal noise, as luminance at a few per cent.
    if let Some(noise) = vg::shaders::fractal_noise((0.9 / s, 0.9 / s), 2, 7.0, None) {
        let mut p = vg::Paint::default();
        p.set_shader(noise);
        p.set_color_filter(vg::color_filters::matrix(&vg::ColorMatrix::new(
            0.33, 0.33, 0.33, 0.0, 0.0, 0.33, 0.33, 0.33, 0.0, 0.0, 0.33, 0.33, 0.33, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.035,
        ), None));
        c.draw_rect(r, &p);
    }
    c.draw_rect(r, &gfx::shaded(gfx::radial((r.left + r.width() * 0.3, r.top), r.width() * 1.2, &[Rgba::white(0.06), Rgba::white(0.0)], None)));
    c.draw_rect(r, &gfx::shaded(gfx::radial((r.left + r.width() * 0.8, r.bottom), r.width(), &[Rgba::white(0.035), Rgba::white(0.0)], None)));
    c.restore();
    gfx::ring(c, &rr, gfx::hairline(s), &gfx::fill(rgba(0x2A2A2A)));
    gfx::top_highlight(c, &rr.with_inset((gfx::hairline(s), gfx::hairline(s))), gfx::hairline(s), Rgba::white(0.08));
}

/// A recessed well (the plate, readouts): fill, inner shadow, a faint lip.
fn recess(c: &vg::Canvas, r: vg::Rect, radius: f32, s: f32, fill: vg::Paint, shadow: f32) {
    let rr = gfx::rrect(r, radius * s);
    c.draw_rect(vg::Rect::from_xywh(r.left + radius * s, r.bottom, r.width() - 2.0 * radius * s, gfx::hairline(s)), &gfx::fill(Rgba::white(0.05)));
    c.draw_rrect(rr, &fill);
    gfx::inner_shadow(c, &rr, 0.0, s, shadow * s, Rgba::black(0.8));
}

fn draw_lock(c: &vg::Canvas, o: (f32, f32), s: f32, ink: Rgba) {
    c.draw_path(&gfx::svg_path("M0.5 4H7.5V8.5H0.5Z", o, s), &gfx::fill(ink));
    c.draw_path(&gfx::svg_path("M2 4V2.8a2 2 0 0 1 4 0V4", o, s), &gfx::stroke(ink, 1.2 * s));
}

/// What a strip's static image shows.
#[derive(Clone, Debug, PartialEq)]
struct StaticKey {
    main: bool,
    colour: u32,
    number: String,
    name: String,
    badge: String,
    locked: bool,
    w: i32,
    h: i32,
    s: u32,
}

/// Renders everything under the meters (see the module docs).
fn render_static(k: &StaticKey, s: f32) -> Option<vg::Image> {
    let mut surface = vg::surfaces::raster_n32_premul((k.w.max(1), k.h.max(1)))?;
    let c = surface.canvas();
    c.clear(vg::Color::TRANSPARENT);
    let w = if k.main { MAIN_W } else { STRIP_W };
    let at = |x: f32, y: f32| (x * s, y * s);
    let rect = |x: f32, y: f32, ww: f32, hh: f32| vg::Rect::from_xywh(x * s, y * s, ww * s, hh * s);
    // Room for the panel's shadow: the image is the strip's box; the shadow
    // that falls outside it is drawn by the area (MixerArea) once.
    if k.main {
        draw_panel(c, rect(0.0, 0.0, w, STRIP_H), s, rgba(0x141414), rgba(0x0B0B0B));
    } else {
        draw_panel(c, rect(0.0, 0.0, w, STRIP_H), s, rgba(0x1E1E1E), rgba(0x141414));
    }
    // Everything inside is placed from the padding box (INSET).
    c.translate((INSET * s, INSET * s));
    // The name plate: recessed, the colour as a lit 4 px bar on top.
    let plate = rect(8.0, 8.0, w - 16.0, 40.0);
    let (p0, p1) = if k.main { (rgba(0x0A0A0A), rgba(0x111111)) } else { (rgba(0x0E0E0E), rgba(0x151515)) };
    recess(c, plate, 4.0, s, gfx::vgrad(plate, p0, p1), 2.0);
    c.save();
    c.clip_rrect(gfx::rrect(plate, 4.0 * s), vg::ClipOp::Intersect, true);
    let bar = vg::Rect::from_xywh(plate.left, plate.top, plate.width(), 4.0 * s);
    let colour = rgba(k.colour);
    // The lit strip's colours (the board: colour → +28 % → +55 % white).
    let stops: [Rgba; 3] = if k.main { [rgba(0x000000), rgba(0x1C1C1C), rgba(0x363636)] } else { crate::theme::palette::strip(colour, 1.0).0 };
    if !k.main {
        let mut glow = gfx::fill(colour.with_alpha(0.35));
        glow.set_mask_filter(vg::MaskFilter::blur(vg::BlurStyle::Normal, 4.0 * s, false));
        c.draw_rect(bar.with_outset((0.0, 2.0 * s)), &glow);
    }
    c.draw_rect(bar, &gfx::shaded(gfx::linear((bar.left, bar.top), (bar.right, bar.top), &stops, Some(&[0.0, 0.55, 1.0]))));
    c.restore();
    // Number (a fixed two-digit slot, right-aligned), name, badge line.
    // Line 1 (number and name, baseline-aligned): its box starts under the
    // 4 px bar and 4 px padding; the 14 px number sets the baseline. Line 2
    // (the badge, 9 px) follows 1 px under line 1's box.
    let line1 = (8.0 + 4.0 + 4.0) * s;
    let nf = gfx::font(Face::SemiBold, 12.0 * s);
    // Main has no number: its line is the 12 px name's.
    let (_, m14) = if k.main { nf.metrics() } else { gfx::font(Face::HeavyItalic, 14.0 * s).metrics() };
    let base1 = (line1 - m14.ascent).round();
    let line2 = line1 + (m14.descent - m14.ascent) + 1.0 * s;
    let mut x = (8.0 + 8.0) * s;
    if !k.main {
        let numf = gfx::font(Face::HeavyItalic, 14.0 * s);
        let slot = if k.number.len() > 2 { 28.0 } else { 20.0 };
        // Tabular figures (the board's `font-variant-numeric: tabular-nums`):
        // every digit in a cell as wide as "0", right-aligned in the slot.
        let cell = gfx::text_width(&numf, "0");
        let mut dx = x + slot * s - cell * k.number.chars().count() as f32;
        for ch in k.number.chars() {
            let g = ch.to_string();
            let gw = gfx::text_width(&numf, &g);
            gfx::draw_text(c, &g, (dx + (cell - gw) * 0.5).round(), base1, &numf, NUM_INK);
            dx += cell;
        }
        x += (slot + 6.0) * s;
    }
    gfx::draw_text_clipped(c, &k.name, x.round(), base1, (w - 8.0) * s - x - 8.0 * s, &nf, INK_HI);
    let bf = gfx::font(Face::SemiBold, 9.0 * s);
    let tr = 0.06 * 9.0 * s;
    let (_, m9) = bf.metrics();
    let base2 = (line2 - m9.ascent).round();
    let mut bx = (8.0 + 8.0) * s;
    if k.locked {
        draw_lock(c, (bx.round(), (base2 - 8.0 * s).round()), s, INK_MID);
        bx += 11.0 * s;
    }
    c.save();
    c.clip_rect(vg::Rect::from_ltrb(plate.left, plate.top, plate.right - 8.0 * s, plate.bottom), None, None);
    gfx::draw_text_tracked(c, &k.badge, bx.round(), base2, &bf, tr, INK_LO);
    c.restore();

    let (glass, cols, segw, slot_x, tick_x, label_r) = if k.main { (M_GLASS, M_SEG_COLS, M_SEG_W, M_SLOT_X, M_TICK_X, M_LABEL_R) } else { (GLASS, SEG_COLS, SEG_W, SLOT_X, TICK_X, LABEL_R) };
    if !k.main {
        // The knob's track ring (270°, the gap at the bottom) and its body
        // (the knurled skirt and the brushed cap): drawn by `draw_knob_body`.
        draw_knob_body(c, at(KNOB_C.0, KNOB_C.1), s, k.locked);
        if k.locked {
            // Locked: the indicator is parked at centre under the veil.
            let o = at(KNOB_C.0, KNOB_C.1);
            draw_indicator(c, o, 0.0, s);
            c.draw_oval(vg::Rect::from_xywh(o.0 - 14.0 * s, o.1 - 14.0 * s, 28.0 * s, 28.0 * s), &gfx::fill(rgba(0x0A0A0A).with_alpha(0.45)));
        }
        // M and S key bodies (the mixer key style).
        for kx in KEYS_X {
            draw_key_body(c, rect(kx, KEYS_Y, KEY, KEY), s);
        }
    } else {
        // Main: PEAK at the top (label, then a 72 × 26 readout).
        let lf = gfx::font(Face::SemiBold, 10.0 * s);
        let ltr = 0.1 * 10.0 * s;
        let lw = gfx::tracked_width("PEAK", &lf, ltr) - ltr;
        gfx::draw_text_tracked(c, "PEAK", (((w - 2.0 * INSET) * s - lw) * 0.5).round(), gfx::css_baseline(&lf, 64.0 * s, 12.0 * s).round(), &lf, ltr, INK_LO);
        let pr = rect((w - 2.0 * INSET - 72.0) * 0.5, 80.0, 72.0, 26.0);
        recess(c, pr, 4.0, s, gfx::fill(RECESS), 1.0);
    }
    // The clip mark's well (tracks; Main's PEAK says it).
    if !k.main {
        let cr = rect(CLIP_RECT.0, CLIP_RECT.1, CLIP_RECT.2, CLIP_RECT.3);
        c.draw_rrect(gfx::rrect(cr, s), &gfx::fill(rgba(0x1A1A1A)));
        gfx::inner_shadow(c, &gfx::rrect(cr, s), 0.0, s, s, Rgba::black(1.0));
    }
    // The meter glass and its unlit segments (their zone colour at 10 %).
    let g = rect(glass.0, 162.0, glass.1, 268.0);
    recess(c, g, 3.0, s, gfx::fill(rgba(0x050505)), 4.0);
    for seg in 0..SEGS {
        let y = METER_TOP + METER_LEN - (seg as f32 + 1.0) * SEG_STEP + 1.0;
        let z = ZONE_LIT[zone(seg)].2;
        for cx_ in cols {
            c.draw_rect(rect(cx_, y, segw, SEG_H), &gfx::fill(rgba(z).with_alpha(UNLIT_A)));
        }
    }
    // The engraved scale: ticks and labels (Inter 600 9/10, INK_LO).
    let sf = gfx::font(Face::SemiBold, 9.0 * s);
    for (db, label) in SCALE {
        let y = METER_TOP + crate::state::fader_y(db, METER_LEN);
        c.draw_rect(rect(tick_x, y, 6.0, 1.0), &gfx::fill(rgba(0x4A4A4A)));
        c.draw_rect(rect(tick_x, y + 1.0, 6.0, 1.0), &gfx::fill(Rgba::white(0.05)));
        let lw = gfx::text_width(&sf, label);
        let base = gfx::css_baseline(&sf, (y - 5.0) * s, 10.0 * s).round();
        gfx::draw_text(c, label, (label_r * s - lw).round(), base, &sf, INK_LO);
    }
    // The fader slot cut into the panel.
    let sl = rect(slot_x, METER_TOP - 6.0, 6.0, METER_LEN + 12.0);
    let srr = gfx::rrect(sl, 3.0 * s);
    // Locked: the whole fader (slot and cap) at 45 % (the board's `opacity: .45`).
    if k.locked {
        c.save_layer_alpha_f(sl.with_outset((4.0 * s, 4.0 * s)), 0.45);
    }
    c.draw_rrect(srr, &gfx::shaded(gfx::linear((sl.left, sl.top), (sl.right, sl.top), &[rgba(0x000000), rgba(0x0B0B0B), rgba(0x000000)], Some(&[0.0, 0.5, 1.0]))));
    gfx::inner_shadow(c, &srr, 0.0, s, 2.0 * s, Rgba::black(0.9));
    // Its lips: a light one under it, a dark one over it (`0 1px 0` white .06,
    // `0 -1px 0` black .6).
    c.draw_rect(vg::Rect::from_xywh(sl.left + 2.0 * s, sl.bottom, sl.width() - 4.0 * s, gfx::hairline(s)), &gfx::fill(Rgba::white(0.06)));
    c.draw_rect(vg::Rect::from_xywh(sl.left + 2.0 * s, sl.top - gfx::hairline(s), sl.width() - 4.0 * s, gfx::hairline(s)), &gfx::fill(Rgba::black(0.6)));
    if k.locked {
        c.restore();
    }
    if !k.main {
        // PEAK: the label and its recessed readout; the readout well at the bottom.
        let lf = gfx::font(Face::SemiBold, 10.0 * s);
        let ltr = 0.1 * 10.0 * s;
        gfx::draw_text_tracked(c, "PEAK", 12.0 * s, gfx::css_baseline(&lf, (PEAK_Y + 4.0) * s, 12.0 * s).round(), &lf, ltr, INK_LO);
        recess(c, rect(w - 2.0 * INSET - 12.0 - 64.0, PEAK_Y, 64.0, 20.0), 4.0, s, gfx::fill(RECESS), 1.0);
    }
    let ro = rect(12.0, READOUT.0, w - 2.0 * INSET - 24.0, READOUT.1);
    recess(c, ro, 4.0, s, gfx::fill(if k.main { RECESS } else { rgba(0x0A0A0A) }), 3.0);
    Some(surface.image_snapshot())
}

/// The glass's diagonal reflection (drawn over the segments).
fn render_glass_overlay(main: bool, s: f32) -> Option<vg::Image> {
    let glass = if main { M_GLASS } else { GLASS };
    let (w, h) = ((glass.1 * s).ceil() as i32, (268.0 * s).ceil() as i32);
    let mut surface = vg::surfaces::raster_n32_premul((w.max(1), h.max(1)))?;
    let c = surface.canvas();
    c.clear(vg::Color::TRANSPARENT);
    let r = vg::Rect::from_xywh(0.0, 0.0, w as f32, h as f32);
    c.clip_rrect(gfx::rrect(r, 3.0 * s), vg::ClipOp::Intersect, true);
    // linear-gradient(115deg, white .10 0%, white .03 35%, transparent 36%).
    let a = 115f32.to_radians();
    let (dx, dy) = (a.sin(), -a.cos());
    let len = r.width() * dx.abs() + r.height() * dy.abs();
    let (cx0, cy0) = (r.center_x() - dx * len * 0.5, r.center_y() - dy * len * 0.5);
    let p = gfx::shaded(gfx::linear((cx0, cy0), (cx0 + dx * len, cy0 + dy * len), &[Rgba::white(0.10), Rgba::white(0.03), Rgba::white(0.0)], Some(&[0.0, 0.35, 0.36])));
    c.draw_rect(r, &p);
    Some(surface.image_snapshot())
}

/// A fully lit meter column (`segs` segments, every zone lit with its glow),
/// `w` px wide: the frame blits its lit part.
fn render_lit_column(segw: f32, s: f32) -> Option<vg::Image> {
    let pad = (4.0 * s).ceil();
    let (w, h) = ((segw * s + 2.0 * pad).ceil() as i32, (METER_LEN * s + 2.0 * pad).ceil() as i32);
    let mut surface = vg::surfaces::raster_n32_premul((w.max(1), h.max(1)))?;
    let c = surface.canvas();
    c.clear(vg::Color::TRANSPARENT);
    for seg in 0..SEGS {
        let y = pad + (METER_LEN - (seg as f32 + 1.0) * SEG_STEP + 1.0) * s;
        let r = vg::Rect::from_xywh(pad, y, segw * s, SEG_H * s);
        let (_, _, glow) = ZONE_LIT[zone(seg)];
        let mut g = gfx::fill(rgba(glow).with_alpha(SEG_GLOW_A));
        g.set_mask_filter(vg::MaskFilter::blur(vg::BlurStyle::Normal, 2.0 * s, false));
        c.draw_rect(r, &g);
    }
    for seg in 0..SEGS {
        let y = pad + (METER_LEN - (seg as f32 + 1.0) * SEG_STEP + 1.0) * s;
        let r = vg::Rect::from_xywh(pad, y, segw * s, SEG_H * s);
        let (edge, mid, _) = ZONE_LIT[zone(seg)];
        c.draw_rect(r, &gfx::shaded(gfx::linear((r.left, r.top), (r.right, r.top), &[rgba(edge), rgba(mid), rgba(edge)], Some(&[0.0, 0.5, 1.0]))));
    }
    Some(surface.image_snapshot())
}

/// The machined knob's body (option A): the 270° track ring, the knurled
/// skirt (28 px), the brushed cap (20 px). Locked: darkened.
fn draw_knob_body(c: &vg::Canvas, o: (f32, f32), s: f32, locked: bool) {
    let r = KNOB_R * s;
    // Track ring: from 225° (bottom left) clockwise 270°, 2 px.
    let ring = vg::Rect::from_xywh(o.0 - r + RING_W * 0.5 * s, o.1 - r + RING_W * 0.5 * s, 2.0 * r - RING_W * s, 2.0 * r - RING_W * s);
    let mut p = gfx::stroke(rgba(0x2A2A2A), RING_W * s);
    p.set_stroke_cap(vg::PaintCap::Butt);
    c.draw_arc(ring, 135.0, 270.0, false, &p);
    // Skirt: knurled (alternating 5° wedges), a soft top light, shadows.
    let sk = 14.0 * s;
    let skirt = vg::Rect::from_xywh(o.0 - sk, o.1 - sk, 2.0 * sk, 2.0 * sk);
    let rr = vg::RRect::new_oval(skirt);
    gfx::drop_shadow(c, &rr, 0.0, 2.0 * s, 4.0 * s, 0.0, Rgba::black(0.75));
    let wedges: Vec<Rgba> = (0..72).map(|i| if i % 2 == 0 { rgba(0x1C1C1C) } else { rgba(0x333333) }).collect();
    let pos: Vec<f32> = (0..72).map(|i| i as f32 / 71.0).collect();
    let mut knurl = vg::Paint::default();
    knurl.set_anti_alias(true);
    knurl.set_shader(sweep(o, &wedges, &pos));
    c.draw_oval(skirt, &knurl);
    c.draw_oval(skirt, &gfx::shaded(gfx::radial((o.0, o.1 - 0.4 * sk), sk * 1.2, &[Rgba::white(0.12), Rgba::white(0.0)], Some(&[0.0, 0.6]))));
    c.draw_arc(skirt.with_inset((0.5 * s, 0.5 * s)), 200.0, 140.0, false, &gfx::stroke(Rgba::white(0.18), s));
    // Cap: brushed (a conic of light and dark), a top highlight.
    let cr = 10.0 * s;
    let cap = vg::Rect::from_xywh(o.0 - cr, o.1 - cr, 2.0 * cr, 2.0 * cr);
    gfx::drop_shadow(c, &vg::RRect::new_oval(cap), 0.0, s, 2.0 * s, 0.0, Rgba::black(0.8));
    let brushed = [0x7E7E7E, 0xC9C9C9, 0x6A6A6A, 0xBEBEBE, 0x727272, 0xC6C6C6, 0x6C6C6C, 0xBCBCBC, 0x7E7E7E].map(rgba);
    let bpos = [0.0, 0.12, 0.25, 0.38, 0.5, 0.63, 0.75, 0.88, 1.0];
    let mut bp = vg::Paint::default();
    bp.set_anti_alias(true);
    bp.set_shader(sweep(o, &brushed, &bpos));
    c.draw_oval(cap, &bp);
    c.draw_oval(cap, &gfx::shaded(gfx::radial((o.0, o.1 - 0.5 * cr), cr * 1.1, &[Rgba::white(0.35), Rgba::white(0.0)], Some(&[0.0, 0.55]))));
    c.draw_arc(cap.with_inset((0.5 * s, 0.5 * s)), 200.0, 140.0, false, &gfx::stroke(Rgba::white(0.6), s));
    let _ = locked;
}

/// The knob's indicator at `ang` degrees from 12 o'clock.
fn draw_indicator(c: &vg::Canvas, o: (f32, f32), ang: f32, s: f32) {
    c.save();
    c.rotate(ang, Some(vg::Point::new(o.0, o.1)));
    let ind = vg::Rect::from_xywh(o.0 - s, o.1 - 8.0 * s, 2.0 * s, 6.0 * s);
    c.draw_rrect(gfx::rrect(ind, s), &gfx::fill(rgba(0x151515)));
    c.restore();
}

/// A sweep gradient starting at 12 o'clock (CSS conic `from 0deg`).
fn sweep(o: (f32, f32), colours: &[Rgba], pos: &[f32]) -> Option<vg::Shader> {
    let c: Vec<vg::Color4f> = colours.iter().map(|c| c.c4f()).collect();
    let g = vg::gradient::Gradient::new(vg::gradient::Colors::new(&c, Some(pos), vg::TileMode::Clamp, None), vg::gradient::Interpolation::default());
    let m = vg::Matrix::rotate_deg_pivot(-90.0, vg::Point::new(o.0, o.1));
    vg::gradient::shaders::sweep_gradient(vg::Point::new(o.0, o.1), (0.0, 360.0), &g, Some(&m))
}

/// The mixer key body: `#0B0B0B → #262626`, a `#101010` border, the top
/// sheen `#333 → #7A7A7A → #303030`, radius 4.
fn draw_key_body(c: &vg::Canvas, r: vg::Rect, s: f32) {
    let rr = gfx::rrect(r, 4.0 * s);
    c.draw_rrect(rr, &gfx::vgrad(r, rgba(0x0B0B0B), rgba(0x262626)));
    gfx::ring(c, &rr, gfx::hairline(s), &gfx::fill(rgba(0x101010)));
    gfx::top_sheen(c, &rr.with_inset((gfx::hairline(s), gfx::hairline(s))), gfx::hairline(s), rgba(0x333333), rgba(0x7A7A7A));
}

/// The plastic cap (30 × 44) with its cast shadow, grip ridges and white
/// centre line, as an image (padding for the shadow), drawn at any y.
fn render_cap(s: f32) -> Option<vg::Image> {
    let pad = (CAP_PAD * s).ceil();
    let (w, h) = ((CAP_W * s + 2.0 * pad).ceil() as i32, (CAP_H * s + 2.0 * pad + 6.0 * s).ceil() as i32);
    let mut surface = vg::surfaces::raster_n32_premul((w.max(1), h.max(1)))?;
    let c = surface.canvas();
    c.clear(vg::Color::TRANSPARENT);
    let r = vg::Rect::from_xywh(pad, pad, CAP_W * s, CAP_H * s);
    // The cast shadow: 6 px down, `filter: blur(5px)` (a 5 px standard deviation).
    let mut sh = gfx::fill(Rgba::black(0.55));
    sh.set_mask_filter(vg::MaskFilter::blur(vg::BlurStyle::Normal, 5.0 * s, false));
    c.draw_rrect(gfx::rrect(r.with_offset((0.0, 6.0 * s)), 5.0 * s), &sh);
    let rr = gfx::rrect(r, 4.0 * s);
    let stops = [0x6A6A6A, 0x4A4A4A, 0x2E2E2E, 0x262626, 0x3A3A3A, 0x505050].map(rgba);
    c.draw_rrect(rr, &gfx::shaded(gfx::linear((r.left, r.top), (r.left, r.bottom), &stops, Some(&[0.0, 0.18, 0.46, 0.54, 0.82, 1.0]))));
    gfx::ring(c, &rr, gfx::hairline(s), &gfx::fill(rgba(0x1E1E1E)));
    let hl = gfx::hairline(s);
    c.draw_rect(vg::Rect::from_xywh(r.left + 2.0 * s, r.top + hl, r.width() - 4.0 * s, hl), &gfx::fill(Rgba::white(0.45)));
    c.draw_rect(vg::Rect::from_xywh(r.left + 2.0 * s, r.bottom - 2.0 * hl, r.width() - 4.0 * s, hl), &gfx::fill(Rgba::white(0.12)));
    // Grip ridges: a dark line and its light edge, twice, top and bottom.
    for y0 in [8.0, CAP_H - 12.0 - 1.0 - 4.0] {
        for dy in [0.0, 4.0] {
            let y = r.top + (y0 + dy) * s;
            c.draw_rect(vg::Rect::from_xywh(r.left + 5.0 * s, y, r.width() - 10.0 * s, hl), &gfx::fill(Rgba::black(0.35)));
            c.draw_rect(vg::Rect::from_xywh(r.left + 5.0 * s, y + hl, r.width() - 10.0 * s, hl), &gfx::fill(Rgba::white(0.12)));
        }
    }
    // The white centre line with its glow.
    let line = vg::Rect::from_xywh(r.left + 3.0 * s, r.top + 21.0 * s, r.width() - 6.0 * s, 2.0 * s);
    let mut glow = gfx::fill(Rgba::white(0.35));
    glow.set_mask_filter(vg::MaskFilter::blur(vg::BlurStyle::Normal, 1.5 * s, false));
    c.draw_rect(line, &glow);
    c.draw_rect(line, &gfx::fill(INK_HI));
    Some(surface.image_snapshot())
}

/// Cached images shared by every strip (per scale).
#[derive(Default)]
struct Shared {
    s: u32,
    cap: Option<vg::Image>,
    lit: Option<vg::Image>,
    lit_main: Option<vg::Image>,
    glass: Option<vg::Image>,
    glass_main: Option<vg::Image>,
}

thread_local! {
    static SHARED: RefCell<Shared> = RefCell::new(Shared::default());
}

fn with_shared<R>(s: f32, f: impl FnOnce(&Shared) -> R) -> R {
    SHARED.with(|sh| {
        let key = (s * 100.0).round() as u32;
        let mut sh = sh.borrow_mut();
        if sh.s != key || sh.cap.is_none() {
            *sh = Shared {
                s: key,
                cap: render_cap(s),
                lit: render_lit_column(SEG_W, s),
                lit_main: render_lit_column(M_SEG_W, s),
                glass: render_glass_overlay(false, s),
                glass_main: render_glass_overlay(true, s),
            };
        }
        f(&sh)
    })
}

/// The lit part of one meter column: `n` segments from the bottom, and the
/// peak-hold segment `peak` (white) if above them.
#[allow(clippy::too_many_arguments)]
fn draw_column(c: &vg::Canvas, img: &vg::Image, x: f32, top: f32, segw: f32, n: usize, peak: usize, s: f32) {
    let pad = (4.0 * s).ceil();
    if n > 0 {
        let y0 = top + (METER_LEN - n as f32 * SEG_STEP) * s;
        c.save();
        c.clip_rect(vg::Rect::from_ltrb(x - pad, y0 - pad, x + segw * s + pad, top + METER_LEN * s + pad), None, None);
        c.draw_image(img, (x - pad, top - pad), None);
        c.restore();
    }
    if peak > n && peak > 0 {
        let y = top + (METER_LEN - peak as f32 * SEG_STEP + 1.0) * s;
        let r = vg::Rect::from_xywh(x, y, segw * s, SEG_H * s);
        let mut g = gfx::fill(Rgba::white(0.5));
        g.set_mask_filter(vg::MaskFilter::blur(vg::BlurStyle::Normal, 2.0 * s, false));
        c.draw_rect(r, &g);
        c.draw_rect(r, &gfx::fill(PEAK_SEG));
    }
}

fn readout_text(c: &vg::Canvas, text: &str, r: vg::Rect, px: f32, tracking: f32, s: f32, lock: bool) {
    let f = gfx::font(Face::SemiBold, px * s);
    let tr = tracking * px * s;
    let mut w = gfx::tracked_width(text, &f, tr) - tr;
    if lock {
        w += 4.0 * s + 8.0 * s;
    }
    let x = (r.center_x() - w * 0.5).round();
    let base = gfx::css_baseline(&f, r.top, r.height()).round();
    let end = x + gfx::draw_text_tracked(c, text, x, base, &f, tr, INK_VAL);
    if lock {
        draw_lock(c, ((end + 4.0 * s - tr).round(), (base - 8.0 * s).round()), s, INK_MID);
    }
}

// ─────────────────────────────── a strip ───────────────────────────────

/// What a strip shows that moves (its redraw key).
#[derive(Clone, Copy, Debug, PartialEq, Default)]
struct Live {
    segs: (u8, u8),
    peak: (u8, u8),
    clip: bool,
}

pub struct Strip {
    s: Signals,
    /// The lane, or `MAIN`.
    lane: usize,
    stat: RefCell<Option<(StaticKey, vg::Image)>>,
    drag: Option<Drag>,
    hover: Option<Part>,
    wheel: crate::input::WheelSteps,
}

#[derive(Clone, Copy, Debug, PartialEq)]
enum Drag {
    /// Knob: (press y, pan then).
    Pan(f32, i8),
    /// Fader: (press y, cap y then).
    Fader(f32, f32),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Part {
    Plate,
    Knob,
    PanValue,
    Mute,
    Solo,
    Clip,
    Fader,
    Peak,
    Readout,
    Other,
}

pub const MAIN: usize = usize::MAX;

impl Strip {
    pub fn new(cx: &mut Context, s: Signals, lane: usize) -> Handle<'_, Self> {
        let main = lane == MAIN;
        let shown = std::cell::Cell::new(Live::default());
        Self { s, lane, stat: RefCell::new(None), drag: None, hover: None, wheel: Default::default() }
            .build(cx, |_| {})
            .width(Pixels(if main { MAIN_W } else { STRIP_W }))
            .height(Pixels(STRIP_H))
            .bind(s.meters, move |mut h| {
                let v = Self::live(&s, lane);
                if v != shown.get() {
                    shown.set(v);
                    h.needs_redraw();
                }
            })
            .bind(s.arrangement, |mut h| h.needs_redraw())
            .bind(s.mute, |mut h| h.needs_redraw())
            .bind(s.solo, |mut h| h.needs_redraw())
            .bind(s.master_db, move |mut h| {
                if main {
                    h.needs_redraw()
                }
            })
            .bind(s.master_peak, move |mut h| {
                if main {
                    h.needs_redraw()
                }
            })
            .bind(s.chip, |mut h| h.needs_redraw())
    }

    fn main(&self) -> bool {
        self.lane == MAIN
    }

    /// (L, R) post-fader levels: a track's level through its fader and its
    /// pan law (both channels the same for a locked or centred track);
    /// Main's output before and after its gain.
    fn levels(s: &Signals, lane: usize) -> (f32, f32, f32, f32) {
        if lane == MAIN {
            let g = crate::state::db_to_gain(s.master_db.get_untracked());
            let (l, r) = s.meters.with_untracked(|m| m.main_lr);
            let (pl, pr) = s.meters.with_untracked(|m| m.main_peak_lr);
            return (l * g, r * g, pl * g, pr * g);
        }
        let Some((db, pan, muted)) = s.arrangement.with_untracked(|a| a.lanes.get(lane).map(|l| (l.vol_db, l.pan, s.mute.with_untracked(|m| m.contains(l.id))))) else { return (0.0, 0.0, 0.0, 0.0) };
        if muted {
            return (0.0, 0.0, 0.0, 0.0);
        }
        let (lv, pk) = s.meters.with_untracked(|m| (m.level_of(lane), m.peak.get(lane).copied().unwrap_or(0.0)));
        let g = crate::state::db_to_gain(db);
        let (gl, gr) = engine::studio::pan_gains(pan);
        (lv * g * gl as f32, lv * g * gr as f32, pk * g * gl as f32, pk * g * gr as f32)
    }

    fn live(s: &Signals, lane: usize) -> Live {
        let (l, r, pl, pr) = Self::levels(s, lane);
        let clip = if lane == MAIN { s.master_peak.get_untracked() > 1.0 } else { super::chrome::VolumeFader::clipped(s, lane) };
        Live { segs: (lit_segments(l) as u8, lit_segments(r) as u8), peak: (lit_segments(pl) as u8, lit_segments(pr) as u8), clip }
    }

    fn part_at(&self, p: (f32, f32)) -> Part {
        let w = if self.main() { MAIN_W } else { STRIP_W };
        let cap_x = if self.main() { M_CAP_X } else { CAP_X };
        let db = self.db();
        let cy = cap_y(db);
        if inside((8.0, 8.0, w - 16.0, 40.0), p) {
            return Part::Plate;
        }
        if inside((12.0, READOUT.0, w - 2.0 * INSET - 24.0, READOUT.1), p) {
            return Part::Readout;
        }
        if self.main() {
            if inside(((w - 2.0 * INSET - 72.0) * 0.5, 64.0, 72.0, 42.0), p) {
                return Part::Peak;
            }
        } else {
            if (p.0 - KNOB_C.0).hypot(p.1 - KNOB_C.1) <= KNOB_R + 2.0 {
                return Part::Knob;
            }
            if inside((40.0, PAN_TEXT_Y - 2.0, 72.0, 16.0), p) {
                return Part::PanValue;
            }
            if inside((KEYS_X[0], KEYS_Y, KEY, KEY), p) {
                return Part::Mute;
            }
            if inside((KEYS_X[1], KEYS_Y, KEY, KEY), p) {
                return Part::Solo;
            }
            if inside((CLIP_RECT.0 - 4.0, CLIP_RECT.1 - 4.0, CLIP_RECT.2 + 8.0, CLIP_RECT.3 + 8.0), p) {
                return Part::Clip;
            }
            if inside((12.0, PEAK_Y, w - 24.0, 20.0), p) {
                return Part::Peak;
            }
        }
        // The fader: the cap, or anywhere on the slot's column.
        if inside((cap_x - 4.0, cy - CAP_H * 0.5, CAP_W + 8.0, CAP_H), p) || inside((cap_x, METER_TOP - 8.0, CAP_W, METER_LEN + 16.0), p) {
            return Part::Fader;
        }
        Part::Other
    }

    fn db(&self) -> f32 {
        if self.main() {
            return self.s.master_db.get_untracked();
        }
        self.s.arrangement.with_untracked(|a| a.lanes.get(self.lane).map_or(0.0, |l| l.vol_db))
    }

    fn lock(&self) -> Option<usize> {
        if self.main() {
            return None;
        }
        self.s.arrangement.with_untracked(|a| a.lanes.get(self.lane).and_then(|l| l.lock))
    }

    fn set_db(&self, cx: &mut EventContext, db: f32) {
        cx.emit(if self.main() { AppEvent::SetMasterDb(db) } else { AppEvent::SetLaneVolume(self.lane, db) });
    }

    fn help(&self, part: Part) -> String {
        use super::chrome::{PanSlider, VolumeFader};
        let lock = self.lock();
        match part {
            Part::Plate if self.main() => "Main: everything you hear, after the master gain · the output device is in Settings > Audio".into(),
            Part::Plate => "Track: click to select · double-click the name to rename".into(),
            Part::Knob | Part::PanValue => match lock {
                Some(c) => format!("Pan: centred while locked to the {} (the console's output is mono)", crate::tracks::channel_name(c)),
                None => PanSlider::HELP.into(),
            },
            Part::Mute => "Mute this track".into(),
            Part::Solo => if crate::shortcuts::MACOS { "Solo this track only · Cmd-click to solo several" } else { "Solo this track only · Ctrl-click to solo several" }.into(),
            Part::Clip => VolumeFader::CLIP_HELP.into(),
            Part::Peak if self.main() => "PEAK: the Main track's sample peak since the last reset (held) · click to reset".into(),
            Part::Peak => "PEAK: this track's peak since the last reset (held) · click to reset".into(),
            Part::Fader | Part::Readout if self.main() => VolumeFader::MAIN_HELP.into(),
            Part::Fader | Part::Readout => match lock {
                Some(c) => crate::tracks::locked_help(c),
                None => VolumeFader::HELP.into(),
            },
            Part::Other => String::new(),
        }
    }

    fn static_key(&self, w: f32, h: f32, s: f32) -> StaticKey {
        if self.main() {
            let device = self.s.out_device.get_untracked();
            return StaticKey { main: true, colour: 0, number: String::new(), name: "Main".into(), badge: format!("OUT \u{B7} {}", device.to_uppercase()), locked: false, w: w.round() as i32, h: h.round() as i32, s: (s * 100.0).round() as u32 };
        }
        self.s.arrangement.with_untracked(|a| {
            let l = &a.lanes[self.lane.min(a.lanes.len().saturating_sub(1))];
            let badge = match l.lock {
                Some(c) => crate::tracks::channel_badge(c).to_string(),
                None => crate::tracks::kind_badge(l.voice).to_uppercase(),
            };
            StaticKey { main: false, colour: crate::state::rgb_hex(l.color), number: format!("{}", self.lane + 1), name: l.name.clone(), badge, locked: l.lock.is_some(), w: w.round() as i32, h: h.round() as i32, s: (s * 100.0).round() as u32 }
        })
    }
}

impl View for Strip {
    fn element(&self) -> Option<&'static str> {
        Some("mixer-strip")
    }

    fn event(&mut self, cx: &mut EventContext, event: &mut Event) {
        let lock = self.lock();
        event.map(|we, meta| match we {
            WindowEvent::MouseMove(..) => {
                let p = local(cx);
                let p = (p.0 - INSET, p.1 - INSET);
                match self.drag {
                    Some(Drag::Pan(y0, pan0)) => {
                        // Up turns right: 100 units over 150 px (Shift: finer).
                        let fine = if cx.modifiers().shift() { 0.25 } else { 1.0 };
                        let v = (pan0 as f32 + (y0 - p.1) * 100.0 / 150.0 * fine).round().clamp(-50.0, 50.0);
                        cx.emit(AppEvent::SetLanePan(self.lane, v as i8));
                    }
                    Some(Drag::Fader(y0, c0)) => {
                        let y = (c0 + p.1 - y0 - METER_TOP).clamp(0.0, METER_LEN);
                        self.set_db(cx, crate::state::pos_to_db(1.0 - y / METER_LEN));
                    }
                    None => {
                        let part = self.part_at(p);
                        if self.hover != Some(part) {
                            self.hover = Some(part);
                            cx.emit(AppEvent::HoverHelp(self.help(part)));
                        }
                    }
                }
            }
            WindowEvent::MouseLeave => {
                self.hover = None;
                cx.emit(AppEvent::HoverHelp(String::new()));
            }
            WindowEvent::MouseDown(MouseButton::Left) => {
                let p = local(cx);
                let p = (p.0 - INSET, p.1 - INSET);
                if !self.main() {
                    cx.emit(AppEvent::SelectTrack(self.lane, false));
                }
                match self.part_at(p) {
                    Part::Knob if lock.is_none() => {
                        let pan = self.s.arrangement.with_untracked(|a| a.lanes.get(self.lane).map_or(0, |l| l.pan));
                        self.drag = Some(Drag::Pan(p.1, pan));
                        cx.emit(AppEvent::GestureBegin);
                        cx.capture();
                    }
                    Part::Fader if lock.is_none() => {
                        let cy = cap_y(self.db());
                        // On the cap: grab it where it is; elsewhere it jumps.
                        let grab = if (p.1 - cy).abs() <= CAP_H * 0.5 { cy } else { p.1 };
                        self.drag = Some(Drag::Fader(p.1, grab));
                        cx.emit(AppEvent::GestureBegin);
                        cx.capture();
                        if grab == p.1 {
                            let y = (p.1 - METER_TOP).clamp(0.0, METER_LEN);
                            self.set_db(cx, crate::state::pos_to_db(1.0 - y / METER_LEN));
                        }
                    }
                    Part::Mute => cx.emit(AppEvent::ToggleMute(self.lane)),
                    Part::Solo => cx.emit(AppEvent::ToggleSolo(self.lane)),
                    Part::Clip => cx.emit(AppEvent::ResetTrackPeak(self.lane)),
                    Part::Peak if self.main() => cx.emit(AppEvent::ResetPeak),
                    Part::Peak => cx.emit(AppEvent::ResetTrackPeak(self.lane)),
                    Part::Knob | Part::Fader => cx.emit(AppEvent::HoverHelp(self.help(Part::Fader))),
                    _ => {}
                }
                meta.consume();
            }
            WindowEvent::MouseUp(MouseButton::Left) if self.drag.is_some() => {
                self.drag = None;
                cx.release();
                cx.emit(AppEvent::GestureEnd);
                meta.consume();
            }
            WindowEvent::MouseDoubleClick(MouseButton::Left) => {
                if self.main() {
                    meta.consume();
                    return;
                }
                let l = local(cx);
                let target = match self.part_at((l.0 - INSET, l.1 - INSET)) {
                    Part::Plate => Some(crate::edit::RenameTarget::Track(self.lane)),
                    Part::PanValue => Some(crate::edit::RenameTarget::Pan(self.lane)),
                    Part::Readout => Some(crate::edit::RenameTarget::Volume(self.lane)),
                    _ => None,
                };
                if let Some(t) = target {
                    cx.emit(AppEvent::OpenRename(t));
                }
                meta.consume();
            }
            WindowEvent::MouseScroll(_, y) if !cx.modifiers().alt() => {
                let l = local(cx);
                let part = self.part_at((l.0 - INSET, l.1 - INSET));
                if matches!(part, Part::Knob | Part::PanValue | Part::Fader | Part::Readout) {
                    let steps = self.wheel.steps(*y);
                    if steps != 0 && lock.is_none() {
                        if matches!(part, Part::Knob | Part::PanValue) {
                            let pan = self.s.arrangement.with_untracked(|a| a.lanes.get(self.lane).map_or(0, |l| l.pan)) as i32;
                            cx.emit(AppEvent::SetLanePan(self.lane, (pan + steps).clamp(-50, 50) as i8));
                        } else {
                            let db = self.db();
                            let base = if db == f32::NEG_INFINITY { crate::state::VOL_DB_FLOOR } else { db };
                            self.set_db(cx, (base + steps as f32).min(crate::state::VOL_DB_MAX));
                        }
                        cx.emit(AppEvent::Commit);
                    }
                    meta.consume();
                }
            }
            _ => {}
        });
    }

    fn draw(&self, cx: &mut DrawContext, canvas: &Canvas) {
        let _perf = crate::perf::scope(crate::perf::Span::Rows);
        let b = cx.bounds();
        if !self.main() {
            register(self.lane, b);
        }
        // Off-screen strips (scrolled away) draw nothing (Main is never off).
        if self.lane != MAIN && !AREA.with(|a| a.get()).is_none_or(|(l, r)| b.x + b.w >= l && b.x <= r) {
            return;
        }
        let s = cx.scale_factor();
        let r = gfx::snap(b.into());
        let main = self.main();
        if !main && self.s.arrangement.with_untracked(|a| self.lane >= a.lanes.len()) {
            return;
        }
        let key = self.static_key(r.width(), r.height(), s);
        {
            let mut st = self.stat.borrow_mut();
            if st.as_ref().is_none_or(|(k, _)| *k != key) {
                *st = render_static(&key, s).map(|img| (key.clone(), img));
            }
            if let Some((_, img)) = st.as_ref() {
                canvas.draw_image(img, (r.left, r.top), None);
            }
        }
        let at = |x: f32, y: f32| (r.left + (x + INSET) * s, r.top + (y + INSET) * s);
        let lock = self.lock();
        let live = Self::live(&self.s, self.lane);
        let (cols, segw, glass, cap_x) = if main { (M_SEG_COLS, M_SEG_W, M_GLASS, M_CAP_X) } else { (SEG_COLS, SEG_W, GLASS, CAP_X) };
        with_shared(s, |sh| {
            // Meters: the lit part of each column and the peak hold.
            if let Some(img) = if main { sh.lit_main.as_ref() } else { sh.lit.as_ref() } {
                for (k, x) in cols.iter().enumerate() {
                    let (n, pk) = if k == 0 { (live.segs.0, live.peak.0) } else { (live.segs.1, live.peak.1) };
                    let (px, py) = at(*x, METER_TOP);
                    draw_column(canvas, img, px.round(), py.round(), segw, n as usize, pk as usize, s);
                }
            }
            if let Some(g) = if main { sh.glass_main.as_ref() } else { sh.glass.as_ref() } {
                let (gx, gy) = at(glass.0, 162.0);
                canvas.draw_image(g, (gx.round(), gy.round()), None);
            }
            // The cap (locked: dimmed and parked at 0 dB).
            if let Some(cap) = sh.cap.as_ref() {
                let db = if lock.is_some() { 0.0 } else { self.db() };
                let cy = cap_y(db);
                let pad = (CAP_PAD * s).ceil();
                let (x, y) = at(cap_x, cy - CAP_H * 0.5);
                let mut p = vg::Paint::default();
                if lock.is_some() {
                    p.set_alpha_f(0.45);
                }
                canvas.draw_image(cap, ((x - pad).round(), (y - pad).round()), Some(&p));
            }
        });
        let f10 = gfx::font(Face::SemiBold, 10.0 * s);
        if main {
            let pr = vg::Rect::from_xywh(at((MAIN_W - 2.0 * INSET - 72.0) * 0.5, 80.0).0, at(0.0, 80.0).1, 72.0 * s, 26.0 * s);
            readout_text(canvas, &super::main_track::peak_text(self.s.master_peak.get_untracked()), pr, 11.0, 0.0, s, false);
            let ro = vg::Rect::from_xywh(at(12.0, 0.0).0, at(0.0, READOUT.0).1, (MAIN_W - 2.0 * INSET - 24.0) * s, READOUT.1 * s);
            readout_text(canvas, &crate::state::format_db(self.db()), ro, 11.0, 0.02, s, false);
            return;
        }
        // The track's moving parts.
        let Some((pan, id, cap, voice)) = self.s.arrangement.with_untracked(|a| a.lanes.get(self.lane).map(|l| (l.pan, l.id, l.voices, l.voice))) else { return };
        // Knob: the indicator and the value ring (white, from centre).
        let o = at(KNOB_C.0, KNOB_C.1);
        let ang = if lock.is_some() { 0.0 } else { pan as f32 / crate::state::PAN_MAX as f32 * KNOB_SWING };
        if lock.is_none() && pan != 0 {
            let rr = KNOB_R * s - RING_W * 0.5 * s;
            let oval = vg::Rect::from_xywh(o.0 - rr, o.1 - rr, 2.0 * rr, 2.0 * rr);
            let mut p = gfx::stroke(INK_HI, RING_W * s);
            p.set_stroke_cap(vg::PaintCap::Butt);
            canvas.draw_arc(oval, -90.0, ang, false, &p);
        }
        if lock.is_none() {
            draw_indicator(canvas, o, ang, s);
        }
        // The pan value under the knob ("C" and the lock glyph when locked).
        let pt = vg::Rect::from_xywh(r.left, at(0.0, PAN_TEXT_Y).1, r.width(), 12.0 * s);
        let pan_text = if lock.is_some() { "C".to_string() } else { crate::state::format_pan(pan) };
        {
            let w = gfx::text_width(&f10, &pan_text) + if lock.is_some() { 12.0 * s } else { 0.0 };
            let x = (pt.center_x() - w * 0.5).round();
            let base = gfx::css_baseline(&f10, pt.top, pt.height()).round();
            let end = x + gfx::text_width(&f10, &pan_text);
            gfx::draw_text(canvas, &pan_text, x, base, &f10, INK_MID);
            if lock.is_some() {
                draw_lock(canvas, ((end + 4.0 * s).round(), (base - 8.0 * s).round()), s, INK_MID);
            }
        }
        // M / S glyphs: lime when on (a toggle on-state), else grey.
        let muted = self.s.mute.with_untracked(|m| m.contains(id));
        let soloed = self.s.solo.with_untracked(|m| m.contains(id));
        let kf = gfx::font(Face::Bold, 12.0 * s);
        for (k, (g, on)) in [("M", muted), ("S", soloed)].into_iter().enumerate() {
            let kr = vg::Rect::from_xywh(at(KEYS_X[k], 0.0).0, at(0.0, KEYS_Y).1, KEY * s, KEY * s);
            let w = gfx::text_width(&kf, g);
            let x = (kr.center_x() - w * 0.5).round();
            let base = gfx::baseline_for_center(&kf, kr.center_y()).round();
            if on {
                canvas.draw_str(g, (x, base), &kf, &super::topbar::glow(ACCENT.with_alpha(0.85), 6.0 * s));
                gfx::draw_text(canvas, g, x, base, &kf, ACCENT);
            } else {
                gfx::draw_text(canvas, g, x, base, &kf, rgba(0x8C8C8C));
            }
        }
        // The clip mark: white and glowing while held.
        if live.clip {
            let cr = vg::Rect::from_xywh(at(CLIP_RECT.0, 0.0).0, at(0.0, CLIP_RECT.1).1, CLIP_RECT.2 * s, CLIP_RECT.3 * s);
            let mut g = gfx::fill(Rgba::white(0.6));
            g.set_mask_filter(vg::MaskFilter::blur(vg::BlurStyle::Normal, 2.0 * s, false));
            canvas.draw_rect(cr, &g);
            canvas.draw_rrect(gfx::rrect(cr, s), &gfx::fill(Rgba::white(1.0)));
        }
        // PEAK's readout, VOICES n / ∞ (studio) or the channel (locked).
        let held = self.s.meters.with_untracked(|m| m.held_of(id));
        let pr = vg::Rect::from_xywh(at(STRIP_W - 2.0 * INSET - 12.0 - 64.0, 0.0).0, at(0.0, PEAK_Y).1, 64.0 * s, 20.0 * s);
        readout_text(canvas, &super::main_track::peak_text(held), pr, 10.0, 0.0, s, false);
        let line = match lock {
            Some(c) => crate::tracks::channel_name(c).to_uppercase().replacen(' ', " \u{B7} ", 1),
            None => {
                let n = self.s.meters.with_untracked(|m| m.voices_of(id));
                let cap = if cap == 0 { "\u{221E}".to_string() } else { cap.to_string() };
                let _ = voice;
                format!("VOICES {n} / {cap}")
            }
        };
        let vf = gfx::font(Face::SemiBold, 10.0 * s);
        let vtr = 0.08 * 10.0 * s;
        let vw = gfx::tracked_width(&line, &vf, vtr) - vtr;
        gfx::draw_text_tracked(canvas, &line, (r.center_x() - vw * 0.5).round(), gfx::css_baseline(&vf, at(0.0, VOICES_Y).1, 12.0 * s).round(), &vf, vtr, INK_LO);
        // The dB readout (locked: 0.0 dB and the lock glyph).
        let ro = vg::Rect::from_xywh(at(12.0, 0.0).0, at(0.0, READOUT.0).1, (STRIP_W - 2.0 * INSET - 24.0) * s, READOUT.1 * s);
        let db = if lock.is_some() { 0.0 } else { self.db() };
        readout_text(canvas, &crate::state::format_db(db), ro, 11.0, 0.02, s, lock.is_some());
    }
}

thread_local! {
    /// The strips viewport (physical x span): strips outside it skip drawing.
    static AREA: std::cell::Cell<Option<(f32, f32)>> = const { std::cell::Cell::new(None) };
}

// ─────────────────────────────── the budget bar ───────────────────────────────

/// The channel-budget bar across the top (VISION.md form: "FAMICOM + VRC6 ·
/// Pulse 2/2 · Triangle 1/1 …"), red where over, with a status on the right.
pub struct BudgetBar {
    s: Signals,
}

impl View for BudgetBar {
    fn element(&self) -> Option<&'static str> {
        Some("budget-bar")
    }

    fn event(&mut self, cx: &mut EventContext, event: &mut Event) {
        event.map(|we, _| {
            if let WindowEvent::MouseMove(..) = we {
                cx.emit(AppEvent::HoverHelp("The channel budget: how many of the hardware profile's channels the locked tracks use · the profile is in File > Project Settings".into()));
            }
        });
    }

    fn draw(&self, cx: &mut DrawContext, canvas: &Canvas) {
        let s = cx.scale_factor();
        let r = gfx::snap(cx.bounds().into());
        let rr = gfx::rrect(r, 6.0 * s);
        canvas.draw_rect(vg::Rect::from_xywh(r.left + 6.0 * s, r.bottom, r.width() - 12.0 * s, gfx::hairline(s)), &gfx::fill(Rgba::white(0.05)));
        canvas.draw_rrect(rr, &gfx::fill(RECESS));
        gfx::inner_shadow(canvas, &rr, 0.0, s, s, Rgba::black(0.7));
        let chip = self.s.chip.get_untracked();
        let f = gfx::font(Face::SemiBold, 11.0 * s);
        let base = gfx::baseline_for_center(&f, r.center_y()).round();
        let mut x = r.left + 16.0 * s;
        let title = chip.label().to_uppercase();
        let title = if chip == crate::state::SoundChip::StudioOnly { "NO HARDWARE PROFILE".to_string() } else { title.replace("NES (2A03)", "NES \u{B7} 2A03") };
        x += gfx::draw_text_tracked(canvas, &title, x.round(), base, &f, 0.1 * 11.0 * s, INK_LO) + 16.0 * s - 0.1 * 11.0 * s;
        let budget = self.s.arrangement.with_untracked(|a| crate::tracks::budget(a, chip));
        let parts = budget.parts(false);
        let over = budget.over();
        for (i, (text, over_part)) in parts.iter().enumerate() {
            let put = |t: &str, x: &mut f32, ink: Rgba| {
                gfx::draw_text(canvas, t, x.round(), base, &f, ink);
                *x += gfx::text_width(&f, t);
            };
            if i > 0 {
                x += 10.0 * s;
                put("\u{B7}", &mut x, rgba(0x4A4A4A));
                x += 10.0 * s;
            }
            // "Pulse 2/2": the name, then the count brighter.
            let (name, count) = text.rsplit_once(' ').unwrap_or((text.as_str(), ""));
            put(name, &mut x, if *over_part { OVER_INK } else { INK_MID });
            x += 4.0 * s;
            put(count, &mut x, if *over_part { OVER_INK } else { INK_HI });
        }
        let status = if chip == crate::state::SoundChip::StudioOnly {
            "Studio tracks only: no channel budget".to_string()
        } else if over {
            "Over budget: some locked tracks have no channel".to_string()
        } else {
            "All locked tracks within budget".to_string()
        };
        let sf = gfx::font(Face::Medium, 11.0 * s);
        let sw = gfx::text_width(&sf, &status);
        gfx::draw_text(canvas, &status, (r.right - 16.0 * s - sw).round(), base, &sf, if over { OVER_INK } else { INK_LO });
    }
}

// ─────────────────────────────── the view ───────────────────────────────

/// The strips' row: scrolls sideways (wheel, Shift+wheel or the bar under
/// the strips) when the strips don't fit.
pub struct StripsArea {
    s: Signals,
    bar_drag: Option<(f32, f32)>,
}

impl StripsArea {
    fn content_w(&self) -> f32 {
        let n = self.s.arrangement.with_untracked(|a| a.lanes.len()) as f32;
        n * (STRIP_W + STRIP_GAP) - STRIP_GAP
    }

    fn max_scroll(&self, view_w: f32) -> f32 {
        (self.content_w() - view_w).max(0.0)
    }

    fn set_scroll(&self, cx: &mut EventContext, v: f32) {
        let w = cx.bounds().w / cx.scale_factor();
        let v = v.clamp(0.0, self.max_scroll(w));
        if v != self.s.mixer_scroll.get_untracked() {
            self.s.mixer_scroll.set(v);
        }
    }
}

impl View for StripsArea {
    fn element(&self) -> Option<&'static str> {
        Some("mixer-strips")
    }

    fn event(&mut self, cx: &mut EventContext, event: &mut Event) {
        event.map(|we, meta| match we {
            WindowEvent::MouseScroll(x, y) => {
                let d = if x.abs() > y.abs() { -*x } else { -*y };
                let cur = self.s.mixer_scroll.get_untracked();
                self.set_scroll(cx, cur + d * (STRIP_W + STRIP_GAP) * 0.5);
                meta.consume();
            }
            WindowEvent::MouseDown(MouseButton::Left) => {
                let s = cx.scale_factor();
                let b = cx.bounds();
                let (lx, ly) = ((cx.mouse().cursor_x - b.x) / s, (cx.mouse().cursor_y - b.y) / s);
                if ly >= b.h / s - SCROLLBAR_H - SCROLLBAR_GAP * 2.0 {
                    let w = b.w / s;
                    let total = self.content_w().max(w);
                    let thumb = (w / total * w).max(24.0);
                    let cur = self.s.mixer_scroll.get_untracked();
                    let tx = cur / total * w;
                    let grab = if lx >= tx && lx < tx + thumb { lx } else {
                        // A click on the track jumps there.
                        self.set_scroll(cx, (lx - thumb * 0.5) / w * total);
                        lx
                    };
                    self.bar_drag = Some((grab, self.s.mixer_scroll.get_untracked()));
                    cx.capture();
                    meta.consume();
                }
            }
            WindowEvent::MouseMove(..) if self.bar_drag.is_some() => {
                let s = cx.scale_factor();
                let b = cx.bounds();
                let lx = (cx.mouse().cursor_x - b.x) / s;
                let (x0, v0) = self.bar_drag.unwrap_or((0.0, 0.0));
                let w = b.w / s;
                let total = self.content_w().max(w);
                self.set_scroll(cx, v0 + (lx - x0) / w * total);
            }
            WindowEvent::MouseUp(MouseButton::Left) if self.bar_drag.is_some() => {
                self.bar_drag = None;
                cx.release();
            }
            _ => {}
        });
    }

    fn draw(&self, cx: &mut DrawContext, canvas: &Canvas) {
        let s = cx.scale_factor();
        let b = cx.bounds();
        AREA.with(|a| a.set(Some((b.x, b.x + b.w))));
        // The scrollbar, only when the strips don't fit.
        let w = b.w / s;
        let total = self.content_w();
        if total > w {
            let r = gfx::snap(b.into());
            let y = r.bottom - (SCROLLBAR_H + SCROLLBAR_GAP) * s;
            let track = vg::Rect::from_xywh(r.left, y, r.width(), SCROLLBAR_H * s);
            canvas.draw_rrect(gfx::rrect(track, 2.0 * s), &gfx::fill(Rgba::white(0.04)));
            let tw = (w / total * track.width()).max(24.0 * s);
            let tx = track.left + self.s.mixer_scroll.get_untracked() / (total - w).max(1.0) * (track.width() - tw);
            canvas.draw_rrect(gfx::rrect(vg::Rect::from_xywh(tx, y, tw, SCROLLBAR_H * s), 2.0 * s), &gfx::fill(Rgba::white(0.22)));
        }
    }
}

/// The mixer view: the budget bar, the strips (scrolling) and Main.
pub fn build(cx: &mut Context, s: Signals) {
    ZStack::new(cx, move |cx| {
        BudgetBar { s }
            .build(cx, |_| {})
            .position_type(PositionType::Absolute)
            .left(Pixels(MARGIN))
            .right(Pixels(MARGIN))
            .top(Pixels(BUDGET_TOP))
            .height(Pixels(BUDGET_H))
            .bind(s.arrangement, |mut h| h.needs_redraw())
            .bind(s.chip, |mut h| h.needs_redraw());
        StripsArea { s, bar_drag: None }
            .build(cx, move |cx| {
                HStack::new(cx, move |cx| {
                    let count = Memo::new(move |_| s.arrangement.with(|a| a.lanes.len()));
                    Binding::new(cx, count, move |cx| {
                        for i in 0..count.get() {
                            Strip::new(cx, s, i);
                        }
                    });
                })
                .gap(Pixels(STRIP_GAP))
                .width(Auto)
                .height(Pixels(STRIP_H))
                .position_type(PositionType::Absolute)
                .top(Pixels(STRIPS_TOP))
                .left(s.mixer_scroll.map(|v| Pixels(-*v)));
            })
            // The viewport starts at the margin, so a strip scrolled away
            // never shows in it (the board's margins are empty).
            .position_type(PositionType::Absolute)
            .left(Pixels(MARGIN))
            .right(Pixels(MARGIN + MAIN_W + MAIN_GAP))
            .top(Pixels(0.0))
            .bottom(Pixels(0.0))
            .overflow(Overflow::Hidden)
            .bind(s.mixer_scroll, |mut h| h.needs_redraw())
            .bind(s.arrangement, |mut h| h.needs_redraw());
        Strip::new(cx, s, MAIN).position_type(PositionType::Absolute).right(Pixels(MARGIN)).top(Pixels(STRIPS_TOP));
    })
    .class("mixer")
    .width(Stretch(1.0))
    .height(Stretch(1.0));
}

/// The mixer's background (the board's radial light), drawn by the view
/// behind everything.
pub struct MixerBg;

impl View for MixerBg {
    fn draw(&self, cx: &mut DrawContext, canvas: &Canvas) {
        let r: vg::Rect = cx.bounds().into();
        canvas.draw_rect(r, &gfx::fill(rgba(0x0A0A0A)));
        canvas.draw_rect(r, &gfx::shaded(gfx::radial((r.center_x(), r.top), r.width() * 0.8, &[rgba(0x1A1A1A), rgba(0x0A0A0A)], None)));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The board's meter: 65 segments, zones warm < 41, amber < 52, orange
    /// < 58 (0 dB), red above; 0 dB lights 58 segments (89 %).
    #[test]
    fn meter_zones_and_taper_match_the_board() {
        assert_eq!((zone(0), zone(40), zone(41), zone(51), zone(52), zone(57), zone(58), zone(64)), (0, 0, 1, 1, 2, 2, 3, 3));
        assert_eq!(lit_segments(1.0), 57, "0 dB: the top of the orange zone");
        assert_eq!(lit_segments(0.0), 0);
        assert_eq!(lit_segments(2.0), 65, "+6 dB fills the column");
        // The scale's marks at the board's y (from the meter's top).
        for (db, y) in [(6.0, 0.0), (0.0, 29.0), (-6.0, 58.0), (-12.0, 87.0), (-24.0, 144.0), (-36.0, 202.0)] {
            assert!((crate::state::fader_y(db, METER_LEN) - y).abs() <= 1.0, "{db}: {}", crate::state::fader_y(db, METER_LEN));
        }
        // The board's Lead Stack cap at -6 dB: top 36 (centre 58) from the meter top.
        assert_eq!(cap_y(-6.0) - METER_TOP - CAP_H * 0.5, 36.0);
    }
}
