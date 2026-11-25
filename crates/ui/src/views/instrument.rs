//! The device panel's INSTRUMENT section: the selected lane's macros drawn as
//! four step graphs (volume, arpeggio, pitch, duty), editable by dragging
//! across them. Loop (↺) and release (R) points are marked. A kit shows its
//! drum map instead.

use chip::instrument::{Seq, NONE};
use vizia::prelude::*;
use vizia::vg;

use super::gfx::{self, Face};
use crate::theme::tokens::*;
use crate::{AppEvent, Signals};

/// The four macros, in display order: (label, value range shown).
pub const MACROS: [(&str, i32, i32); 4] = [("VOLUME", 0, 15), ("ARPEGGIO", -12, 12), ("PITCH", -32, 32), ("DUTY", 0, 16)];
const GAP: f32 = 8.0;
const LABEL_H: f32 = 16.0;

pub struct InstrumentView {
    s: Signals,
    /// Macro being drawn on, and the last step written.
    drawing: Option<(usize, usize)>,
}

impl InstrumentView {
    pub fn new(cx: &mut Context, s: Signals) -> Handle<'_, Self> {
        Self { s, drawing: None }
            .build(cx, |_| {})
            .class("instrument-view")
            .cursor(CursorIcon::Crosshair)
            .bind(s.arrangement, |mut h| h.needs_redraw())
            .bind(s.lane_sel, |mut h| h.needs_redraw())
    }

    /// Graph rects (physical px) in a 2×2 grid.
    fn graphs(b: BoundingBox, s: f32) -> [vg::Rect; 4] {
        let w = (b.w - GAP * s) / 2.0;
        let h = (b.h - GAP * s) / 2.0;
        std::array::from_fn(|i| {
            let (cx, cy) = ((i % 2) as f32, (i / 2) as f32);
            vg::Rect::from_xywh(b.x + cx * (w + GAP * s), b.y + cy * (h + GAP * s) + LABEL_H * s, w, h - LABEL_H * s)
        })
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
        let lane = self.s.lane_sel.get_untracked();
        let Some(len) = self
            .s
            .arrangement
            .with_untracked(|a| a.instruments.get(lane).filter(|i| !i.is_kit()).map(|i| Self::seq_of(&i.inst, m).len))
        else {
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
            cx.emit(AppEvent::EditMacro { lane, which: m, step: st, value: v as i8 });
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
        let Some(inst) = self.s.arrangement.with_untracked(|a| a.instruments.get(lane).cloned()) else { return };
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
            gfx::draw_text(canvas, &summary, (r.right - sw).round(), (r.top - 5.0 * s).round(), &fm, TEXT_GHOST);
            // Inset well.
            let rr = gfx::rrect(r, RADIUS_FIELD * s);
            canvas.draw_rrect(rr, &gfx::vgrad(r, Rgba::hex(0x070707), Rgba::hex(0x0E0E0E)));
            gfx::inner_shadow(canvas, &rr, 0.0, 1.0 * s, 3.0 * s, Rgba::black(0.8));
            gfx::ring(canvas, &rr, gfx::hairline(s), &gfx::fill(STROKE.with_alpha(0.5)));
            if q.is_empty() {
                let f = gfx::font(Face::Regular, 10.0 * s);
                let t = "not used";
                let tw = gfx::text_width(&f, t);
                gfx::draw_text(canvas, t, (r.center_x() - tw * 0.5).round(), gfx::baseline_for_center(&f, r.center_y()).round(), &f, TEXT_GHOST);
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
    }
}

fn note_name(p: u8) -> String {
    const N: [&str; 12] = ["C", "C#", "D", "D#", "E", "F", "F#", "G", "G#", "A", "A#", "B"];
    format!("{}{}", N[p as usize % 12], p as i32 / 12 - 1)
}
