//! The channel light: a glowing lens that pulses on note-on (used by the
//! inspector's meter bank; the track rows' colour strips pulse too).

use vizia::prelude::*;
use vizia::vg;

use super::gfx;
use crate::theme::tokens::*;

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
    p.set_dither(gfx::dither_on());
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
