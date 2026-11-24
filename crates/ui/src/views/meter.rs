//! Per-channel vertical level meters in inset wells (one view draws all 8).

use vizia::prelude::*;
use vizia::vg;

use super::gfx::{self, Face};
use super::visualizer::draw_light;
use crate::theme::tokens::*;
use crate::Signals;

pub struct MeterBank {
    s: Signals,
}

impl MeterBank {
    pub fn new(cx: &mut Context, s: Signals) -> Handle<'_, Self> {
        Self { s }
            .build(cx, |_| {})
            .bind(s.meters, |mut h| h.needs_redraw())
    }
}

const WELL_W: f32 = 14.0;
const LABEL_H: f32 = 18.0;
const LIGHT_H: f32 = 18.0;
/// Segment pitch for the LED-ladder texture (logical px).
const SEG: f32 = 3.0;
/// Level thresholds where the gradient turns amber / red.
const AMBER_AT: f32 = 0.62;
const RED_AT: f32 = 0.84;

impl View for MeterBank {
    fn draw(&self, cx: &mut DrawContext, canvas: &Canvas) {
        let _perf = crate::perf::scope(crate::perf::Span::Meters);
        let b = cx.bounds();
        let s = cx.scale_factor();
        let m = self.s.meters.get_untracked();
        let lanes: Vec<(Rgba, &'static str)> = self
            .s
            .arrangement
            .with_untracked(|a| a.lanes.iter().map(|l| (l.color, l.short)).collect());
        let n = lanes.len().min(8);
        let col_w = b.w / n as f32;
        let well_w = WELL_W * s;
        let top = b.y + LIGHT_H * s;
        let bottom = b.y + b.h - LABEL_H * s;
        let h = (bottom - top).max(10.0);
        let f = gfx::font(Face::SemiBold, 9.0 * s);

        for (i, &(lane_col, short)) in lanes.iter().enumerate().take(n) {
            let cxm = (b.x + col_w * (i as f32 + 0.5)).round();
            draw_light(canvas, cxm, b.y + 6.0 * s, 3.5 * s, lane_col, m.light[i], s);

            let well = vg::Rect::from_xywh(cxm - well_w * 0.5, top, well_w, h);
            let rr = gfx::rrect(well, 4.0 * s);
            // Bottom bevel highlight, then the well itself.
            let mut hl = rr;
            hl.offset((0.0, 1.0 * s));
            canvas.draw_rrect(hl, &gfx::fill(Rgba::white(0.06)));
            canvas.draw_rrect(
                rr,
                &gfx::vgrad(well, Rgba::hex(0x06070A), Rgba::hex(0x0C0D11)),
            );

            let lvl = m.level[i].clamp(0.0, 1.0);
            let pad = 2.0 * s;
            let inner = vg::Rect::from_ltrb(
                well.left + pad,
                well.top + pad,
                well.right - pad,
                well.bottom - pad,
            );
            let fill_top = inner.bottom - inner.height() * lvl;
            canvas.save();
            canvas.clip_rrect(gfx::rrect(inner, 2.5 * s), vg::ClipOp::Intersect, true);
            let grad = |a: f32| {
                gfx::linear(
                    (inner.left, inner.bottom),
                    (inner.left, inner.top),
                    &[
                        METER_GREEN.with_alpha(a),
                        METER_GREEN.with_alpha(a),
                        METER_AMBER.with_alpha(a),
                        METER_RED.with_alpha(a),
                    ],
                    Some(&[0.0, AMBER_AT, RED_AT, 1.0]),
                )
            };
            let mut p = vg::Paint::default();
            p.set_anti_alias(true);
            // Unlit ladder, faintly tinted.
            p.set_shader(grad(0.07));
            canvas.draw_rect(inner, &p);
            if lvl > 0.0 {
                let lit = vg::Rect::from_ltrb(inner.left, fill_top, inner.right, inner.bottom);
                p.set_shader(grad(1.0));
                canvas.draw_rect(lit, &p);
                // Sheen on the left third for a rounded-tube feel.
                canvas.draw_rect(
                    vg::Rect::from_ltrb(
                        inner.left,
                        fill_top,
                        inner.left + inner.width() * 0.4,
                        inner.bottom,
                    ),
                    &gfx::fill(Rgba::white(0.14)),
                );
            }
            // Segment gaps: one rect with a repeating hard-stop gradient (a dark
            // row every SEG px, counted up from the bottom) instead of ~50 rects.
            let period = SEG * s;
            let gap_h = s.max(1.0);
            let gap_col = Rgba::hex(0x07080B).with_alpha(0.8);
            let clear = gap_col.with_alpha(0.0);
            let edge = 1.0 - gap_h / period;
            let base = inner.bottom.round();
            let mut gp = vg::Paint::default();
            gp.set_shader(gfx::linear_tiled(
                (inner.left, base),
                (inner.left, base - period),
                &[clear, clear, gap_col, gap_col],
                Some(&[0.0, edge, edge, 1.0]),
                vg::TileMode::Repeat,
            ));
            canvas.draw_rect(
                vg::Rect::from_ltrb(inner.left, inner.top, inner.right, base - period + gap_h),
                &gp,
            );
            // Peak-hold line.
            let pk = m.peak[i].clamp(0.0, 1.0);
            if pk > 0.01 {
                let py = inner.bottom - inner.height() * pk;
                let pc = if pk > RED_AT {
                    METER_RED
                } else if pk > AMBER_AT {
                    METER_AMBER
                } else {
                    METER_GREEN
                };
                canvas.draw_rect(
                    vg::Rect::from_xywh(inner.left, py - 1.0 * s, inner.width(), 2.0 * s),
                    &gfx::fill(pc.lighten(0.35)),
                );
            }
            canvas.restore();
            gfx::inner_shadow(canvas, &rr, 0.0, 1.5 * s, 3.0 * s, Rgba::black(0.9));
            gfx::inner_border(canvas, &rr, s, Rgba::black(0.6));

            let w = gfx::text_width(&f, short);
            gfx::draw_text(
                canvas,
                short,
                cxm - w * 0.5,
                b.y + b.h - 3.0 * s,
                &f,
                TEXT_LO,
            );
        }
    }
}
