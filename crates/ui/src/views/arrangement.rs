//! Arrangement view (lanes, grid, clips, playhead) and the track header column.
//!
//! All clips are painted by this single view rather than one vizia view per
//! clip: hit-testing, snapping and z-order are trivial and 200+ clips cost one
//! draw call site instead of 200 entities with style/layout overhead.

use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::rc::Rc;

use vizia::prelude::*;
use vizia::vg;

use super::clip::{self, ClipAnim, ClipPaint};
use super::gfx::{self, Face};
use super::chrome::TrackRow;

thread_local! {
    /// The arrangement's left and right edges and scale as last drawn
    /// (physical px), so a browser drag can find the clip under the pointer.
    static ARRANGEMENT_AT: Cell<Option<(f32, f32, f32)>> = const { Cell::new(None) };
}

/// The arrangement's left and right edges and scale as last drawn
/// (physical px).
pub fn arrangement_at() -> Option<(f32, f32, f32)> {
    ARRANGEMENT_AT.with(|c| c.get())
}

/// The clip under a window position (physical px), on a lane the browser
/// knows the row of; the newest (topmost) clip wins.
pub fn clip_at(a: &state::Arrangement, x: f32, y: f32) -> Option<u32> {
    let (left, right, s) = ARRANGEMENT_AT.with(|c| c.get())?;
    if x < left || x > right {
        return None;
    }
    let lane = super::browser::lane_at(x, y)?;
    let tick = a.view.x_to_tick(((x - left) / s) as f64);
    a.clips
        .iter()
        .rev()
        .find(|c| c.lane == lane && (c.start_ticks as f64) <= tick && tick < c.end_ticks() as f64)
        .map(|c| c.id)
}
use crate::notes::Note;
use crate::state::{self, Clip, TimeSel, MIN_CLIP_TICKS, TICKS_PER_BAR};
use crate::theme::tokens::*;
use crate::Signals;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Zone {
    Body,
    LeftEdge,
    RightEdge,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum DragMode {
    Move,
    ResizeLeft,
    ResizeRight,
}

#[derive(Clone, Debug)]
struct Origin {
    id: u32,
    lane: usize,
    start: u32,
    len: u32,
    notes: Vec<Note>,
}

#[derive(Clone, Debug)]
struct Drag {
    mode: DragMode,
    /// Mouse position at press, logical px relative to the view.
    press: (f32, f32),
    anchor: u32,
    origins: Vec<Origin>,
    moved: bool,
    /// Plain click on an already-selected clip collapses the selection on release.
    collapse_to: Option<u32>,
    /// The snapped tick and the track under the press: a click without a
    /// drag puts the insert marker there (A1, R38: Split acts on it).
    press_at: (u32, usize),
}

pub struct Arrangement {
    s: Signals,
    drag: Option<Drag>,
    hover: Option<(u32, Zone)>,
    cursor: CursorIcon,
    anim: RefCell<HashMap<u32, ClipAnim>>,
    last_draw: Cell<Option<Instant>>,
    /// Set by `draw` while any clip easing is still in flight.
    animating: Rc<Cell<bool>>,
    /// Retained layer: lanes, grid and all settled clips, re-rendered only
    /// when its content hash changes (see `draw`).
    layer: RefCell<StaticLayer>,
    /// Scrollbar thumb being dragged, and the scrollbar under the mouse.
    sb_drag: Option<BarDrag>,
    sb_hover: Option<Axis>,
    /// A time-selection drag in empty lane space.
    time_drag: Option<TimeDrag>,
    /// The lane under the mouse (for the dimmed-lane hint, F-04).
    hover_lane: Option<usize>,
}

#[derive(Clone, Copy, Debug)]
struct TimeDrag {
    /// Snapped tick and lane under the press, and the press position.
    tick: u32,
    lane: usize,
    press: (f32, f32),
    moved: bool,
}

/// Which scrollbar.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Axis {
    /// Time (bottom edge).
    H,
    /// Lanes (right edge), only while they overflow.
    V,
}

/// A scrollbar's geometry along its axis, in logical px relative to the view.
#[derive(Clone, Copy, Debug)]
struct Bar {
    axis: Axis,
    track_start: f32,
    track_len: f32,
    thumb_start: f32,
    thumb_len: f32,
    /// The bar's position across its axis (its top or left edge).
    cross: f32,
    /// Scroll units (ticks or lane px) per px of thumb travel.
    units_per_px: f64,
}

#[derive(Clone, Copy, Debug)]
struct BarDrag {
    axis: Axis,
    /// Mouse position along the axis at press, and the scroll value then.
    press: f32,
    start: f64,
    units_per_px: f64,
}

/// Scrollbar thickness, inset from the edges, the hit band, and the shortest
/// thumb (logical px); extra bars of room after the song's end.
const BAR_THICK: f32 = 4.0;
const BAR_INSET: f32 = 4.0;
const BAR_HIT: f32 = 12.0;
const BAR_MIN_THUMB: f32 = 24.0;
const SONG_TAIL_BARS: u32 = 4;

#[derive(Default)]
struct StaticLayer {
    surface: Option<vg::Surface>,
    image: Option<vg::Image>,
    key: u64,
}

impl Arrangement {
    pub fn new(cx: &mut Context, s: Signals) -> Handle<'_, Self> {
        let animating = Rc::new(Cell::new(false));
        Self {
            s,
            drag: None,
            hover: None,
            cursor: CursorIcon::Default,
            anim: RefCell::new(HashMap::new()),
            last_draw: Cell::new(None),
            animating: animating.clone(),
            layer: RefCell::new(StaticLayer::default()),
            sb_drag: None,
            sb_hover: None,
            time_drag: None,
            hover_lane: None,
        }
        .build(cx, |_| {})
        .navigable(true)
        .height(Stretch(1.0))
        .width(Stretch(1.0))
        .bind(s.arrangement, |mut h| h.needs_redraw())
        .bind(s.transport, |mut h| h.needs_redraw())
        .bind(s.loop_on, |mut h| h.needs_redraw())
        .bind(s.lane_scroll, |mut h| h.needs_redraw())
        .bind(s.lane_scroll_max, |mut h| h.needs_redraw())
        .bind(s.time_sel, |mut h| h.needs_redraw())
        .bind(s.drag_preset, |mut h| h.needs_redraw())
        .bind(s.chip, |mut h| h.needs_redraw())
        .bind(s.guide, |mut h| h.needs_redraw())
        // Per-frame redraws only while easing is in flight.
        .bind(s.frame, move |mut h| {
            // Note: NOT `Handle::modify` here, it emits a full-window redraw.
            if animating.get() {
                h.needs_redraw();
            }
        })
    }

    /// Logical mouse position in lane space: relative to the view, plus the
    /// vertical lane scroll (lanes scroll when the piano roll leaves too little
    /// room for all of them).
    fn local_mouse(&self, cx: &EventContext) -> (f32, f32) {
        let b = cx.bounds();
        let s = cx.scale_factor();
        (
            (cx.mouse().cursor_x - b.x) / s,
            (cx.mouse().cursor_y - b.y) / s + self.s.lane_scroll.get_untracked(),
        )
    }

    fn hit_test(a: &state::Arrangement, x: f32, y: f32) -> Option<(u32, Zone)> {
        let lane = a.lane_at(y)?;
        let lane_h = a.lanes[lane].pitch();
        let ly = y - a.lane_top(lane);
        if !(CLIP_V_INSET..=lane_h - CLIP_V_INSET).contains(&ly) {
            return None;
        }
        // Topmost first: clips are drawn in list order.
        for c in a.clips.iter().rev().filter(|c| c.lane == lane) {
            let x0 = a.view.tick_to_x(c.start_ticks as f64) as f32;
            let x1 = a.view.tick_to_x(c.end_ticks() as f64) as f32;
            if x >= x0 && x <= x1 {
                // Edge zones shrink on tiny clips so the body stays grabbable.
                let zone_w = CLIP_RESIZE_ZONE.min((x1 - x0) * 0.25);
                let zone = if x - x0 <= zone_w {
                    Zone::LeftEdge
                } else if x1 - x <= zone_w {
                    Zone::RightEdge
                } else {
                    Zone::Body
                };
                return Some((c.id, zone));
            }
        }
        None
    }

    fn set_cursor(&mut self, cx: &mut EventContext, icon: CursorIcon) {
        if self.cursor != icon {
            self.cursor = icon;
            cx.emit(WindowEvent::SetCursor(icon));
        }
    }

    fn on_press(&mut self, cx: &mut EventContext) {
        let (mx, my) = self.local_mouse(cx);
        let mods = *cx.modifiers();
        let arr = self.s.arrangement;
        let hit = arr.with_untracked(|a| Self::hit_test(a, mx, my));

        // F-04: a dimmed VRC6 lane takes no edits (its hover says why).
        let lane = arr.with_untracked(|a| a.lane_at_clamped(my));
        if crate::state::lane_off(self.s.chip.get_untracked(), lane) {
            if !mods.shift() {
                arr.update(|a| a.selection.clear());
            }
            self.s.time_sel.set(None);
            return;
        }

        let extend = mods.shift() && self.s.time_sel.with_untracked(|t| t.is_some());
        let hit = if extend { None } else { hit };
        let Some((id, zone)) = hit else {
            if mods.shift() {
                if let Some(ts) = self.s.time_sel.get_untracked() {
                    let (tick, lane) = self.tick_lane_at(mx, my, !crate::input::bypass_snap(&mods));
                    let g = self.s.arrangement.with_untracked(|a| a.grid().ticks);
                    let sel = TimeSel {
                        start: ts.start.min(tick),
                        end: ts.end.max(if tick >= ts.end { tick + g } else { tick }).max(ts.start.min(tick) + 1),
                        lane0: ts.lane0.min(lane),
                        lane1: ts.lane1.max(lane),
                    };
                    self.s.time_sel.set(Some(sel));
                    cx.focus();
                    return;
                }
            }
            if !mods.shift() {
                arr.update(|a| a.selection.clear());
            }
            // Drag across empty space selects time; a click sets the
            // insert marker.
            let (tick, lane) = self.tick_lane_at(mx, my, !crate::input::bypass_snap(&mods));
            self.time_drag = Some(TimeDrag {
                tick,
                lane,
                press: (mx, my),
                moved: false,
            });
            self.s.time_sel.set(None);
            cx.capture();
            cx.focus();
            return;
        };
        self.s.time_sel.set(None);

        let mut collapse_to = None;
        arr.update(|a| {
            if mods.shift() {
                if let Some(i) = a.selection.iter().position(|s| *s == id) {
                    a.selection.remove(i);
                } else {
                    a.selection.push(id);
                }
            } else if !a.is_selected(id) {
                a.selection = vec![id];
            } else {
                collapse_to = Some(id);
            }
        });

        let still_selected = arr.with_untracked(|a| a.is_selected(id));
        if !still_selected {
            return;
        }

        let mode = match zone {
            Zone::Body => DragMode::Move,
            Zone::LeftEdge => DragMode::ResizeLeft,
            Zone::RightEdge => DragMode::ResizeRight,
        };

        let mut anchor = id;
        // Ctrl-drag (Option-drag on macOS) duplicates the selection and drags the copies.
        if mode == DragMode::Move && crate::input::duplicate_drag(&mods) {
            arr.update(|a| {
                let mut new_sel = Vec::new();
                let originals: Vec<Clip> = a
                    .selection
                    .iter()
                    .filter_map(|sid| a.clip(*sid).cloned())
                    .collect();
                // Duplicates are independent copies (no linked clips yet).
                for mut c in originals {
                    let new_id = a.next_id;
                    a.next_id += 1;
                    if c.id == id {
                        anchor = new_id;
                    }
                    c.id = new_id;
                    c.rev = state::Rev::next();
                    new_sel.push(new_id);
                    a.push_clip(c);
                }
                a.selection = new_sel;
            });
            collapse_to = None;
        }

        let origins = arr.with_untracked(|a| {
            a.selection
                .iter()
                .filter_map(|sid| a.clip(*sid))
                .map(|c| Origin {
                    id: c.id,
                    lane: c.lane,
                    start: c.start_ticks,
                    len: c.len_ticks,
                    notes: c.notes.clone(),
                })
                .collect::<Vec<_>>()
        });
        let ids: Vec<u32> = origins.iter().map(|o| o.id).collect();
        arr.update(|a| {
            a.raise(&ids);
            // The engine hears these trimming what they overlap during the drag.
            a.editing = ids.clone();
        });

        cx.emit(crate::AppEvent::GestureBegin);
        let press_at = self.tick_lane_at(mx, my, !crate::input::bypass_snap(&mods));
        self.drag = Some(Drag {
            mode,
            press: (mx, my),
            anchor,
            origins,
            moved: false,
            collapse_to,
            press_at,
        });
        cx.capture();
        cx.focus();
    }

    /// Snapped (or exact) tick and the lane at lane-space point (x, y).
    fn tick_lane_at(&self, x: f32, y: f32, snap: bool) -> (u32, usize) {
        self.s.arrangement.with_untracked(|a| {
            let t = a.view.x_to_tick(x as f64).max(0.0) as i64;
            let t = if snap { a.grid().round(t) } else { t };
            let lane = a.lane_at_clamped(y);
            (t.max(0) as u32, lane)
        })
    }

    fn on_time_drag(&mut self, cx: &mut EventContext) {
        let (mx, my) = self.local_mouse(cx);
        let snap = !crate::input::bypass_snap(cx.modifiers());
        let Some(td) = self.time_drag.as_mut() else { return };
        if !td.moved && (mx - td.press.0).abs() < 3.0 && (my - td.press.1).abs() < 3.0 {
            return;
        }
        td.moved = true;
        let td = *td;
        let (tick, lane) = self.tick_lane_at(mx, my, snap);
        let (start, end) = (td.tick.min(tick), td.tick.max(tick));
        let grid = self.s.arrangement.with_untracked(|a| a.grid().ticks);
        let sel = TimeSel {
            start,
            end: if end > start { end } else { start + grid },
            lane0: td.lane.min(lane),
            lane1: td.lane.max(lane),
        };
        self.s.time_sel.set_if_changed(Some(sel));
    }

    fn on_drag(&mut self, cx: &mut EventContext) {
        let (mx, my) = self.local_mouse(cx);
        let no_snap = crate::input::bypass_snap(cx.modifiers());
        let Some(drag) = self.drag.as_mut() else {
            return;
        };
        let (dx, dy) = (mx - drag.press.0, my - drag.press.1);
        if !drag.moved && dx.abs() < 3.0 && dy.abs() < 3.0 {
            return;
        }
        drag.moved = true;
        let drag = drag.clone();

        self.s.arrangement.update(|a| {
            let snap = a.grid();
            let d_ticks = a.view.px_to_ticks(dx as f64);
            let snap_t = |t: f64| -> i64 {
                if no_snap {
                    t.round() as i64
                } else {
                    snap.round(t.round() as i64)
                }
            };
            let Some(anchor) = drag.origins.iter().find(|o| o.id == drag.anchor) else {
                return;
            };
            // Clips can't be moved onto a dimmed VRC6 lane (F-04).
            let open_lanes = (0..a.lanes.len()).take_while(|&l| !crate::state::lane_off(self.s.chip.get_untracked(), l)).count();
            let lanes = open_lanes.max(1) as i64;

            match drag.mode {
                DragMode::Move => {
                    // Snap the anchor's new start; everyone moves by the same delta.
                    let mut delta = snap_t(anchor.start as f64 + d_ticks) - anchor.start as i64;
                    let min_start = drag
                        .origins
                        .iter()
                        .map(|o| o.start as i64)
                        .min()
                        .unwrap_or(0);
                    delta = delta.max(-min_start);
                    // Whole rows: the row under the mouse against the row pressed.
                    let mut dlane = a.lane_at_clamped(drag.press.1 + dy) as i64 - a.lane_at_clamped(drag.press.1) as i64;
                    let (lo, hi) = drag
                        .origins
                        .iter()
                        .fold((i64::MAX, i64::MIN), |(lo, hi), o| {
                            (lo.min(o.lane as i64), hi.max(o.lane as i64))
                        });
                    dlane = dlane.clamp(-lo, (lanes - 1 - hi).max(-lo));
                    for o in &drag.origins {
                        if let Some(c) = a.clip_mut(o.id) {
                            c.start_ticks = (o.start as i64 + delta) as u32;
                            c.lane = (o.lane as i64 + dlane) as usize;
                        }
                    }
                }
                DragMode::ResizeRight => {
                    let end = anchor.start as i64 + anchor.len as i64;
                    let delta = snap_t(end as f64 + d_ticks) - end;
                    for o in &drag.origins {
                        if let Some(c) = a.clip_mut(o.id) {
                            c.len_ticks = (o.len as i64 + delta).max(MIN_CLIP_TICKS as i64) as u32;
                            c.notes = o.notes.clone();
                        }
                    }
                }
                DragMode::ResizeLeft => {
                    let delta = snap_t(anchor.start as f64 + d_ticks) - anchor.start as i64;
                    for o in &drag.origins {
                        let end = o.start as i64 + o.len as i64;
                        let new_start =
                            (o.start as i64 + delta).clamp(0, end - MIN_CLIP_TICKS as i64);
                        let shift = new_start - o.start as i64;
                        if let Some(c) = a.clip_mut(o.id) {
                            c.start_ticks = new_start as u32;
                            c.len_ticks = (end - new_start) as u32;
                            // Keep notes at their absolute song positions.
                            c.notes = o
                                .notes
                                .iter()
                                .filter(|n| n.start as i64 >= shift)
                                .map(|n| Note {
                                    start: (n.start as i64 - shift) as u32,
                                    ..*n
                                })
                                .collect();
                        }
                    }
                }
            }
        });
    }

    fn on_release(&mut self, cx: &mut EventContext) {
        if let Some(td) = self.time_drag.take() {
            cx.release();
            // A click (no drag) moves the insert marker; while stopped the
            // playhead goes there (playback starts from it).
            if !td.moved {
                cx.emit(crate::AppEvent::SetInsert(td.tick, td.lane));
            }
            return;
        }
        if let Some(drag) = self.drag.take() {
            cx.release();
            cx.emit(crate::AppEvent::GestureEnd);
            // Drop: the moved/resized/duplicated clips win and trim (or split)
            // whatever they overlap in their lane.
            self.s.arrangement.update(|a| {
                let winners = std::mem::take(&mut a.editing);
                if drag.moved {
                    a.resolve_overlaps(&winners);
                }
            });
            if !drag.moved {
                if let Some(id) = drag.collapse_to {
                    self.s.arrangement.update(|a| a.selection = vec![id]);
                }
                cx.emit(crate::AppEvent::SetInsert(drag.press_at.0, drag.press_at.1));
            }
        }
    }

    fn update_hover(&mut self, cx: &mut EventContext) {
        let (mx, my) = self.local_mouse(cx);
        let lane = self.s.arrangement.with_untracked(|a| a.lane_at(my));
        if lane != self.hover_lane {
            let off = |l: Option<usize>| l.is_some_and(|l| crate::state::lane_off(self.s.chip.get_untracked(), l));
            if off(lane) || off(self.hover_lane) {
                cx.needs_redraw();
            }
            self.hover_lane = lane;
        }
        let hit = self.s.arrangement.with_untracked(|a| Self::hit_test(a, mx, my));
        if hit.map(|h| h.0) != self.hover.map(|h| h.0) {
            cx.needs_redraw();
        }
        self.hover = hit;
        let icon = match hit {
            Some((_, Zone::LeftEdge | Zone::RightEdge)) => CursorIcon::EwResize,
            _ => CursorIcon::Default,
        };
        self.set_cursor(cx, icon);
    }

    /// Right-click: a menu for the clip under the mouse (A18), or for the
    /// time selection / empty space (A19; outside a selection a one-bar
    /// selection is made where you clicked).
    fn on_context_menu(&mut self, cx: &mut EventContext) {
        let (mx, my) = self.local_mouse(cx);
        if crate::state::lane_off(self.s.chip.get_untracked(), self.s.arrangement.with_untracked(|a| a.lane_at_clamped(my))) {
            return;
        }
        let hit = self.s.arrangement.with_untracked(|a| Self::hit_test(a, mx, my));
        let site = if let Some((id, _)) = hit {
            self.s.arrangement.update(|a| {
                if !a.is_selected(id) {
                    a.selection = vec![id];
                }
            });
            self.s.time_sel.set(None);
            crate::edit::MenuSite::Clip(id)
        } else {
            let tick = self.s.arrangement.with_untracked(|a| a.view.x_to_tick(mx as f64));
            let (_, lane) = self.tick_lane_at(mx, my, false);
            let inside = self.s.time_sel.get_untracked().is_some_and(|ts| ts.contains(tick, lane));
            if !inside {
                let bar = TICKS_PER_BAR;
                let start = (tick.max(0.0) as u32) / bar * bar;
                self.s.time_sel.set(Some(TimeSel { start, end: start + bar, lane0: lane, lane1: lane }));
                self.s.arrangement.update(|a| a.selection.clear());
                cx.emit(crate::AppEvent::SetInsert(start, lane));
            }
            crate::edit::MenuSite::Empty
        };
        let s = cx.scale_factor();
        cx.emit(crate::AppEvent::ContextMenu(site, cx.mouse().cursor_x / s, cx.mouse().cursor_y / s));
    }

    /// Mouse position relative to the view, without the lane scroll.
    fn view_mouse(cx: &EventContext) -> (f32, f32) {
        let b = cx.bounds();
        let s = cx.scale_factor();
        ((cx.mouse().cursor_x - b.x) / s, (cx.mouse().cursor_y - b.y) / s)
    }

    /// The time scrollbar along the bottom (always shown) and the lane
    /// scrollbar along the right (only while lanes overflow), for a view of
    /// `w` x `h` logical px.
    fn bars(&self, a: &state::Arrangement, w: f32, h: f32) -> (Bar, Option<Bar>) {
        let v_max = self.s.lane_scroll_max.get_untracked();
        let has_v = v_max > 0.0;
        let corner = if has_v { BAR_INSET + BAR_THICK } else { 0.0 };

        // Time: the song (clips and loop end) plus a tail, or as far as the
        // view already reaches.
        let ppt = a.view.px_per_tick;
        let view_ticks = w as f64 / ppt;
        let song_end = a
            .clips
            .iter()
            .map(|c| c.end_ticks() as f64)
            .fold(self.s.transport.with_untracked(|t| t.loop_end_ticks), f64::max);
        let total = (song_end + (SONG_TAIL_BARS * TICKS_PER_BAR) as f64)
            .max(a.view.scroll_ticks + view_ticks);
        let len = (w - 2.0 * BAR_INSET - corner).max(1.0);
        let thumb = (len * (view_ticks / total) as f32).clamp(BAR_MIN_THUMB.min(len), len);
        let range = (total - view_ticks).max(0.0);
        let t = if range > 0.0 { (a.view.scroll_ticks / range).clamp(0.0, 1.0) as f32 } else { 0.0 };
        let hbar = Bar {
            axis: Axis::H,
            track_start: BAR_INSET,
            track_len: len,
            thumb_start: BAR_INSET + t * (len - thumb),
            thumb_len: thumb,
            cross: h - BAR_INSET - BAR_THICK,
            units_per_px: if len > thumb { range / (len - thumb) as f64 } else { 0.0 },
        };

        let vbar = has_v.then(|| {
            let content = a.lanes_height();
            let len = (h - 2.0 * BAR_INSET - BAR_INSET - BAR_THICK).max(1.0);
            let thumb = (len * h / content).clamp(BAR_MIN_THUMB.min(len), len);
            let t = (self.s.lane_scroll.get_untracked() / v_max).clamp(0.0, 1.0);
            Bar {
                axis: Axis::V,
                track_start: BAR_INSET,
                track_len: len,
                thumb_start: BAR_INSET + t * (len - thumb),
                thumb_len: thumb,
                cross: w - BAR_INSET - BAR_THICK,
                units_per_px: if len > thumb { (v_max / (len - thumb)) as f64 } else { 0.0 },
            }
        });
        (hbar, vbar)
    }

    /// The scrollbar under view-relative point (x, y), if any.
    fn bar_at(&self, cx: &EventContext, x: f32, y: f32) -> Option<Bar> {
        let s = cx.scale_factor();
        let (w, h) = (cx.bounds().w / s, cx.bounds().h / s);
        let (hb, vb) = self.s.arrangement.with_untracked(|a| self.bars(a, w, h));
        if let Some(vb) = vb {
            if x >= w - BAR_HIT && y >= vb.track_start && y <= vb.track_start + vb.track_len {
                return Some(vb);
            }
        }
        (y >= h - BAR_HIT && x >= hb.track_start && x <= hb.track_start + hb.track_len)
            .then_some(hb)
    }

    fn scroll_value(&self, axis: Axis) -> f64 {
        match axis {
            Axis::H => self.s.arrangement.with_untracked(|a| a.view.scroll_ticks),
            Axis::V => self.s.lane_scroll.get_untracked() as f64,
        }
    }

    fn set_scroll(&self, cx: &mut EventContext, axis: Axis, v: f64) {
        match axis {
            Axis::H => self.s.arrangement.update(|a| a.view.scroll_ticks = v.max(0.0)),
            Axis::V => {
                let cur = self.s.lane_scroll.get_untracked();
                cx.emit(crate::AppEvent::ScrollLanes(v as f32 - cur));
            }
        }
    }

    /// Mouse down on a scrollbar: grab the thumb, or jump so the thumb is
    /// centred on the click and grab it there. Returns false if no bar was hit.
    fn bar_press(&mut self, cx: &mut EventContext) -> bool {
        let (x, y) = Self::view_mouse(cx);
        let Some(bar) = self.bar_at(cx, x, y) else { return false };
        let pos = if bar.axis == Axis::H { x } else { y };
        let mut start = self.scroll_value(bar.axis);
        let on_thumb = pos >= bar.thumb_start && pos <= bar.thumb_start + bar.thumb_len;
        if !on_thumb {
            let centre = bar.thumb_start + bar.thumb_len * 0.5;
            start = (start + (pos - centre) as f64 * bar.units_per_px).max(0.0);
            self.set_scroll(cx, bar.axis, start);
        }
        self.sb_drag = Some(BarDrag {
            axis: bar.axis,
            press: pos,
            start,
            units_per_px: bar.units_per_px,
        });
        cx.capture();
        cx.needs_redraw();
        true
    }

    fn bar_drag(&mut self, cx: &mut EventContext) {
        let Some(d) = self.sb_drag else { return };
        let (x, y) = Self::view_mouse(cx);
        let pos = if d.axis == Axis::H { x } else { y };
        let v = (d.start + (pos - d.press) as f64 * d.units_per_px).max(0.0);
        self.set_scroll(cx, d.axis, v);
    }

    /// Hover over a scrollbar; returns true while the mouse is on one.
    fn bar_hover(&mut self, cx: &mut EventContext) -> bool {
        let (x, y) = Self::view_mouse(cx);
        let axis = self.bar_at(cx, x, y).map(|b| b.axis);
        if axis != self.sb_hover {
            self.sb_hover = axis;
            cx.needs_redraw();
        }
        axis.is_some()
    }

    /// Lanes, grid and loop shading (the bottom of the retained layer).
    fn draw_background(
        &self,
        canvas: &Canvas,
        a: &state::Arrangement,
        rect: vg::Rect,
        s: f32,
        lanes_bottom: f32,
        loop_end: f64,
    ) {
        let view = a.view;
        let x_of = |t: f64| rect.left + view.tick_to_x(t) as f32 * s;
        let lanes_top = lanes_bottom - a.lanes_height() * s;
        let row_y = |i: usize| lanes_top + a.lane_top(i) * s;
        // Below the lanes: the canvas gradient (no lanes to draw there).
        canvas.draw_rect(rect, &gfx::vgrad(rect, CANVAS_TOP, CANVAS_BOTTOM));
        // Lane rows (VISUAL-SPEC.md §2.4): alternating #171717 / #141414,
        // each 72 px over its #1A1A1A hairline.
        let hl = gfx::hairline(s);
        for i in 0..a.lanes.len() {
            let y0 = row_y(i).round();
            let y1 = row_y(i + 1).round();
            if y1 < rect.top || y0 > rect.bottom {
                continue;
            }
            canvas.draw_rect(vg::Rect::from_ltrb(rect.left, y0, rect.right, y1 - hl), &gfx::fill(if i % 2 == 1 { ROW_B } else { ROW_A }));
            canvas.draw_rect(vg::Rect::from_ltrb(rect.left, y1 - hl, rect.right, y1), &gfx::fill(LINE));
        }

        // Grid over the rows: bars #262626, beats #1D1D1D, sixteenths faint
        // when zoomed in (hairlines stop at each row's hairline).
        let ppb = view.px_per_bar() as f32;
        let first_bar = (view.scroll_ticks / TICKS_PER_BAR as f64).floor() as i64;
        let nbars = (rect.width() / s / ppb).ceil() as i64 + 2;
        let bar_line = gfx::fill(GRID_BAR);
        let beat_line = gfx::fill(GRID_BEAT);
        let six_line = gfx::fill(Rgba::white(0.01));
        let grid_top = lanes_top.max(rect.top);
        let grid_h = lanes_bottom - grid_top;
        for bar in first_bar.max(0)..first_bar + nbars {
            let bx = x_of((bar as u32 * TICKS_PER_BAR) as f64);
            let six_px = ppb / 16.0;
            for k in 1..16u32 {
                let x = (bx + k as f32 * six_px * s).round();
                if k % 4 == 0 {
                    if ppb >= 48.0 {
                        canvas.draw_rect(vg::Rect::from_xywh(x, grid_top, hl, grid_h), &beat_line);
                    }
                } else if six_px >= 10.0 {
                    canvas.draw_rect(vg::Rect::from_xywh(x, grid_top, hl, grid_h), &six_line);
                }
            }
            canvas.draw_rect(vg::Rect::from_xywh(bx.round(), grid_top, hl, grid_h), &bar_line);
        }
        // Row hairlines stay on top of the grid.
        for i in 1..=a.lanes.len() {
            let y1 = row_y(i).round();
            canvas.draw_rect(vg::Rect::from_ltrb(rect.left, y1 - hl, rect.right, y1), &gfx::fill(LINE));
        }
        let _ = loop_end;
    }
}

impl View for Arrangement {
    fn element(&self) -> Option<&'static str> {
        Some("arrangement")
    }

    fn event(&mut self, cx: &mut EventContext, event: &mut Event) {
        event.map(|we, meta| match we {
            WindowEvent::MouseDown(MouseButton::Left) => {
                cx.emit(crate::AppEvent::EditorFocus(crate::shortcuts::Ctx::Arrangement));
                if self.s.guide.get_untracked() {
                    cx.emit(crate::AppEvent::CloseGuide);
                }
                if !self.bar_press(cx) {
                    self.on_press(cx);
                }
                meta.consume();
            }
            WindowEvent::MouseDown(MouseButton::Right) => {
                cx.emit(crate::AppEvent::EditorFocus(crate::shortcuts::Ctx::Arrangement));
                self.on_context_menu(cx);
                meta.consume();
            }
            WindowEvent::MouseDoubleClick(MouseButton::Left)
                if {
                    let (x, y) = Self::view_mouse(cx);
                    self.bar_at(cx, x, y).is_some()
                } =>
            {
                meta.consume();
            }
            WindowEvent::MouseDoubleClick(MouseButton::Left) => {
                // On a clip: open it in the piano roll. On empty lane space: a
                // new 1-bar clip at the snapped position (it wins overlaps).
                let (mx, my) = self.local_mouse(cx);
                let hit = self.s.arrangement.with_untracked(|a| Self::hit_test(a, mx, my));
                let off = crate::state::lane_off(self.s.chip.get_untracked(), self.s.arrangement.with_untracked(|a| a.lane_at_clamped(my)));
                match hit {
                    _ if off => {}
                    Some((id, _)) => cx.emit(crate::AppEvent::OpenPiano(id)),
                    None => {
                        let lane = self.s.arrangement.with_untracked(|a| a.lane_at(my));
                        let no_snap = crate::input::bypass_snap(cx.modifiers());
                        if let Some(lane) = lane {
                            let random = self.s.settings.with_untracked(|st| st.new_clip_colour == "random");
                            self.s.arrangement.update(|a| {
                                let t = a.view.x_to_tick(mx as f64).max(0.0) as i64;
                                let g = a.grid().ticks as i64;
                                let start = if no_snap { t } else { t.div_euclid(g) * g };
                                let id = a.add_clip(lane, start as u32, TICKS_PER_BAR);
                                a.colour_new_clips(&[id], random);
                                a.selection = vec![id];
                            });
                            cx.emit(crate::AppEvent::Commit);
                        }
                    }
                }
                meta.consume();
            }
            WindowEvent::MouseMove(_, _) => {
                if self.sb_drag.is_some() {
                    self.bar_drag(cx);
                } else if self.time_drag.is_some() {
                    self.on_time_drag(cx);
                } else if self.drag.is_none() && self.bar_hover(cx) {
                    // Over a scrollbar: no clip hover.
                    if self.hover.take().is_some() {
                        cx.needs_redraw();
                    }
                    self.set_cursor(cx, CursorIcon::Default);
                } else if self.drag.is_some() {
                    self.on_drag(cx);
                    let icon = match self.drag.as_ref().map(|d| d.mode) {
                        Some(DragMode::Move) => CursorIcon::Grabbing,
                        _ => CursorIcon::EwResize,
                    };
                    self.set_cursor(cx, icon);
                } else {
                    self.update_hover(cx);
                }
            }
            WindowEvent::MouseUp(MouseButton::Left) => {
                if self.sb_drag.take().is_some() {
                    cx.release();
                    cx.needs_redraw();
                    self.bar_hover(cx);
                    return;
                }
                self.on_release(cx);
                self.update_hover(cx);
            }
            WindowEvent::GeometryChanged(_) => {
                let w = cx.bounds().w / cx.scale_factor();
                if self.s.arrangement.with_untracked(|a| a.view_w != w) {
                    self.s.arrangement.update(|a| a.view_w = w);
                }
            }
            WindowEvent::MouseLeave if self.drag.is_none() => {
                self.hover = None;
                if self.sb_drag.is_none() {
                    self.sb_hover = None;
                }
                cx.needs_redraw();
                self.set_cursor(cx, CursorIcon::Default);
            }
            WindowEvent::MouseScroll(sx, sy) => {
                let (mx, _) = self.local_mouse(cx);
                let zoom = crate::input::zoom_wheel(cx.modifiers());
                let (sx, sy) = (*sx, *sy);
                // Lanes overflowing (piano roll open): the wheel scrolls them
                // vertically; Shift-wheel scrolls in time.
                let overflow = self.s.lane_scroll_max.get_untracked() > 0.0;
                if overflow && !zoom && !cx.modifiers().shift() && sy != 0.0 {
                    cx.emit(crate::AppEvent::ScrollLanes(-sy * LANE_SCROLL_PER_NOTCH));
                    meta.consume();
                    return;
                }
                self.s.arrangement.update(|a| {
                    if zoom {
                        a.view.zoom_at(mx as f64, 1.15f64.powf(sy as f64));
                    } else {
                        // Vertical wheel scrolls horizontally: all lanes fit on screen.
                        const SCROLL_PX_PER_NOTCH: f64 = 60.0;
                        let px = (sx as f64 - sy as f64) * SCROLL_PX_PER_NOTCH;
                        a.view.scroll_ticks =
                            (a.view.scroll_ticks + a.view.px_to_ticks(px)).max(0.0);
                    }
                });
                meta.consume();
            }
            WindowEvent::KeyDown(code, key) => {
                use crate::shortcuts::{lookup, Action, Ctx};
                let mods = *cx.modifiers();
                // Match the logical key too: the physical code can be
                // unidentified (e.g. remote desktop, injected input).
                let logical_delete = matches!(key, Some(Key::Delete | Key::Backspace)) && !crate::input::command(&mods);
                let action = if logical_delete { Some(Action::Delete) } else { lookup(*code, &mods, Ctx::Arrangement) };
                // Esc and the app-wide keys bubble up to the app (F-30).
                match action {
                    Some(Action::Delete) => {
                        // The time selection's contents or the clips (A11).
                        cx.emit(crate::AppEvent::Run(Action::Delete));
                        meta.consume();
                    }
                    Some(Action::OpenClip) => {
                        let one = self.s.arrangement.with_untracked(|a| {
                            (a.selection.len() == 1).then(|| a.selection[0])
                        });
                        if let Some(id) = one {
                            cx.emit(crate::AppEvent::OpenPiano(id));
                            meta.consume();
                        }
                    }
                    Some(Action::SelectAll) => {
                        let chip = self.s.chip.get_untracked();
                        self.s.arrangement.update(|a| {
                            a.selection = a.clips.iter().filter(|c| !crate::state::lane_off(chip, c.lane)).map(|c| c.id).collect()
                        });
                        meta.consume();
                    }
                    _ => {}
                }
            }
            _ => {}
        });
    }

    fn draw(&self, cx: &mut DrawContext, canvas: &Canvas) {
        let _perf = crate::perf::scope(crate::perf::Span::Arrangement);
        crate::perf::log_gl_info_once();
        let b = cx.bounds();
        let s = cx.scale_factor();
        let rect: vg::Rect = b.into();
        ARRANGEMENT_AT.with(|a| a.set(Some((rect.left, rect.right, s))));

        let dt = gfx::frame_dt(&self.last_draw);

        let playhead = self.s.transport.with_untracked(|t| t.playhead_ticks);
        let playing = self.s.transport.with_untracked(|t| t.playing);
        let hover = self.hover.map(|h| h.0);
        let dragging = self.drag.as_ref().filter(|d| d.moved);

        canvas.save();
        canvas.clip_rect(rect, None, None);

        self.s.arrangement.with_untracked(|a| {
            let view = a.view;
            let x_of = |t: f64| rect.left + view.tick_to_x(t) as f32 * s;
            // Lanes scroll vertically when they don't all fit; rows have
            // their own heights (polish 2 §4).
            let lanes_top = (rect.top - self.s.lane_scroll.get_untracked() * s).round();
            let lanes_bottom = lanes_top + a.lanes_height() * s;
            let row_y = |i: usize| lanes_top + a.lane_top(i) * s;
            let row_h = |i: usize| a.lanes[i].pitch() * s;
            let loop_end = self.s.transport.with_untracked(|t| t.loop_end_ticks);

            // Animation targets; a clip is "live" (drawn every frame on top of
            // the retained layer) while hovered, dragged or easing.
            let mut anim = self.anim.borrow_mut();
            let mut any_animating = false;
            anim.retain(|id, _| a.clips.iter().any(|c| c.id == *id));

            let px_per_tick = view.px_per_tick as f32 * s;
            let paints: Vec<(bool, ClipPaint)> = a
                .clips
                .iter()
                .filter_map(|c| {
                    let x0 = x_of(c.start_ticks as f64);
                    let x1 = x_of(c.end_ticks() as f64);
                    let st = anim.entry(c.id).or_default();
                    let is_dragged =
                        dragging.is_some_and(|d| d.origins.iter().any(|o| o.id == c.id));
                    let target = ClipAnim {
                        hover: if hover == Some(c.id) && self.drag.is_none() {
                            1.0
                        } else {
                            0.0
                        },
                        selected: if a.is_selected(c.id) { 1.0 } else { 0.0 },
                        drag: if is_dragged { 1.0 } else { 0.0 },
                    };
                    let easing = st.step(target, dt);
                    any_animating |= easing;
                    let live = easing || st.hover > 0.0 || st.drag > 0.0;
                    if x1 < rect.left - 20.0 * s || x0 > rect.right + 20.0 * s {
                        return None;
                    }
                    let (y, lane_h) = (row_y(c.lane) + CLIP_V_INSET * s, row_h(c.lane));
                    // Snap edges to device pixels for crisp borders. A clip
                    // starts on its grid line and ends 1 px short, so abutting
                    // clips show a 1 px gap (the owner's call over the
                    // mockup's 2 px inset on both ends, DP-D23).
                    let r = vg::Rect::from_ltrb(
                        x0.round(),
                        y.round(),
                        (x1.round() - gfx::hairline(s)).max(x0.round() + s),
                        (y + lane_h - gfx::hairline(s) - 2.0 * CLIP_V_INSET * s).round(),
                    );
                    // Lift dragged clips by a pixel as they "float".
                    let r = r.with_offset((0.0, -st.drag * 1.5 * s));
                    Some((
                        live,
                        ClipPaint {
                            rect: r,
                            color: c.color.unwrap_or(a.lanes[c.lane].color),
                            name: &c.name,
                            inst: c
                                .instrument
                                .filter(|id| *id != a.instruments.lane_id(c.lane))
                                .and_then(|id| a.instruments.get(id))
                                .map(|i| i.name.as_str()),
                            notes: &c.notes,
                            len_ticks: c.len_ticks,
                            anim: *st,
                            px_per_tick,
                            visible: (rect.left, rect.right),
                            scale: s,
                        },
                    ))
                })
                .collect();
            drop(anim);
            self.animating.set(any_animating);

            // Retained layer: re-render only when what it shows changes.
            let key = {
                use std::hash::{Hash, Hasher};
                let mut h = std::collections::hash_map::DefaultHasher::new();
                for v in [rect.left, rect.top, rect.width(), rect.height(), s] {
                    v.to_bits().hash(&mut h);
                }
                view.px_per_tick.to_bits().hash(&mut h);
                view.scroll_ticks.to_bits().hash(&mut h);
                lanes_top.to_bits().hash(&mut h);
                loop_end.to_bits().hash(&mut h);
                self.s.transport.with_untracked(|t| t.loop_start_ticks).to_bits().hash(&mut h);
                self.s.loop_on.get_untracked().hash(&mut h);
                for (live, p) in &paints {
                    if !live {
                        for v in [
                            p.rect.left,
                            p.rect.top,
                            p.rect.right,
                            p.rect.bottom,
                            p.anim.selected,
                        ] {
                            v.to_bits().hash(&mut h);
                        }
                        for v in [p.color.r, p.color.g, p.color.b] {
                            v.to_bits().hash(&mut h);
                        }
                        p.name.hash(&mut h);
                        p.inst.hash(&mut h);
                        p.len_ticks.hash(&mut h);
                        for n in p.notes {
                            (n.start, n.len, n.pitch).hash(&mut h);
                        }
                    }
                }
                h.finish()
            };
            let mut layer = self.layer.borrow_mut();
            if layer.image.is_none() || layer.key != key {
                layer.image = None;
                let (w, h) = (rect.width().ceil() as i32, rect.height().ceil() as i32);
                let reuse = layer
                    .surface
                    .as_ref()
                    .is_some_and(|sf| sf.width() == w && sf.height() == h);
                if !reuse {
                    layer.surface =
                        canvas.new_surface(&vg::ImageInfo::new_n32_premul((w, h), None), None);
                }
                if let Some(surface) = layer.surface.as_mut() {
                    let lc = surface.canvas();
                    lc.clear(vg::Color::TRANSPARENT);
                    lc.save();
                    lc.translate((-rect.left, -rect.top));
                    self.draw_background(lc, a, rect, s, lanes_bottom, loop_end);
                    for (_, p) in paints.iter().filter(|(live, _)| !live) {
                        clip::draw_clip_shadow(lc, p);
                    }
                    for (_, p) in paints.iter().filter(|(live, _)| !live) {
                        clip::draw_clip_body(lc, p);
                    }
                    lc.restore();
                    layer.image = Some(surface.image_snapshot());
                    layer.key = key;
                }
            }
            match layer.image.as_ref() {
                Some(img) => {
                    canvas.draw_image(img, (rect.left, rect.top), None);
                }
                None => self.draw_background(canvas, a, rect, s, lanes_bottom, loop_end),
            }
            drop(layer);

            // Ghost outlines at original positions while dragging.
            if let Some(d) = dragging {
                for o in &d.origins {
                    let x0 = x_of(o.start as f64);
                    let x1 = x_of((o.start + o.len) as f64);
                    let (y, lane_h) = (row_y(o.lane) + CLIP_V_INSET * s, row_h(o.lane));
                    let r = vg::Rect::from_ltrb(
                        x0.round(),
                        y.round(),
                        x1.round(),
                        (y + lane_h - gfx::hairline(s) - 2.0 * CLIP_V_INSET * s).round(),
                    );
                    let col = a.clip(o.id).and_then(|c| c.color).unwrap_or(a.lanes[o.lane].color);
                    clip::draw_ghost(canvas, r, col, s);
                }
            }

            // Live clips on top: hovered/easing first, then dragged ("floating")
            // ones so their raised shadow falls over neighbours.
            let is_floating = |p: &ClipPaint| p.anim.drag > 0.01;
            for (_, p) in paints.iter().filter(|(live, p)| *live && !is_floating(p)) {
                clip::draw_clip_shadow(canvas, p);
                clip::draw_clip_body(canvas, p);
            }
            for (_, p) in paints.iter().filter(|(live, p)| *live && is_floating(p)) {
                clip::draw_clip_shadow(canvas, p);
                clip::draw_clip_body(canvas, p);
            }

            // Snap guide while dragging/resizing.
            if let Some(d) = dragging {
                if let Some(c) = a.clip(d.anchor) {
                    let t = match d.mode {
                        DragMode::ResizeRight => c.end_ticks(),
                        _ => c.start_ticks,
                    };
                    let gx = x_of(t as f64).round();
                    canvas.draw_rect(
                        vg::Rect::from_xywh(gx, rect.top, s, lanes_bottom - rect.top),
                        &gfx::fill(NEUTRAL_HI.with_alpha(0.6)),
                    );
                    let f = gfx::font(Face::MonoMedium, 10.0 * s);
                    let label = crate::state::Transport::position_string(t as f64);
                    let w = gfx::text_width(&f, &label) + 12.0 * s;
                    let pill = vg::Rect::from_xywh(gx + 4.0 * s, rect.top + 4.0 * s, w, 16.0 * s);
                    let rr = gfx::rrect(pill, 8.0 * s);
                    gfx::drop_shadow(canvas, &rr, 0.0, 2.0 * s, 6.0 * s, 0.0, Rgba::black(0.4));
                    canvas.draw_rrect(rr, &gfx::vgrad(pill, NEUTRAL_HI.lighten(0.05), NEUTRAL_HI.darken(0.15)));
                    gfx::top_sheen(canvas, &rr, s, NEUTRAL_HI, Rgba::white(0.8));
                    gfx::draw_text(
                        canvas,
                        &label,
                        pill.left + 6.0 * s,
                        gfx::baseline_for_center(&f, pill.center_y()).round(),
                        &f,
                        Rgba::hex(0x0A0A0A),
                    );
                }
            }

            // Playhead: 1 px #E6E6E6 at 85% (neutral; the display shows the state).
            let px = x_of(playhead).round();
            if px >= rect.left && px <= rect.right {
                canvas.draw_rect(
                    vg::Rect::from_xywh(px, rect.top, gfx::hairline(s), lanes_bottom - rect.top),
                    &gfx::fill(NEUTRAL_HI.with_alpha(0.85)),
                );
            }

            // The insert marker while playing (stopped, the playhead sits on it).
            if playing {
                let ix = x_of(a.insert.0 as f64).round();
                if ix >= rect.left && ix <= rect.right {
                    let lane = a.insert.1.min(a.lanes.len() - 1);
                    canvas.draw_rect(vg::Rect::from_xywh(ix, row_y(lane), s, row_h(lane)), &gfx::fill(TEXT_HI.with_alpha(0.7)));
                }
            }

            // Getting started (A22, F-14): until the first clip exists, or
            // when Help > Getting Started asks for it.
            if a.clips.is_empty() || self.s.guide.get_untracked() {
                draw_guide(canvas, rect, s, !a.clips.is_empty());
            }

            // Time selection: an accent wash over the selected lanes and range.
            if let Some(ts) = self.s.time_sel.get_untracked() {
                let sel = vg::Rect::from_ltrb(
                    x_of(ts.start as f64).round(),
                    row_y(ts.lane0.min(a.lanes.len())).round(),
                    x_of(ts.end as f64).round(),
                    row_y((ts.lane1 + 1).min(a.lanes.len())).round(),
                );
                canvas.draw_rect(sel, &gfx::fill(Rgba::white(0.045)));
                let edge = gfx::fill(Rgba::white(0.14));
                let hl = gfx::hairline(s);
                canvas.draw_rect(vg::Rect::from_xywh(sel.left, sel.top, hl, sel.height()), &edge);
                canvas.draw_rect(vg::Rect::from_xywh(sel.right - hl, sel.top, hl, sel.height()), &edge);
            }

            // Scrollbars: time along the bottom, lanes along the right while
            // they overflow. Thin pills that brighten on hover and drag.
            let (hb, vb) = self.bars(a, rect.width() / s, rect.height() / s);
            for bar in std::iter::once(hb).chain(vb) {
                let (track, thumb) = match bar.axis {
                    Axis::H => (
                        vg::Rect::from_xywh(bar.track_start, bar.cross, bar.track_len, BAR_THICK),
                        vg::Rect::from_xywh(bar.thumb_start, bar.cross, bar.thumb_len, BAR_THICK),
                    ),
                    Axis::V => (
                        vg::Rect::from_xywh(bar.cross, bar.track_start, BAR_THICK, bar.track_len),
                        vg::Rect::from_xywh(bar.cross, bar.thumb_start, BAR_THICK, bar.thumb_len),
                    ),
                };
                let to_px = |r: vg::Rect| {
                    gfx::snap(vg::Rect::from_xywh(
                        rect.left + r.left * s,
                        rect.top + r.top * s,
                        r.width() * s,
                        r.height() * s,
                    ))
                };
                let (track, thumb) = (to_px(track), to_px(thumb));
                let active = self.sb_drag.is_some_and(|d| d.axis == bar.axis);
                let hot = self.sb_hover == Some(bar.axis);
                let rad = BAR_THICK * 0.5 * s;
                canvas.draw_rrect(gfx::rrect(track, rad), &gfx::fill(Rgba::white(0.04)));
                let alpha = if active { 0.45 } else if hot { 0.32 } else { 0.18 };
                canvas.draw_rrect(gfx::rrect(thumb, rad), &gfx::fill(Rgba::white(alpha)));
            }
        });

        // Dragging a preset from the browser: dim lanes it can't play on,
        // tint the one it would land on.
        if self.s.drag_preset.with_untracked(|d| d.is_some()) {
            for lane in 0..8 {
                let (Some((fits, target)), Some((y0, y1))) =
                    (super::browser::drag_state(&self.s, lane), super::browser::lane_span(lane))
                else {
                    continue;
                };
                let band = vg::Rect::from_ltrb(rect.left, y0.max(rect.top), rect.right, y1.min(rect.bottom));
                if band.height() <= 0.0 {
                    continue;
                }
                if !fits {
                    canvas.draw_rect(band, &gfx::fill(Rgba::black(0.55)));
                } else if target {
                    canvas.draw_rect(band, &gfx::fill(NEUTRAL_HI.with_alpha(0.06)));
                }
            }
            // Over a clip: the preset becomes that clip's own instrument.
            if let Some(id) = self.s.drag_preset.get_untracked().and_then(|d| d.clip) {
                self.s.arrangement.with_untracked(|a| {
                    let (Some(c), Some((y0, y1))) = (a.clip(id), a.clip(id).and_then(|c| super::browser::lane_span(c.lane))) else { return };
                    let x0 = rect.left + a.view.tick_to_x(c.start_ticks as f64) as f32 * s;
                    let x1 = rect.left + a.view.tick_to_x(c.end_ticks() as f64) as f32 * s;
                    let r = vg::Rect::from_ltrb(x0.max(rect.left), y0 + CLIP_V_INSET * s, x1.min(rect.right), y1 - CLIP_V_INSET * s);
                    let rr = gfx::rrect(r, RADIUS_CLIP * s);
                    canvas.draw_rrect(rr, &gfx::fill(NEUTRAL_HI.with_alpha(0.12)));
                    gfx::ring(canvas, &rr, 2.0 * s, &gfx::fill(NEUTRAL_HI));
                });
            }
        }

        // F-04: the VRC6 lanes under the 2A03 profile are dimmed; the lane
        // under the mouse says why.
        let chip = self.s.chip.get_untracked();
        if crate::state::lane_off(chip, chip::driver::LANE_V1) {
            let lanes_top = (rect.top - self.s.lane_scroll.get_untracked() * s).round();
            let (lanes, tops): (usize, Vec<(f32, f32)>) = self.s.arrangement.with_untracked(|a| (a.lanes.len(), (0..a.lanes.len()).map(|i| (a.lane_top(i), a.lanes[i].pitch())).collect()));
            let f = gfx::font(Face::Medium, OFF_HINT_PX * s);
            for lane in (0..lanes).filter(|&l| crate::state::lane_off(chip, l)) {
                let band = vg::Rect::from_xywh(rect.left, lanes_top + tops[lane].0 * s, rect.width(), tops[lane].1 * s);
                canvas.draw_rect(band, &gfx::fill(Rgba::black(OFF_DIM)));
                if self.hover_lane == Some(lane) {
                    let tw = gfx::text_width(&f, crate::state::VRC6_OFF_HINT);
                    let x = (rect.left + (rect.width() - tw) * 0.5).max(rect.left + 12.0 * s).round();
                    let base = gfx::baseline_for_center(&f, band.center_y()).round();
                    gfx::draw_text(canvas, crate::state::VRC6_OFF_HINT, x, base, &f, TEXT_HI);
                }
            }
        }

        // (No shade under the ruler: Option D has one hairline, no stacked edges.)
        canvas.restore();
    }
}

/// The getting-started card (A22, F-14) centred over the lanes. `dismiss`:
/// it was opened from the Help menu and closes on a click.
fn draw_guide(canvas: &Canvas, rect: vg::Rect, s: f32, dismiss: bool) {
    use crate::shortcuts::{label, Action};
    let lines: [String; 5] = [
        "Double-click an empty lane to make a clip; double-click the clip to write notes.".into(),
        "Drag a preset from the browser onto a track to give it a sound.".into(),
        "Drop a MIDI file on the window to start from it (File > New from MIDI File).".into(),
        format!("Space plays from the insert marker · {} to undo.", label(Action::Undo)),
        "Help > Keyboard Shortcuts lists every key.".into(),
    ];
    let w = GUIDE_W * s;
    let h = (GUIDE_PAD * 2.0 + GUIDE_TITLE_H + lines.len() as f32 * GUIDE_LINE_H + if dismiss { GUIDE_LINE_H } else { 0.0 }) * s;
    let card = gfx::snap(vg::Rect::from_xywh(rect.center_x() - w * 0.5, rect.top + (rect.height() - h) * 0.4, w, h));
    let rr = gfx::rrect(card, RADIUS_BOX * s);
    gfx::drop_shadow(canvas, &rr, 0.0, 6.0 * s, 18.0 * s, 0.0, Rgba::black(0.55));
    canvas.draw_rrect(rr, &gfx::vgrad(card, BAND_TOP, BAND_BOTTOM));
    gfx::ring_vgrad(canvas, &rr, gfx::hairline(s), STROKE, Rgba::black(0.5));
    let ft = gfx::font(Face::SemiBold, GUIDE_TITLE_PX * s);
    let f = gfx::font(Face::Medium, GUIDE_TEXT_PX * s);
    let x = card.left + GUIDE_PAD * s;
    let mut y = card.top + GUIDE_PAD * s;
    gfx::draw_text(canvas, "GETTING STARTED", x.round(), (y + GUIDE_TITLE_PX * s).round(), &ft, NEUTRAL_HI);
    y += GUIDE_TITLE_H * s;
    for l in &lines {
        gfx::draw_text_clipped(canvas, l, x.round(), (y + GUIDE_TEXT_PX * s).round(), w - 2.0 * GUIDE_PAD * s, &f, TEXT_HI);
        y += GUIDE_LINE_H * s;
    }
    if dismiss {
        gfx::draw_text(canvas, "Click anywhere or press Esc to close.", x.round(), (y + GUIDE_TEXT_PX * s).round(), &f, TEXT_MID);
    }
}

/// Guide card geometry (logical px).
const GUIDE_W: f32 = 600.0;
const GUIDE_PAD: f32 = 20.0;
const GUIDE_TITLE_H: f32 = 28.0;
const GUIDE_TITLE_PX: f32 = 13.0;
const GUIDE_LINE_H: f32 = 24.0;
const GUIDE_TEXT_PX: f32 = 13.0;

/// Lane scroll per wheel notch (logical px).
pub const LANE_SCROLL_PER_NOTCH: f32 = 60.0;
/// A dimmed VRC6 lane (F-04): the black veil over it and its hint's size.
pub const OFF_DIM: f32 = 0.62;
const OFF_HINT_PX: f32 = 12.0;

// ───────────────────────────── Track headers ─────────────────────────────

/// The cell beside the ruler: "TRACKS" (Inter 600 11, tracking .08em,
/// #808080, 16 px in) on the ruler's #0F0F0F, over its one #1E1E1E hairline.
pub struct TracksCell;

impl View for TracksCell {
    fn element(&self) -> Option<&'static str> {
        Some("tracks-cell")
    }

    fn draw(&self, cx: &mut DrawContext, canvas: &Canvas) {
        let s = cx.scale_factor();
        let r = gfx::snap(cx.bounds().into());
        canvas.draw_rect(r, &gfx::fill(RULER_BG));
        let hl = gfx::hairline(s);
        canvas.draw_rect(vg::Rect::from_ltrb(r.left, r.bottom - hl, r.right, r.bottom), &gfx::fill(LINE_SOFT));
        let f = gfx::font(Face::SemiBold, 11.0 * s);
        let base = (r.top + super::ruler::LABEL_BASELINE * s).round();
        gfx::draw_text_tracked(canvas, "TRACKS", (r.left + 16.0 * s).round(), base, &f, 0.08 * 11.0 * s, TEXT_LO);
    }
}

/// Builds the track header column: the empty band cell beside the ruler,
/// then one [`TrackRow`] per lane.
pub fn track_headers(cx: &mut Context, s: Signals) -> Handle<'_, VStack> {
    VStack::new(cx, |cx| {
        TracksCell.build(cx, |_| {}).class("band-cell");
        // The rows scroll with the lanes; the viewport clips them.
        VStack::new(cx, |cx| {
            VStack::new(cx, |cx| {
                let lanes = s.arrangement.with_untracked(|a| a.lanes.len());
                for i in 0..lanes {
                    TrackRow::new(cx, s, i).height(Memo::new(move |_| s.arrangement.with(|a| a.lanes[i].pitch())).map(|h| Pixels(*h)));
                }
            })
            .height(Auto)
            .position_type(PositionType::Absolute)
            .top(s.lane_scroll.map(|v| Pixels(-*v)));
        })
        .height(Stretch(1.0))
        .overflow(Overflow::Hidden);
    })
    .class("track-column")
}
