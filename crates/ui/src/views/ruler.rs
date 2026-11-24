//! Bar/beat ruler with a draggable playhead handle.

use vizia::prelude::*;
use vizia::vg;

use super::gfx::{self, Face};
use crate::state::*;
use crate::theme::tokens::*;
use crate::{AppEvent, Signals};

pub struct Ruler {
    s: Signals,
    scrubbing: bool,
    /// Retained background, ticks and labels; re-rendered when the view changes.
    layer: std::cell::RefCell<Option<(u64, vg::Image)>>,
    /// Pre-rendered playhead handle (glow + shape) and its scale factor.
    handle: std::cell::RefCell<Option<(f32, vg::Image)>>,
}

/// Handle geometry (logical px): half width and height.
const HANDLE_W: f32 = 6.0;
const HANDLE_H: f32 = 12.0;
/// Padding around the handle image for its glow (logical px).
const HANDLE_PAD: f32 = 8.0;

impl Ruler {
    pub fn new(cx: &mut Context, s: Signals) -> Handle<'_, Self> {
        Self {
            s,
            scrubbing: false,
            layer: Default::default(),
            handle: Default::default(),
        }
        .build(cx, |_| {})
        .cursor(CursorIcon::Hand)
        .bind(s.transport, |mut h| h.needs_redraw())
        .bind(s.arrangement, |mut h| h.needs_redraw())
    }

    fn set_from_mouse(&self, cx: &mut EventContext) {
        let b = cx.bounds();
        let s = cx.scale_factor();
        let x = ((cx.mouse().cursor_x - b.x) / s) as f64;
        let (view, snap) = self.s.arrangement.with_untracked(|a| (a.view, a.snap));
        let mut t = view.x_to_tick(x).max(0.0);
        if !cx.modifiers().alt() {
            t = snap.round(t as i64).max(0) as f64;
        }
        cx.emit(AppEvent::SetPlayhead(t));
    }
}

/// Picks a bar-label step so labels never collide.
pub fn label_step(px_per_bar: f64) -> u32 {
    [1u32, 2, 4, 8, 16]
        .into_iter()
        .find(|n| px_per_bar * *n as f64 >= 44.0)
        .unwrap_or(32)
}

impl View for Ruler {
    fn element(&self) -> Option<&'static str> {
        Some("ruler")
    }

    fn event(&mut self, cx: &mut EventContext, event: &mut Event) {
        event.map(|we, meta| match we {
            WindowEvent::MouseDown(MouseButton::Left) => {
                self.scrubbing = true;
                cx.capture();
                self.set_from_mouse(cx);
                meta.consume();
            }
            WindowEvent::MouseMove(_, _) if self.scrubbing => self.set_from_mouse(cx),
            WindowEvent::MouseUp(MouseButton::Left) if self.scrubbing => {
                self.scrubbing = false;
                cx.release();
            }
            _ => {}
        });
    }

    fn draw(&self, cx: &mut DrawContext, canvas: &Canvas) {
        let _perf = crate::perf::scope(crate::perf::Span::Ruler);
        let b = cx.bounds();
        let s = cx.scale_factor();
        let rect: vg::Rect = b.into();
        let (view, loop_end) = (
            self.s.arrangement.with_untracked(|a| a.view),
            self.s.transport.with_untracked(|t| t.loop_end_ticks),
        );
        let ph = self.s.transport.with_untracked(|t| t.playhead_ticks);

        canvas.save();
        canvas.clip_rect(rect, None, None);

        // Static part, retained until the view, loop or bounds change.
        let key = {
            use std::hash::{Hash, Hasher};
            let mut h = std::collections::hash_map::DefaultHasher::new();
            for v in [rect.left, rect.top, rect.width(), rect.height(), s] {
                v.to_bits().hash(&mut h);
            }
            view.px_per_tick.to_bits().hash(&mut h);
            view.scroll_ticks.to_bits().hash(&mut h);
            loop_end.to_bits().hash(&mut h);
            h.finish()
        };
        let mut layer = self.layer.borrow_mut();
        if layer.as_ref().is_none_or(|(k, _)| *k != key) {
            let (w, h) = (rect.width().ceil() as i32, rect.height().ceil() as i32);
            *layer = canvas
                .new_surface(&vg::ImageInfo::new_n32_premul((w, h), None), None)
                .map(|mut surface| {
                    let lc = surface.canvas();
                    lc.translate((-rect.left, -rect.top));
                    draw_static(lc, rect, s, &view, loop_end);
                    (key, surface.image_snapshot())
                });
        }
        match layer.as_ref() {
            Some((_, img)) => {
                canvas.draw_image(img, (rect.left, rect.top), None);
            }
            None => draw_static(canvas, rect, s, &view, loop_end),
        }
        drop(layer);

        // Playhead handle, blitted from a pre-rendered image.
        let px = (rect.left + view.tick_to_x(ph) as f32 * s).round();
        if px >= rect.left - 10.0 * s && px <= rect.right + 10.0 * s {
            let mut handle = self.handle.borrow_mut();
            if handle.as_ref().is_none_or(|(hs, _)| *hs != s) {
                *handle = render_handle(s).map(|img| (s, img));
            }
            if let Some((_, img)) = handle.as_ref() {
                let pad = HANDLE_PAD * s;
                let left = px - HANDLE_W * s - pad;
                let top = rect.bottom - HANDLE_H * s - pad;
                canvas.draw_image(img, (left, top), None);
            }
        }
        canvas.restore();
    }
}

/// Background, loop band, bar/beat ticks, labels and bottom divider.
fn draw_static(canvas: &Canvas, rect: vg::Rect, s: f32, view: &Viewport, loop_end: f64) {
    canvas.draw_rect(
        rect,
        &gfx::vgrad(rect, Rgba::hex(0x1B1E26), Rgba::hex(0x15171D)),
    );

    // Loop region band.
    let lx0 = rect.left + view.tick_to_x(0.0) as f32 * s;
    let lx1 = rect.left + view.tick_to_x(loop_end) as f32 * s;
    let band = vg::Rect::from_ltrb(
        lx0.max(rect.left),
        rect.top + 3.0 * s,
        lx1.min(rect.right),
        rect.top + 7.0 * s,
    );
    if band.width() > 0.0 {
        canvas.draw_rrect(
            gfx::rrect(band, 2.0 * s),
            &gfx::fill(ACCENT.with_alpha(0.35)),
        );
    }

    let ppb = view.px_per_bar();
    let step = label_step(ppb);
    let first_bar = (view.scroll_ticks / TICKS_PER_BAR as f64).floor() as i64;
    let bars_visible = (rect.width() as f64 / s as f64 / ppb).ceil() as i64 + 2;
    let f = gfx::font(Face::Mono, 10.0 * s);
    let tick = gfx::fill(TEXT_LO.with_alpha(0.5));
    let tick_minor = gfx::fill(TEXT_LO.with_alpha(0.25));
    for bar in first_bar.max(0)..first_bar + bars_visible {
        let x =
            (rect.left + view.tick_to_x((bar as u32 * TICKS_PER_BAR) as f64) as f32 * s).round();
        let major = (bar as u32).is_multiple_of(step);
        let h = if major { 10.0 } else { 6.0 } * s;
        canvas.draw_rect(
            vg::Rect::from_xywh(x, rect.bottom - h, s, h),
            if major { &tick } else { &tick_minor },
        );
        if major {
            let label = format!("{}", bar + 1);
            gfx::draw_text(
                canvas,
                &label,
                x + 4.0 * s,
                rect.top + 19.0 * s,
                &f,
                TEXT_LO,
            );
        }
        // Beat ticks when there is room.
        if ppb >= 60.0 {
            for beat in 1..BEATS_PER_BAR {
                let bx = (x + (beat * PPQ) as f32 * view.px_per_tick as f32 * s).round();
                canvas.draw_rect(
                    vg::Rect::from_xywh(bx, rect.bottom - 3.0 * s, s, 3.0 * s),
                    &tick_minor,
                );
            }
        }
    }

    // Bottom divider.
    canvas.draw_rect(
        vg::Rect::from_xywh(rect.left, rect.bottom - s, rect.width(), s),
        &gfx::fill(Rgba::black(0.5)),
    );
}

/// The playhead handle (soft glow + rounded pentagon) rendered once per scale.
fn render_handle(s: f32) -> Option<vg::Image> {
    let pad = HANDLE_PAD * s;
    let (w, h) = (HANDLE_W * s, HANDLE_H * s);
    let iw = (2.0 * (w + pad)).ceil() as i32;
    let ih = (h + 2.0 * pad).ceil() as i32;
    let mut surface = vg::surfaces::raster_n32_premul((iw, ih))?;
    let c = surface.canvas();
    c.clear(vg::Color::TRANSPARENT);
    // Tip of the handle at (cx, bottom).
    let (cx, bottom) = (w + pad, h + pad);
    let mut pb = vg::PathBuilder::new();
    pb.move_to((cx - w, bottom - h));
    pb.line_to((cx + w, bottom - h));
    pb.line_to((cx + w, bottom - 6.0 * s));
    pb.line_to((cx, bottom));
    pb.line_to((cx - w, bottom - 6.0 * s));
    pb.close();
    let path = pb.detach();
    let mut glow = gfx::fill(ACCENT.with_alpha(0.6));
    glow.set_mask_filter(vg::MaskFilter::blur(vg::BlurStyle::Normal, 3.0 * s, false));
    c.draw_path(&path, &glow);
    let mut p = gfx::fill(Rgba::hex(0xD9D0FF));
    p.set_path_effect(vg::PathEffect::corner_path(1.5 * s));
    c.draw_path(&path, &p);
    Some(surface.image_snapshot())
}
