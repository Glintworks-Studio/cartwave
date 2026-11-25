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
    /// Loop brace drag: what's grabbed, press x, and the loop at press.
    brace: Option<(BracePart, f32, f64, f64)>,
    /// Retained background, ticks and labels; re-rendered when the view changes.
    layer: std::cell::RefCell<Option<(u64, vg::Image)>>,
    /// Pre-rendered playhead handle (glow + shape) and its scale factor.
    handle: std::cell::RefCell<Option<(f32, vg::Image)>>,
}

/// Which part of the loop brace a drag holds.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum BracePart {
    Start,
    End,
    Body,
}

/// Loop brace: top and height in the ruler, and the grab zone at its ends
/// (logical px).
const BRACE_TOP: f32 = 4.0;
const BRACE_H: f32 = 8.0;
const BRACE_EDGE: f32 = 4.0;

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
            brace: None,
            layer: Default::default(),
            handle: Default::default(),
        }
        .build(cx, |_| {})
        .bind(s.transport, |mut h| h.needs_redraw())
        .bind(s.arrangement, |mut h| h.needs_redraw())
        .bind(s.loop_on, |mut h| h.needs_redraw())
    }

    /// The part of the loop brace under the mouse, if any.
    fn brace_at(&self, cx: &EventContext) -> Option<BracePart> {
        let b = cx.bounds();
        let s = cx.scale_factor();
        let (x, y) = ((cx.mouse().cursor_x - b.x) / s, (cx.mouse().cursor_y - b.y) / s);
        if !(BRACE_TOP - 2.0..=BRACE_TOP + BRACE_H + 2.0).contains(&y) {
            return None;
        }
        let (view, (ls, le)) = (
            self.s.arrangement.with_untracked(|a| a.view),
            self.s.transport.with_untracked(|t| (t.loop_start_ticks, t.loop_end_ticks)),
        );
        let (x0, x1) = (view.tick_to_x(ls) as f32, view.tick_to_x(le) as f32);
        if (x - x0).abs() <= BRACE_EDGE {
            Some(BracePart::Start)
        } else if (x - x1).abs() <= BRACE_EDGE {
            Some(BracePart::End)
        } else if x > x0 && x < x1 {
            Some(BracePart::Body)
        } else {
            None
        }
    }

    fn drag_brace(&self, cx: &mut EventContext) {
        let Some((part, x0, s0, e0)) = self.brace else { return };
        let b = cx.bounds();
        let x = (cx.mouse().cursor_x - b.x) / cx.scale_factor();
        let (view, snap) = self.s.arrangement.with_untracked(|a| (a.view, a.snap));
        let d = view.px_to_ticks((x - x0) as f64);
        let q = |t: f64| -> f64 {
            if crate::input::bypass_snap(cx.modifiers()) {
                t.max(0.0)
            } else {
                snap.round(t as i64).max(0) as f64
            }
        };
        let g = snap.ticks() as f64;
        let (start, end) = match part {
            BracePart::Body => {
                let ns = q(s0 + d);
                (ns, ns + (e0 - s0))
            }
            BracePart::Start => (q(s0 + d).min(e0 - g), e0),
            BracePart::End => (s0, q(e0 + d).max(s0 + g)),
        };
        cx.emit(AppEvent::SetLoop(start, end));
    }

    fn set_from_mouse(&self, cx: &mut EventContext) {
        let b = cx.bounds();
        let s = cx.scale_factor();
        let x = ((cx.mouse().cursor_x - b.x) / s) as f64;
        let (view, snap) = self.s.arrangement.with_untracked(|a| (a.view, a.snap));
        let mut t = view.x_to_tick(x).max(0.0);
        if !crate::input::bypass_snap(cx.modifiers()) {
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
                if let Some(part) = self.brace_at(cx) {
                    let b = cx.bounds();
                    let x = (cx.mouse().cursor_x - b.x) / cx.scale_factor();
                    let (ls, le) = self
                        .s
                        .transport
                        .with_untracked(|t| (t.loop_start_ticks, t.loop_end_ticks));
                    self.brace = Some((part, x, ls, le));
                } else {
                    self.scrubbing = true;
                    self.set_from_mouse(cx);
                }
                cx.capture();
                meta.consume();
            }
            WindowEvent::MouseDoubleClick(MouseButton::Left) if self.brace_at(cx).is_some() => {
                // Ableton: double-click the brace to switch the loop on/off.
                cx.emit(AppEvent::ToggleLoop);
                meta.consume();
            }
            WindowEvent::MouseMove(_, _) if self.scrubbing => self.set_from_mouse(cx),
            WindowEvent::MouseMove(_, _) if self.brace.is_some() => self.drag_brace(cx),
            WindowEvent::MouseMove(_, _) => {
                let icon = match self.brace_at(cx) {
                    Some(BracePart::Start | BracePart::End) => CursorIcon::EwResize,
                    Some(BracePart::Body) => CursorIcon::Grab,
                    None => CursorIcon::Hand,
                };
                cx.emit(WindowEvent::SetCursor(icon));
            }
            WindowEvent::MouseLeave if self.brace.is_none() && !self.scrubbing => {
                cx.emit(WindowEvent::SetCursor(CursorIcon::Default));
            }
            WindowEvent::MouseUp(MouseButton::Left) if self.scrubbing || self.brace.is_some() => {
                self.scrubbing = false;
                self.brace = None;
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
        let (view, loop_end, loop_start) = (
            self.s.arrangement.with_untracked(|a| a.view),
            self.s.transport.with_untracked(|t| t.loop_end_ticks),
            self.s.transport.with_untracked(|t| t.loop_start_ticks),
        );
        let ph = self.s.transport.with_untracked(|t| t.playhead_ticks);
        let loop_on = self.s.loop_on.get_untracked();

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
            loop_start.to_bits().hash(&mut h);
            loop_on.hash(&mut h);
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
                    draw_static(lc, rect, s, &view, (loop_start, loop_end), loop_on);
                    (key, surface.image_snapshot())
                });
        }
        match layer.as_ref() {
            Some((_, img)) => {
                canvas.draw_image(img, (rect.left, rect.top), None);
            }
            None => draw_static(canvas, rect, s, &view, (loop_start, loop_end), loop_on),
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

/// Bar label size and baseline (logical px).
const LABEL_PX: f32 = 10.0;
const LABEL_BASELINE: f32 = 20.0;
/// Tick heights from the bottom edge (logical px).
const TICK_MAJOR: f32 = 8.0;
const TICK_MINOR: f32 = 4.0;

/// Background, loop band, bar/beat ticks, labels and bottom divider.
fn draw_static(
    canvas: &Canvas,
    rect: vg::Rect,
    s: f32,
    view: &Viewport,
    (loop_start, loop_end): (f64, f64),
    loop_on: bool,
) {
    // The band: same gradient and top hairline as the cell beside it.
    canvas.draw_rect(rect, &gfx::vgrad(rect, BAND_TOP, BAND_BOTTOM));
    canvas.draw_rect(vg::Rect::from_xywh(rect.left, rect.top, rect.width(), s), &gfx::fill(LINE));

    // Loop brace (Ableton): a bar over the loop range; drag the body to
    // move it, the ends to resize. Lit while the loop light is on.
    let lx0 = (rect.left + view.tick_to_x(loop_start) as f32 * s).round();
    let lx1 = (rect.left + view.tick_to_x(loop_end) as f32 * s).round();
    let brace = vg::Rect::from_ltrb(lx0, rect.top + BRACE_TOP * s, lx1, rect.top + (BRACE_TOP + BRACE_H) * s);
    if brace.right > rect.left && brace.left < rect.right {
        let rr = gfx::rrect(brace, 2.0 * s);
        let hl = gfx::hairline(s);
        if loop_on {
            canvas.draw_rrect(rr, &gfx::vgrad(brace, ACCENT.with_alpha(0.45), ACCENT.with_alpha(0.25)));
            gfx::ring_vgrad(canvas, &rr, hl, ACCENT.lighten(0.2), ACCENT.darken(0.3).with_alpha(0.6));
        } else {
            canvas.draw_rrect(rr, &gfx::fill(Rgba::white(0.06)));
            gfx::ring(canvas, &rr, hl, &gfx::fill(Rgba::white(0.12)));
        }
    }

    let ppb = view.px_per_bar();
    let step = label_step(ppb);
    let first_bar = (view.scroll_ticks / TICKS_PER_BAR as f64).floor() as i64;
    let bars_visible = (rect.width() as f64 / s as f64 / ppb).ceil() as i64 + 2;
    let f = gfx::font(Face::Medium, LABEL_PX * s);
    let tick = gfx::fill(Rgba::hex(0x3A3A3A));
    let tick_minor = gfx::fill(Rgba::hex(0x262626));
    for bar in first_bar.max(0)..first_bar + bars_visible {
        let x =
            (rect.left + view.tick_to_x((bar as u32 * TICKS_PER_BAR) as f64) as f32 * s).round();
        let major = (bar as u32).is_multiple_of(step);
        let h = if major { TICK_MAJOR } else { TICK_MINOR } * s;
        canvas.draw_rect(
            vg::Rect::from_xywh(x, rect.bottom - s - h, s, h),
            if major { &tick } else { &tick_minor },
        );
        if major {
            let label = format!("{}", bar + 1);
            gfx::draw_text(
                canvas,
                &label,
                (x + 4.0 * s).round(),
                (rect.top + LABEL_BASELINE * s).round(),
                &f,
                TEXT_LO,
            );
        }
        // Beat ticks when there is room.
        if ppb >= 60.0 {
            for beat in 1..BEATS_PER_BAR {
                let bx = (x + (beat * PPQ) as f32 * view.px_per_tick as f32 * s).round();
                canvas.draw_rect(
                    vg::Rect::from_xywh(bx, rect.bottom - s - TICK_MINOR * s * 0.5, s, TICK_MINOR * s * 0.5),
                    &tick_minor,
                );
            }
        }
    }

    // Bottom edge: the dark line above the first track row's hairline.
    canvas.draw_rect(
        vg::Rect::from_xywh(rect.left, rect.bottom - s, rect.width(), s),
        &gfx::fill(Rgba::hex(0x090909)),
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
    let mut glow = gfx::fill(ACCENT.with_alpha(0.5));
    glow.set_mask_filter(vg::MaskFilter::blur(vg::BlurStyle::Normal, 3.0 * s, false));
    c.draw_path(&path, &glow);
    let mut p = gfx::vgrad(
        vg::Rect::from_xywh(cx - w, bottom - h, 2.0 * w, h),
        ACCENT.lighten(0.25),
        ACCENT.darken(0.1),
    );
    p.set_path_effect(vg::PathEffect::corner_path(1.5 * s));
    c.draw_path(&path, &p);
    Some(surface.image_snapshot())
}
