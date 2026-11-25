//! The piano roll: edits the notes of one clip (PIANOROLL.md §2).
//!
//! One custom view paints everything (keys, ruler, grid, notes, volume lane),
//! like the arrangement: hit-testing is plain arithmetic and 2,000 notes cost
//! one draw call site. Static content (rows, grid, notes, volume bars) is
//! rendered into a retained GPU layer keyed by what it shows; the playhead,
//! the hovered row and the marquee are drawn live on top (PERF.md H2/H3).
//!
//! The look is deliberately plain (a redesign is under way); the interaction
//! and the model operations (`notes.rs`) are the permanent part.

use std::cell::RefCell;
use std::hash::{Hash, Hasher};

use vizia::prelude::*;
use vizia::vg;

use super::gfx::{self, Face};
use crate::input;
use crate::notes::{self, Note, RowKind, MIN_NOTE_TICKS};
use crate::state::{Clip, PianoState, PPQ, TICKS_PER_BAR};
use crate::theme::tokens::*;
use crate::{AppEvent, Signals};

/// Width of the key / row-label column.
pub const KEYS_W: f32 = 64.0;
/// Height of the bar-number ruler.
pub const RULER_H: f32 = 18.0;
/// Height of the volume lane (lanes with volume only), and the gap above it.
pub const VOL_H: f32 = 56.0;
pub const VOL_GAP: f32 = 6.0;
/// Width of the resize zones at a note's ends.
const EDGE_PX: f32 = 5.0;
/// Pointer travel before a press becomes a drag.
const DRAG_SLOP: f32 = 3.0;
/// Grid lines closer than this are not drawn.
const MIN_LINE_GAP: f32 = 5.0;

#[derive(Clone, Copy, Debug, PartialEq)]
struct Geo {
    w: f32,
    h: f32,
    grid_x0: f32,
    grid_y0: f32,
    grid_y1: f32,
    vol: Option<(f32, f32)>,
}

impl Geo {
    fn new(w: f32, h: f32, lane: usize) -> Self {
        let has_vol = RowKind::has_volume(lane);
        let grid_y1 = if has_vol { h - VOL_H - VOL_GAP } else { h };
        Self {
            w,
            h,
            grid_x0: KEYS_W,
            grid_y0: RULER_H,
            grid_y1: grid_y1.max(RULER_H + 1.0),
            vol: has_vol.then_some((h - VOL_H, h)),
        }
    }
}

/// Coordinate mapping for the open clip (logical px relative to the view).
#[derive(Clone, Copy, Debug)]
struct Map {
    geo: Geo,
    kind: RowKind,
    ppt: f64,
    scroll: f64,
    row_h: f32,
    top: f32,
}

impl Map {
    fn new(p: &PianoState, geo: Geo, kind: RowKind) -> Self {
        // Noise (16 rows) and DPCM (4) rows grow to fill the panel.
        const MAX_ROW_H: f32 = 32.0;
        let row_h = match kind {
            RowKind::Piano { .. } => p.row_h,
            _ => ((geo.grid_y1 - geo.grid_y0) / kind.rows() as f32).clamp(p.row_h, MAX_ROW_H),
        };
        let visible = (geo.grid_y1 - geo.grid_y0) / row_h;
        let max_top = (kind.rows() as f32 - visible).max(0.0);
        Self { geo, kind, ppt: p.px_per_tick, scroll: p.scroll_ticks, row_h, top: p.top.clamp(0.0, max_top) }
    }

    fn x_of(&self, tick: f64) -> f32 {
        self.geo.grid_x0 + ((tick - self.scroll) * self.ppt) as f32
    }

    fn tick_at(&self, x: f32) -> f64 {
        self.scroll + (x - self.geo.grid_x0) as f64 / self.ppt
    }

    /// Top y of a row (rows count up from the bottom; the top row is drawn first).
    fn y_of_row(&self, row: u32) -> f32 {
        let from_top = (self.kind.rows() - 1 - row) as f32;
        self.geo.grid_y0 + (from_top - self.top) * self.row_h
    }

    fn row_at(&self, y: f32) -> Option<u32> {
        let from_top = ((y - self.geo.grid_y0) / self.row_h + self.top).floor();
        let rows = self.kind.rows() as f32;
        (from_top >= 0.0 && from_top < rows).then_some((rows - 1.0 - from_top) as u32)
    }

    /// Row delta for a vertical pointer movement (up = positive).
    fn rows_for_dy(&self, dy: f32) -> i32 {
        (-dy / self.row_h).round() as i32
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Zone {
    Body,
    Left,
    Right,
}

#[derive(Clone, Debug)]
enum Drag {
    /// Drawing a new note: drag sets its length.
    Draw { id: u32, start: u32 },
    Move { anchor: u32, origins: Vec<Note>, press: (f32, f32), moved: bool, collapse_to: Option<u32>, row: i32 },
    Resize { left: bool, anchor: u32, origins: Vec<Note>, press: (f32, f32), moved: bool },
    Marquee { press: (f32, f32), cur: (f32, f32), base: Vec<u32> },
    /// Setting volumes; `paint` adds notes passed over.
    Volume { targets: Vec<u32>, paint: bool },
    /// A key held down (audition).
    Key,
    Ruler,
}

pub struct PianoRoll {
    s: Signals,
    drag: Option<Drag>,
    hover_row: Option<u32>,
    hover_zone: Option<Zone>,
    cursor: CursorIcon,
    layer: RefCell<Layer>,
}

#[derive(Default)]
struct Layer {
    surface: Option<vg::Surface>,
    image: Option<vg::Image>,
    key: u64,
}

impl PianoRoll {
    pub fn new(cx: &mut Context, s: Signals) -> Handle<'_, Self> {
        Self { s, drag: None, hover_row: None, hover_zone: None, cursor: CursorIcon::Default, layer: RefCell::new(Layer::default()) }
            .build(cx, |_| {})
            .navigable(true)
            .width(Stretch(1.0))
            .height(Stretch(1.0))
            .bind(s.arrangement, |mut h| h.needs_redraw())
            .bind(s.piano, |mut h| h.needs_redraw())
            .bind(s.transport, |mut h| h.needs_redraw())
    }

    fn local(cx: &EventContext) -> (f32, f32) {
        let b = cx.bounds();
        let s = cx.scale_factor();
        ((cx.mouse().cursor_x - b.x) / s, (cx.mouse().cursor_y - b.y) / s)
    }

    /// (open clip id, a copy of the clip's header data, piano state).
    fn open(&self) -> Option<(u32, usize, u32, u32, PianoState)> {
        let p = self.s.piano.get_untracked();
        let cid = p.clip?;
        self.s
            .arrangement
            .with_untracked(|a| a.clip(cid).map(|c| (cid, c.lane, c.start_ticks, c.len_ticks)))
            .map(|(id, lane, start, len)| (id, lane, start, len, p))
    }

    fn map(&self, cx: &EventContext, lane: usize, p: &PianoState) -> Map {
        let b = cx.bounds();
        let s = cx.scale_factor();
        Map::new(p, Geo::new(b.w / s, b.h / s, lane), RowKind::for_lane(lane))
    }

    fn edit<R>(&self, cid: u32, f: impl FnOnce(&mut Clip) -> R) -> Option<R> {
        let mut out = None;
        self.s.arrangement.update(|a| {
            if let Some(c) = a.clip_mut(cid) {
                out = Some(f(c));
            }
        });
        out
    }

    fn set_sel(&self, sel: Vec<u32>) {
        self.s.piano.update(|p| p.sel = sel);
    }

    fn hit_note(c: &Clip, m: &Map, x: f32, y: f32) -> Option<(u32, Zone)> {
        let row = m.row_at(y)?;
        let t = m.tick_at(x);
        c.notes
            .iter()
            .rev()
            .filter(|n| n.start < c.len_ticks && m.kind.row_of(n.pitch) == row)
            .find(|n| t >= n.start as f64 && t <= n.end().min(c.len_ticks) as f64)
            .map(|n| {
                let (x0, x1) = (m.x_of(n.start as f64), m.x_of(n.end().min(c.len_ticks) as f64));
                let zone_w = EDGE_PX.min((x1 - x0) * 0.25);
                let zone = if x - x0 <= zone_w {
                    Zone::Left
                } else if x1 - x <= zone_w {
                    Zone::Right
                } else {
                    Zone::Body
                };
                (n.id, zone)
            })
    }

    fn snap(cx: &EventContext, grid: notes::Grid, t: f64) -> i64 {
        if input::bypass_snap(cx.modifiers()) {
            t.round() as i64
        } else {
            grid.round(t.round() as i64)
        }
    }

    fn audition(cx: &mut EventContext, lane: usize, pitch: Option<u8>, volume: Option<u8>) {
        cx.emit(AppEvent::Preview { lane, note: pitch.map(|p| (p, volume)) });
    }

    fn on_press(&mut self, cx: &mut EventContext) {
        let Some((cid, lane, clip_start, clip_len, p)) = self.open() else { return };
        let (x, y) = Self::local(cx);
        let m = self.map(cx, lane, &p);
        let mods = *cx.modifiers();
        cx.focus();

        // Ruler: move the playhead.
        if y < m.geo.grid_y0 {
            let t = Self::snap(cx, p.grid, m.tick_at(x)).clamp(0, clip_len as i64);
            cx.emit(AppEvent::SetPlayhead((clip_start as i64 + t) as f64));
            self.drag = Some(Drag::Ruler);
            cx.capture();
            return;
        }
        // Keys: audition the row.
        if x < m.geo.grid_x0 && y < m.geo.grid_y1 {
            if let Some(row) = m.row_at(y).filter(|r| m.kind.playable_row(*r)) {
                Self::audition(cx, lane, Some(m.kind.pitch_of(row)), None);
                self.drag = Some(Drag::Key);
                cx.capture();
            }
            return;
        }
        // Volume lane.
        if let Some((v0, v1)) = m.geo.vol.filter(|(v0, _)| y >= *v0) {
            let hit = self.s.arrangement.with_untracked(|a| {
                let c = a.clip(cid)?;
                let t = m.tick_at(x);
                c.notes
                    .iter()
                    .filter(|n| n.start < c.len_ticks)
                    .find(|n| t >= n.start as f64 && t < n.end().min(c.len_ticks) as f64)
                    .map(|n| n.id)
            });
            if let Some(id) = hit {
                let multi = p.sel.contains(&id) && p.sel.len() > 1;
                let targets = if multi { p.sel.clone() } else { vec![id] };
                let v = volume_at(y, v0, v1);
                self.edit(cid, |c| notes::set_volume(&mut c.notes, &targets, Some(v)));
                self.drag = Some(Drag::Volume { targets, paint: !multi });
                cx.emit(AppEvent::GestureBegin);
                cx.capture();
            }
            return;
        }
        if x < m.geo.grid_x0 || y >= m.geo.grid_y1 {
            return;
        }

        let hit = self.s.arrangement.with_untracked(|a| a.clip(cid).and_then(|c| Self::hit_note(c, &m, x, y)));
        let Some((id, zone)) = hit else {
            if mods.shift() {
                // Shift-drag on empty space: marquee (adds to the selection).
                self.drag = Some(Drag::Marquee { press: (x, y), cur: (x, y), base: p.sel.clone() });
                cx.capture();
                return;
            }
            // Draw a note of the current grid length.
            let Some(row) = m.row_at(y).filter(|r| m.kind.playable_row(*r)) else { return };
            let t = m.tick_at(x);
            let start = if input::bypass_snap(&mods) { t.floor() as i64 } else { p.grid.floor(t.floor() as i64) };
            if start < 0 || start >= clip_len as i64 {
                return;
            }
            let pitch = m.kind.pitch_of(row);
            let start = start as u32;
            let len = p.grid.ticks().min(clip_len - start).max(1);
            let new_id = self.edit(cid, |c| {
                let mut next = c.next_note_id;
                let id = notes::add(&mut c.notes, &mut next, start, len, pitch);
                c.next_note_id = next;
                id
            });
            if let Some(id) = new_id {
                self.set_sel(vec![id]);
                self.s.arrangement.update(|a| a.editing_notes = Some((cid, vec![id])));
                self.drag = Some(Drag::Draw { id, start });
                Self::audition(cx, lane, Some(pitch), None);
                cx.emit(AppEvent::GestureBegin);
                cx.capture();
            }
            return;
        };

        // On a note.
        if mods.shift() && zone == Zone::Body {
            let mut sel = p.sel.clone();
            if let Some(i) = sel.iter().position(|s| *s == id) {
                sel.remove(i);
            } else {
                sel.push(id);
            }
            self.set_sel(sel);
            return;
        }
        let mut collapse_to = None;
        let mut sel = p.sel.clone();
        if !sel.contains(&id) {
            sel = vec![id];
        } else {
            collapse_to = Some(id);
        }
        let mut anchor = id;
        if zone == Zone::Body && input::duplicate_drag(&mods) {
            // Ctrl-drag (Option-drag on macOS): drag copies; the originals stay.
            let copies = self.edit(cid, |c| {
                let mut next = c.next_note_id;
                let mut new = Vec::new();
                let originals: Vec<Note> = c.notes.iter().filter(|n| sel.contains(&n.id)).copied().collect();
                for n in originals {
                    let nid = next;
                    next += 1;
                    if n.id == id {
                        anchor = nid;
                    }
                    new.push(nid);
                    c.notes.push(Note { id: nid, ..n });
                }
                c.next_note_id = next;
                new
            });
            sel = copies.unwrap_or_default();
            collapse_to = None;
        }
        self.set_sel(sel.clone());
        let origins: Vec<Note> = self
            .s
            .arrangement
            .with_untracked(|a| a.clip(cid).map(|c| c.notes.iter().filter(|n| sel.contains(&n.id)).copied().collect()))
            .unwrap_or_default();
        self.s.arrangement.update(|a| a.editing_notes = Some((cid, sel.clone())));
        if let Some(n) = origins.iter().find(|n| n.id == anchor) {
            Self::audition(cx, lane, Some(n.pitch), n.volume);
        }
        self.drag = Some(match zone {
            Zone::Body => Drag::Move { anchor, origins, press: (x, y), moved: false, collapse_to, row: 0 },
            Zone::Left | Zone::Right => Drag::Resize { left: zone == Zone::Left, anchor, origins, press: (x, y), moved: false },
        });
        cx.emit(AppEvent::GestureBegin);
        cx.capture();
    }

    fn on_drag(&mut self, cx: &mut EventContext) {
        let Some((cid, lane, clip_start, clip_len, p)) = self.open() else { return };
        let (x, y) = Self::local(cx);
        let m = self.map(cx, lane, &p);
        let no_snap = input::bypass_snap(cx.modifiers());
        let snap = |t: f64| if no_snap { t.round() as i64 } else { p.grid.round(t.round() as i64) };
        let Some(drag) = self.drag.as_mut() else { return };
        match drag {
            Drag::Ruler => {
                let t = Self::snap(cx, p.grid, m.tick_at(x)).clamp(0, clip_len as i64);
                cx.emit(AppEvent::SetPlayhead((clip_start as i64 + t) as f64));
            }
            Drag::Key => {}
            Drag::Draw { id, start } => {
                let (id, start) = (*id, *start);
                let end = snap(m.tick_at(x)).min(clip_len as i64);
                let min = if no_snap { MIN_NOTE_TICKS } else { p.grid.ticks() } as i64;
                let len = (end - start as i64).max(min) as u32;
                self.edit(cid, |c| {
                    if let Some(n) = c.notes.iter_mut().find(|n| n.id == id) {
                        n.len = len;
                    }
                });
            }
            Drag::Marquee { press, cur, base } => {
                *cur = (x, y);
                let (x0, x1) = (press.0.min(x), press.0.max(x));
                let (y0, y1) = (press.1.min(y), press.1.max(y));
                let (t0, t1) = (m.tick_at(x0), m.tick_at(x1));
                let mut sel = base.clone();
                self.s.arrangement.with_untracked(|a| {
                    if let Some(c) = a.clip(cid) {
                        for n in c.notes.iter().filter(|n| n.start < c.len_ticks) {
                            let row = m.kind.row_of(n.pitch);
                            let ry0 = m.y_of_row(row);
                            let overlaps = (n.end() as f64) > t0
                                && (n.start as f64) < t1
                                && ry0 + m.row_h > y0
                                && ry0 < y1;
                            if overlaps && !sel.contains(&n.id) {
                                sel.push(n.id);
                            }
                        }
                    }
                });
                self.set_sel(sel);
            }
            Drag::Volume { targets, paint } => {
                let Some((v0, v1)) = m.geo.vol else { return };
                let v = volume_at(y, v0, v1);
                if *paint {
                    let t = m.tick_at(x);
                    let under = self.s.arrangement.with_untracked(|a| {
                        a.clip(cid).and_then(|c| {
                            c.notes
                                .iter()
                                .filter(|n| n.start < c.len_ticks)
                                .find(|n| t >= n.start as f64 && t < n.end().min(c.len_ticks) as f64)
                                .map(|n| n.id)
                        })
                    });
                    if let Some(id) = under.filter(|id| !targets.contains(id)) {
                        targets.push(id);
                    }
                }
                let targets = targets.clone();
                self.edit(cid, |c| notes::set_volume(&mut c.notes, &targets, Some(v)));
            }
            Drag::Move { anchor, origins, press, moved, row, .. } => {
                let (dx, dy) = (x - press.0, y - press.1);
                if !*moved && dx.abs() < DRAG_SLOP && dy.abs() < DRAG_SLOP {
                    return;
                }
                *moved = true;
                let Some(a) = origins.iter().find(|n| n.id == *anchor).copied() else { return };
                let min_start = origins.iter().map(|n| n.start).min().unwrap_or(0) as i64;
                let max_start = origins.iter().map(|n| n.start).max().unwrap_or(0) as i64;
                // Keep every note starting inside the clip.
                let delta = (snap(a.start as f64 + dx as f64 / m.ppt) - a.start as i64)
                    .clamp(-min_start, clip_len as i64 - 1 - max_start);
                let rows: Vec<u32> = origins.iter().map(|n| m.kind.row_of(n.pitch)).collect();
                let (plo, phi) = m.kind.playable_rows();
                let (lo, hi) = (*rows.iter().min().unwrap_or(&0), *rows.iter().max().unwrap_or(&0));
                let drow = m.rows_for_dy(dy).clamp(plo as i32 - lo as i32, phi as i32 - hi as i32);
                let origins = origins.clone();
                let new_pitch = m.kind.pitch_of((m.kind.row_of(a.pitch) as i32 + drow).max(0) as u32);
                if drow != *row {
                    *row = drow;
                    Self::audition(cx, lane, Some(new_pitch), a.volume);
                }
                self.edit(cid, |c| {
                    for o in &origins {
                        if let Some(n) = c.notes.iter_mut().find(|n| n.id == o.id) {
                            n.start = (o.start as i64 + delta) as u32;
                            n.pitch = m.kind.pitch_of((m.kind.row_of(o.pitch) as i32 + drow).max(0) as u32);
                        }
                    }
                });
            }
            Drag::Resize { left, anchor, origins, press, moved } => {
                let dx = x - press.0;
                if !*moved && dx.abs() < DRAG_SLOP {
                    return;
                }
                *moved = true;
                let Some(a) = origins.iter().find(|n| n.id == *anchor).copied() else { return };
                let min = if no_snap { MIN_NOTE_TICKS } else { p.grid.ticks().min(MIN_NOTE_TICKS.max(1)) } as i64;
                let origins = origins.clone();
                let left = *left;
                let delta = if left {
                    snap(a.start as f64 + dx as f64 / m.ppt) - a.start as i64
                } else {
                    snap(a.end() as f64 + dx as f64 / m.ppt) - a.end() as i64
                };
                self.edit(cid, |c| {
                    for o in &origins {
                        if let Some(n) = c.notes.iter_mut().find(|n| n.id == o.id) {
                            let end = o.end() as i64;
                            if left {
                                let s = (o.start as i64 + delta).clamp(0, end - min);
                                n.start = s as u32;
                                n.len = (end - s) as u32;
                            } else {
                                n.len = (o.len as i64 + delta).max(min) as u32;
                            }
                        }
                    }
                });
            }
        }
    }

    fn on_release(&mut self, cx: &mut EventContext) {
        let Some(drag) = self.drag.take() else { return };
        cx.release();
        let open = self.open();
        let lane = open.as_ref().map(|o| o.1);
        match drag {
            Drag::Key => {
                if let Some(lane) = lane {
                    Self::audition(cx, lane, None, None);
                }
            }
            Drag::Ruler | Drag::Marquee { .. } => {}
            Drag::Volume { .. } => cx.emit(AppEvent::GestureEnd),
            Drag::Draw { id, .. } => {
                self.finish_drag(&[id]);
                if let Some(lane) = lane {
                    Self::audition(cx, lane, None, None);
                }
                cx.emit(AppEvent::GestureEnd);
            }
            Drag::Move { origins, moved, collapse_to, .. } => {
                let ids: Vec<u32> = origins.iter().map(|n| n.id).collect();
                self.finish_drag(&ids);
                if !moved {
                    if let Some(id) = collapse_to {
                        self.set_sel(vec![id]);
                    }
                }
                if let Some(lane) = lane {
                    Self::audition(cx, lane, None, None);
                }
                cx.emit(AppEvent::GestureEnd);
            }
            Drag::Resize { origins, .. } => {
                let ids: Vec<u32> = origins.iter().map(|n| n.id).collect();
                self.finish_drag(&ids);
                if let Some(lane) = lane {
                    Self::audition(cx, lane, None, None);
                }
                cx.emit(AppEvent::GestureEnd);
            }
        }
    }

    /// Drop: the dragged notes win (monophonic rule) and the live preview of
    /// that in the engine ends.
    fn finish_drag(&self, winners: &[u32]) {
        self.s.arrangement.update(|a| {
            if let Some((cid, _)) = a.editing_notes.take() {
                if let Some(c) = a.clip_mut(cid) {
                    let mut next = c.next_note_id;
                    notes::resolve_mono(&mut c.notes, winners, &mut next);
                    c.next_note_id = next;
                }
            }
        });
        // Notes removed by the rule leave the selection.
        let Some(cid) = self.s.piano.with_untracked(|p| p.clip) else { return };
        let alive: Vec<u32> = self
            .s
            .arrangement
            .with_untracked(|a| a.clip(cid).map(|c| c.notes.iter().map(|n| n.id).collect()))
            .unwrap_or_default();
        self.s.piano.update(|p| p.sel.retain(|id| alive.contains(id)));
    }

    fn update_hover(&mut self, cx: &mut EventContext) {
        let Some((cid, lane, _, _, p)) = self.open() else { return };
        let (x, y) = Self::local(cx);
        let m = self.map(cx, lane, &p);
        let in_rows = y >= m.geo.grid_y0 && y < m.geo.grid_y1;
        let row = if in_rows { m.row_at(y) } else { None };
        if row != self.hover_row {
            self.hover_row = row;
            cx.needs_redraw();
            let text = row.map(|r| hover_text(m.kind, r, lane, self.s)).unwrap_or_default();
            self.s.piano.update(|p| p.hover = text);
        }
        let zone = if in_rows && x >= m.geo.grid_x0 {
            self.s.arrangement.with_untracked(|a| a.clip(cid).and_then(|c| Self::hit_note(c, &m, x, y))).map(|h| h.1)
        } else {
            None
        };
        self.hover_zone = zone;
        let icon = match zone {
            Some(Zone::Left | Zone::Right) => CursorIcon::EwResize,
            Some(Zone::Body) => CursorIcon::Grab,
            None => CursorIcon::Default,
        };
        self.set_cursor(cx, icon);
    }

    fn set_cursor(&mut self, cx: &mut EventContext, icon: CursorIcon) {
        if self.cursor != icon {
            self.cursor = icon;
            cx.emit(WindowEvent::SetCursor(icon));
        }
    }

    fn on_key(&mut self, cx: &mut EventContext, code: Code, key: Option<&Key>) -> bool {
        let Some((cid, lane, clip_start, clip_len, p)) = self.open() else { return false };
        let mods = *cx.modifiers();
        let cmd = input::command(&mods);
        let kind = RowKind::for_lane(lane);
        let sel = p.sel.clone();
        let is_delete = matches!(key, Some(Key::Delete | Key::Backspace)) || matches!(code, Code::Delete | Code::Backspace);
        match code {
            _ if is_delete => {
                self.edit(cid, |c| notes::delete(&mut c.notes, &sel));
                self.set_sel(Vec::new());
            }
            Code::Escape => cx.emit(AppEvent::ClosePiano),
            Code::KeyA if cmd => {
                let all = self
                    .s
                    .arrangement
                    .with_untracked(|a| a.clip(cid).map(|c| c.audible_notes().map(|n| n.id).collect()))
                    .unwrap_or_default();
                self.set_sel(all);
                return true;
            }
            Code::KeyD if cmd => {
                let new = self.edit(cid, |c| {
                    let mut next = c.next_note_id;
                    let ids = notes::duplicate_right(&mut c.notes, &mut next, &sel, kind);
                    c.next_note_id = next;
                    ids
                });
                self.set_sel(new.unwrap_or_default());
            }
            Code::KeyC if cmd => {
                cx.emit(AppEvent::CopyNotes { cut: false });
                return true;
            }
            Code::KeyX if cmd => cx.emit(AppEvent::CopyNotes { cut: true }),
            Code::KeyV if cmd => cx.emit(AppEvent::PasteNotes),
            Code::KeyQ if !cmd => {
                self.edit(cid, |c| {
                    let mut next = c.next_note_id;
                    notes::quantize(&mut c.notes, &sel, p.grid, &mut next);
                    c.next_note_id = next;
                });
            }
            Code::ArrowUp | Code::ArrowDown => {
                let dir = if code == Code::ArrowUp { 1 } else { -1 };
                let step = if mods.shift() { kind.octave_rows() } else { 1 };
                self.edit(cid, |c| {
                    let mut next = c.next_note_id;
                    notes::transpose(&mut c.notes, &sel, dir * step, kind, &mut next);
                    c.next_note_id = next;
                });
            }
            Code::ArrowLeft | Code::ArrowRight => {
                let dir: i64 = if code == Code::ArrowRight { 1 } else { -1 };
                self.edit(cid, |c| {
                    let mut next = c.next_note_id;
                    let max = c.notes.iter().filter(|n| sel.contains(&n.id)).map(|n| n.start).max().unwrap_or(0) as i64;
                    let dt = dir * p.grid.ticks() as i64;
                    // Don't push notes past the clip end (they'd vanish).
                    if max + dt < clip_len as i64 {
                        notes::nudge(&mut c.notes, &sel, dt, &mut next);
                    }
                    c.next_note_id = next;
                });
            }
            _ => return false,
        }
        let _ = clip_start;
        // Notes removed by the rule leave the selection.
        let alive: Vec<u32> = self
            .s
            .arrangement
            .with_untracked(|a| a.clip(cid).map(|c| c.notes.iter().map(|n| n.id).collect()))
            .unwrap_or_default();
        self.s.piano.update(|p| p.sel.retain(|id| alive.contains(id)));
        cx.emit(AppEvent::Commit);
        true
    }
}

/// Volume (0..=15) for a pointer y in the volume lane.
fn volume_at(y: f32, v0: f32, v1: f32) -> u8 {
    let t = 1.0 - ((y - v0 - 4.0) / (v1 - v0 - 8.0)).clamp(0.0, 1.0);
    (t * notes::MAX_VOLUME as f32).round() as u8
}

/// The hover read-out for a row: note name, period, tuning error.
fn hover_text(kind: RowKind, row: u32, lane: usize, s: Signals) -> String {
    match kind {
        RowKind::Piano { .. } => {
            let name = notes::note_name(kind.pitch_of(row));
            match kind.tuning(row) {
                Some((cents, t)) => format!("{name}   period {t}   {cents:+.1} cents"),
                None => format!("{name}   not playable on this channel"),
            }
        }
        RowKind::Noise => {
            let fixed = s.dials.with_untracked(|d| d.get(1).copied()).unwrap_or(-1.0);
            let note = if lane == 3 && fixed >= 0.0 { "   (the Period control overrides the row)" } else { "" };
            format!("noise period index {}{note}", 15 - row)
        }
        RowKind::Dpcm => format!("{} (sample {row})", notes::DPCM_NAMES[row as usize]),
    }
}

impl View for PianoRoll {
    fn element(&self) -> Option<&'static str> {
        Some("pianoroll")
    }

    fn event(&mut self, cx: &mut EventContext, event: &mut Event) {
        event.map(|we, meta| match we {
            WindowEvent::MouseDown(MouseButton::Left) => {
                self.on_press(cx);
                meta.consume();
            }
            WindowEvent::MouseDoubleClick(MouseButton::Left) => {
                // Double-click on a volume bar: back to the channel default.
                if let Some((cid, lane, _, _, p)) = self.open() {
                    let (x, y) = Self::local(cx);
                    let m = self.map(cx, lane, &p);
                    if m.geo.vol.is_some_and(|(v0, _)| y >= v0) {
                        let t = m.tick_at(x);
                        let hit = self.s.arrangement.with_untracked(|a| {
                            a.clip(cid).and_then(|c| {
                                c.notes.iter().find(|n| t >= n.start as f64 && t < n.end() as f64).map(|n| n.id)
                            })
                        });
                        if let Some(id) = hit {
                            let targets = if p.sel.contains(&id) { p.sel.clone() } else { vec![id] };
                            self.edit(cid, |c| notes::set_volume(&mut c.notes, &targets, None));
                            cx.emit(AppEvent::Commit);
                        }
                    } else {
                        // Elsewhere a double-click acts as a press.
                        self.on_press(cx);
                    }
                }
                meta.consume();
            }
            WindowEvent::MouseMove(..) => {
                if self.drag.is_some() {
                    self.on_drag(cx);
                    cx.needs_redraw();
                } else {
                    self.update_hover(cx);
                }
            }
            WindowEvent::MouseUp(MouseButton::Left) => {
                self.on_release(cx);
                self.update_hover(cx);
                cx.needs_redraw();
            }
            WindowEvent::MouseLeave if self.drag.is_none() => {
                self.hover_row = None;
                self.s.piano.update(|p| p.hover.clear());
                self.set_cursor(cx, CursorIcon::Default);
                cx.needs_redraw();
            }
            WindowEvent::GeometryChanged(_) => {
                let b = cx.bounds();
                let w = b.w / cx.scale_factor() - KEYS_W;
                self.s.piano.update(|p| p.width_px = w);
            }
            WindowEvent::MouseScroll(sx, sy) => {
                let (x, _) = Self::local(cx);
                let mods = *cx.modifiers();
                let (sx, sy) = (*sx, *sy);
                self.s.piano.update(|p| {
                    if input::zoom_wheel(&mods) {
                        // Zoom around the pointer.
                        let anchor = p.scroll_ticks + (x - KEYS_W).max(0.0) as f64 / p.px_per_tick;
                        let ppb = (p.px_per_tick * TICKS_PER_BAR as f64 * 1.15f64.powf(sy as f64))
                            .clamp(PianoState::MIN_PX_PER_BAR, PianoState::MAX_PX_PER_BAR);
                        p.px_per_tick = ppb / TICKS_PER_BAR as f64;
                        p.scroll_ticks = (anchor - (x - KEYS_W).max(0.0) as f64 / p.px_per_tick).max(0.0);
                    } else if mods.shift() {
                        const ROWS_PER_NOTCH: f32 = 3.0;
                        let d = if sy != 0.0 { sy } else { sx };
                        p.top = (p.top - d * ROWS_PER_NOTCH).max(0.0);
                    } else {
                        const PX_PER_NOTCH: f64 = 60.0;
                        let px = (sx as f64 - sy as f64) * PX_PER_NOTCH;
                        p.scroll_ticks = (p.scroll_ticks + px / p.px_per_tick).max(0.0);
                    }
                });
                meta.consume();
            }
            WindowEvent::KeyDown(code, key)
                if self.on_key(cx, *code, key.as_ref()) => {
                    meta.consume();
                }
            _ => {}
        });
    }

    fn draw(&self, cx: &mut DrawContext, canvas: &Canvas) {
        let _perf = crate::perf::scope(crate::perf::Span::PianoRoll);
        let p = self.s.piano.get_untracked();
        let Some(cid) = p.clip else { return };
        let b = cx.bounds();
        let s = cx.scale_factor();
        let rect: vg::Rect = b.into();
        let (playhead, playing) = self.s.transport.with_untracked(|t| (t.playhead_ticks, t.playing));

        canvas.save();
        canvas.clip_rect(rect, None, None);
        self.s.arrangement.with_untracked(|a| {
            let Some(c) = a.clip(cid) else { return };
            let lane = c.lane;
            let color = a.lanes[lane].color;
            let m = Map::new(&p, Geo::new(b.w / s, b.h / s, lane), RowKind::for_lane(lane));
            let px = |v: f32| v * s;

            // Retained layer: everything but the playhead, hover and marquee.
            let key = {
                let mut h = std::collections::hash_map::DefaultHasher::new();
                for v in [rect.left, rect.top, rect.width(), rect.height(), s, m.row_h, m.top] {
                    v.to_bits().hash(&mut h);
                }
                (m.ppt.to_bits(), m.scroll.to_bits(), c.id, c.rev.0, &p.sel, p.grid.ticks(), p.lane_volume).hash(&mut h);
                h.finish()
            };
            let mut layer = self.layer.borrow_mut();
            if layer.image.is_none() || layer.key != key {
                layer.image = None;
                let (w, hgt) = (rect.width().ceil() as i32, rect.height().ceil() as i32);
                if !layer.surface.as_ref().is_some_and(|sf| sf.width() == w && sf.height() == hgt) {
                    layer.surface = canvas.new_surface(&vg::ImageInfo::new_n32_premul((w, hgt), None), None);
                }
                if let Some(surface) = layer.surface.as_mut() {
                    let lc = surface.canvas();
                    lc.clear(vg::Color::TRANSPARENT);
                    draw_static(lc, s, &m, c, color, &p);
                    layer.image = Some(surface.image_snapshot());
                    layer.key = key;
                }
            }
            if let Some(img) = layer.image.as_ref() {
                canvas.draw_image(img, (rect.left, rect.top), None);
            }
            drop(layer);

            // Hovered row (keys + a faint band across the grid).
            if let Some(row) = self.hover_row {
                let y = m.y_of_row(row);
                if y + m.row_h > m.geo.grid_y0 && y < m.geo.grid_y1 {
                    let y0 = y.max(m.geo.grid_y0);
                    let y1 = (y + m.row_h).min(m.geo.grid_y1);
                    canvas.draw_rect(
                        vg::Rect::from_ltrb(rect.left, rect.top + px(y0), rect.right, rect.top + px(y1)),
                        &gfx::fill(Rgba::white(0.04)),
                    );
                    canvas.draw_rect(
                        vg::Rect::from_ltrb(rect.left, rect.top + px(y0), rect.left + px(KEYS_W), rect.top + px(y1)),
                        &gfx::fill(ACCENT.with_alpha(0.35)),
                    );
                }
            }

            // Marquee.
            if let Some(Drag::Marquee { press, cur, .. }) = &self.drag {
                let r = vg::Rect::from_ltrb(
                    rect.left + px(press.0.min(cur.0)),
                    rect.top + px(press.1.min(cur.1)),
                    rect.left + px(press.0.max(cur.0)),
                    rect.top + px(press.1.max(cur.1)),
                );
                canvas.draw_rect(r, &gfx::fill(ACCENT.with_alpha(0.12)));
                canvas.draw_rect(r, &gfx::stroke(ACCENT.with_alpha(0.8), s));
            }

            // Playhead.
            let rel = playhead - c.start_ticks as f64;
            if rel >= 0.0 && rel <= c.len_ticks as f64 {
                let x = rect.left + px(m.x_of(rel)).round();
                if x >= rect.left + px(KEYS_W) && x <= rect.right {
                    let alpha = if playing { 0.95 } else { 0.6 };
                    canvas.draw_rect(
                        vg::Rect::from_xywh(x, rect.top, s, rect.height()),
                        &gfx::fill(Rgba::hex(0xD9D0FF).with_alpha(alpha)),
                    );
                }
            }
        });
        canvas.restore();
    }
}

/// Rows, grid, keys, ruler, notes and volume bars (into the layer; origin =
/// the view's top-left, physical px).
fn draw_static(canvas: &vg::Canvas, s: f32, m: &Map, c: &Clip, color: Rgba, p: &PianoState) {
    let g = m.geo;
    let px = |v: f32| v * s;
    let w = px(g.w);
    let (gx0, gy0, gy1) = (px(g.grid_x0), px(g.grid_y0), px(g.grid_y1));
    let kind = m.kind;
    let label_font = gfx::font(Face::Mono, 9.5 * s);

    // Background.
    canvas.draw_rect(vg::Rect::from_ltrb(0.0, 0.0, w, px(g.h)), &gfx::fill(BG_INSET.lighten(0.02)));

    // Rows.
    canvas.save();
    canvas.clip_rect(vg::Rect::from_ltrb(0.0, gy0, w, gy1), None, None);
    let first = m.row_at(g.grid_y0).unwrap_or(kind.rows() - 1);
    let last = m.row_at(g.grid_y1 - 0.01).unwrap_or(0);
    for row in last..=first {
        let y = px(m.y_of_row(row));
        let rh = px(m.row_h);
        let black = matches!(kind, RowKind::Piano { .. }) && is_black_key(kind.pitch_of(row));
        let playable = kind.playable_row(row);
        let base = match (playable, black) {
            (false, _) => Rgba::hex(0x0B0C0F),
            (true, true) => Rgba::hex(0x15171D),
            (true, false) => Rgba::hex(0x1B1E25),
        };
        canvas.draw_rect(vg::Rect::from_xywh(gx0, y, w - gx0, rh), &gfx::fill(base));
        // Row separators; stronger between octaves (B|C) on piano rows.
        let octave = matches!(kind, RowKind::Piano { .. }) && kind.pitch_of(row).is_multiple_of(12);
        let line = if octave { Rgba::white(0.08) } else { Rgba::black(0.35) };
        canvas.draw_rect(vg::Rect::from_xywh(gx0, (y + rh - s).round(), w - gx0, s), &gfx::fill(line));
        // Keys / labels.
        let piano = matches!(kind, RowKind::Piano { .. });
        let key_col = match (playable, black) {
            (false, _) => Rgba::hex(0x0E0F13),
            (true, true) => Rgba::hex(0x202330),
            // Noise / DPCM rows are labels, not keys: dark, light text.
            (true, false) if !piano => Rgba::hex(0x22252E),
            (true, false) => Rgba::hex(0xC9CCD6),
        };
        let kx1 = if black { gx0 * 0.62 } else { gx0 - s };
        canvas.draw_rect(vg::Rect::from_ltrb(0.0, y, kx1, y + rh - s), &gfx::fill(key_col));
        let label = match kind {
            RowKind::Piano { .. } if kind.pitch_of(row).is_multiple_of(12) => Some(kind.row_label(row)),
            RowKind::Piano { .. } => None,
            _ => Some(kind.row_label(row)),
        };
        if let (Some(l), true) = (label, rh >= 8.0 * s) {
            let ink = match (playable, black) {
                (false, _) => TEXT_LO.with_alpha(0.5),
                (true, false) if piano => Rgba::hex(0x30333D),
                _ => TEXT_HI,
            };
            let lx = if piano { gx0 - px(30.0) } else { px(6.0) };
            gfx::draw_text(canvas, &l, lx, gfx::baseline_for_center(&label_font, y + rh * 0.5), &label_font, ink);
        }
    }
    canvas.restore();

    // Vertical grid: bars, beats, grid lines; everything right of the clip end shaded.
    let clip_end_x = px(m.x_of(c.len_ticks as f64));
    let vlines = |top: f32, bottom: f32| {
        let t0 = m.tick_at(g.grid_x0).max(0.0);
        let t1 = m.tick_at(g.w).min(c.len_ticks as f64);
        let step = p.grid.ticks() as f64;
        let grid_px = (step * m.ppt) as f32;
        let beat_px = (PPQ as f64 * m.ppt) as f32;
        let mut t = (t0 / step).floor() * step;
        while t <= t1 {
            let x = px(m.x_of(t)).round();
            let ti = t.round() as u32;
            let (alpha, show) = if ti.is_multiple_of(TICKS_PER_BAR) {
                (0.16, true)
            } else if ti.is_multiple_of(PPQ) {
                (0.08, beat_px >= MIN_LINE_GAP)
            } else {
                (0.035, grid_px >= MIN_LINE_GAP)
            };
            if show && x >= gx0 {
                canvas.draw_rect(vg::Rect::from_ltrb(x, top, x + s, bottom), &gfx::fill(Rgba::white(alpha)));
            }
            t += step;
        }
        // Bars even when the grid is too dense to draw.
        if grid_px < MIN_LINE_GAP {
            let mut bt = (t0 / TICKS_PER_BAR as f64).floor() * TICKS_PER_BAR as f64;
            while bt <= t1 {
                let x = px(m.x_of(bt)).round();
                if x >= gx0 {
                    canvas.draw_rect(vg::Rect::from_ltrb(x, top, x + s, bottom), &gfx::fill(Rgba::white(0.16)));
                }
                bt += TICKS_PER_BAR as f64;
            }
        }
    };
    vlines(gy0, gy1);
    if let Some((v0, v1)) = g.vol {
        canvas.draw_rect(vg::Rect::from_ltrb(gx0, px(v0), w, px(v1)), &gfx::fill(BG_INSET));
        vlines(px(v0), px(v1));
    }
    if clip_end_x < w {
        canvas.draw_rect(vg::Rect::from_ltrb(clip_end_x.max(gx0), gy0, w, px(g.h)), &gfx::fill(Rgba::black(0.45)));
        canvas.draw_rect(vg::Rect::from_ltrb(clip_end_x, 0.0, clip_end_x + s, px(g.h)), &gfx::fill(color.with_alpha(0.7)));
    }

    // Ruler: song bar numbers.
    canvas.draw_rect(vg::Rect::from_ltrb(0.0, 0.0, w, gy0), &gfx::fill(BG_1));
    canvas.draw_rect(vg::Rect::from_ltrb(0.0, gy0 - s, w, gy0), &gfx::fill(LINE));
    {
        let f = gfx::font(Face::MonoMedium, 9.5 * s);
        let bar_px = (TICKS_PER_BAR as f64 * m.ppt) as f32;
        let every = if bar_px >= 40.0 { 1 } else if bar_px >= 16.0 { 4 } else { 16 };
        let t0 = m.tick_at(g.grid_x0).max(0.0);
        let mut bt = (t0 / TICKS_PER_BAR as f64).floor() * TICKS_PER_BAR as f64;
        while bt <= m.tick_at(g.w).min(c.len_ticks as f64) {
            let song_bar = (c.start_ticks as f64 + bt) / TICKS_PER_BAR as f64;
            let x = px(m.x_of(bt)).round();
            if (bt / TICKS_PER_BAR as f64) as i64 % every == 0 && x >= gx0 {
                canvas.draw_rect(vg::Rect::from_ltrb(x, gy0 * 0.45, x + s, gy0), &gfx::fill(TEXT_LO));
                let label = if c.start_ticks.is_multiple_of(TICKS_PER_BAR) {
                    format!("{}", song_bar.round() as i64 + 1)
                } else {
                    format!("{:.2}", song_bar + 1.0)
                };
                gfx::draw_text(canvas, &label, x + px(3.0), gy0 - px(5.0), &f, TEXT_LO);
            }
            bt += TICKS_PER_BAR as f64;
        }
    }

    // Notes (only the ones inside the clip are shown).
    canvas.save();
    canvas.clip_rect(vg::Rect::from_ltrb(gx0, gy0, w, gy1), None, None);
    let lane_vol = p.lane_volume;
    for n in c.audible_notes() {
        let row = kind.row_of(n.pitch);
        let y = px(m.y_of_row(row));
        if y > gy1 || y + px(m.row_h) < gy0 {
            continue;
        }
        let x0 = px(m.x_of(n.start as f64));
        let x1 = px(m.x_of(n.end().min(c.len_ticks) as f64));
        if x1 < gx0 || x0 > w {
            continue;
        }
        let selected = p.sel.contains(&n.id);
        let vol = n.volume.unwrap_or(lane_vol) as f32 / notes::MAX_VOLUME as f32;
        let body = if selected { color.lighten(0.35) } else { color.mix(Rgba::hex(0x1B1E25), 0.45 * (1.0 - vol)) };
        let r = vg::Rect::from_ltrb(x0.round(), (y + s).round(), (x1.round() - s).max(x0.round() + s), (y + px(m.row_h) - s).round());
        canvas.draw_rect(r, &gfx::fill(body));
        canvas.draw_rect(vg::Rect::from_ltrb(r.left, r.top, r.left + s, r.bottom), &gfx::fill(Rgba::black(0.35)));
        if selected {
            canvas.draw_rect(r, &gfx::stroke(Rgba::white(0.9), s));
        }
    }
    canvas.restore();

    // Volume lane bars.
    if let Some((v0, v1)) = g.vol {
        let (v0, v1) = (px(v0), px(v1));
        canvas.save();
        canvas.clip_rect(vg::Rect::from_ltrb(gx0, v0, w, v1), None, None);
        let inner = |v: f32| v1 - px(4.0) - v * (v1 - v0 - px(8.0));
        for lvl in [0.0, 0.5, 1.0] {
            canvas.draw_rect(vg::Rect::from_ltrb(gx0, inner(lvl).round(), w, inner(lvl).round() + s), &gfx::fill(Rgba::white(0.05)));
        }
        for n in c.audible_notes() {
            let x0 = px(m.x_of(n.start as f64)).round();
            let x1 = px(m.x_of(n.end().min(c.len_ticks) as f64));
            if x1 < gx0 || x0 > w {
                continue;
            }
            let bw = (x1 - x0 - s).clamp(2.0 * s, 8.0 * s);
            let v = n.volume.unwrap_or(lane_vol) as f32 / notes::MAX_VOLUME as f32;
            let top = inner(v);
            let selected = p.sel.contains(&n.id);
            let col = if n.volume.is_none() { color.with_alpha(0.35) } else if selected { color.lighten(0.35) } else { color };
            canvas.draw_rect(vg::Rect::from_ltrb(x0, top, x0 + bw, v1 - px(4.0)), &gfx::fill(col));
            canvas.draw_rect(vg::Rect::from_ltrb(x0, top, x0 + bw.max(px(6.0)), top + px(2.0)), &gfx::fill(col.lighten(0.3)));
        }
        canvas.restore();
        let f = gfx::font(Face::Medium, 9.5 * s);
        gfx::draw_text(canvas, "VOLUME", px(8.0), gfx::baseline_for_center(&f, (v0 + v1) * 0.5), &f, TEXT_LO);
    }

    // Keys column divider.
    canvas.draw_rect(vg::Rect::from_ltrb(gx0 - s, 0.0, gx0, px(g.h)), &gfx::fill(LINE));
}

fn is_black_key(pitch: u8) -> bool {
    matches!(pitch % 12, 1 | 3 | 6 | 8 | 10)
}
