//! Procedurally drawn transport glyphs (no icon fonts / SVG).

use vizia::prelude::*;
use vizia::vg;

use super::gfx;
use crate::theme::tokens::*;
use crate::Signals;

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum GlyphKind {
    Play,
    Stop,
    Record,
}

pub struct TransportGlyph {
    kind: GlyphKind,
    s: Signals,
}

impl TransportGlyph {
    pub fn new(cx: &mut Context, kind: GlyphKind, s: Signals) -> Handle<'_, Self> {
        Self { kind, s }
            .build(cx, |_| {})
            .size(Pixels(16.0))
            .hoverable(false)
            // The transport signal changes every frame while playing (playhead);
            // only redraw when what the glyph shows changes.
            .bind(s.transport, {
                let shown = std::cell::Cell::new(None);
                move |mut h| {
                    let t = s.transport.get_untracked();
                    let state = Some((t.playing, t.record_armed));
                    if shown.replace(state) != state {
                        h.needs_redraw();
                    }
                }
            })
    }
}

impl View for TransportGlyph {
    fn element(&self) -> Option<&'static str> {
        Some("transport-glyph")
    }

    fn draw(&self, cx: &mut DrawContext, canvas: &Canvas) {
        let _perf = crate::perf::scope(crate::perf::Span::Glyphs);
        let b = cx.bounds();
        let s = cx.scale_factor();
        let (x0, y0) = (b.x + b.w * 0.5, b.y + b.h * 0.5);
        let t = self.s.transport.get_untracked();
        match self.kind {
            GlyphKind::Play => {
                let col = if t.playing {
                    Rgba::hex(0x7CF29C)
                } else {
                    TEXT_HI
                };
                let r = 6.0 * s;
                let mut pb = vg::PathBuilder::new();
                pb.move_to((x0 - r * 0.6, y0 - r));
                pb.line_to((x0 + r * 0.95, y0));
                pb.line_to((x0 - r * 0.6, y0 + r));
                pb.close();
                let path = pb.detach();
                if t.playing {
                    let mut glow = gfx::fill(col.with_alpha(0.55));
                    glow.set_mask_filter(vg::MaskFilter::blur(
                        vg::BlurStyle::Normal,
                        3.0 * s,
                        false,
                    ));
                    canvas.draw_path(&path, &glow);
                }
                let mut p = gfx::fill(col);
                p.set_path_effect(vg::PathEffect::corner_path(1.5 * s));
                canvas.draw_path(&path, &p);
            }
            GlyphKind::Stop => {
                let r = 5.0 * s;
                let rr = gfx::rrect(
                    vg::Rect::from_xywh(x0 - r, y0 - r, r * 2.0, r * 2.0),
                    1.5 * s,
                );
                canvas.draw_rrect(rr, &gfx::fill(TEXT_HI));
            }
            GlyphKind::Record => {
                let r = 5.5 * s;
                let col = if t.record_armed {
                    RECORD
                } else {
                    RECORD.mix(TEXT_LO, 0.35)
                };
                if t.record_armed {
                    let mut glow = gfx::fill(RECORD.with_alpha(0.7));
                    glow.set_mask_filter(vg::MaskFilter::blur(
                        vg::BlurStyle::Normal,
                        4.0 * s,
                        false,
                    ));
                    canvas.draw_circle((x0, y0), r * 1.1, &glow);
                }
                let mut p = vg::Paint::default();
                p.set_anti_alias(true);
                p.set_shader(gfx::radial(
                    (x0 - r * 0.35, y0 - r * 0.4),
                    r * 1.6,
                    &[col.lighten(0.35), col, col.darken(0.25)],
                    Some(&[0.0, 0.5, 1.0]),
                ));
                canvas.draw_circle((x0, y0), r, &p);
            }
        }
    }
}
