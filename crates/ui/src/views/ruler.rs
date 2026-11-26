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

/// Loop brace: 4 px along the ruler's top (VISUAL-SPEC.md §2.3); it can be
/// grabbed a little below that, and at its ends (logical px).
const BRACE_TOP: f32 = 0.0;
const BRACE_H: f32 = 4.0;
const BRACE_GRAB_H: f32 = 8.0;
const BRACE_EDGE: f32 = 4.0;
/// The brace's gradient while looping (top → bottom) and its glow; neutral
/// grey while the loop is off.
const BRACE_ON: [Rgba; 2] = [Rgba::hex(0xD3E05E), Rgba::hex(0xA9B83A)];
const BRACE_OFF: [Rgba; 2] = [Rgba::hex(0x5A5A5A), Rgba::hex(0x404040)];
const BRACE_GLOW_A: f32 = 0.45;
const BRACE_GLOW_R: f32 = 6.0;

/// The playhead marker: the mockup's 12 × 18 pentagon-with-stem, its top
/// 10 px below the ruler's top (logical px).
const HANDLE_W: f32 = 12.0;
const HANDLE_TOP: f32 = 10.0;
/// Padding around the handle image (its 1 px stroke).
const HANDLE_PAD: f32 = 2.0;

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
        if !(BRACE_TOP - 2.0..=BRACE_TOP + BRACE_GRAB_H).contains(&y) {
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
        let (view, snap) = self.s.arrangement.with_untracked(|a| (a.view, a.grid()));
        let d = view.px_to_ticks((x - x0) as f64);
        let q = |t: f64| -> f64 {
            if crate::input::bypass_snap(cx.modifiers()) {
                t.max(0.0)
            } else {
                snap.round(t as i64).max(0) as f64
            }
        };
        let g = snap.ticks as f64;
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
        let (view, snap) = self.s.arrangement.with_untracked(|a| (a.view, a.grid()));
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
                    // One brace drag is one undo step (F-33).
                    cx.emit(AppEvent::GestureBegin);
                } else {
                    self.scrubbing = true;
                    self.set_from_mouse(cx);
                }
                cx.capture();
                meta.consume();
            }
            WindowEvent::MouseDoubleClick(MouseButton::Left) if self.brace_at(cx).is_some() => {
                // Double-click the brace to switch the loop on/off.
                cx.emit(AppEvent::ToggleLoop);
                meta.consume();
            }
            // A14, R32: a double-click on the ruler zooms to the time
            // selection, or with none to the whole song.
            WindowEvent::MouseDoubleClick(MouseButton::Left) => {
                cx.emit(AppEvent::Run(crate::shortcuts::Action::ZoomToFit));
                meta.consume();
            }
            // A20: Set Loop to Selection, Zoom to Fit.
            WindowEvent::MouseDown(MouseButton::Right) => {
                let s = cx.scale_factor();
                cx.emit(AppEvent::ContextMenu(crate::edit::MenuSite::Ruler, cx.mouse().cursor_x / s, cx.mouse().cursor_y / s));
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
                if self.brace.take().is_some() {
                    cx.emit(AppEvent::GestureEnd);
                }
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
                // The marker's centre sits on the playhead line's centre.
                let pad = HANDLE_PAD * s;
                let left = (px + 0.5 * gfx::hairline(s) - HANDLE_W * 0.5 * s - pad).round();
                let top = (rect.top + HANDLE_TOP * s - pad).round();
                canvas.draw_image(img, (left, top), None);
            }
        }
        canvas.restore();
    }
}

/// Bar numbers: Inter 500 11, 4 px after the bar line, baseline 18
/// (logical px); the TRACKS cell shares the baseline.
const LABEL_PX: f32 = 11.0;
pub const LABEL_BASELINE: f32 = 18.0;
/// Tick heights from the bottom edge (logical px): beats, sixteenths.
const TICK_BEAT: f32 = 8.0;
const TICK_SIXTEENTH: f32 = 4.0;
/// Narrowest sixteenth (logical px) that still gets a tick.
const SIXTEENTH_MIN_PX: f32 = 5.0;

/// Background, loop brace, bar lines, ticks, labels and the bottom hairline.
fn draw_static(
    canvas: &Canvas,
    rect: vg::Rect,
    s: f32,
    view: &Viewport,
    (loop_start, loop_end): (f64, f64),
    loop_on: bool,
) {
    canvas.draw_rect(rect, &gfx::fill(RULER_BG));
    let hl = gfx::hairline(s);

    let ppb = view.px_per_bar();
    let step = label_step(ppb);
    let first_bar = (view.scroll_ticks / TICKS_PER_BAR as f64).floor() as i64;
    let bars_visible = (rect.width() as f64 / s as f64 / ppb).ceil() as i64 + 2;
    let f = gfx::font(Face::Medium, LABEL_PX * s);
    let major_ink = gfx::fill(RULER_TICK);
    let minor_ink = gfx::fill(RULER_TICK_MINOR);
    let ticks_bottom = rect.bottom - hl;
    let six_px = ppb as f32 / 16.0;
    for bar in first_bar.max(0)..first_bar + bars_visible {
        let x = (rect.left + view.tick_to_x((bar as u32 * TICKS_PER_BAR) as f64) as f32 * s).round();
        let major = (bar as u32).is_multiple_of(step);
        if major {
            // A labelled bar: a line the ruler's full height, and its number.
            canvas.draw_rect(vg::Rect::from_ltrb(x, rect.top, x + hl, ticks_bottom), &major_ink);
            let label = format!("{}", bar + 1);
            gfx::draw_text(canvas, &label, (x + 4.0 * s).round(), (rect.top + LABEL_BASELINE * s).round(), &f, TEXT_LO);
        } else {
            canvas.draw_rect(vg::Rect::from_ltrb(x, ticks_bottom - TICK_BEAT * s, x + hl, ticks_bottom), &major_ink);
        }
        // Beat ticks (8 px) and sixteenth ticks (4 px) when there is room.
        if ppb >= 60.0 {
            for k in 1..16u32 {
                let beat = k % 4 == 0;
                if !beat && six_px < SIXTEENTH_MIN_PX {
                    continue;
                }
                let tx = (x + k as f32 * six_px * s).round();
                let h = if beat { TICK_BEAT } else { TICK_SIXTEENTH } * s;
                canvas.draw_rect(vg::Rect::from_ltrb(tx, ticks_bottom - h, tx + hl, ticks_bottom), if beat { &major_ink } else { &minor_ink });
            }
        }
    }

    // The loop brace over the loop range: lime while looping (with a soft
    // glow), neutral grey while off. Drag the body to move it, the ends to
    // resize it; a double-click switches the loop.
    let lx0 = (rect.left + view.tick_to_x(loop_start) as f32 * s).round();
    let lx1 = (rect.left + view.tick_to_x(loop_end) as f32 * s).round();
    let brace = vg::Rect::from_ltrb(lx0, rect.top + BRACE_TOP * s, lx1, rect.top + (BRACE_TOP + BRACE_H) * s);
    if brace.right > rect.left && brace.left < rect.right {
        let [top, bottom] = if loop_on { BRACE_ON } else { BRACE_OFF };
        if loop_on {
            let mut g = gfx::fill(ACCENT.with_alpha(BRACE_GLOW_A));
            g.set_mask_filter(vg::MaskFilter::blur(vg::BlurStyle::Normal, BRACE_GLOW_R * 0.5 * s, false));
            canvas.draw_rect(brace, &g);
        }
        canvas.draw_rect(brace, &gfx::vgrad(brace, top, bottom));
    }

    // The one hairline where the ruler meets the tracks.
    canvas.draw_rect(vg::Rect::from_ltrb(rect.left, rect.bottom - hl, rect.right, rect.bottom), &gfx::fill(LINE_SOFT));
}

/// The playhead marker: a pentagon (`#E6E6E6`, 1 px white stroke) with a
/// 1 px stem down to the ruler's bottom, rendered once per scale.
fn render_handle(s: f32) -> Option<vg::Image> {
    let pad = HANDLE_PAD * s;
    let stem_end = (RULER_HEIGHT - HANDLE_TOP) * s;
    let iw = (HANDLE_W * s + 2.0 * pad).ceil() as i32;
    let ih = (stem_end + 2.0 * pad).ceil() as i32;
    let mut surface = vg::surfaces::raster_n32_premul((iw, ih))?;
    let c = surface.canvas();
    c.clear(vg::Color::TRANSPARENT);
    let o = (pad, pad);
    let body = gfx::svg_path("M1 0.5H11V7.5L6 13L1 7.5Z", o, s);
    c.draw_path(&body, &gfx::fill(NEUTRAL_HI));
    let mut edge = gfx::stroke(Rgba::white(1.0), s);
    edge.set_stroke_join(vg::PaintJoin::Round);
    c.draw_path(&body, &edge);
    // The stem: x 5.5..6.5 of the marker, from its tip to the ruler's bottom.
    c.draw_rect(vg::Rect::from_ltrb(pad + 5.5 * s, pad + 13.0 * s, pad + 6.5 * s, pad + stem_end), &gfx::fill(NEUTRAL_HI));
    Some(surface.image_snapshot())
}
