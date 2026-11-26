//! The Main track (BATCH4A_PROMPT.md §7): pinned at the bottom of the track
//! list, above the status bar; it never scrolls, the tracks scroll above it.
//! The reference puts the Main track at "the bottom end of the Arrangement
//! View" (Live 12 manual, ch. 18 Mixing, §18.4 Return Tracks and the Main
//! track); that it stays put while the tracks scroll is ours (the manual
//! doesn't say).
//!
//! The header: a black strip (the `main_strip` recipe; black is reserved for
//! Main), no number and no M/S, the name "Main", an OUT badge with the output
//! device, PEAK (the sample peak since the last reset, held; a click resets
//! it) and the master fader (the tracks' dB meter-fader, −inf … +6 dB). It
//! follows the header layouts (side, strip order, width): every position
//! comes from `HeaderLayout`. The lane: darker (`#0D0D0D`), the grid and
//! the playhead only (Batch 5 puts master automation there).

use std::cell::RefCell;

use vizia::prelude::*;
use vizia::vg;

use super::chrome::{VolumeFader, MAIN};
use super::gfx::{self, Face};
use crate::state::TICKS_PER_BAR;
use crate::theme::palette;
use crate::theme::tokens::*;
use crate::{AppEvent, Signals};

/// The Main row: 72 px plus its 1 px divider on top (the lanes are 73 px
/// shorter).
pub const MAIN_ROW_H: f32 = 72.0;
pub const MAIN_DIVIDER: Rgba = Rgba::hex(0x2E2E2E);
pub const MAIN_LANE: Rgba = Rgba::hex(0x0D0D0D);
/// The Main row's total height (divider + row).
pub const MAIN_TOTAL_H: f32 = MAIN_ROW_H + 1.0;

const NAME_PX: f32 = 14.0;
const NAME_TOP: f32 = 14.5;
const BADGE_Y: f32 = 40.0;
const BADGE_H: f32 = 18.0;
const BADGE_PAD: f32 = 4.0;
const BADGE_PX: f32 = 11.0;
const BADGE_BASELINE: f32 = 53.0;
const BADGE_INK: Rgba = Rgba::hex(0x9A9A9A);
const DEVICE_INK: Rgba = Rgba::hex(0x808080);
const LABEL_PX: f32 = 10.0;
const LABEL_INK: Rgba = Rgba::hex(0x808080);
const VALUE_PX: f32 = 12.0;
const VALUE_INK: Rgba = Rgba::hex(0xE6E6E6);
/// PEAK in column A: the label's and the value's tops.
const PEAK_LABEL_Y: f32 = 16.0;
const PEAK_VALUE_Y: f32 = 32.0;
/// The soft shadow the Main row casts on the tracks above it.
const SHADOW_H: f32 = 8.0;

/// The peak as text: dBFS with one decimal, "−inf" at silence.
pub fn peak_text(peak: f32) -> String {
    if peak <= 0.0 {
        "\u{2212}inf".into()
    } else {
        let db = 20.0 * peak.log10();
        // Over full scale it says so: "+3.2".
        let t = if db > 0.0 { format!("+{db:.1}") } else { format!("{db:.1}") };
        t.replace('-', "\u{2212}")
    }
}

pub struct MainHeader {
    s: Signals,
    text: RefCell<Option<(String, vg::Image)>>,
}

impl MainHeader {
    pub fn new(cx: &mut Context, s: Signals) -> Handle<'_, Self> {
        Self { s, text: RefCell::new(None) }
            .build(cx, move |cx| {
                VolumeFader::new(cx, s, MAIN)
                    .position_type(PositionType::Absolute)
                    .left(s.header.map(|l| Pixels(l.col_b())))
                    .top(Pixels(1.0));
            })
            .class("main-header")
            .height(Pixels(MAIN_TOTAL_H))
            .bind(s.master_peak, |mut h| h.needs_redraw())
            .bind(s.master_db, |mut h| h.needs_redraw())
            .bind(s.out_device, |mut h| h.needs_redraw())
            .bind(s.header, |mut h| h.needs_redraw())
    }

    /// The PEAK value's box (logical, row-relative): a click resets it.
    fn peak_rect(&self) -> (f32, f32, f32, f32) {
        let l = self.s.header.get_untracked();
        (l.col_a(), 1.0 + PEAK_LABEL_Y - 2.0, 56.0, PEAK_VALUE_Y - PEAK_LABEL_Y + 16.0)
    }
}

impl View for MainHeader {
    fn element(&self) -> Option<&'static str> {
        Some("main-header")
    }

    fn event(&mut self, cx: &mut EventContext, event: &mut Event) {
        event.map(|we, meta| match we {
            WindowEvent::MouseMove(..) => {
                let (x, y) = local(cx);
                let r = self.peak_rect();
                let on = x >= r.0 && x < r.0 + r.2 && y >= r.1 && y < r.1 + r.3;
                cx.emit(AppEvent::HoverHelp(
                    if on { "PEAK: the Main track's sample peak since the last reset (held) · click to reset" } else { "Main: everything you hear, after the master gain · the output device is in Settings > Audio" }.into(),
                ));
            }
            WindowEvent::MouseDown(MouseButton::Left) => {
                let (x, y) = local(cx);
                let r = self.peak_rect();
                if x >= r.0 && x < r.0 + r.2 && y >= r.1 && y < r.1 + r.3 {
                    cx.emit(AppEvent::ResetPeak);
                }
                meta.consume();
            }
            WindowEvent::MouseLeave => cx.emit(AppEvent::HoverHelp(String::new())),
            _ => {}
        });
    }

    fn draw(&self, cx: &mut DrawContext, canvas: &Canvas) {
        let b = cx.bounds();
        let s = cx.scale_factor();
        let r = gfx::snap(b.into());
        let hl = gfx::hairline(s);
        let lay = self.s.header.get_untracked();
        // The divider on top, then the row.
        canvas.draw_rect(vg::Rect::from_xywh(r.left, r.top, r.width(), hl), &gfx::fill(MAIN_DIVIDER));
        let body = vg::Rect::from_ltrb(r.left, r.top + hl, r.right, r.bottom);
        canvas.draw_rect(body, &gfx::fill(BG_0));
        // The black strip (`main_strip`): black → #1C1C1C → #363636 across,
        // a #4A4A4A highlight on its outer edge, a 6% inner glow.
        let sx = (r.left + lay.strip_x() * s).round();
        let strip = vg::Rect::from_ltrb(sx, body.top, (sx + 16.0 * s).round(), body.bottom);
        let m = palette::main_strip();
        let (a, z) = if lay.spill_right() { ((strip.left, strip.top), (strip.right, strip.top)) } else { ((strip.right, strip.top), (strip.left, strip.top)) };
        canvas.draw_rect(strip, &gfx::shaded(gfx::linear(a, z, &m.fill, Some(&[0.0, 0.5, 1.0]))));
        let edge_x = if lay.spill_right() { strip.left } else { strip.right - hl };
        canvas.draw_rect(vg::Rect::from_xywh(edge_x, strip.top, hl, strip.height()), &gfx::fill(m.edge));
        gfx::ring(canvas, &gfx::rrect(strip.with_inset((hl, hl)), 0.0), hl, &gfx::fill(Rgba::white(m.glow)));

        // Name, badge, PEAK: one image per content (the device name, the
        // peak text, the layout and scale).
        let device = self.s.out_device.get_untracked();
        let peak = peak_text(self.s.master_peak.get_untracked());
        let db = crate::state::format_db(self.s.master_db.get_untracked());
        let key = format!("{db}|{device}\u{1}{peak}\u{1}{}\u{1}{}\u{1}{}\u{1}{}\u{1}{}", lay.w, lay.right, lay.by_clips, r.width(), (s * 100.0).round());
        let mut text = self.text.borrow_mut();
        if text.as_ref().is_none_or(|(k, _)| *k != key) {
            *text = render_text(&lay, &device, &peak, &db, r.width(), r.height(), s).map(|img| (key, img));
        }
        if let Some((_, img)) = text.as_ref() {
            canvas.draw_image(img, (r.left, r.top), None);
        }
    }
}

fn render_text(l: &super::header_layout::HeaderLayout, device: &str, peak: &str, db: &str, w: f32, h: f32, s: f32) -> Option<vg::Image> {
    let mut surface = vg::surfaces::raster_n32_premul((w.round().max(1.0) as i32, h.round().max(1.0) as i32))?;
    let canvas = surface.canvas();
    canvas.clear(vg::Color::TRANSPARENT);
    let hl = gfx::hairline(s);
    let top = hl;
    // "Main" where a track's name is.
    let f = gfx::font(Face::SemiBold, NAME_PX * s);
    let (_, m) = f.metrics();
    let base = (top + NAME_TOP * s - m.ascent).round() + s.round();
    gfx::draw_text_clipped(canvas, "Main", (l.name_x() * s).round(), base, l.name_w() * s, &f, TEXT_HI);
    // The OUT badge and the device name after it.
    let bf = gfx::font(Face::SemiBold, BADGE_PX * s);
    let bw = gfx::text_width(&bf, "OUT") + (2.0 + 2.0 * BADGE_PAD) * s;
    let badge = vg::Rect::from_xywh((l.name_x() * s).round(), (top + BADGE_Y * s).round(), bw.round(), BADGE_H * s);
    gfx::ring(canvas, &gfx::rrect(badge, RADIUS_KEY * s), hl, &gfx::fill(STROKE));
    let bb = (top + BADGE_BASELINE * s).round();
    gfx::draw_text(canvas, "OUT", (badge.left + hl + BADGE_PAD * s).round(), bb, &bf, BADGE_INK);
    let df = gfx::font(Face::Medium, 12.0 * s);
    let dx = badge.right + 8.0 * s;
    gfx::draw_text_clipped(canvas, device, dx.round(), bb, (l.name_x() + l.name_w()) * s - dx, &df, DEVICE_INK);
    // PEAK in column A: the label, then the value.
    let lf = gfx::font(Face::SemiBold, LABEL_PX * s);
    let tr = 0.1 * LABEL_PX * s;
    let lw = gfx::tracked_width("PEAK", &lf, tr);
    let cx = (l.col_a() + 28.0) * s;
    let lb = gfx::css_baseline(&lf, top + PEAK_LABEL_Y * s, 12.0 * s).round();
    gfx::draw_text_tracked(canvas, "PEAK", (cx - lw * 0.5).round(), lb, &lf, tr, LABEL_INK);
    let vf = gfx::font(Face::SemiBold, VALUE_PX * s);
    let vw = gfx::text_width(&vf, peak);
    let vb = gfx::css_baseline(&vf, top + PEAK_VALUE_Y * s, 16.0 * s).round();
    gfx::draw_text(canvas, peak, (cx - vw * 0.5).round(), vb, &vf, VALUE_INK);
    // The master fader's dB value, where a track's is (polish 2 §3 type).
    let r = super::chrome::db_value_rect(l, MAIN_ROW_H);
    let df = gfx::font(Face::SemiBold, 10.0 * s);
    let dtr = 0.02 * 10.0 * s;
    let dw = gfx::tracked_width(db, &df, dtr);
    let db_base = gfx::css_baseline(&df, top + r.1 * s, r.3 * s).round();
    gfx::draw_text_tracked(canvas, db, ((r.0 + r.2 * 0.5) * s - dw * 0.5).round(), db_base, &df, dtr, Rgba::hex(0x9A9A9A));
    Some(surface.image_snapshot())
}

fn local(cx: &EventContext) -> (f32, f32) {
    let b = cx.bounds();
    let s = cx.scale_factor();
    ((cx.mouse().cursor_x - b.x) / s, (cx.mouse().cursor_y - b.y) / s)
}

/// The Main track's lane: `#0D0D0D`, the grid and the playhead only.
pub struct MainLane {
    s: Signals,
}

impl MainLane {
    pub fn new(cx: &mut Context, s: Signals) -> Handle<'_, Self> {
        let last = std::cell::Cell::new((f64::NAN, f64::NAN, f64::NAN));
        Self { s }
            .build(cx, |_| {})
            .class("main-lane")
            .height(Pixels(MAIN_TOTAL_H))
            .bind(s.arrangement, move |mut h| {
                let v = s.arrangement.with_untracked(|a| (a.view.scroll_ticks, a.view.px_per_bar(), 0.0));
                if v.0 != last.get().0 || v.1 != last.get().1 {
                    last.set(v);
                    h.needs_redraw();
                }
            })
            .bind(s.transport, |mut h| h.needs_redraw())
    }
}

impl View for MainLane {
    fn element(&self) -> Option<&'static str> {
        Some("main-lane")
    }

    fn event(&mut self, cx: &mut EventContext, event: &mut Event) {
        event.map(|we, _| {
            if let WindowEvent::MouseMove(..) = we {
                cx.emit(AppEvent::HoverHelp("Main's lane: master automation arrives with automation".into()));
            }
        });
    }

    fn draw(&self, cx: &mut DrawContext, canvas: &Canvas) {
        let b = cx.bounds();
        let s = cx.scale_factor();
        let r = gfx::snap(b.into());
        let hl = gfx::hairline(s);
        canvas.draw_rect(vg::Rect::from_xywh(r.left, r.top, r.width(), hl), &gfx::fill(MAIN_DIVIDER));
        let body = vg::Rect::from_ltrb(r.left, r.top + hl, r.right, r.bottom);
        canvas.draw_rect(body, &gfx::fill(MAIN_LANE));
        let (view, playhead) = (self.s.arrangement.with_untracked(|a| a.view), self.s.transport.with_untracked(|t| t.playhead_ticks));
        let x_of = |t: f64| r.left + view.tick_to_x(t) as f32 * s;
        let ppb = view.px_per_bar() as f32;
        let first_bar = (view.scroll_ticks / TICKS_PER_BAR as f64).floor() as i64;
        let nbars = (r.width() / s / ppb).ceil() as i64 + 2;
        for bar in first_bar.max(0)..first_bar + nbars {
            let bx = x_of((bar as u32 * TICKS_PER_BAR) as f64);
            if ppb >= 48.0 {
                for k in 1..4u32 {
                    let x = (bx + k as f32 * ppb / 4.0 * s).round();
                    canvas.draw_rect(vg::Rect::from_xywh(x, body.top, hl, body.height()), &gfx::fill(GRID_BEAT));
                }
            }
            canvas.draw_rect(vg::Rect::from_xywh(bx.round(), body.top, hl, body.height()), &gfx::fill(GRID_BAR));
        }
        let px = x_of(playhead).round();
        if px >= r.left && px < r.right {
            canvas.draw_rect(vg::Rect::from_xywh(px, body.top, hl, body.height()), &gfx::fill(NEUTRAL_HI.with_alpha(0.85)));
        }
    }
}

/// The soft shadow the Main row casts upward onto the tracks (drawn just
/// above it, over the rows; it takes no input).
pub struct MainShadow;

impl MainShadow {
    pub fn new(cx: &mut Context) -> Handle<'_, Self> {
        Self.build(cx, |_| {}).height(Pixels(SHADOW_H)).hoverable(false)
    }
}

impl View for MainShadow {
    fn draw(&self, cx: &mut DrawContext, canvas: &Canvas) {
        let r: vg::Rect = cx.bounds().into();
        canvas.draw_rect(r, &gfx::vgrad(r, Rgba::black(0.0), Rgba::black(0.28)));
    }
}
