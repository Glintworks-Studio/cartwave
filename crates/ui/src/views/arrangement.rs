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
use crate::notes::Note;
use crate::state::{self, Clip, MIN_CLIP_TICKS, TICKS_PER_BAR};
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
}

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
        }
        .build(cx, |_| {})
        .navigable(true)
        .height(Stretch(1.0))
        .width(Stretch(1.0))
        .bind(s.arrangement, |mut h| h.needs_redraw())
        .bind(s.transport, |mut h| h.needs_redraw())
        .bind(s.lane_h, |mut h| h.needs_redraw())
        .bind(s.loop_on, |mut h| h.needs_redraw())
        // Per-frame redraws only while easing is in flight.
        .bind(s.frame, move |mut h| {
            // Note: NOT `Handle::modify` here, it emits a full-window redraw.
            if animating.get() {
                h.needs_redraw();
            }
        })
    }

    /// Logical mouse position relative to the view.
    fn local_mouse(cx: &EventContext) -> (f32, f32) {
        let b = cx.bounds();
        let s = cx.scale_factor();
        (
            (cx.mouse().cursor_x - b.x) / s,
            (cx.mouse().cursor_y - b.y) / s,
        )
    }

    fn hit_test(a: &state::Arrangement, x: f32, y: f32, lane_h: f32) -> Option<(u32, Zone)> {
        let lane = (y / lane_h).floor();
        if lane < 0.0 {
            return None;
        }
        let lane = lane as usize;
        let ly = y - lane as f32 * lane_h;
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
        let (mx, my) = Self::local_mouse(cx);
        let mods = *cx.modifiers();
        let arr = self.s.arrangement;
        let lane_h = self.s.lane_h.get_untracked();
        let hit = arr.with_untracked(|a| Self::hit_test(a, mx, my, lane_h));

        let Some((id, zone)) = hit else {
            if !mods.shift() {
                arr.update(|a| a.selection.clear());
            }
            return;
        };

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
        self.drag = Some(Drag {
            mode,
            press: (mx, my),
            anchor,
            origins,
            moved: false,
            collapse_to,
        });
        cx.capture();
        cx.focus();
    }

    fn on_drag(&mut self, cx: &mut EventContext) {
        let (mx, my) = Self::local_mouse(cx);
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
            let snap = a.snap;
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
            let lanes = a.lanes.len() as i64;

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
                    let mut dlane = (dy / self.s.lane_h.get_untracked()).round() as i64;
                    let (lo, hi) = drag
                        .origins
                        .iter()
                        .fold((i64::MAX, i64::MIN), |(lo, hi), o| {
                            (lo.min(o.lane as i64), hi.max(o.lane as i64))
                        });
                    dlane = dlane.clamp(-lo, lanes - 1 - hi);
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
            }
        }
    }

    fn update_hover(&mut self, cx: &mut EventContext) {
        let (mx, my) = Self::local_mouse(cx);
        let lane_h = self.s.lane_h.get_untracked();
        let hit = self
            .s
            .arrangement
            .with_untracked(|a| Self::hit_test(a, mx, my, lane_h));
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
        let lane_h = self.s.lane_h.get_untracked() * s;
        // The canvas: one gradient, lit from the top and falling to black,
        // as in the prototype. Lanes are only hairlines on it.
        canvas.draw_rect(rect, &gfx::vgrad(rect, CANVAS_TOP, CANVAS_BOTTOM));
        let lane_line = gfx::fill(Rgba::white(0.035));
        for i in 1..=a.lanes.len() {
            let y = (rect.top + i as f32 * lane_h).round();
            canvas.draw_rect(vg::Rect::from_xywh(rect.left, y - s, rect.width(), s), &lane_line);
        }

        // Grid: bars, beats, and sixteenths when zoomed in.
        let ppb = view.px_per_bar() as f32;
        let first_bar = (view.scroll_ticks / TICKS_PER_BAR as f64).floor() as i64;
        let nbars = (rect.width() / s / ppb).ceil() as i64 + 2;
        let bar_line = gfx::fill(Rgba::white(0.05));
        let beat_line = gfx::fill(Rgba::white(0.022));
        let six_line = gfx::fill(Rgba::white(0.01));
        let grid_h = lanes_bottom - rect.top;
        for bar in first_bar.max(0)..first_bar + nbars {
            let bx = x_of((bar as u32 * TICKS_PER_BAR) as f64);
            canvas.draw_rect(
                vg::Rect::from_xywh(bx.round(), rect.top, s, grid_h),
                &bar_line,
            );
            let six_px = ppb / 16.0;
            for k in 1..16u32 {
                let x = (bx + k as f32 * six_px).round();
                if k % 4 == 0 {
                    if ppb >= 48.0 {
                        canvas.draw_rect(vg::Rect::from_xywh(x, rect.top, s, grid_h), &beat_line);
                    }
                } else if six_px >= 10.0 {
                    canvas.draw_rect(vg::Rect::from_xywh(x, rect.top, s, grid_h), &six_line);
                }
            }
        }

        // Past the loop end: dimmed (only while the loop light is on).
        let lex = x_of(loop_end);
        if self.s.loop_on.get_untracked() && lex > rect.left && lex < rect.right {
            canvas.draw_rect(
                vg::Rect::from_ltrb(lex, rect.top, rect.right, lanes_bottom),
                &gfx::fill(Rgba::black(0.28)),
            );
        }
    }
}

impl View for Arrangement {
    fn element(&self) -> Option<&'static str> {
        Some("arrangement")
    }

    fn event(&mut self, cx: &mut EventContext, event: &mut Event) {
        event.map(|we, meta| match we {
            WindowEvent::MouseDown(MouseButton::Left) => {
                self.on_press(cx);
                meta.consume();
            }
            WindowEvent::MouseDoubleClick(MouseButton::Left) => {
                // On a clip: open it in the piano roll. On empty lane space: a
                // new 1-bar clip at the snapped position (it wins overlaps).
                let (mx, my) = Self::local_mouse(cx);
                let lane_h = self.s.lane_h.get_untracked();
                let hit = self.s.arrangement.with_untracked(|a| Self::hit_test(a, mx, my, lane_h));
                match hit {
                    Some((id, _)) => cx.emit(crate::AppEvent::OpenPiano(id)),
                    None => {
                        let lane = (my / lane_h).floor();
                        let no_snap = crate::input::bypass_snap(cx.modifiers());
                        let lanes = self.s.arrangement.with_untracked(|a| a.lanes.len());
                        if lane >= 0.0 && (lane as usize) < lanes {
                            self.s.arrangement.update(|a| {
                                let t = a.view.x_to_tick(mx as f64).max(0.0) as i64;
                                let g = a.snap.ticks() as i64;
                                let start = if no_snap { t } else { t.div_euclid(g) * g };
                                let id = a.add_clip(lane as usize, start as u32, TICKS_PER_BAR);
                                a.selection = vec![id];
                            });
                            cx.emit(crate::AppEvent::Commit);
                        }
                    }
                }
                meta.consume();
            }
            WindowEvent::MouseMove(_, _) => {
                if self.drag.is_some() {
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
                self.on_release(cx);
                self.update_hover(cx);
            }
            WindowEvent::MouseLeave if self.drag.is_none() => {
                self.hover = None;
                cx.needs_redraw();
                self.set_cursor(cx, CursorIcon::Default);
            }
            WindowEvent::MouseScroll(sx, sy) => {
                let (mx, _) = Self::local_mouse(cx);
                let zoom = crate::input::zoom_wheel(cx.modifiers());
                let (sx, sy) = (*sx, *sy);
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
            WindowEvent::KeyDown(code, key) => match code {
                // Match the logical key too: the physical code can be
                // unidentified (e.g. remote desktop, injected input).
                _ if matches!(key, Some(Key::Delete | Key::Backspace)) => {
                    self.s.arrangement.update(|a| a.delete_selected());
                    cx.emit(crate::AppEvent::Commit);
                    meta.consume();
                }
                Code::Delete | Code::Backspace => {
                    self.s.arrangement.update(|a| a.delete_selected());
                    cx.emit(crate::AppEvent::Commit);
                    meta.consume();
                }
                Code::Enter | Code::NumpadEnter => {
                    let one = self.s.arrangement.with_untracked(|a| {
                        (a.selection.len() == 1).then(|| a.selection[0])
                    });
                    if let Some(id) = one {
                        cx.emit(crate::AppEvent::OpenPiano(id));
                        meta.consume();
                    }
                }
                Code::Escape => self.s.arrangement.update(|a| a.selection.clear()),
                Code::KeyA if crate::input::command(cx.modifiers()) => {
                    self.s
                        .arrangement
                        .update(|a| a.selection = a.clips.iter().map(|c| c.id).collect());
                    meta.consume();
                }
                _ => {}
            },
            _ => {}
        });
    }

    fn draw(&self, cx: &mut DrawContext, canvas: &Canvas) {
        let _perf = crate::perf::scope(crate::perf::Span::Arrangement);
        crate::perf::log_gl_info_once();
        let b = cx.bounds();
        let s = cx.scale_factor();
        let rect: vg::Rect = b.into();

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
            let lane_h = self.s.lane_h.get_untracked() * s;

            let lanes_bottom = rect.top + a.lanes.len() as f32 * lane_h;
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
                    let y = rect.top + c.lane as f32 * lane_h + CLIP_V_INSET * s;
                    // Snap edges to device pixels for crisp borders.
                    // A 1 px gutter on the right keeps abutting clips apart.
                    let r = vg::Rect::from_ltrb(
                        x0.round(),
                        y.round(),
                        (x1.round() - s).max(x0.round() + s),
                        (y + lane_h - 2.0 * CLIP_V_INSET * s).round(),
                    );
                    // Lift dragged clips by a pixel as they "float".
                    let r = r.with_offset((0.0, -st.drag * 1.5 * s));
                    Some((
                        live,
                        ClipPaint {
                            rect: r,
                            color: a.lanes[c.lane].color,
                            name: &c.name,
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
                loop_end.to_bits().hash(&mut h);
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
                        p.color.r.to_bits().hash(&mut h);
                        p.name.hash(&mut h);
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
                    let y = rect.top + o.lane as f32 * lane_h + CLIP_V_INSET * s;
                    let r = vg::Rect::from_ltrb(
                        x0.round(),
                        y.round(),
                        x1.round(),
                        (y + lane_h - 2.0 * CLIP_V_INSET * s).round(),
                    );
                    clip::draw_ghost(canvas, r, a.lanes[o.lane].color, s);
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
                        &gfx::fill(ACCENT.with_alpha(0.6)),
                    );
                    let f = gfx::font(Face::MonoMedium, 10.0 * s);
                    let label = crate::state::Transport::position_string(t as f64);
                    let w = gfx::text_width(&f, &label) + 12.0 * s;
                    let pill = vg::Rect::from_xywh(gx + 4.0 * s, rect.top + 4.0 * s, w, 16.0 * s);
                    let rr = gfx::rrect(pill, 8.0 * s);
                    gfx::drop_shadow(canvas, &rr, 0.0, 2.0 * s, 6.0 * s, 0.0, Rgba::black(0.4));
                    canvas.draw_rrect(rr, &gfx::vgrad(pill, ACCENT.lighten(0.05), ACCENT.darken(0.15)));
                    gfx::top_sheen(canvas, &rr, s, ACCENT, Rgba::white(0.8));
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

            // Playhead: the accent lime, glowing while playing.
            let px = x_of(playhead).round();
            if px >= rect.left && px <= rect.right {
                let glow_a = if playing { 0.5 } else { 0.22 };
                let mut glow = gfx::fill(ACCENT.with_alpha(glow_a));
                glow.set_mask_filter(vg::MaskFilter::blur(vg::BlurStyle::Normal, 3.0 * s, false));
                canvas.draw_rect(
                    vg::Rect::from_xywh(px - s, rect.top, 3.0 * s, lanes_bottom - rect.top),
                    &glow,
                );
                let ink = if playing { ACCENT.lighten(0.2) } else { ACCENT.mix(TEXT_HI, 0.4) };
                canvas.draw_rect(
                    vg::Rect::from_xywh(px, rect.top, s, lanes_bottom - rect.top),
                    &gfx::fill(ink),
                );
            }
        });

        // Top edge inner shadow under the ruler for depth.
        canvas.draw_rect(
            vg::Rect::from_xywh(rect.left, rect.top, rect.width(), 8.0 * s),
            &{
                let mut p = vg::Paint::default();
                p.set_shader(gfx::linear(
                    (rect.left, rect.top),
                    (rect.left, rect.top + 8.0 * s),
                    &[Rgba::black(0.3), Rgba::black(0.0)],
                    None,
                ));
                p
            },
        );
        canvas.restore();
    }
}

// ───────────────────────────── Track headers ─────────────────────────────

/// Builds the track header column: the empty band cell beside the ruler,
/// then one [`TrackRow`] per lane.
pub fn track_headers(cx: &mut Context, s: Signals) -> Handle<'_, VStack> {
    VStack::new(cx, |cx| {
        Element::new(cx).class("band-cell");
        let lanes = s.arrangement.with_untracked(|a| a.lanes.len());
        for i in 0..lanes {
            TrackRow::new(cx, s, i).height(s.lane_h.map(|h| Pixels(*h)));
        }
    })
    .class("track-column")
}
