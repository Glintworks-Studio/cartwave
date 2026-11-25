//! Procedural clip rendering. The arrangement view owns interaction; this file
//! only knows how to paint one clip given its animated visual state.

use vizia::prelude::*;
use vizia::vg;

use super::gfx::{self, Face};
use crate::notes::Note;
use crate::theme::tokens::*;

/// Eased 0..1 animation values for one clip.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct ClipAnim {
    pub hover: f32,
    pub selected: f32,
    pub drag: f32,
}

impl ClipAnim {
    pub fn step(&mut self, target: ClipAnim, dt: f32) -> bool {
        self.hover = gfx::approach(self.hover, target.hover, dt, EASE_TAU);
        self.selected = gfx::approach(self.selected, target.selected, dt, EASE_TAU);
        self.drag = gfx::approach(self.drag, target.drag, dt, EASE_TAU);
        let settled = (self.hover - target.hover).abs() < 0.002
            && (self.selected - target.selected).abs() < 0.002
            && (self.drag - target.drag).abs() < 0.002;
        if settled {
            *self = target;
        }
        !settled
    }
}

pub struct ClipPaint<'a> {
    /// Physical-pixel rectangle of the clip body.
    pub rect: vg::Rect,
    pub color: Rgba,
    pub name: &'a str,
    pub notes: &'a [Note],
    pub len_ticks: u32,
    pub anim: ClipAnim,
    /// Physical px per tick, for placing notes.
    pub px_per_tick: f32,
    /// Visible horizontal span (physical) used to cull note bars.
    pub visible: (f32, f32),
    pub scale: f32,
}

/// Rough perceived luminance, used to pick dark or light ink.
fn luminance(c: Rgba) -> f32 {
    0.2126 * c.r + 0.7152 * c.g + 0.0722 * c.b
}

/// Shared shadow pass, drawn for all clips before any bodies so neighbouring
/// shadows never overlap a body.
pub fn draw_clip_shadow(canvas: &Canvas, p: &ClipPaint) {
    let s = p.scale;
    let rr = gfx::rrect(p.rect, RADIUS_CLIP * s);
    let a = p.anim;
    // 1. Soft outer drop shadow, raised while dragging.
    let dy = (2.0 + 6.0 * a.drag) * s;
    let blur = (8.0 + 12.0 * a.drag) * s;
    let alpha = 0.32 + 0.18 * a.drag;
    gfx::drop_shadow(canvas, &rr, 0.0, dy, blur, 0.0, Rgba::black(alpha));
    // Selection glow in the lane colour.
    if a.selected > 0.01 {
        gfx::drop_shadow(
            canvas,
            &rr,
            0.0,
            0.0,
            14.0 * s,
            1.5 * s,
            p.color.lighten(0.15).with_alpha(0.75 * a.selected),
        );
    }
}

pub fn draw_clip_body(canvas: &Canvas, p: &ClipPaint) {
    let s = p.scale;
    let r = p.rect;
    let rr = gfx::rrect(r, RADIUS_CLIP * s);
    let a = p.anim;

    let dragging_alpha = 1.0 - 0.1 * a.drag;
    let layered = dragging_alpha < 0.999;
    if layered {
        canvas.save_layer_alpha_f(r.with_outset((2.0 * s, 2.0 * s)), dragging_alpha);
    }

    // Hover lifts brightness slightly.
    let base = p.color.lighten(0.07 * a.hover + 0.04 * a.selected);

    // 2. Base gradient: a deep tint of the lane colour, lighter at the top
    // (about ±10% lightness so it reads as lit, not as a 2008 button).
    let body = base.mix(BG_1, 0.5);
    let top = body.lighten(0.04);
    let bottom = body.darken(0.16);
    canvas.draw_rrect(rr, &gfx::vgrad(r, top, bottom));

    // Header strip: a lighter band holding the name.
    let header_h = (CLIP_HEADER_HEIGHT * s).min(r.height());
    canvas.save();
    canvas.clip_rrect(rr, vg::ClipOp::Intersect, true);
    let header = vg::Rect::from_xywh(r.left, r.top, r.width(), header_h);
    canvas.draw_rect(
        header,
        &gfx::vgrad(header, base.lighten(0.02), base.darken(0.10)),
    );
    // Hairline under the header.
    canvas.draw_rect(
        vg::Rect::from_xywh(r.left, r.top + header_h - s, r.width(), s),
        &gfx::fill(Rgba::black(0.14)),
    );

    // Header ink adapts to the lane colour; the body is always dark.
    let dark_ink = luminance(base) > 0.42;
    let ink = if dark_ink {
        Rgba::hex(0x0B0C10).with_alpha(0.88)
    } else {
        Rgba::white(0.96)
    };

    // Mini note preview.
    if !p.notes.is_empty() && r.height() > header_h + 8.0 * s {
        let (lo, hi) = p.notes.iter().fold((u8::MAX, 0u8), |(lo, hi), n| {
            (lo.min(n.pitch), hi.max(n.pitch))
        });
        let span = (hi.saturating_sub(lo)).max(6) as f32;
        let area_top = r.top + header_h + 4.0 * s;
        let area_bot = r.bottom - 5.0 * s;
        let bar_h = (2.5 * s).max(((area_bot - area_top) / (span + 1.0)).min(3.5 * s));
        let note_ink = base.lighten(0.25).with_alpha(0.85);
        let paint = gfx::fill(note_ink);
        for n in p.notes {
            let x0 = r.left + n.start as f32 * p.px_per_tick;
            let x1 = r.left + (n.start + n.len).min(p.len_ticks) as f32 * p.px_per_tick;
            if x1 < p.visible.0 || x0 > p.visible.1 || x0 >= r.right {
                continue;
            }
            let t = (n.pitch - lo) as f32 / span;
            let y = area_bot - bar_h - t * (area_bot - area_top - bar_h);
            let w = (x1 - x0 - 1.0 * s).max(1.0 * s);
            canvas.draw_rrect(
                gfx::rrect(vg::Rect::from_xywh(x0 + 0.5 * s, y, w, bar_h), 1.0 * s),
                &paint,
            );
        }
    }

    // Name.
    if r.width() > 18.0 * s {
        let f = gfx::font(Face::SemiBold, 10.5 * s);
        let base_y = gfx::baseline_for_center(&f, r.top + header_h * 0.5);
        let x = r.left.max(p.visible.0) + 7.0 * s;
        gfx::draw_text_clipped(canvas, p.name, x, base_y, r.right - x - 5.0 * s, &f, ink);
    }

    // 5. Subtle inner shadow at the bottom edge.
    canvas.restore();
    gfx::inner_shadow(canvas, &rr, 0.0, -2.0 * s, 4.0 * s, Rgba::black(0.22));

    // 3. 1px inner top highlight: "lit from above".
    gfx::top_highlight(canvas, &rr, s, Rgba::white(0.16 + 0.08 * a.hover));

    // 4. 1px inner border, darker than the base; brightens when selected.
    let border = Rgba::black(0.38).mix(p.color.lighten(0.65).with_alpha(0.95), a.selected);
    gfx::inner_border(canvas, &rr, s, border);

    if layered {
        canvas.restore();
    }
}

/// Dashed outline left at a clip's original position while it is dragged.
pub fn draw_ghost(canvas: &Canvas, rect: vg::Rect, color: Rgba, s: f32) {
    let rr = gfx::rrect(rect.with_inset((0.5 * s, 0.5 * s)), RADIUS_CLIP * s);
    canvas.draw_rrect(rr, &gfx::fill(color.with_alpha(0.07)));
    let mut p = gfx::stroke(color.lighten(0.3).with_alpha(0.55), s);
    p.set_path_effect(vg::PathEffect::dash(&[4.0 * s, 3.0 * s], 0.0));
    canvas.draw_rrect(rr, &p);
}
