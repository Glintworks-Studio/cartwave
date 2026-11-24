//! Strip of 8 channel lights that pulse when that channel's mock notes play.

use vizia::prelude::*;
use vizia::vg;

use super::gfx::{self, Face};
use crate::theme::tokens::*;
use crate::Signals;

pub struct ChannelLights {
    s: Signals,
}

impl ChannelLights {
    pub fn new(cx: &mut Context, s: Signals) -> Handle<'_, Self> {
        Self { s }
            .build(cx, |_| {})
            .bind(s.meters, |mut h| h.needs_redraw())
    }
}

/// Draws one glowing "LED" lens centred at (x, y).
pub fn draw_light(canvas: &Canvas, x: f32, y: f32, r: f32, color: Rgba, level: f32, s: f32) {
    let level = level.clamp(0.0, 1.0);
    // Inset socket with a bottom bevel highlight.
    canvas.draw_circle(
        (x, y + 0.75 * s),
        r + 2.0 * s,
        &gfx::fill(Rgba::white(0.06)),
    );
    canvas.draw_circle((x, y), r + 2.0 * s, &gfx::fill(BG_INSET));
    // Glow.
    if level > 0.01 {
        let mut glow = gfx::fill(color.with_alpha(0.6 * level));
        glow.set_mask_filter(vg::MaskFilter::blur(vg::BlurStyle::Normal, r * 1.1, false));
        canvas.draw_circle((x, y), r * 1.7, &glow);
    }
    // Lens.
    let off = color.darken(0.72);
    let lit = off.mix(color, 0.2 + 0.8 * level);
    let mut p = vg::Paint::default();
    p.set_anti_alias(true);
    p.set_shader(gfx::radial(
        (x - r * 0.3, y - r * 0.35),
        r * 1.5,
        &[lit.lighten(0.2 + 0.45 * level), lit, lit.darken(0.35)],
        Some(&[0.0, 0.55, 1.0]),
    ));
    canvas.draw_circle((x, y), r, &p);
    // Specular dot.
    canvas.draw_circle(
        (x - r * 0.32, y - r * 0.38),
        r * 0.28,
        &gfx::fill(Rgba::white(0.16 + 0.3 * level)),
    );
}

impl View for ChannelLights {
    fn draw(&self, cx: &mut DrawContext, canvas: &Canvas) {
        let _perf = crate::perf::scope(crate::perf::Span::Lights);
        let b = cx.bounds();
        let s = cx.scale_factor();
        let lights = self.s.meters.with_untracked(|m| m.light);
        let lanes: Vec<(Rgba, &'static str)> = self
            .s
            .arrangement
            .with_untracked(|a| a.lanes.iter().map(|l| (l.color, l.short)).collect());
        let n = lanes.len().min(8);
        let step = b.w / n as f32;
        let f = gfx::font(Face::SemiBold, 8.0 * s);
        for (i, &(col, short)) in lanes.iter().enumerate().take(n) {
            let x = b.x + step * (i as f32 + 0.5);
            let y = b.y + 10.0 * s;
            draw_light(canvas, x, y, 5.0 * s, col, lights[i], s);
            let w = gfx::text_width(&f, short);
            let tc = TEXT_LO.mix(col.lighten(0.3), lights[i] * 0.8);
            gfx::draw_text(canvas, short, x - w * 0.5, b.y + 29.0 * s, &f, tc);
        }
    }
}
