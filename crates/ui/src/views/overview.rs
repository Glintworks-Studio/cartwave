//! The Overview (polish 2 §7.3; board `E1-overview`): a 32 px strip above the
//! ruler, aligned with the lanes, showing the whole arrangement (one row per
//! track, or several tracks to a row when there are many), the visible part
//! as a neutral outline, the loop (only while it's on) and the playhead.
//!
//! Gestures share the ruler's navigation drag (`nav`): drag left or right to
//! scroll, up or down to zoom anchored where you grabbed; a click outside the
//! outline jumps there; a double-click zooms as on the ruler (P2-R: the
//! manual's Arrangement Overview, Live 12 §6.1).
//!
//! The clips are drawn into a cached image, redrawn only when the clips,
//! their colours, the track count, the song's length, the width or the
//! scale change; scrolling only moves the outline.

use std::cell::RefCell;

use vizia::prelude::*;
use vizia::vg;

use super::gfx::{self, Face};
use crate::state::{Viewport, TICKS_PER_BAR};
use crate::theme::tokens::*;
use crate::{AppEvent, Signals};

/// The label cell's text baseline (logical px from the top): the 11 px
/// label's line box centred in the 32 px band.
pub const LABEL_BASELINE: f32 = 20.0;
/// Rows: from y 3, 6 px tall, 1 px apart, at least 1 px; 2 px free at the
/// bottom (over the hairline).
const ROWS_TOP: f32 = 3.0;
const ROWS_BOTTOM_PAD: f32 = 2.0;
const ROW_MAX: f32 = 6.0;
const ROW_GAP: f32 = 1.0;
/// The visible part: 1 px `#8C8C8C`, radius 2, white 4% with an 8% inner top
/// highlight, from y 2 to 1 px above the bottom. Not lime.
const OUTLINE: Rgba = Rgba::hex(0x8C8C8C);
const OUTLINE_TOP: f32 = 2.0;
const OUTLINE_BOTTOM: f32 = 1.0;
const OUTLINE_RADIUS: f32 = 2.0;
/// The loop: a 2 px lime line at 55% (only while looping, an "on" state);
/// the playhead 1 px `#E6E6E6` at 60%.
const LOOP_H: f32 = 2.0;
const LOOP_A: f32 = 0.55;
const PLAYHEAD_A: f32 = 0.6;
/// The strip's surface.
const STRIP_BG: Rgba = Rgba::hex(0x0A0A0A);

pub const HELP: &str = "Overview: drag to scroll · drag up or down to zoom · click outside the frame to jump there · double-click to fit";

/// The song's length in whole bars: the last clip or the loop end, at least
/// four bars (the overview's extent and its label).
pub fn song_bars(s: &Signals) -> u32 {
    let loop_end = s.transport.with_untracked(|t| t.loop_end_ticks) as u32;
    let end = s.arrangement.with_untracked(|a| a.song_end()).max(loop_end).max(4 * TICKS_PER_BAR);
    end.div_ceil(TICKS_PER_BAR)
}

/// How rows map to tracks: (row height, tracks per row, rows). Up to 6 px
/// rows, shrinking to 1 px; past that several tracks merge into a row, so
/// 50+ tracks still read as one strip.
pub fn row_plan(tracks: usize, band_h: f32) -> (f32, usize, usize) {
    let avail = band_h - ROWS_TOP - ROWS_BOTTOM_PAD;
    let n = tracks.max(1);
    let h = ((avail + ROW_GAP) / n as f32 - ROW_GAP).floor().min(ROW_MAX);
    if h >= 1.0 {
        return (h, 1, n);
    }
    let max_rows = ((avail + ROW_GAP) / (1.0 + ROW_GAP)).floor().max(1.0) as usize;
    let per = n.div_ceil(max_rows);
    (1.0, per, n.div_ceil(per))
}

// ───────────────────────────── Navigation drag ─────────────────────────────

/// A navigation drag on the ruler or the Overview: one code path (§7.3).
/// Vertical movement zooms (down: in), anchored at the grabbed tick;
/// horizontal movement scrolls. A press that moves less than 3 px is a click.
#[derive(Clone, Copy, Debug)]
pub struct NavDrag {
    /// The press (window logical px).
    pub press: (f32, f32),
    /// The tick grabbed and where it was in the arrangement (view px).
    pub anchor_tick: f64,
    pub anchor_x: f32,
    pub view0: Viewport,
    pub moved: bool,
}

/// Movement under this (logical px) is a click (§7.1).
pub const CLICK_SLOP: f32 = 3.0;
/// Zoom per px of vertical drag (exponential: smooth at every zoom).
pub const ZOOM_PER_PX: f64 = 0.012;

impl NavDrag {
    pub fn new(press: (f32, f32), view: Viewport, anchor_x: f32) -> Self {
        Self { press, anchor_tick: view.x_to_tick(anchor_x as f64), anchor_x, view0: view, moved: false }
    }

    /// Past the click threshold?
    pub fn update_moved(&mut self, now: (f32, f32)) -> bool {
        if !self.moved && ((now.0 - self.press.0).abs() >= CLICK_SLOP || (now.1 - self.press.1).abs() >= CLICK_SLOP) {
            self.moved = true;
        }
        self.moved
    }

    /// The view for a vertical movement `dy` (px, down = in) with the
    /// anchor tick shown at `anchor_x_now` (view px).
    pub fn view(&self, dy: f32, anchor_x_now: f32) -> Viewport {
        let factor = (dy as f64 * ZOOM_PER_PX).exp();
        let ppb = (self.view0.px_per_bar() * factor).clamp(Viewport::MIN_PX_PER_BAR, Viewport::MAX_PX_PER_BAR);
        let ppt = ppb / TICKS_PER_BAR as f64;
        Viewport { px_per_tick: ppt, scroll_ticks: (self.anchor_tick - anchor_x_now as f64 / ppt).max(0.0) }
    }
}

// ───────────────────────────── The strip ─────────────────────────────

pub struct Overview {
    s: Signals,
    drag: Option<(NavDrag, f64)>,
    cache: RefCell<Option<(u64, vg::Image)>>,
}

impl Overview {
    pub fn new(cx: &mut Context, s: Signals) -> Handle<'_, Self> {
        Self { s, drag: None, cache: RefCell::new(None) }
            .build(cx, |_| {})
            .height(Pixels(OVERVIEW_HEIGHT))
            .bind(s.arrangement, |mut h| h.needs_redraw())
            .bind(s.transport, |mut h| h.needs_redraw())
            .bind(s.loop_on, |mut h| h.needs_redraw())
    }

    /// Song ticks per overview px for a strip `w` px wide (logical).
    fn ticks_per_px(&self, w: f32) -> f64 {
        (song_bars(&self.s) * TICKS_PER_BAR) as f64 / w.max(1.0) as f64
    }

    fn local(cx: &EventContext) -> (f32, f32, f32) {
        let b = cx.bounds();
        let s = cx.scale_factor();
        ((cx.mouse().cursor_x - b.x) / s, (cx.mouse().cursor_y - b.y) / s, b.w / s)
    }
}

impl View for Overview {
    fn element(&self) -> Option<&'static str> {
        Some("overview")
    }

    fn event(&mut self, cx: &mut EventContext, event: &mut Event) {
        event.map(|we, meta| match we {
            WindowEvent::MouseDown(MouseButton::Left) => {
                let (x, y, w) = Self::local(cx);
                let tpx = self.ticks_per_px(w);
                let tick = x as f64 * tpx;
                let (view, vw) = self.s.arrangement.with_untracked(|a| (a.view, a.view_w));
                let (v0, v1) = (view.scroll_ticks, view.x_to_tick(vw as f64));
                // Outside the outline: jump there (the view centred on it).
                let view = if tick < v0 || tick > v1 {
                    let half = (v1 - v0) * 0.5;
                    let v = Viewport { scroll_ticks: (tick - half).max(0.0), ..view };
                    self.s.arrangement.update(|a| a.view = v);
                    v
                } else {
                    view
                };
                // The grabbed tick's place in the arrangement.
                let ax = view.tick_to_x(tick) as f32;
                let win = (cx.mouse().cursor_x / cx.scale_factor(), cx.mouse().cursor_y / cx.scale_factor());
                self.drag = Some((NavDrag::new(win, view, ax), tpx));
                let _ = y;
                cx.capture();
                meta.consume();
            }
            WindowEvent::MouseMove(_, _) => {
                let win = (cx.mouse().cursor_x / cx.scale_factor(), cx.mouse().cursor_y / cx.scale_factor());
                if let Some((d, tpx)) = self.drag.as_mut() {
                    if d.update_moved(win) {
                        // Horizontal: the visible part follows the mouse (its
                        // grabbed tick moves by the overview's scale); vertical: zoom.
                        let dx_ticks = (win.0 - d.press.0) as f64 * *tpx;
                        let (dy, d) = (win.1 - d.press.1, *d);
                        let mut v = d.view(dy, d.anchor_x);
                        v.scroll_ticks = (d.anchor_tick + dx_ticks - d.anchor_x as f64 / v.px_per_tick).max(0.0);
                        self.s.arrangement.update(|a| a.view = v);
                        cx.emit(WindowEvent::SetCursor(CursorIcon::AllScroll));
                    }
                } else {
                    cx.emit(AppEvent::HoverHelp(HELP.into()));
                    cx.emit(WindowEvent::SetCursor(CursorIcon::Hand));
                }
            }
            WindowEvent::MouseUp(MouseButton::Left) if self.drag.is_some() => {
                let (d, _) = self.drag.take().unwrap();
                cx.release();
                cx.emit(WindowEvent::SetCursor(CursorIcon::Hand));
                if crate::dev::enabled() {
                    let v = self.s.arrangement.with_untracked(|a| a.view);
                    eprintln!("dev: overview {} px_per_bar {:.2} scroll {:.0}", if d.moved { "drag" } else { "click" }, v.px_per_bar(), v.scroll_ticks);
                }
                meta.consume();
            }
            WindowEvent::MouseDoubleClick(MouseButton::Left) => {
                cx.emit(AppEvent::Run(crate::shortcuts::Action::ZoomToFit));
                meta.consume();
            }
            WindowEvent::MouseLeave if self.drag.is_none() => {
                cx.emit(WindowEvent::SetCursor(CursorIcon::Default));
                cx.emit(AppEvent::HoverHelp(String::new()));
            }
            _ => {}
        });
    }

    fn draw(&self, cx: &mut DrawContext, canvas: &Canvas) {
        let _perf = crate::perf::scope(crate::perf::Span::Ruler);
        let s = cx.scale_factor();
        let r = gfx::snap(cx.bounds().into());
        let hl = gfx::hairline(s);
        let w = r.width() / s;
        let tpx = self.ticks_per_px(w);
        let k = s as f64 / tpx; // device px per tick

        // The clips, cached (§7.3: redrawn only when clips, colours or the
        // track count change).
        let key = self.s.arrangement.with_untracked(|a| {
            use std::hash::{Hash, Hasher};
            let mut h = std::collections::hash_map::DefaultHasher::new();
            for v in [r.width(), r.height(), s] {
                v.to_bits().hash(&mut h);
            }
            tpx.to_bits().hash(&mut h);
            a.lanes.len().hash(&mut h);
            for l in &a.lanes {
                crate::state::rgb_hex(l.color).hash(&mut h);
            }
            for c in &a.clips {
                (c.id, c.lane, c.start_ticks, c.len_ticks, c.color.map(crate::state::rgb_hex)).hash(&mut h);
            }
            h.finish()
        });
        let mut cache = self.cache.borrow_mut();
        if cache.as_ref().is_none_or(|(k0, _)| *k0 != key) {
            let (iw, ih) = (r.width().ceil() as i32, r.height().ceil() as i32);
            *cache = canvas.new_surface(&vg::ImageInfo::new_n32_premul((iw, ih), None), None).map(|mut surface| {
                let c = surface.canvas();
                c.clear(vg::Color::TRANSPARENT);
                c.draw_rect(vg::Rect::from_xywh(0.0, 0.0, r.width(), r.height()), &gfx::fill(STRIP_BG));
                self.s.arrangement.with_untracked(|a| {
                    let (rh, per, _) = row_plan(a.lanes.len(), OVERVIEW_HEIGHT);
                    for cl in &a.clips {
                        let row = cl.lane / per;
                        let y = ((ROWS_TOP + row as f32 * (rh + ROW_GAP)) * s).round();
                        let x0 = (cl.start_ticks as f64 * k).round() as f32;
                        let x1 = ((cl.end_ticks() as f64 * k).round() as f32 - hl).max(x0 + hl);
                        let col = cl.color.unwrap_or(a.lanes[cl.lane].color);
                        let o = crate::theme::palette::overview(col);
                        let body = vg::Rect::from_ltrb(x0, y, x1, y + (rh * s).round().max(hl));
                        c.draw_rect(body, &gfx::fill(o.body));
                        c.draw_rect(vg::Rect::from_ltrb(x0, y, x1, y + hl), &gfx::fill(o.top));
                    }
                });
                (key, surface.image_snapshot())
            });
        }
        match cache.as_ref() {
            Some((_, img)) => {
                canvas.draw_image(img, (r.left, r.top), None);
            }
            None => {
                canvas.draw_rect(r, &gfx::fill(STRIP_BG));
            }
        }
        drop(cache);

        // The loop, only while it's on.
        let (ls, le, ph) = self.s.transport.with_untracked(|t| (t.loop_start_ticks, t.loop_end_ticks, t.playhead_ticks));
        if self.s.loop_on.get_untracked() {
            let lr = vg::Rect::from_ltrb((r.left + (ls * k) as f32).round(), r.top, (r.left + (le * k) as f32).round(), r.top + LOOP_H * s);
            canvas.draw_rect(lr, &gfx::fill(ACCENT.with_alpha(LOOP_A)));
        }
        // The playhead.
        let px = (r.left + (ph * k) as f32).round();
        canvas.draw_rect(vg::Rect::from_ltrb(px, r.top + OUTLINE_TOP * s, px + hl, r.bottom - OUTLINE_BOTTOM * s), &gfx::fill(NEUTRAL_HI.with_alpha(PLAYHEAD_A)));
        // The visible part.
        let (view, vw) = self.s.arrangement.with_untracked(|a| (a.view, a.view_w));
        let x0 = (r.left + (view.scroll_ticks * k) as f32).round();
        let x1 = (r.left + (view.x_to_tick(vw as f64) * k) as f32).round().max(x0 + 4.0 * s);
        let frame = vg::Rect::from_ltrb(x0, r.top + OUTLINE_TOP * s, x1, r.bottom - OUTLINE_BOTTOM * s);
        let rr = gfx::rrect(frame, OUTLINE_RADIUS * s);
        canvas.draw_rrect(rr, &gfx::fill(Rgba::white(0.04)));
        canvas.save();
        canvas.clip_rrect(rr, vg::ClipOp::Intersect, true);
        canvas.draw_rect(vg::Rect::from_xywh(frame.left, frame.top + hl, frame.width(), hl), &gfx::fill(Rgba::white(0.08)));
        canvas.restore();
        gfx::ring(canvas, &rr, hl, &gfx::fill(OUTLINE));
        let _ = Face::Medium;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// §7.3: 6 px rows 1 px apart; smaller with more tracks down to 1 px;
    /// then tracks merge so 50+ still fit.
    #[test]
    fn rows_shrink_then_merge() {
        assert_eq!(row_plan(4, 32.0), (6.0, 1, 4));
        assert_eq!(row_plan(8, 32.0), (2.0, 1, 8));
        assert_eq!(row_plan(14, 32.0), (1.0, 1, 14));
        let (h, per, rows) = row_plan(60, 32.0);
        assert_eq!(h, 1.0);
        assert!(per >= 2 && rows * 2 - 1 <= 27, "{per} {rows}");
    }

    /// The ruler and the Overview zoom around the grabbed tick.
    #[test]
    fn nav_drag_keeps_the_grabbed_tick() {
        // Scrolled in, so a drag to the right doesn't run into the song's start.
        let v0 = Viewport { scroll_ticks: 8.0 * TICKS_PER_BAR as f64, ..Viewport::default() };
        let d = NavDrag::new((0.0, 0.0), v0, 300.0);
        let v = d.view(40.0, 300.0);
        assert!(v.px_per_bar() > v0.px_per_bar());
        assert!((v.x_to_tick(300.0) - d.anchor_tick).abs() < 1e-6);
        let v = d.view(0.0, 400.0);
        assert!((v.x_to_tick(400.0) - d.anchor_tick).abs() < 1e-6, "horizontal: content follows the mouse");
    }
}
