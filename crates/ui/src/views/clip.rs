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
    /// The clip's own instrument, when it differs from its lane's default.
    pub inst: Option<&'a str>,
    pub notes: &'a [Note],
    pub len_ticks: u32,
    pub anim: ClipAnim,
    /// Physical px per tick, for placing notes.
    pub px_per_tick: f32,
    /// Visible horizontal span (physical) used to cull note bars.
    pub visible: (f32, f32),
    pub scale: f32,
}

/// Option D clips (VISUAL-SPEC.md §2.4): a body of the lane colour at
/// 22% → 10% (the lane grid shows through), a 16 px header in the full lane
/// colour with Inter 600 11 #111111 text, a 1 px border in the lane colour
/// at 50%, radius 4. Hover and selection brighten the tint and the border.
const BODY_TOP_A: f32 = 0.22;
const BODY_BOTTOM_A: f32 = 0.10;
const EDGE_A: f32 = 0.5;
const NAME_PX: f32 = 11.0;
const NAME_PAD: f32 = 8.0;

/// Shadow pass, drawn for all clips before any bodies: the mockup's clips
/// sit flat on the lanes; only selection (a glow in the lane colour) and a
/// drag (a lifted shadow) cast one.
pub fn draw_clip_shadow(canvas: &Canvas, p: &ClipPaint) {
    let s = p.scale;
    let rr = gfx::rrect(p.rect, RADIUS_CLIP * s);
    let a = p.anim;
    if a.drag > 0.01 {
        gfx::drop_shadow(canvas, &rr, 0.0, 8.0 * a.drag * s, 18.0 * a.drag * s, 0.0, Rgba::black(0.6 * a.drag));
    }
    if a.selected > 0.01 {
        gfx::drop_shadow(canvas, &rr, 0.0, 0.0, 12.0 * s, 1.0 * s, p.color.with_alpha(0.55 * a.selected));
    }
}

pub fn draw_clip_body(canvas: &Canvas, p: &ClipPaint) {
    let s = p.scale;
    let r = p.rect;
    let rr = gfx::rrect(r, RADIUS_CLIP * s);
    let a = p.anim;
    let hl = gfx::hairline(s);

    let dragging_alpha = 1.0 - 0.1 * a.drag;
    let layered = dragging_alpha < 0.999;
    if layered {
        canvas.save_layer_alpha_f(r.with_outset((2.0 * s, 2.0 * s)), dragging_alpha);
    }

    let lane = p.color.lighten(0.06 * a.hover + 0.04 * a.selected);
    let tint = 0.04 * a.hover + 0.06 * a.selected;
    canvas.draw_rrect(rr, &gfx::vgrad(r, lane.with_alpha(BODY_TOP_A + tint), lane.with_alpha(BODY_BOTTOM_A + tint)));

    // Header: 16 px inside the border, the full lane colour.
    let inner = r.with_inset((hl, hl));
    let header_h = (CLIP_HEADER_HEIGHT * s).min(inner.height());
    canvas.save();
    canvas.clip_rrect(gfx::rrect(inner, (RADIUS_CLIP * s - hl).max(0.0)), vg::ClipOp::Intersect, true);
    let header = vg::Rect::from_xywh(inner.left, inner.top, inner.width(), header_h);
    canvas.draw_rect(header, &gfx::fill(lane));

    // Mini note preview in the lane colour (kept from before: not in the mockup).
    if !p.notes.is_empty() && r.height() > header_h + 8.0 * s {
        let (lo, hi) = p.notes.iter().fold((u8::MAX, 0u8), |(lo, hi), n| (lo.min(n.pitch), hi.max(n.pitch)));
        let span = (hi.saturating_sub(lo)).max(6) as f32;
        let area_top = inner.top + header_h + 4.0 * s;
        let area_bot = r.bottom - 4.0 * s;
        let bar_h = (2.0 * s).max(((area_bot - area_top) / (span + 1.0)).min(3.0 * s));
        let paint = gfx::fill(lane.lighten(0.12).with_alpha(0.92));
        for n in p.notes {
            let x0 = r.left + n.start as f32 * p.px_per_tick;
            let x1 = r.left + (n.start + n.len).min(p.len_ticks) as f32 * p.px_per_tick;
            if x1 < p.visible.0 || x0 > p.visible.1 || x0 >= r.right {
                continue;
            }
            let t = (n.pitch - lo) as f32 / span;
            let y = area_bot - bar_h - t * (area_bot - area_top - bar_h);
            let w = (x1 - x0 - 1.0 * s).max(1.0 * s);
            canvas.draw_rrect(gfx::rrect(vg::Rect::from_xywh(x0 + 0.5 * s, y, w, bar_h), 1.0 * s), &paint);
        }
    }

    // Name: Inter 600 11, #111111, 8 px in, centred in the header.
    if r.width() > 16.0 * s {
        let f = gfx::font(Face::SemiBold, NAME_PX * s);
        let base_y = gfx::css_baseline(&f, inner.top, header_h).round();
        let x = (inner.left.max(p.visible.0) + NAME_PAD * s).round();
        let text = match p.inst {
            Some(i) => format!("{}  ·  {i}", p.name),
            None => p.name.to_string(),
        };
        gfx::draw_text_clipped(canvas, &text, x, base_y, r.right - x - 4.0 * s, &f, CLIP_INK);
    }
    canvas.restore();

    // The 1 px border in the lane colour; selection brightens it.
    let sel = a.selected;
    gfx::ring(canvas, &rr, hl, &gfx::fill(lane.with_alpha(EDGE_A + 0.15 * a.hover + 0.45 * sel)));

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
