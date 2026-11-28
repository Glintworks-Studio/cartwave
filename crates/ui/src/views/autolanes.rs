//! Automation lanes in the arrangement (Batch 5; Live 12 manual ch. 26,
//! UX.md §5.6): their geometry, drawing (VISUAL-SPEC.md §14) and the editor
//! that turns mouse gestures into envelope edits.
//!
//! - **Lanes** sit under each track's row in Automation Mode (A), one row
//!   height each (the headers' row heights). A locked track's lanes draw
//!   **steps**: the value quantised at each frame tick, what the chip plays.
//! - **Editing** (the reference's breakpoint behaviour): a click on the line
//!   adds a breakpoint; a click on a breakpoint deletes it (VISION.md, R108);
//!   a drag moves one (snapped to the grid unless Alt, Shift for fine
//!   vertical moves), never past its neighbours; Alt-drag on a segment bends
//!   it; a drag elsewhere in the lane selects time (its envelope then moves,
//!   copies, pastes and deletes); a double-click away from the line adds a
//!   point (R106). In **Draw Mode** (B) a drag draws steps at the grid. Every
//!   gesture is one undo step (`GestureBegin` … `GestureEnd`, or a `Commit`).
//! - **Text**: Inter only (">"-style "›" separators; the chevron is drawn).
//! - **Colours**: the warm white line and dots, grey when overridden, a
//!   neutral selection; never lime, never the track colour.

use vizia::prelude::*;
use vizia::vg;

use super::gfx::{self, Face};
use crate::automation::{self as au, AutoKey, AutoOwner, Envelope, Pt};
use crate::params::Param;
use crate::state::{self, Arrangement};
use crate::theme::tokens::*;
use crate::Signals;

/// One shown lane: where it is and what it automates.
#[derive(Clone, Debug)]
pub struct LaneGeo {
    /// The track row it belongs to (`usize::MAX`: Main).
    pub track: usize,
    /// Its place among the track's lanes.
    pub k: usize,
    pub key: AutoKey,
    pub param: Param,
    /// Lane space (logical px from the first row's top; Main: the view's).
    pub top: f32,
    pub h: f32,
    /// Quantised per frame (a locked track).
    pub stepped: bool,
    pub overridden: bool,
}

impl LaneGeo {
    /// The value area's y for value `v`.
    pub fn y_of(&self, v: f32) -> f32 {
        let (t, b) = (self.top + AUTO_INSET, self.top + self.h - AUTO_INSET - 1.0);
        b - au::norm(&self.param, v) * (b - t)
    }

    /// The value at y (snapped to the parameter's steps).
    pub fn v_of(&self, y: f32) -> f32 {
        let (t, b) = (self.top + AUTO_INSET, self.top + self.h - AUTO_INSET - 1.0);
        au::denorm(&self.param, ((b - y) / (b - t).max(1.0)).clamp(0.0, 1.0))
    }

    pub fn contains(&self, y: f32) -> bool {
        y >= self.top && y < self.top + self.h
    }
}

/// Every lane shown in the arrangement now (Automation Mode on).
pub fn lanes(a: &Arrangement) -> Vec<LaneGeo> {
    let mut out = Vec::new();
    if !a.auto_mode {
        return out;
    }
    for i in 0..a.lanes.len() {
        let l = &a.lanes[i];
        let owner = AutoOwner::Track(l.id);
        let reg = au::registry_for(&a.lanes, owner).unwrap_or_default();
        let top = a.lane_top(i);
        for (k, id) in a.auto_lanes(i).into_iter().enumerate() {
            let Some(param) = reg.iter().find(|p| p.id == id).cloned() else { continue };
            let key = (owner, id);
            let overridden = a.automation.overridden.contains(&key);
            out.push(LaneGeo { track: i, k, key, param, top: top + l.pitch() * (k + 1) as f32, h: l.pitch() - state::ROW_HAIRLINE, stepped: l.lock.is_some(), overridden });
        }
    }
    out
}

/// The Main gain's lane, drawn over the Main row (`top`, `h`: the row).
pub fn main_lane(a: &Arrangement, top: f32, h: f32) -> Option<LaneGeo> {
    if !a.auto_mode {
        return None;
    }
    let param = crate::params::registry(crate::params::Owner::Main).into_iter().next()?;
    let key = (AutoOwner::Main, param.id.to_string());
    let overridden = a.automation.overridden.contains(&key);
    Some(LaneGeo { track: usize::MAX, k: 0, key, param, top, h, stepped: false, overridden })
}

/// The value the parameter has without automation (the dashed line).
pub fn static_value(a: &Arrangement, master_db: f32, g: &LaneGeo) -> f32 {
    let Some(l) = a.lanes.get(g.track) else { return if master_db.is_finite() { master_db } else { au::DB_FLOOR } };
    let pid = |id: &str| crate::automation::target_of(id, l.voice).and_then(|t| if let chip::auto::AutoTarget::Chip(p) = t { Some(p) } else { None });
    match g.param.id {
        "track.gain" => {
            if l.vol_db.is_finite() {
                l.vol_db
            } else {
                au::DB_FLOOR
            }
        }
        "track.pan" => l.pan as f32,
        "voice.unison" => l.unison.voices as f32,
        "voice.detune" => l.unison.detune as f32,
        "voice.drift" => l.unison.drift as f32,
        id => match pid(id) {
            Some(p) => l.params[p as usize] as f32,
            None => g.param.default,
        },
    }
}

/// A value as the lane header shows it.
pub fn value_text(p: &Param, v: f32) -> String {
    match p.id {
        "track.gain" | "main.gain" => crate::state::format_db(if v <= au::DB_FLOOR { f32::NEG_INFINITY } else { v }),
        "track.pan" => crate::state::format_pan(v.round() as i8),
        "track.active" | "chip.loop" | "macro.gate" => if v >= 0.5 { "On" } else { "Off" }.into(),
        _ if p.stepped => format!("{}", v.round() as i32),
        _ => format!("{v:.1}"),
    }
}

/// The lane's label: "Device ▸ Parameter" (Live's two choosers, as one).
pub fn lane_label(p: &Param) -> String {
    let device = if p.id.starts_with("track.") || p.id.starts_with("main.") { "Mixer" } else { "Chip Synth" };
    format!("{device} \u{203A} {}", p.name)
}

// ───────────────────────────── Drawing ─────────────────────────────

/// Draws `lanes` (lane-space tops; `y0`: the screen y of lane space 0, px)
/// with their envelopes, over `rect` (the lanes' screen area).
#[allow(clippy::too_many_arguments)]
pub fn draw(canvas: &vg::Canvas, a: &Arrangement, (master_db, bpm): (f32, f32), lanes: &[LaneGeo], rect: vg::Rect, y0: f32, s: f32, ed: &Editor) {
    let view = a.view;
    let x_of = |t: f64| rect.left + view.tick_to_x(t) as f32 * s;
    let t_of = |x: f32| view.x_to_tick(((x - rect.left) / s) as f64).max(0.0);
    let hl = gfx::hairline(s);
    for g in lanes {
        let top = y0 + g.top * s;
        let band = vg::Rect::from_ltrb(rect.left, top.round(), rect.right, (top + g.h * s).round());
        if band.bottom < rect.top || band.top > rect.bottom {
            continue;
        }
        if g.track != usize::MAX {
            canvas.draw_rect(band, &gfx::fill(AUTO_LANE_BG));
            canvas.draw_rect(vg::Rect::from_ltrb(rect.left, band.bottom, rect.right, band.bottom + hl), &gfx::fill(LINE));
        }
        // The lane's time selection: a neutral wash.
        if let Some(sel) = ed.sel.as_ref().filter(|s| s.key == g.key) {
            let r = vg::Rect::from_ltrb(x_of(sel.t0 as f64).round(), band.top, x_of(sel.t1 as f64).round(), band.bottom);
            canvas.draw_rect(r, &gfx::fill(Rgba::white(0.06)));
            let edge = gfx::fill(Rgba::white(0.18));
            canvas.draw_rect(vg::Rect::from_xywh(r.left, r.top, hl, r.height()), &edge);
            canvas.draw_rect(vg::Rect::from_xywh(r.right - hl, r.top, hl, r.height()), &edge);
        }
        let sy = |v: f32| y0 + g.y_of(v) * s;
        let env = a.automation.get(&g.key);
        let col = if g.overridden { AUTO_OVERRIDDEN } else { AUTO_LINE };
        let w = AUTO_LINE_W * s;
        let Some(env) = env else {
            // No automation: the control's own value, dashed and dim.
            let y = sy(static_value(a, master_db, g)).round() + 0.5;
            let mut x = rect.left;
            let dash = gfx::fill(AUTO_LINE.with_alpha(0.35));
            while x < rect.right {
                canvas.draw_rect(vg::Rect::from_xywh(x, y - w * 0.5, 6.0 * s, w), &dash);
                x += 10.0 * s;
            }
            continue;
        };
        let mut path = vg::PathBuilder::new();
        let (left_t, right_t) = (t_of(rect.left), t_of(rect.right));
        if g.stepped {
            // What the chip plays: the value quantised per frame tick (one
            // sample per pixel column at least).
            let frame = 960.0 * bpm as f64 / 60.0 / chip::tables::FRAME_HZ_NTSC;
            let px_t = view.px_to_ticks(1.0).max(1e-6);
            let step = frame.max(px_t);
            let mut t = (left_t / frame).floor() * frame;
            let mut last: Option<f32> = None;
            while t <= right_t + step {
                let v = au::quantised(&g.param, env.value_at(t).unwrap_or(0.0));
                let (x, y) = (x_of(t), sy(v));
                match last {
                    None => path.move_to((x, y)),
                    Some(lv) if lv != v => {
                        path.line_to((x, sy(lv)));
                        path.line_to((x, y))
                    }
                    _ => &mut path,
                };
                last = Some(v);
                t += step;
            }
            if let Some(lv) = last {
                path.line_to((rect.right, sy(lv)));
            }
        } else {
            let pts = &env.points;
            let first = pts.partition_point(|p| (p.t as f64) < left_t).saturating_sub(1);
            path.move_to((rect.left, sy(env.value_at(left_t).unwrap_or(0.0))));
            for i in first..pts.len() {
                let p = pts[i];
                if p.t as f64 > right_t {
                    path.line_to((x_of(p.t as f64), sy(p.v)));
                    break;
                }
                if p.t as f64 >= left_t {
                    path.line_to((x_of(p.t as f64), sy(p.v)));
                }
                // A curved segment in pieces.
                if p.c != 0.0 {
                    if let Some(n) = pts.get(i + 1) {
                        for j in 1..16 {
                            let t = p.t as f64 + (n.t as f64 - p.t as f64) * j as f64 / 16.0;
                            if t >= left_t && t <= right_t {
                                path.line_to((x_of(t), sy(env.value_at(t).unwrap_or(p.v))));
                            }
                        }
                    }
                }
            }
            if pts.last().is_some_and(|p| (p.t as f64) < right_t) {
                path.line_to((rect.right, sy(pts.last().unwrap().v)));
            }
        }
        canvas.save();
        canvas.clip_rect(band, None, None);
        let mut paint = gfx::stroke(col, w);
        paint.set_anti_alias(true);
        canvas.draw_path(&path.detach(), &paint);
        // Breakpoints: 6 px dots (cull to the visible range).
        let r = AUTO_POINT_D * 0.5 * s;
        let lo = env.points.partition_point(|p| (p.t as f64) < left_t);
        let hi = env.points.partition_point(|p| (p.t as f64) <= right_t);
        // When zoomed far out, dots closer than 3 px are skipped (the line shows them).
        let mut last_x = f32::NEG_INFINITY;
        for (i, p) in env.points[lo..hi].iter().enumerate() {
            let i = lo + i;
            let (x, y) = (x_of(p.t as f64), sy(p.v));
            let picked = ed.point_selected(&g.key, i);
            if !picked && x - last_x < 3.0 * s {
                continue;
            }
            last_x = x;
            let hot = ed.hover.as_ref().is_some_and(|h| h.0 == g.key && h.1 == Some(i));
            if picked {
                // Neutral selection: a white outline with a soft glow.
                canvas.draw_circle((x, y), r + 4.0 * s, &gfx::fill(Rgba::white(0.10)));
                canvas.draw_circle((x, y), r + 1.5 * s, &gfx::fill(Rgba::white(1.0)));
            }
            canvas.draw_circle((x, y), if hot { r + 1.0 * s } else { r }, &gfx::fill(col));
        }
        canvas.restore();
    }
}

/// The lane header's text (inside the arrangement's header column).
pub fn draw_header(canvas: &vg::Canvas, g: &LaneGeo, value: f32, r: vg::Rect, s: f32, last: bool, hover: Option<HeaderPart>) {
    canvas.draw_rect(r, &gfx::fill(AUTO_LANE_BG));
    let f = gfx::font(Face::Medium, 11.0 * s);
    let fv = gfx::font(Face::MonoMedium, 10.0 * s);
    let label = lane_label(&g.param);
    let col = if hover == Some(HeaderPart::Chooser) { TEXT_HI } else { AUTO_LABEL };
    let base = (r.top + 18.0 * s).round();
    gfx::draw_text(canvas, &label, r.left + 16.0 * s, base, &f, col);
    // The chooser's chevron (drawn: Inter has no small down triangle).
    let cx0 = r.left + 16.0 * s + gfx::text_width(&f, &label) + 7.0 * s;
    let cy0 = base - 6.0 * s;
    let mut tri = vg::PathBuilder::new();
    tri.move_to((cx0, cy0));
    tri.line_to((cx0 + 6.0 * s, cy0));
    tri.line_to((cx0 + 3.0 * s, cy0 + 3.5 * s));
    tri.close();
    canvas.draw_path(&tri.detach(), &gfx::fill(col));
    let v = value_text(&g.param, value);
    gfx::draw_text(canvas, &v, r.left + 16.0 * s, (r.top + 34.0 * s).round(), &fv, if g.overridden { AUTO_OVERRIDDEN.lighten(0.25) } else { AUTO_LABEL });
    // − (hide) and, on the last lane, + (another parameter).
    for (part, glyph, x) in [(HeaderPart::Remove, "\u{2212}", r.right - 24.0 * s), (HeaderPart::Add, "+", r.right - 48.0 * s)] {
        if part == HeaderPart::Add && !last {
            continue;
        }
        let b = vg::Rect::from_xywh(x, r.top + 6.0 * s, 18.0 * s, 18.0 * s);
        let hot = hover == Some(part);
        canvas.draw_rrect(gfx::rrect(b, 4.0 * s), &gfx::fill(Rgba::white(if hot { 0.12 } else { 0.05 })));
        let gw = gfx::text_width(&f, glyph);
        gfx::draw_text(canvas, glyph, (b.center_x() - gw * 0.5).round(), gfx::baseline_for_center(&f, b.center_y()).round(), &f, if hot { TEXT_HI } else { AUTO_LABEL });
    }
}

/// The parts of a lane header.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HeaderPart {
    Chooser,
    Add,
    Remove,
}

/// Which part of a header `(x, y)` (header-local logical px) is on.
pub fn header_part(w: f32, x: f32, y: f32, last: bool) -> Option<HeaderPart> {
    if (6.0..24.0).contains(&y) {
        if x >= w - 24.0 && x < w - 6.0 {
            return Some(HeaderPart::Remove);
        }
        if last && x >= w - 48.0 && x < w - 30.0 {
            return Some(HeaderPart::Add);
        }
    }
    (x >= 10.0 && x < w - 52.0 && y < 26.0).then_some(HeaderPart::Chooser)
}

/// The hover help for a lane (status bar).
pub fn help(g: &LaneGeo, draw: bool) -> String {
    let mut t = if draw {
        format!("{}: Draw Mode draws steps at the grid (B turns it off)", lane_label(&g.param))
    } else {
        format!(
            "{}: click the line to add a breakpoint · click a breakpoint to delete it · drag to move it (Shift: fine, Alt: off the grid) · Alt-drag a segment to curve it · drag in the lane to select time",
            lane_label(&g.param)
        )
    };
    if g.stepped {
        t.push_str(" · locked: plays per frame at the chip's steps");
    }
    if g.overridden {
        t.push_str(" · overridden by hand: Re-Enable Automation in the transport");
    }
    t
}

// ───────────────────────────── The editor ─────────────────────────────

/// A time selection in one lane.
#[derive(Clone, Debug, PartialEq)]
pub struct LaneSel {
    pub key: AutoKey,
    pub t0: u32,
    pub t1: u32,
}

#[derive(Clone, Debug)]
enum Gesture {
    /// A press on breakpoint `i`: a click deletes it, a drag moves it.
    Point { key: AutoKey, i: usize, press: (f32, f32), orig: (u32, f32), moved: bool },
    /// Alt-drag on the segment starting at `i`.
    Curve { key: AutoKey, i: usize, press_y: f32, orig: f32, rising: bool },
    /// A press on the line: a click adds a point there; a drag (it was
    /// added on press) moves it.
    Added { key: AutoKey, i: usize, press: (f32, f32), orig: (u32, f32), moved: bool },
    /// Draw Mode: steps at the grid while dragging.
    Draw { key: AutoKey },
    /// A drag selecting time in a lane.
    Select { key: AutoKey, t0: u32, press: (f32, f32), moved: bool },
    /// Dragging the selected time's envelope.
    MoveSel { key: AutoKey, sel: (u32, u32), press_x: f32, base: Envelope },
}

/// The editing state one view keeps (the arrangement, the Main row).
#[derive(Default)]
pub struct Editor {
    gesture: Option<Gesture>,
    /// The lane's selected time (Copy / Paste / Delete act on it).
    pub sel: Option<LaneSel>,
    /// The lane under the mouse and the breakpoint under it.
    pub hover: Option<(AutoKey, Option<usize>)>,
    /// The breakpoint last pressed (drawn selected).
    pub picked: Option<(AutoKey, usize)>,
}

impl Editor {
    pub fn busy(&self) -> bool {
        self.gesture.is_some()
    }

    fn point_selected(&self, key: &AutoKey, i: usize) -> bool {
        self.picked.as_ref().is_some_and(|(k, j)| k == key && *j == i) || self.gesture.as_ref().is_some_and(|g| matches!(g, Gesture::Point { key: k, i: j, moved: true, .. } | Gesture::Added { key: k, i: j, moved: true, .. } if k == key && *j == i))
    }

    /// The breakpoint of `env` near screen-space (x, y) (lane-space px).
    fn point_at(a: &Arrangement, g: &LaneGeo, env: &Envelope, x: f32, y: f32) -> Option<usize> {
        let t0 = a.view.x_to_tick((x - AUTO_HIT_POINT) as f64);
        let t1 = a.view.x_to_tick((x + AUTO_HIT_POINT) as f64);
        let lo = env.points.partition_point(|p| (p.t as f64) < t0);
        let mut best: Option<(usize, f32)> = None;
        for (j, p) in env.points.iter().enumerate().skip(lo) {
            if p.t as f64 > t1 {
                break;
            }
            let (px, py) = (a.view.tick_to_x(p.t as f64) as f32, g.y_of(p.v));
            let d = ((px - x).powi(2) + (py - y).powi(2)).sqrt();
            if d <= AUTO_HIT_POINT && best.is_none_or(|b| d < b.1) {
                best = Some((j, d));
            }
        }
        best.map(|b| b.0)
    }

    /// Whether (x, y) is on the envelope's line.
    fn on_line(a: &Arrangement, g: &LaneGeo, env: &Envelope, x: f32, y: f32) -> bool {
        let t = a.view.x_to_tick(x as f64).max(0.0);
        let v = env.value_at(t).unwrap_or(g.param.default);
        let v = if g.stepped { au::quantised(&g.param, v) } else { v };
        (g.y_of(v) - y).abs() <= AUTO_HIT_LINE
    }

    /// Hover: the lane and breakpoint under the mouse (redraw on change).
    pub fn hover(&mut self, a: &Arrangement, lanes: &[LaneGeo], x: f32, y: f32) -> bool {
        let now = lanes.iter().find(|g| g.contains(y)).map(|g| {
            let i = a.automation.get(&g.key).and_then(|e| Self::point_at(a, g, e, x, y));
            (g.key.clone(), i)
        });
        let changed = now != self.hover;
        self.hover = now;
        changed
    }

    /// A left press at lane-space (x, y). Returns true when an automation
    /// lane took it (the caller captures the mouse).
    pub fn press(&mut self, cx: &mut EventContext, s: &Signals, lanes: &[LaneGeo], x: f32, y: f32) -> bool {
        let Some(g) = lanes.iter().find(|g| g.contains(y)).cloned() else { return false };
        let mods = *cx.modifiers();
        let draw = s.piano.with_untracked(|p| p.draw);
        let (snap_t, env) = s.arrangement.with_untracked(|a| {
            let t = a.view.x_to_tick(x as f64).max(0.0);
            let t = if crate::input::bypass_snap(&mods) { t.round() as i64 } else { a.grid().round(t.round() as i64) };
            (t.max(0) as u32, a.automation.get(&g.key).cloned())
        });
        cx.emit(crate::AppEvent::EditorFocus(crate::shortcuts::Ctx::Arrangement));
        self.picked = None;
        if draw {
            self.sel = None;
            cx.emit(crate::AppEvent::GestureBegin);
            self.gesture = Some(Gesture::Draw { key: g.key.clone() });
            self.draw_at(s, &g, x, y);
            return true;
        }
        let hit = env.as_ref().and_then(|e| s.arrangement.with_untracked(|a| Self::point_at(a, &g, e, x, y)));
        if let (Some(i), Some(e)) = (hit, env.as_ref()) {
            let p = e.points[i];
            self.sel = None;
            cx.emit(crate::AppEvent::GestureBegin);
            self.gesture = Some(Gesture::Point { key: g.key.clone(), i, press: (x, y), orig: (p.t, p.v), moved: false });
            return true;
        }
        let on_line = env.as_ref().is_none_or(|e| s.arrangement.with_untracked(|a| Self::on_line(a, &g, e, x, y)));
        // Alt-drag on a sloped segment bends it.
        if mods.alt() && !g.stepped {
            if let Some(e) = env.as_ref() {
                let t = s.arrangement.with_untracked(|a| a.view.x_to_tick(x as f64));
                if let Some(i) = e.segment_at(t) {
                    let (p, n) = (e.points[i], e.points[i + 1]);
                    if p.v != n.v {
                        cx.emit(crate::AppEvent::GestureBegin);
                        self.gesture = Some(Gesture::Curve { key: g.key.clone(), i, press_y: y, orig: p.c, rising: n.v > p.v });
                        return true;
                    }
                }
            }
        }
        // Inside the lane's time selection: drag its envelope.
        if let Some(sel) = self.sel.clone().filter(|sl| sl.key == g.key) {
            let t = s.arrangement.with_untracked(|a| a.view.x_to_tick(x as f64));
            if t >= sel.t0 as f64 && t < sel.t1 as f64 && !on_line {
                if let Some(e) = env.clone() {
                    cx.emit(crate::AppEvent::GestureBegin);
                    self.gesture = Some(Gesture::MoveSel { key: g.key.clone(), sel: (sel.t0, sel.t1), press_x: x, base: e });
                    return true;
                }
            }
        }
        if on_line {
            // A click on the line adds a breakpoint (on the line's value; a
            // new envelope starts at the mouse).
            let v = match env.as_ref() {
                Some(e) => e.value_at(snap_t as f64).unwrap_or_else(|| g.v_of(y)),
                None => s.arrangement.with_untracked(|a| static_value(a, s.master_db.get_untracked(), &g)),
            };
            let v = if g.param.stepped { v.round() } else { v };
            cx.emit(crate::AppEvent::GestureBegin);
            let mut i = 0;
            s.arrangement.update(|a| {
                i = a.automation.edit(&g.key).add(snap_t, v);
            });
            self.sel = None;
            self.gesture = Some(Gesture::Added { key: g.key.clone(), i, press: (x, y), orig: (snap_t, v), moved: false });
            return true;
        }
        // Elsewhere: select time in this lane.
        self.sel = None;
        self.gesture = Some(Gesture::Select { key: g.key.clone(), t0: snap_t, press: (x, y), moved: false });
        true
    }

    fn draw_at(&mut self, s: &Signals, g: &LaneGeo, x: f32, y: f32) {
        let v = g.v_of(y);
        s.arrangement.update(|a| {
            let t = a.view.x_to_tick(x as f64).max(0.0) as u32;
            let grid = a.grid().ticks.max(1);
            a.automation.edit(&g.key).draw_steps(t, t + 1, grid, |_| v);
        });
    }

    /// The mouse moved with the button down.
    pub fn drag(&mut self, cx: &mut EventContext, s: &Signals, lanes: &[LaneGeo], x: f32, y: f32) {
        let mods = *cx.modifiers();
        let Some(gest) = self.gesture.as_mut() else { return };
        let key = match gest {
            Gesture::Point { key, .. } | Gesture::Curve { key, .. } | Gesture::Added { key, .. } | Gesture::Draw { key } | Gesture::Select { key, .. } | Gesture::MoveSel { key, .. } => key.clone(),
        };
        let Some(g) = lanes.iter().find(|g| g.key == key).cloned() else { return };
        match gest {
            Gesture::Point { i, press, orig, moved, .. } | Gesture::Added { i, press, orig, moved, .. } => {
                if !*moved && (x - press.0).abs() < 3.0 && (y - press.1).abs() < 3.0 {
                    return;
                }
                *moved = true;
                // Shift: fine vertical moves (a tenth of the mouse's travel).
                let dy = if mods.shift() { (y - press.1) * 0.1 } else { y - press.1 };
                let v = g.v_of(g.y_of(orig.1) + dy);
                let (i, orig, press) = (*i, *orig, *press);
                let mut at = (orig.0, v);
                s.arrangement.update(|a| {
                    let dt = a.view.px_to_ticks((x - press.0) as f64);
                    let t = (orig.0 as f64 + dt).max(0.0);
                    let t = if crate::input::bypass_snap(&mods) || mods.shift() { t.round() as i64 } else { a.grid().round(t.round() as i64) };
                    at = a.automation.edit(&g.key).move_to(i, t.max(0) as u32, v);
                });
                let _ = at;
            }
            Gesture::Curve { i, press_y, orig, rising, .. } => {
                // Up bends a rising segment toward its end value early.
                let d = (*press_y - y) / 60.0 * if *rising { 1.0 } else { -1.0 };
                let c = (*orig + d).clamp(-0.95, 0.95);
                let i = *i;
                s.arrangement.update(|a| a.automation.edit(&g.key).set_curve(i, c));
            }
            Gesture::Draw { .. } => self.draw_at(s, &g, x, y),
            Gesture::Select { t0, press, moved, .. } => {
                if !*moved && (x - press.0).abs() < 3.0 {
                    return;
                }
                *moved = true;
                let t0 = *t0;
                let t = s.arrangement.with_untracked(|a| {
                    let t = a.view.x_to_tick(x as f64).max(0.0) as i64;
                    a.grid().round(t).max(0) as u32
                });
                let (a0, a1) = (t0.min(t), t0.max(t));
                self.sel = (a1 > a0).then(|| LaneSel { key: g.key.clone(), t0: a0, t1: a1 });
            }
            Gesture::MoveSel { sel, press_x, base, .. } => {
                let (sel, press_x, base) = (*sel, *press_x, base.clone());
                let dt = s.arrangement.with_untracked(|a| {
                    let d = a.view.px_to_ticks((x - press_x) as f64) as i64;
                    let g = a.grid().ticks as i64;
                    if crate::input::bypass_snap(&mods) { d } else { (d as f64 / g as f64).round() as i64 * g }
                });
                let dt = dt.max(-(sel.0 as i64));
                s.arrangement.update(|a| {
                    let e = a.automation.edit(&g.key);
                    *e = base.clone();
                    e.shift_range(sel.0, sel.1, dt);
                });
                self.sel = Some(LaneSel { key: g.key.clone(), t0: (sel.0 as i64 + dt) as u32, t1: (sel.1 as i64 + dt) as u32 });
            }
        }
    }

    /// The button came up: finish the gesture (one undo step).
    pub fn release(&mut self, cx: &mut EventContext, s: &Signals) {
        let Some(g) = self.gesture.take() else { return };
        match g {
            Gesture::Point { key, i, moved, .. } => {
                if !moved {
                    // A click on a breakpoint deletes it (VISION.md, R108).
                    s.arrangement.update(|a| {
                        a.automation.edit(&key).delete(i);
                        a.automation.prune();
                    });
                    cx.emit(crate::AppEvent::HoverHelp("Breakpoint deleted".into()));
                } else {
                    self.picked = Some((key, i));
                }
                cx.emit(crate::AppEvent::GestureEnd);
            }
            Gesture::Added { key, i, .. } => {
                self.picked = Some((key, i));
                cx.emit(crate::AppEvent::GestureEnd);
            }
            Gesture::Curve { .. } | Gesture::Draw { .. } | Gesture::MoveSel { .. } => {
                s.arrangement.update(|a| a.automation.prune());
                cx.emit(crate::AppEvent::GestureEnd);
            }
            Gesture::Select { moved, t0, .. } => {
                if !moved {
                    // A click in empty lane space: the insert marker, as in
                    // the clip area.
                    let lane = s.arrangement.with_untracked(|a| a.insert.1);
                    cx.emit(crate::AppEvent::SetInsert(t0, lane));
                }
            }
        }
        if crate::dev::enabled() {
            let n = s.arrangement.with_untracked(|a| a.automation.point_count());
            eprintln!("dev: automation points {n}");
        }
    }

    /// A double-click away from the line adds a breakpoint at the mouse
    /// (R106). Returns whether a lane took it.
    pub fn double_click(&mut self, cx: &mut EventContext, s: &Signals, lanes: &[LaneGeo], x: f32, y: f32) -> bool {
        let Some(g) = lanes.iter().find(|g| g.contains(y)).cloned() else { return false };
        if s.piano.with_untracked(|p| p.draw) {
            return true;
        }
        let mods = *cx.modifiers();
        let on_point = s.arrangement.with_untracked(|a| a.automation.get(&g.key).and_then(|e| Self::point_at(a, &g, e, x, y)).is_some());
        if on_point {
            return true;
        }
        let v = g.v_of(y);
        s.arrangement.update(|a| {
            let t = a.view.x_to_tick(x as f64).max(0.0);
            let t = if crate::input::bypass_snap(&mods) { t.round() as i64 } else { a.grid().round(t.round() as i64) };
            a.automation.edit(&g.key).add(t.max(0) as u32, v);
        });
        cx.emit(crate::AppEvent::Commit);
        true
    }
}

// ───────────────────────────── Lane headers ─────────────────────────────

/// A track's automation lane headers, under its row in the header column:
/// the parameter chooser ("Device ▸ Parameter"), the value at the playhead,
/// − (hide the lane) and, on the last one, + (another parameter).
pub struct AutoLaneHeaders {
    s: Signals,
    lane: usize,
    hover: Option<(usize, HeaderPart)>,
}

impl AutoLaneHeaders {
    pub fn new(cx: &mut Context, s: Signals, lane: usize) -> Handle<'_, Self> {
        Self { s, lane, hover: None }
            .build(cx, |_| {})
            .width(Stretch(1.0))
            .bind(s.arrangement, |mut h| h.needs_redraw())
            .bind(s.transport, |mut h| h.needs_redraw())
    }

    fn geos(&self) -> Vec<LaneGeo> {
        self.s.arrangement.with_untracked(|a| lanes(a).into_iter().filter(|g| g.track == self.lane).collect())
    }

    /// (lane k, part) under the mouse.
    fn at(&self, cx: &EventContext) -> Option<(usize, HeaderPart)> {
        let b = cx.bounds();
        let s = cx.scale_factor();
        let (x, y) = ((cx.mouse().cursor_x - b.x) / s, (cx.mouse().cursor_y - b.y) / s);
        let geos = self.geos();
        let pitch = self.s.arrangement.with_untracked(|a| a.lanes.get(self.lane).map_or(1.0, |l| l.pitch()));
        let k = (y / pitch).floor().max(0.0) as usize;
        let last = k + 1 == geos.len();
        geos.get(k)?;
        header_part(b.w / s, x, y - k as f32 * pitch, last).map(|p| (k, p))
    }
}

impl View for AutoLaneHeaders {
    fn element(&self) -> Option<&'static str> {
        Some("auto-lane-headers")
    }

    fn event(&mut self, cx: &mut EventContext, event: &mut Event) {
        event.map(|we, meta| match we {
            WindowEvent::MouseMove(..) => {
                let now = self.at(cx);
                if now != self.hover {
                    self.hover = now;
                    cx.needs_redraw();
                    let geos = self.geos();
                    let help = match now {
                        Some((k, HeaderPart::Chooser)) => geos.get(k).map(|g| format!("{}: choose the parameter this lane shows (Show Automated Parameters Only is in the list)", lane_label(&g.param))),
                        Some((_, HeaderPart::Add)) => Some("Show another parameter in a new lane".into()),
                        Some((_, HeaderPart::Remove)) => Some("Hide this lane (its automation stays)".into()),
                        None => geos.first().map(|_| "An automation lane: its parameter and the value at the playhead".into()),
                    };
                    cx.emit(crate::AppEvent::HoverHelp(help.unwrap_or_default()));
                }
            }
            WindowEvent::MouseLeave => {
                if self.hover.take().is_some() {
                    cx.needs_redraw();
                }
                cx.emit(crate::AppEvent::HoverHelp(String::new()));
            }
            WindowEvent::MouseDown(MouseButton::Left) => {
                let s = cx.scale_factor();
                match self.at(cx) {
                    Some((k, HeaderPart::Chooser)) => {
                        cx.emit(crate::AppEvent::ContextMenu(crate::edit::MenuSite::AutoParam(self.lane, k), cx.mouse().cursor_x / s, cx.mouse().cursor_y / s));
                    }
                    Some((_, HeaderPart::Add)) => cx.emit(crate::AppEvent::Menu(crate::state::MenuAction::AutoLaneAdd(self.lane))),
                    Some((k, HeaderPart::Remove)) => cx.emit(crate::AppEvent::Menu(crate::state::MenuAction::AutoLaneRemove(self.lane, k))),
                    None => {}
                }
                meta.consume();
            }
            _ => {}
        });
    }

    fn draw(&self, cx: &mut DrawContext, canvas: &vg::Canvas) {
        let b = cx.bounds();
        if b.h < 1.0 {
            return;
        }
        let s = cx.scale_factor();
        let geos = self.geos();
        let (pitch, playhead) = (self.s.arrangement.with_untracked(|a| a.lanes.get(self.lane).map_or(1.0, |l| l.pitch())), self.s.transport.with_untracked(|t| t.playhead_ticks));
        let master = self.s.master_db.get_untracked();
        let hl = gfx::hairline(s);
        for (k, g) in geos.iter().enumerate() {
            let top = b.y + k as f32 * pitch * s;
            let r = vg::Rect::from_xywh(b.x, top.round(), b.w, (g.h * s).round());
            let value = self.s.arrangement.with_untracked(|a| match a.automation.get(&g.key) {
                Some(e) if !g.overridden => {
                    let v = e.value_at(playhead).unwrap_or(0.0);
                    if g.stepped { crate::automation::quantised(&g.param, v) } else { v }
                }
                _ => static_value(a, master, g),
            });
            let hover = self.hover.filter(|h| h.0 == k).map(|h| h.1);
            draw_header(canvas, g, value, r, s, k + 1 == geos.len(), hover);
            canvas.draw_rect(vg::Rect::from_ltrb(b.x, r.bottom, b.x + b.w, r.bottom + hl), &gfx::fill(LINE));
        }
    }
}

/// Copies a selected lane time's envelope (relative points).
pub fn copy(a: &Arrangement, sel: &LaneSel) -> Vec<Pt> {
    a.automation.get(&sel.key).map(|e| e.copy_range(sel.t0, sel.t1)).unwrap_or_default()
}
