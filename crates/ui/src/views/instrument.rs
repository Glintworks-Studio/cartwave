//! The device panel's INSTRUMENT section: the selected lane's macros drawn as
//! four step graphs (volume, arpeggio, pitch, duty), editable by dragging
//! across them, and the vibrato row (depth, rate, delay: drag or wheel).
//! Loop (↺) and release (R) points are marked. A kit shows its drum map
//! instead.

use chip::instrument::{Seq, NONE};
use vizia::prelude::*;
use vizia::vg;
use vizia::vg::Contains;

use super::gfx::{self, Face};
use crate::theme::tokens::*;
use crate::{AppEvent, Signals};

/// The four macros, in display order: (label, value range shown).
pub const MACROS: [(&str, i32, i32); 4] = [("VOLUME", 0, 15), ("ARPEGGIO", -12, 12), ("PITCH", -32, 32), ("DUTY", 0, 16)];
const GAP: f32 = 8.0;
const LABEL_H: f32 = 16.0;
/// The vibrato row under the graphs.
const VIB_H: f32 = 20.0;
/// Vibrato fields: (label, max). Depth in cents, period and delay in frames.
const VIB: [(&str, u8); 3] = [("DEPTH", 100), ("RATE", 30), ("DELAY", 60)];
/// Drag distance per vibrato step, logical px.
const VIB_DRAG_PX: f32 = 4.0;

pub struct InstrumentView {
    s: Signals,
    /// Macro being drawn on, and the last step written.
    drawing: Option<(usize, usize)>,
    /// Vibrato field being dragged: (field, start y, start value).
    vib_drag: Option<(usize, f32, u8)>,
    wheel: crate::input::WheelSteps,
}

impl InstrumentView {
    pub fn new(cx: &mut Context, s: Signals) -> Handle<'_, Self> {
        Self { s, drawing: None, vib_drag: None, wheel: Default::default() }
            .build(cx, |_| {})
            .class("instrument-view")
            .cursor(CursorIcon::Crosshair)
            .bind(s.arrangement, |mut h| h.needs_redraw())
            .bind(s.lane_sel, |mut h| h.needs_redraw())
    }

    /// Graph rects (physical px) in a 2×2 grid, above the vibrato row.
    fn graphs(b: BoundingBox, s: f32) -> [vg::Rect; 4] {
        let w = (b.w - GAP * s) / 2.0;
        let h = (b.h - (VIB_H + GAP) * s - GAP * s) / 2.0;
        std::array::from_fn(|i| {
            let (cx, cy) = ((i % 2) as f32, (i / 2) as f32);
            vg::Rect::from_xywh(b.x + cx * (w + GAP * s), b.y + cy * (h + GAP * s) + LABEL_H * s, w, h - LABEL_H * s)
        })
    }

    /// The vibrato fields' rects (physical px): the row under the graphs,
    /// after its label.
    fn vib_fields(b: BoundingBox, s: f32) -> [vg::Rect; 3] {
        let y = b.y + b.h - VIB_H * s;
        let x0 = b.x + 64.0 * s;
        let w = ((b.w - 64.0 * s) / 3.0).max(0.0);
        std::array::from_fn(|i| vg::Rect::from_xywh(x0 + i as f32 * w, y, w - 4.0 * s, VIB_H * s))
    }

    /// The instrument the inspector shows (the selected clip's, else the lane's).
    fn current(&self) -> Option<(crate::instruments::InstId, crate::instruments::LaneInstrument)> {
        let lane = self.s.lane_sel.get_untracked();
        self.s.arrangement.with_untracked(|a| {
            let id = a.current_instrument(lane);
            a.instruments.get(id).map(|i| (id, i.clone()))
        })
    }

    fn vibrato(&self) -> Option<chip::instrument::Vibrato> {
        self.current().filter(|(_, i)| !i.is_kit()).map(|(_, i)| i.inst.vibrato)
    }

    fn vib_get(v: &chip::instrument::Vibrato, i: usize) -> u8 {
        [v.depth, v.period, v.delay][i]
    }

    fn vib_text(v: &chip::instrument::Vibrato, i: usize) -> String {
        match i {
            0 if v.depth == 0 => "off".into(),
            0 => format!("{}c", v.depth),
            1 => format!("{:.1}Hz", 60.1 / v.period.max(1) as f32),
            _ => format!("{}ms", (v.delay as f32 * 1000.0 / 60.1).round() as u32),
        }
    }

    fn set_vib(&self, cx: &mut EventContext, which: usize, value: i32) {
        let Some((inst, _)) = self.current() else { return };
        let lo = if which == 1 { 1 } else { 0 };
        let v = value.clamp(lo, VIB[which].1 as i32) as u8;
        cx.emit(AppEvent::EditVibrato { inst, which, value: v });
    }

    fn seq_of(inst: &chip::instrument::Instrument, i: usize) -> &Seq {
        match i {
            0 => &inst.volume,
            1 => &inst.arp,
            2 => &inst.pitch,
            _ => &inst.duty,
        }
    }

    /// Writes the value under the pointer into macro `m`.
    fn paint(&mut self, cx: &mut EventContext, m: usize) {
        let Some((inst, len)) = self.current().filter(|(_, i)| !i.is_kit()).map(|(id, i)| (id, Self::seq_of(&i.inst, m).len)) else {
            return;
        };
        if len == 0 {
            return;
        }
        let r = Self::graphs(cx.bounds(), cx.scale_factor())[m];
        let (x, y) = (cx.mouse().cursor_x, cx.mouse().cursor_y);
        let step = (((x - r.left) / r.width()) * len as f32).floor().clamp(0.0, len as f32 - 1.0) as usize;
        let (_, lo, hi) = MACROS[m];
        let t = ((r.bottom - y) / r.height()).clamp(0.0, 1.0);
        let v = (lo as f32 + t * (hi - lo) as f32).round() as i32;
        // Fill the steps skipped by a fast drag.
        let from = self.drawing.map_or(step, |(_, s)| s);
        let (a, b) = if from <= step { (from, step) } else { (step, from) };
        for st in a..=b {
            cx.emit(AppEvent::EditMacro { inst, which: m, step: st, value: v as i8 });
        }
        self.drawing = Some((m, step));
    }
}

impl View for InstrumentView {
    fn element(&self) -> Option<&'static str> {
        Some("instrument-view")
    }

    fn event(&mut self, cx: &mut EventContext, event: &mut Event) {
        event.map(|we, meta| match we {
            WindowEvent::MouseDown(MouseButton::Left)
                if self.vibrato().is_some() && Self::vib_fields(cx.bounds(), cx.scale_factor()).iter().any(|r| r.contains(vg::Point::new(cx.mouse().cursor_x, cx.mouse().cursor_y))) =>
            {
                let (x, y) = (cx.mouse().cursor_x, cx.mouse().cursor_y);
                let f = Self::vib_fields(cx.bounds(), cx.scale_factor()).iter().position(|r| r.contains(vg::Point::new(x, y))).unwrap_or(0);
                let v = self.vibrato().map_or(0, |v| Self::vib_get(&v, f));
                self.vib_drag = Some((f, y, v));
                cx.capture();
                cx.emit(AppEvent::GestureBegin);
                meta.consume();
            }
            WindowEvent::MouseScroll(_, sy) => {
                let (x, y) = (cx.mouse().cursor_x, cx.mouse().cursor_y);
                if let (Some(v), Some(f)) =
                    (self.vibrato(), Self::vib_fields(cx.bounds(), cx.scale_factor()).iter().position(|r| r.contains(vg::Point::new(x, y))))
                {
                    let steps = self.wheel.steps(*sy);
                    if steps != 0 {
                        // Rate is shown in Hz: more Hz = fewer frames per cycle.
                        let dir = if f == 1 { -1 } else { 1 };
                        self.set_vib(cx, f, Self::vib_get(&v, f) as i32 + dir * steps);
                    }
                    meta.consume();
                }
            }
            WindowEvent::MouseDown(MouseButton::Left) => {
                let (x, y) = (cx.mouse().cursor_x, cx.mouse().cursor_y);
                let gs = Self::graphs(cx.bounds(), cx.scale_factor());
                if let Some(m) = gs.iter().position(|r| x >= r.left && x <= r.right && y >= r.top && y <= r.bottom) {
                    cx.capture();
                    cx.emit(AppEvent::GestureBegin);
                    self.drawing = None;
                    self.paint(cx, m);
                    meta.consume();
                }
            }
            WindowEvent::MouseMove(..) => {
                if let Some((m, _)) = self.drawing {
                    self.paint(cx, m);
                }
                if let Some((f, y0, v0)) = self.vib_drag {
                    let steps = ((y0 - cx.mouse().cursor_y) / (VIB_DRAG_PX * cx.scale_factor())).round() as i32;
                    let dir = if f == 1 { -1 } else { 1 };
                    self.set_vib(cx, f, v0 as i32 + dir * steps);
                }
            }
            WindowEvent::MouseUp(MouseButton::Left) if self.vib_drag.is_some() => {
                self.vib_drag = None;
                cx.release();
                cx.emit(AppEvent::GestureEnd);
                meta.consume();
            }
            WindowEvent::MouseUp(MouseButton::Left) if self.drawing.is_some() => {
                self.drawing = None;
                cx.release();
                cx.emit(AppEvent::GestureEnd);
                meta.consume();
            }
            _ => {}
        });
    }

    fn draw(&self, cx: &mut DrawContext, canvas: &Canvas) {
        let b = cx.bounds();
        let s = cx.scale_factor();
        let lane = self.s.lane_sel.get_untracked();
        let Some((_, inst)) = self.current() else { return };
        let color = self
            .s
            .arrangement
            .with_untracked(|a| a.lanes.get(lane).map_or(ACCENT, |l| l.color));
        let fl = gfx::font(Face::SemiBold, 9.0 * s);
        let fm = gfx::font(Face::Mono, 9.0 * s);

        if inst.is_kit() {
            // Drum map: one row per piece.
            let f = gfx::font(Face::Medium, 12.0 * s);
            let mut lo = 0u8;
            for (i, k) in inst.kit.iter().enumerate() {
                let y = b.y + (i as f32 * 24.0 + 16.0) * s;
                let range = format!("{} – {}", note_name(lo), note_name(k.top.min(127)));
                gfx::draw_text(canvas, &range, b.x.round(), y.round(), &fm, TEXT_LO);
                gfx::draw_text(canvas, &k.name, (b.x + 96.0 * s).round(), y.round(), &f, TEXT_HI);
                let bar = vg::Rect::from_xywh(b.x + 200.0 * s, y - 14.0 * s, (b.w - 200.0 * s).max(0.0), 16.0 * s);
                let vol = k.inst.volume.values();
                // The piece's volume envelope as tiny bars.
                if !vol.is_empty() && bar.width() > 8.0 {
                    let bw = (bar.width() / vol.len() as f32).min(8.0 * s);
                    for (j, v) in vol.iter().enumerate() {
                        let h = bar.height() * (*v as f32 / 15.0);
                        let r = vg::Rect::from_xywh(bar.left + bw * j as f32, bar.bottom - h, (bw - s).max(s), h);
                        canvas.draw_rect(r, &gfx::vgrad(r, color.with_alpha(0.9), color.darken(0.3).with_alpha(0.7)));
                    }
                }
                lo = k.top.saturating_add(1);
            }
            return;
        }

        for (m, r) in Self::graphs(b, s).into_iter().enumerate() {
            let q = Self::seq_of(&inst.inst, m);
            let (label, lo, hi) = MACROS[m];
            gfx::draw_text(canvas, label, r.left.round(), (r.top - 5.0 * s).round(), &fl, TEXT_LO);
            let summary = if q.is_empty() { "—".to_string() } else { format!("{} steps", q.len) };
            let sw = gfx::text_width(&fm, &summary);
            gfx::draw_text(canvas, &summary, (r.right - sw).round(), (r.top - 5.0 * s).round(), &fm, TEXT_LO);
            // Inset well.
            let rr = gfx::rrect(r, RADIUS_FIELD * s);
            canvas.draw_rrect(rr, &gfx::vgrad(r, Rgba::hex(0x070707), Rgba::hex(0x0E0E0E)));
            gfx::inner_shadow(canvas, &rr, 0.0, 1.0 * s, 3.0 * s, Rgba::black(0.8));
            gfx::ring(canvas, &rr, gfx::hairline(s), &gfx::fill(STROKE.with_alpha(0.5)));
            if q.is_empty() {
                let f = gfx::font(Face::Regular, 10.0 * s);
                let t = "not used";
                let tw = gfx::text_width(&f, t);
                gfx::draw_text(canvas, t, (r.center_x() - tw * 0.5).round(), gfx::baseline_for_center(&f, r.center_y()).round(), &f, TEXT_LO);
                continue;
            }
            canvas.save();
            canvas.clip_rrect(rr, None, Some(true));
            let n = q.len as f32;
            let span = (hi - lo) as f32;
            let zero_y = r.bottom - r.height() * ((0 - lo) as f32 / span);
            if lo < 0 {
                canvas.draw_rect(vg::Rect::from_xywh(r.left, zero_y.round(), r.width(), gfx::hairline(s)), &gfx::fill(LINE.lighten(0.05)));
            }
            let bw = r.width() / n;
            for (j, v) in q.values().iter().enumerate() {
                let v = (*v as i32).clamp(lo, hi);
                let y = r.bottom - r.height() * ((v - lo) as f32 / span);
                let x0 = r.left + bw * j as f32;
                let (top, bot) = if lo < 0 { (y.min(zero_y), y.max(zero_y)) } else { (y, r.bottom) };
                let bar = vg::Rect::from_ltrb(x0 + 1.0 * s, top, x0 + bw - 1.0 * s, bot.max(top + 1.5 * s));
                let in_loop = q.loop_at != NONE && j as u8 >= q.loop_at && (q.release_at == NONE || j as u8 <= q.release_at || q.loop_at > q.release_at);
                let after_rel = q.release_at != NONE && j as u8 > q.release_at;
                let c = if after_rel { color.darken(0.35) } else { color };
                let alpha = if in_loop { 1.0 } else { 0.78 };
                canvas.draw_rect(bar, &gfx::vgrad(bar, c.lighten(0.1).with_alpha(alpha), c.darken(0.25).with_alpha(alpha)));
            }
            // Loop and release markers.
            for (idx, txt, col) in [(q.loop_at, "↺", ACCENT), (q.release_at, "R", RECORD)] {
                if idx != NONE {
                    let x = (r.left + bw * idx as f32).round();
                    canvas.draw_rect(vg::Rect::from_xywh(x, r.top, gfx::hairline(s), r.height()), &gfx::fill(col.with_alpha(0.7)));
                    gfx::draw_text(canvas, txt, x + 3.0 * s, r.top + 10.0 * s, &fl, col);
                }
            }
            canvas.restore();
        }

        // Vibrato row.
        let v = inst.inst.vibrato;
        let fr = Self::vib_fields(b, s);
        let base = |r: &vg::Rect| (r.top + VIB_H * s * 0.5 + 3.5 * s).round();
        gfx::draw_text(canvas, "VIBRATO", b.x.round(), base(&fr[0]), &fl, TEXT_LO);
        for (i, r) in fr.iter().enumerate() {
            let rr = gfx::rrect(*r, RADIUS_FIELD * s);
            canvas.draw_rrect(rr, &gfx::vgrad(*r, Rgba::hex(0x070707), Rgba::hex(0x0E0E0E)));
            gfx::ring(canvas, &rr, gfx::hairline(s), &gfx::fill(STROKE.with_alpha(0.5)));
            gfx::draw_text(canvas, VIB[i].0, (r.left + 6.0 * s).round(), base(r), &fl, TEXT_LO);
            let t = Self::vib_text(&v, i);
            let tw = gfx::text_width(&fm, &t);
            let live = v.depth > 0 || i == 0;
            gfx::draw_text(canvas, &t, (r.right - 6.0 * s - tw).round(), base(r), &fm, if live { TEXT_HI } else { TEXT_LO });
        }
    }
}

fn note_name(p: u8) -> String {
    const N: [&str; 12] = ["C", "C#", "D", "D#", "E", "F", "F#", "G", "G#", "A", "A#", "B"];
    format!("{}{}", N[p as usize % 12], p as i32 / 12 - 1)
}
