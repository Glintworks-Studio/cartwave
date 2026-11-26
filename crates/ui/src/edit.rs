//! Batch 3 (UX.md §5): Ableton-parity editing in the arrangement and the
//! commands around it: the clip clipboard, split and consolidate, the time
//! commands, zoom, the grid keys, the insert marker, in-place rename, track
//! selection and the context menus. Piano-roll commands are forwarded to the
//! piano roll view (`PianoCmd`), which owns the note operations.
//!
//! Every edit goes through the model operations in `state.rs` (unit-tested)
//! and ends in one `commit()`: one undo step, and the dirty flag.

use vizia::prelude::*;

use crate::shortcuts::{self, Action, Ctx};
use crate::state::{self, ContextMenuSpec, MenuAction, MenuItem, SubMenu, TimeSel, TICKS_PER_BAR};
use crate::theme::tokens::Rgba;
use crate::{AppState, PianoCmd};

/// Where a context menu was opened.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum MenuSite {
    Clip(u32),
    /// Empty lane space or the time selection.
    Empty,
    Ruler,
    Track(usize),
    /// The piano roll, on a note or not.
    Piano(bool),
    /// A user preset in the browser (browser index).
    Preset(usize),
    /// The spectrum well: its styles.
    Spectrum,
    /// Settings ▸ Editing: the default track colour's grid.
    DefaultTrackColour,
    /// The status bar's warning icon: the persistent problems (polish 2 §6.4).
    Problems,
    /// The status bar's message, whole (polish 2 §6.2).
    Message,
}

/// What an in-place rename edits.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum RenameTarget {
    Clip(u32),
    Track(usize),
    Preset(usize),
    /// The display's BPM (a double-click types a tempo).
    Bpm,
    /// A track's volume / pan value (a double-click types it, polish 2 §3).
    Volume(usize),
    Pan(usize),
}

/// The open rename field: its target, place (window logical px) and the
/// text it starts with.
#[derive(Clone, Debug, PartialEq)]
pub struct RenameSpec {
    pub target: RenameTarget,
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub text: String,
}

/// Height of the rename field (logical px).
pub const RENAME_H: f32 = 24.0;

// Why a command can't run (F-24: disabled items say why).
pub const WHY_NO_CLIPS: &str = "Select one or more clips first.";
pub const WHY_NO_RANGE: &str = "Select clips, or drag across empty lane space to select time.";
pub const WHY_NO_TIMESEL: &str = "Drag across empty lane space to select time first.";
pub const WHY_EMPTY_CLIPBOARD: &str = "Copy or cut clips (or time) first.";
pub const WHY_EMPTY_NOTE_CLIPBOARD: &str = "Copy or cut notes first.";
pub const WHY_NO_NOTES: &str = "Select one or more notes first.";
pub const WHY_ONE_CLIP: &str = "Select at least two clips on a track, or a time range.";
pub const WHY_NOT_UNIQUE: &str = "The clip already has an instrument of its own that no other clip shares.";
pub const WHY_TRACK_INST: &str = "The clip already plays its track's instrument.";

/// Piano-roll commands the root forwards to the piano roll view.
fn piano_action(a: Action) -> bool {
    use Action::*;
    matches!(
        a,
        Cut | Copy
            | Paste
            | Duplicate
            | Delete
            | SelectAll
            | Quantize
            | Split
            | Consolidate
            | ZoomIn
            | ZoomOut
            | ZoomToSelection
            | ZoomBack
            | GridNarrow
            | GridWiden
            | GridTriplet
            | GridSnap
            | GridAdaptive
            | TransposeUp
            | TransposeDown
            | OctaveUp
            | OctaveDown
            | NudgeLeft
            | NudgeRight
            | Shorten
            | Lengthen
            | PrevNote
            | NextNote
    )
}

impl AppState {
    /// The piano roll is the editor that takes edit commands.
    pub(crate) fn piano_active(&self) -> bool {
        self.s.piano_open.get_untracked() && self.editor == Ctx::PianoRoll
    }

    /// Tracks that take edits (F-04: not the dimmed VRC6 lanes).
    fn lane_open(&self) -> impl Fn(usize) -> bool {
        let chip = self.s.chip.get_untracked();
        let off: Vec<bool> = self.s.arrangement.with_untracked(|a| (0..a.lanes.len()).map(|l| state::lane_off(chip, a, l)).collect());
        move |l| !off.get(l).copied().unwrap_or(false)
    }

    /// The time selection, else the selected clips' span.
    pub(crate) fn time_range(&self) -> Option<(u32, u32)> {
        self.s.time_sel.get_untracked().map(|t| (t.start, t.end)).or_else(|| self.s.arrangement.with_untracked(|a| a.selection_span()))
    }

    /// Moves the insert marker (and, while stopped, the playhead).
    pub(crate) fn set_insert(&mut self, tick: u32, lane: usize) {
        self.s.arrangement.update(|a| a.insert = (tick, lane.min(a.lanes.len().saturating_sub(1))));
        if !self.s.transport.with_untracked(|t| t.playing) {
            self.send(engine::Command::Seek(tick as f64));
            self.s.transport.update(|t| t.playhead_ticks = tick as f64);
        }
    }

    /// Batch 3 actions, from a key or a menu. `None`: not one of them.
    pub(crate) fn run_b3(&mut self, cx: &mut EventContext, a: Action) -> Option<bool> {
        if self.piano_active() && piano_action(a) {
            cx.emit_custom(Event::new(PianoCmd(a)).target(Entity::root()).propagate(Propagation::Subtree));
            return Some(true);
        }
        match a {
            Action::Copy => self.arr_copy(false),
            Action::Cut => self.arr_copy(true),
            Action::Paste => self.arr_paste(),
            Action::Duplicate => self.arr_duplicate(),
            Action::Delete => self.arr_delete(),
            Action::CutTime | Action::CopyTime | Action::PasteTime | Action::DuplicateTime | Action::DeleteTime | Action::InsertSilence => self.time_cmd(a),
            Action::Split => self.split(),
            Action::Consolidate => self.consolidate(),
            Action::Rename => self.rename_selected(),
            Action::ZoomIn | Action::ZoomOut | Action::ZoomToSelection | Action::ZoomBack | Action::ZoomToFit | Action::FitSong => self.zoom(a),
            Action::GridNarrow | Action::GridWiden | Action::GridTriplet | Action::GridSnap | Action::GridAdaptive => self.grid_cmd(a),
            Action::DrawMode => {
                self.s.piano.update(|p| p.draw = !p.draw);
                let on = self.s.piano.with_untracked(|p| p.draw);
                self.set_status(if on { "Draw Mode on: a click in the piano roll draws a note." } else { "Draw Mode off: click selects, drag selects several, double-click adds or deletes a note." }.into());
            }
            Action::MarkerLeft | Action::MarkerRight => self.marker_step(if a == Action::MarkerLeft { -1 } else { 1 }),
            Action::TrackUp | Action::TrackDown => self.track_step(if a == Action::TrackUp { -1 } else { 1 }),
            Action::Continue => {
                if !self.s.transport.with_untracked(|t| t.playing) {
                    let at = self.stop_point;
                    self.send(engine::Command::Seek(at));
                    self.send(engine::Command::Play);
                    self.s.transport.update(|t| {
                        t.playhead_ticks = at;
                        t.playing = true;
                    });
                }
            }
            Action::GettingStarted => self.s.guide.set(true),
            _ => return None,
        }
        Some(true)
    }

    // ───────────────────────────── Clipboard ─────────────────────────────

    /// Cmd+C / Cmd+X: the time selection's contents on its tracks, else the
    /// selected clips (A9).
    fn arr_copy(&mut self, cut: bool) {
        let ts = self.s.time_sel.get_untracked();
        let cb = self.s.arrangement.with_untracked(|a| match ts {
            Some(t) => Some(a.copy_range(t.start, t.end, t.lane0..=t.lane1, false)),
            None => a.copy_clips(&a.selection),
        });
        let Some(cb) = cb.filter(|c| !c.clips.is_empty() || ts.is_some()) else {
            return self.set_status(WHY_NO_RANGE.into());
        };
        let n = cb.clips.len();
        self.clip_board = Some(cb);
        if cut {
            self.arr_delete();
        } else {
            self.set_status(format!("Copied {n} clip(s)."));
        }
    }

    /// Cmd+V: at the insert marker, on its track; repeated pastes follow on
    /// (the marker moves to the end of what was pasted).
    fn arr_paste(&mut self) {
        let Some(cb) = self.clip_board.clone() else { return self.set_status(WHY_EMPTY_CLIPBOARD.into()) };
        let (at, lane) = self.s.arrangement.with_untracked(|a| a.insert);
        let open = self.lane_open();
        let mut n = 0;
        self.s.arrangement.update(|a| n = a.paste(&cb, at, lane, &open).len());
        self.s.time_sel.set(None);
        self.set_insert(at.saturating_add(cb.len), lane);
        self.commit();
        if n < cb.clips.len() {
            self.set_status(format!("Pasted {n} of {} clip(s): the rest would land past the last track or on a dimmed VRC6 track.", cb.clips.len()));
        }
    }

    /// Cmd+D: copies of the selected clips, or of the time selection's
    /// contents, placed right after it (A10).
    fn arr_duplicate(&mut self) {
        if let Some(t) = self.s.time_sel.get_untracked() {
            let open = self.lane_open();
            let len = t.end - t.start;
            self.s.arrangement.update(|a| {
                let cb = a.copy_range(t.start, t.end, t.lane0..=t.lane1, false);
                a.paste(&cb, t.end, t.lane0, &open);
            });
            self.s.time_sel.set(Some(TimeSel { start: t.end, end: t.end + len, ..t }));
            self.commit();
        } else if self.s.arrangement.with_untracked(|a| !a.selection.is_empty()) {
            self.s.arrangement.update(|a| a.duplicate_selected());
            self.commit();
        } else {
            self.set_status(WHY_NO_RANGE.into());
        }
    }

    /// Del: the time selection's contents (clips split at its edges), else
    /// the selected clips (A11, F-22).
    fn arr_delete(&mut self) {
        if let Some(t) = self.s.time_sel.get_untracked() {
            self.s.arrangement.update(|a| {
                a.clear_range(t.start, t.end, t.lane0..=t.lane1);
            });
        } else {
            self.s.arrangement.update(|a| a.delete_selected());
        }
        self.commit();
    }

    /// The …Time commands (R46) and Insert Silence (R47): every track.
    fn time_cmd(&mut self, a: Action) {
        let lanes = self.s.arrangement.with_untracked(|a| a.lanes.len());
        let open = self.lane_open();
        match a {
            Action::PasteTime => {
                let Some(cb) = self.clip_board.clone() else { return self.set_status(WHY_EMPTY_CLIPBOARD.into()) };
                let (at, lane) = self.s.arrangement.with_untracked(|a| a.insert);
                self.s.arrangement.update(|a| {
                    a.insert_time(at, cb.len);
                    a.paste(&cb, at, lane, &open);
                });
                self.s.time_sel.set(Some(TimeSel { start: at, end: at + cb.len, lane0: 0, lane1: lanes - 1 }));
            }
            Action::InsertSilence => {
                // At the time selection, as long as it is (R47).
                let Some(t) = self.s.time_sel.get_untracked() else { return self.set_status(format!("{WHY_NO_TIMESEL} Insert Silence inserts that much empty time there.")) };
                self.s.arrangement.update(|a| a.insert_time(t.start, t.end - t.start));
            }
            _ => {
                let Some((start, end)) = self.time_range() else { return self.set_status(WHY_NO_RANGE.into()) };
                let len = end - start;
                match a {
                    Action::CopyTime => {
                        self.clip_board = Some(self.s.arrangement.with_untracked(|a| a.copy_range(start, end, 0..=lanes - 1, true)));
                        return self.set_status("Copied the time selection on every track.".into());
                    }
                    Action::CutTime => {
                        self.clip_board = Some(self.s.arrangement.with_untracked(|a| a.copy_range(start, end, 0..=lanes - 1, true)));
                        self.s.arrangement.update(|a| a.delete_time(start, end));
                        self.s.time_sel.set(None);
                    }
                    Action::DeleteTime => {
                        self.s.arrangement.update(|a| a.delete_time(start, end));
                        self.s.time_sel.set(None);
                    }
                    _ => {
                        // Duplicate Time: a copy of the range inserted after it.
                        self.s.arrangement.update(|a| {
                            let cb = a.copy_range(start, end, 0..=lanes - 1, true);
                            a.insert_time(end, len);
                            a.paste(&cb, end, 0, &open);
                        });
                        self.s.time_sel.set(Some(TimeSel { start: end, end: end + len, lane0: 0, lane1: lanes - 1 }));
                    }
                }
            }
        }
        self.commit();
    }

    // ───────────────────────────── Split / consolidate ─────────────────────────────

    /// Cmd+E (R38): the time selection becomes its own clip(s); else the
    /// selected clips (or, with none, the clips on the marker's track) are
    /// split at the insert marker.
    fn split(&mut self) {
        if let Some(t) = self.s.time_sel.get_untracked() {
            self.s.arrangement.update(|a| {
                let ids = a.isolate(t.start, t.end, t.lane0..=t.lane1);
                a.selection = ids;
            });
            self.s.time_sel.set(None);
            return self.commit();
        }
        let (at, lane) = self.s.arrangement.with_untracked(|a| a.insert);
        let mut n = 0;
        self.s.arrangement.update(|a| {
            let sel = a.selection.clone();
            let ids: Vec<u32> = a
                .clips
                .iter()
                .filter(|c| if sel.is_empty() { c.lane == lane } else { sel.contains(&c.id) })
                .filter(|c| c.start_ticks < at && c.end_ticks() > at)
                .map(|c| c.id)
                .collect();
            for id in ids {
                if let Some(r) = a.split_clip(id, at) {
                    n += 1;
                    if sel.contains(&id) {
                        a.selection.push(r);
                    }
                }
            }
        });
        if n == 0 {
            return self.set_status("Nothing to split: put the insert marker inside a clip (click it), or select time.".into());
        }
        self.commit();
    }

    /// Cmd+J (R39): the selected clips on each track become one clip; with
    /// a time selection, its contents on each track (spanning it exactly).
    fn consolidate(&mut self) {
        let ts = self.s.time_sel.get_untracked();
        let mut res = (Vec::new(), Vec::new());
        self.s.arrangement.update(|a| {
            res = match ts {
                Some(t) => {
                    let ids = a.isolate(t.start, t.end, t.lane0..=t.lane1);
                    a.consolidate(&ids, Some((t.start, t.end)))
                }
                None => {
                    let sel = a.selection.clone();
                    a.consolidate(&sel, None)
                }
            };
        });
        let (made, mixed) = res;
        if ts.is_some() && !made.is_empty() {
            self.s.time_sel.set(None);
        }
        if !mixed.is_empty() {
            self.set_status(state::CONSOLIDATE_MIXED.into());
        } else if made.is_empty() {
            return self.set_status(WHY_ONE_CLIP.into());
        }
        self.commit();
    }

    // ───────────────────────────── Zoom and the grid ─────────────────────────────

    /// Z / X / + / − / Zoom to Fit in the arrangement (A14, R30–R34).
    fn zoom(&mut self, a: Action) {
        let view = self.s.arrangement.with_untracked(|a| a.view);
        let w = self.s.arrangement.with_untracked(|a| a.view_w);
        match a {
            Action::ZoomIn | Action::ZoomOut => {
                // Around the insert marker when it's in view, else the centre.
                let at = self.s.arrangement.with_untracked(|a| a.view.tick_to_x(a.insert.0 as f64));
                let anchor = if (0.0..=w as f64).contains(&at) { at } else { w as f64 * 0.5 };
                let f = if a == Action::ZoomIn { ZOOM_STEP } else { 1.0 / ZOOM_STEP };
                self.s.arrangement.update(|a| a.view.zoom_at(anchor, f));
            }
            Action::ZoomBack => match self.zoom_back.pop() {
                Some(v) => {
                    self.s.arrangement.update(|a| a.view = v);
                    if crate::dev::enabled() {
                        eprintln!("dev: zoom ZoomBack px_per_bar {:.2} scroll {:.0}", v.px_per_bar(), v.scroll_ticks);
                    }
                }
                None => self.set_status("No earlier zoom to go back to.".into()),
            },
            // W: the whole song; pressed again (nothing moved since), back (P2-R: it toggles).
            Action::FitSong => {
                let end = self.song_fit_end();
                let v = state::view_for(0, end, w);
                if v == view && !self.zoom_back.is_empty() {
                    return self.zoom(Action::ZoomBack);
                }
                self.zoom_back.push(view);
                if self.zoom_back.len() > MAX_ZOOM_BACK {
                    self.zoom_back.remove(0);
                }
                self.s.arrangement.update(|a| a.view = v);
                if crate::dev::enabled() {
                    eprintln!("dev: zoom FitSong range {:?} px_per_bar {:.2} scroll {:.0}", (0, end), v.px_per_bar(), v.scroll_ticks);
                }
            }
            _ => {
                // Z: the selection. Zoom to Fit (a double-click on the
                // ruler, R32): the time selection if there is one, else the
                // whole song.
                let range = if a == Action::ZoomToSelection {
                    match self.time_range() {
                        Some(r) => r,
                        None => return self.set_status(WHY_NO_RANGE.into()),
                    }
                } else {
                    self.s.time_sel.get_untracked().map(|t| (t.start, t.end)).unwrap_or_else(|| (0, self.song_fit_end()))
                };
                self.zoom_back.push(view);
                if self.zoom_back.len() > MAX_ZOOM_BACK {
                    self.zoom_back.remove(0);
                }
                let v = state::view_for(range.0, range.1, w);
                self.s.arrangement.update(|a| a.view = v);
                if crate::dev::enabled() {
                    eprintln!("dev: zoom {:?} range {range:?} px_per_bar {:.2} scroll {:.0}", a, v.px_per_bar(), v.scroll_ticks);
                }
            }
        }
    }

    /// The end of "the whole song": the last clip or the loop end, at least four bars.
    pub(crate) fn song_fit_end(&self) -> u32 {
        let loop_end = self.s.transport.with_untracked(|t| t.loop_end_ticks) as u32;
        self.s.arrangement.with_untracked(|a| a.song_end()).max(loop_end).max(4 * TICKS_PER_BAR)
    }

    /// Cmd+1 … Cmd+5 in the arrangement (A17, R56–R59).
    fn grid_cmd(&mut self, a: Action) {
        self.s.arrangement.update(|ar| {
            let o = &mut ar.grid_opts;
            match a {
                Action::GridNarrow if o.adaptive => o.density = (o.density - 1).max(-2),
                Action::GridWiden if o.adaptive => o.density = (o.density + 1).min(2),
                Action::GridNarrow => ar.snap = ar.snap.step(-1),
                Action::GridWiden => ar.snap = ar.snap.step(1),
                Action::GridTriplet => o.triplet = !o.triplet,
                Action::GridSnap => o.snap = !o.snap,
                _ => o.adaptive = !o.adaptive,
            }
        });
        let label = self.s.arrangement.with_untracked(|a| a.grid_label());
        self.set_status(format!("Arrangement grid: {label}"));
    }

    // ───────────────────────────── Keyboard navigation ─────────────────────────────

    /// ← → (R50, R51): nudge the selected clips by the grid, else move the
    /// insert marker.
    fn marker_step(&mut self, dir: i64) {
        let g = self.s.arrangement.with_untracked(|a| a.grid().ticks) as i64;
        if self.s.arrangement.with_untracked(|a| !a.selection.is_empty()) {
            let mut moved = false;
            self.s.arrangement.update(|a| moved = a.nudge_selected(dir * g));
            if moved {
                self.commit();
            }
            return;
        }
        let (at, lane) = self.s.arrangement.with_untracked(|a| a.insert);
        let t = (at as i64 + dir * g).clamp(0, state::MAX_SONG_TICKS as i64) as u32;
        self.set_insert(t, lane);
    }

    /// ↑ ↓ (R50): the track above / below becomes the selected track.
    fn track_step(&mut self, dir: i32) {
        let (n, cur) = self.s.arrangement.with_untracked(|a| (a.lanes.len() as i32, a.track_sel.first().copied().unwrap_or(a.insert.1) as i32));
        let open = self.lane_open();
        let mut l = cur + dir;
        while (0..n).contains(&l) && !open(l as usize) {
            l += dir;
        }
        if !(0..n).contains(&l) {
            return;
        }
        self.select_track(l as usize, false);
    }

    /// T1: a click on a track header (Shift / Ctrl adds or removes it).
    pub(crate) fn select_track(&mut self, lane: usize, add: bool) {
        self.s.arrangement.update(|a| {
            if add {
                if let Some(i) = a.track_sel.iter().position(|l| *l == lane) {
                    a.track_sel.remove(i);
                } else {
                    a.track_sel.push(lane);
                }
            } else {
                a.track_sel = vec![lane];
            }
            a.selection.clear();
            a.insert.1 = lane;
        });
        self.s.time_sel.set(None);
        // I1: the inspector shows the track's instrument.
        if self.s.lane_sel.get_untracked() != lane {
            self.s.dials.set(self.dial_values(lane));
            self.s.lane_sel.set(lane);
        }
    }

    // ───────────────────────────── Rename (A13, T2) ─────────────────────────────

    /// Cmd+R: the selected clip, else the selected track.
    fn rename_selected(&mut self) {
        let (clip, track) = self.s.arrangement.with_untracked(|a| (a.selection.first().copied(), a.track_sel.first().copied()));
        match (clip, track) {
            (Some(id), _) => self.open_rename(RenameTarget::Clip(id)),
            (None, Some(l)) => self.open_rename(RenameTarget::Track(l)),
            _ => self.set_status("Select a clip or a track to rename (Rename renames the selection).".into()),
        }
    }

    /// Opens the rename field over the item (window logical px).
    pub(crate) fn open_rename(&mut self, target: RenameTarget) {
        self.s.menu.set(None);
        let spec = match target {
            RenameTarget::Clip(id) => {
                let Some((lane, start, end, name)) = self.s.arrangement.with_untracked(|a| a.clip(id).map(|c| (c.lane, c.start_ticks, c.end_ticks(), c.name.clone()))) else { return };
                // Bring the clip's start into view first.
                let (w, view) = self.s.arrangement.with_untracked(|a| (a.view_w, a.view));
                let x0 = view.tick_to_x(start as f64);
                if x0 < 0.0 || x0 > w as f64 - 40.0 {
                    self.s.arrangement.update(|a| a.view.scroll_ticks = (start as f64 - a.view.px_to_ticks(40.0)).max(0.0));
                }
                let (Some((left, right, s)), Some(row)) = (crate::views::arrangement::arrangement_at(), crate::views::browser::lane_row(lane)) else { return };
                let view = self.s.arrangement.with_untracked(|a| a.view);
                let x0 = (left / s + view.tick_to_x(start as f64) as f32).max(left / s);
                let x1 = (left / s + view.tick_to_x(end as f64) as f32).min(right / s);
                RenameSpec { target, x: x0, y: row.y / s + RENAME_INSET, w: (x1 - x0).clamp(RENAME_MIN_W, RENAME_MAX_W), text: name }
            }
            RenameTarget::Track(lane) => {
                let Some(row) = crate::views::browser::lane_row(lane) else { return };
                let s = self.scale;
                let name = self.s.arrangement.with_untracked(|a| a.lanes.get(lane).map(|l| l.name.clone())).unwrap_or_default();
                self.select_track(lane, false);
                // Over the number and name (its column moves with the layout).
                let l = self.s.header.get_untracked();
                let x = if l.mirrored() { l.name_x() - RENAME_INSET } else { l.name_x() - (crate::views::header_layout::NAME_X - RENAME_TRACK_X) };
                RenameSpec { target, x: row.x / s + x, y: row.y / s + RENAME_INSET, w: RENAME_TRACK_W, text: name }
            }
            RenameTarget::Volume(lane) | RenameTarget::Pan(lane) => {
                let Some(row) = crate::views::browser::lane_row(lane) else { return };
                let s = self.scale;
                let l = self.s.header.get_untracked();
                let (h, text) = self.s.arrangement.with_untracked(|a| {
                    let t = &a.lanes[lane];
                    (t.height, if matches!(target, RenameTarget::Volume(_)) { crate::state::format_db(t.vol_db) } else { crate::state::format_pan(t.pan) })
                });
                let (x, y, w, lh) = if matches!(target, RenameTarget::Volume(_)) {
                    crate::views::chrome::db_value_rect(&l, h)
                } else {
                    crate::views::chrome::pan_value_rect(&l)
                };
                let fw = RENAME_VALUE_W.max(w);
                RenameSpec { target, x: row.x / s + x + (w - fw) * 0.5, y: row.y / s + y + (lh - RENAME_H) * 0.5, w: fw, text }
            }
            RenameTarget::Preset(i) => {
                let Some(p) = self.preset(i) else { return };
                let (x, y) = self.preset_rename_at;
                RenameSpec { target, x, y, w: RENAME_TRACK_W + 40.0, text: p.name }
            }
            RenameTarget::Bpm => {
                let (x, y, w) = self.bpm_rename_at;
                let bpm = self.s.transport.with_untracked(|t| t.bpm);
                RenameSpec { target, x, y, w, text: format!("{bpm:.2}") }
            }
        };
        self.rename_text = spec.text.clone();
        self.s.rename.set(Some(spec));
    }

    /// Enter, Tab (`next`: the next track, R9) or a click elsewhere.
    pub(crate) fn rename_commit(&mut self, next: bool) {
        let Some(spec) = self.s.rename.get_untracked() else { return };
        self.s.rename.set(None);
        let text: String = self.rename_text.trim().chars().take(crate::user_presets::MAX_NAME).collect();
        if text.is_empty() || text == spec.text {
            if let (true, RenameTarget::Track(l)) = (next, spec.target) {
                self.rename_next(l);
            }
            return;
        }
        match spec.target {
            RenameTarget::Clip(id) => {
                self.s.arrangement.update(|a| {
                    if let Some(c) = a.clip_mut(id) {
                        c.name = text;
                    }
                });
                self.commit();
            }
            RenameTarget::Track(l) => {
                self.s.arrangement.update(|a| {
                    if let Some(lane) = a.lanes.get_mut(l) {
                        lane.name = text;
                    }
                });
                self.commit();
                if next {
                    self.rename_next(l);
                }
            }
            RenameTarget::Volume(l) => match crate::state::parse_db(&text) {
                Some(db) => {
                    let gain_was = self.s.arrangement.with_untracked(|a| a.lanes[l].vol_db);
                    if gain_was != db {
                        self.s.arrangement.update(|a| a.lanes[l].vol_db = db);
                        self.commit();
                    }
                }
                None => self.set_status(format!("\"{text}\" isn't a volume. Type dB, like -3, +2.5 or -inf.")),
            },
            RenameTarget::Pan(l) => match crate::state::parse_pan(&text) {
                Some(p) => {
                    if self.s.arrangement.with_untracked(|a| a.lanes[l].pan) != p {
                        self.s.arrangement.update(|a| a.lanes[l].pan = p);
                        self.commit();
                    }
                }
                None => self.set_status(format!("\"{text}\" isn't a pan value. Type C, 25L, 30R, -25 or +25.")),
            },
            RenameTarget::Bpm => match crate::views::topbar::parse_bpm(&text) {
                Some(b) => {
                    self.send(engine::Command::Bpm(b as f64));
                    self.s.transport.update(|t| t.bpm = b);
                    self.commit();
                }
                None => self.set_status(format!("\"{text}\" isn't a tempo. Type a number of beats per minute.")),
            },
            RenameTarget::Preset(i) => {
                let user = i.saturating_sub(chip::presets::PRESETS.len());
                match crate::user_presets::rename(user, &text) {
                    Ok(n) => {
                        self.s.user_presets.set(crate::user_presets::load());
                        self.set_status(format!("Renamed the preset to \"{n}\"."));
                    }
                    Err(e) => self.set_status(e),
                }
            }
        }
    }

    fn rename_next(&mut self, lane: usize) {
        let n = self.s.arrangement.with_untracked(|a| a.lanes.len());
        if lane + 1 < n {
            self.open_rename(RenameTarget::Track(lane + 1));
        }
    }

    // ───────────────────────────── Context menus ─────────────────────────────

    /// Builds the menu for a right-click at (x, y) (window logical px).
    pub(crate) fn open_context_menu(&mut self, site: MenuSite, x: f32, y: f32) {
        use Action as A;
        let has_cb = self.clip_board.is_some();
        let ts = self.s.time_sel.get_untracked();
        let (sel, multi_lane_pairs) = self.s.arrangement.with_untracked(|a| {
            let n = a.selection.len();
            let mut lanes: Vec<usize> = a.selection.iter().filter_map(|id| a.clip(*id)).map(|c| c.lane).collect();
            let total = lanes.len();
            lanes.sort_unstable();
            lanes.dedup();
            (n, total > lanes.len())
        });
        let items: Vec<MenuItem> = match site {
            MenuSite::Clip(_) => {
                let (unique_off, lane_off) = self.instrument_menu_state();
                vec![
                    MenuItem::cmd(A::Cut),
                    MenuItem::cmd(A::Copy),
                    MenuItem::cmd(A::Paste).off_if(!has_cb, WHY_EMPTY_CLIPBOARD),
                    MenuItem::cmd(A::Duplicate),
                    MenuItem::cmd(A::Delete),
                    MenuItem::sep(),
                    MenuItem::cmd(A::Split).off_if(!self.marker_in_selection(), "Click inside the clip where it should split (that puts the insert marker there)."),
                    MenuItem::cmd(A::Consolidate).off_if(!multi_lane_pairs, WHY_ONE_CLIP),
                    MenuItem::cmd(A::Rename).off_if(sel != 1, "Rename works on one clip at a time."),
                    MenuItem::new(MenuAction::Sub(SubMenu::ClipColor), "Colour"),
                    MenuItem::sep(),
                    MenuItem::new(MenuAction::LoopClips, "Loop Clip").key(shortcuts::label(A::LoopSelection)),
                    MenuItem::sep(),
                    MenuItem::new(MenuAction::MakeUnique, "Make Instrument Unique").off_if(unique_off, WHY_NOT_UNIQUE),
                    MenuItem::new(MenuAction::LaneInstrument, "Use Track Instrument").off_if(lane_off, WHY_TRACK_INST),
                    MenuItem::new(MenuAction::SavePreset, "Save Instrument as Preset"),
                ]
            }
            MenuSite::Empty => vec![
                MenuItem::cmd(A::Paste).off_if(!has_cb, WHY_EMPTY_CLIPBOARD),
                MenuItem::cmd(A::InsertClip).key(shortcuts::label(A::InsertClip)).off_if(ts.is_none(), WHY_NO_TIMESEL),
                MenuItem::cmd(A::LoopSelection).off_if(ts.is_none(), WHY_NO_TIMESEL),
                MenuItem::cmd(A::InsertSilence).off_if(ts.is_none(), WHY_NO_TIMESEL),
                MenuItem::cmd(A::DeleteTime).off_if(ts.is_none(), WHY_NO_TIMESEL),
                MenuItem::sep(),
                MenuItem::new(MenuAction::Sub(SubMenu::Grid), format!("Grid ({})", self.s.arrangement.with_untracked(|a| a.grid_label()))),
            ],
            MenuSite::Ruler => vec![
                MenuItem::new(MenuAction::Cmd(A::LoopSelection), "Set Loop to Selection").key(shortcuts::label(A::LoopSelection)).off_if(self.time_range().is_none(), WHY_NO_RANGE),
                MenuItem::cmd(A::ZoomToFit).key("Double-click"),
            ],
            MenuSite::Track(lane) => vec![
                MenuItem::new(MenuAction::RenameTrack(lane), "Rename").key(shortcuts::label(A::Rename)),
                MenuItem::new(MenuAction::Sub(SubMenu::TrackColor(lane)), "Colour"),
            ],
            MenuSite::Piano(_) => {
                let notes = self.s.piano.with_untracked(|p| !p.sel.is_empty());
                vec![
                    MenuItem::cmd(A::Cut).off_if(!notes, WHY_NO_NOTES),
                    MenuItem::cmd(A::Copy).off_if(!notes, WHY_NO_NOTES),
                    MenuItem::cmd(A::Paste).off_if(self.clipboard.is_empty(), WHY_EMPTY_NOTE_CLIPBOARD),
                    MenuItem::cmd(A::Duplicate).off_if(!notes, WHY_NO_NOTES),
                    MenuItem::cmd(A::Delete).off_if(!notes, WHY_NO_NOTES),
                    MenuItem::sep(),
                    MenuItem::cmd(A::Quantize).off_if(!notes, WHY_NO_NOTES),
                    MenuItem::cmd(A::SelectAll),
                    MenuItem::sep(),
                    MenuItem::cmd(A::DrawMode).checked(self.s.piano.with_untracked(|p| p.draw)),
                ]
            }
            MenuSite::DefaultTrackColour => {
                let cur = crate::file::parse_hex(&self.settings.default_track_colour).ok().map(state::rgb_hex);
                colour_grid(cur, MenuAction::DefaultTrackColour)
            }
            MenuSite::Problems => self
                .s
                .problems
                .get_untracked()
                .into_iter()
                .flat_map(|p| wrap_lines(&p, MESSAGE_WRAP))
                .map(|p| MenuItem::new(MenuAction::Info, p))
                .collect(),
            MenuSite::Message => wrap_lines(&self.s.status.get_untracked(), MESSAGE_WRAP).into_iter().map(|l| MenuItem::new(MenuAction::Info, l)).collect(),
            MenuSite::Spectrum => {
                let cur = crate::views::chrome::SpectrumStyle::from_key(&self.settings.spectrum_style);
                crate::views::chrome::SpectrumStyle::ALL
                    .iter()
                    .map(|st| MenuItem::new(MenuAction::SpectrumStyle(*st), st.label()).checked(*st == cur))
                    .collect()
            }
            MenuSite::Preset(i) => {
                self.preset_rename_at = (x, y);
                vec![
                    MenuItem::new(MenuAction::PresetRename(i), "Rename"),
                    MenuItem::new(MenuAction::PresetDelete(i), "Delete"),
                    MenuItem::new(MenuAction::PresetReveal(i), if shortcuts::MACOS { "Show in Finder" } else { "Show in Explorer" }),
                ]
            }
        };
        if crate::dev::enabled() {
            // The harness checks the items (UX.md §5); "-" marks a disabled one.
            let list: Vec<String> = items.iter().filter(|i| i.action != MenuAction::Separator).map(|i| format!("{}{}", if i.enabled { "" } else { "-" }, i.label)).collect();
            eprintln!("dev: menu {site:?}: {}", list.join("|"));
        }
        let (x, y) = crate::views::chrome::menu_origin_in(self.window_logical, x, y, &items);
        self.s.menu.set(Some(ContextMenuSpec { x, y, items, sub: None }));
    }

    /// (Make Unique is pointless, Use Track Instrument is pointless) for the
    /// selected clips (F-24).
    pub(crate) fn instrument_menu_state(&self) -> (bool, bool) {
        self.s.arrangement.with_untracked(|a| {
            let clips: Vec<&state::Clip> = a.selection.iter().filter_map(|id| a.clip(*id)).collect();
            let lane_off = clips.iter().all(|c| c.instrument.is_none());
            // Unique already: an own instrument nothing else plays.
            let unique_off = clips.iter().all(|c| c.instrument.is_some_and(|i| a.instruments.users(i, &a.clips) == 1));
            (unique_off, lane_off)
        })
    }

    /// The insert marker is strictly inside a selected clip.
    fn marker_in_selection(&self) -> bool {
        self.s.arrangement.with_untracked(|a| {
            let t = a.insert.0;
            a.selection.iter().filter_map(|id| a.clip(*id)).any(|c| c.start_ticks < t && c.end_ticks() > t)
        }) || self.s.time_sel.get_untracked().is_some()
    }

    /// Opens a submenu next to its row.
    pub(crate) fn open_sub(&mut self, sub: SubMenu, x: f32, y: f32) {
        let items: Vec<MenuItem> = match sub {
            // The colour grid (polish §3.4): the 70 palette colours, the
            // current one marked; hovering previews, a click applies to every
            // selected clip or track.
            SubMenu::ClipColor => {
                let cur = self.s.arrangement.with_untracked(|a| a.first_selected().and_then(|c| c.color).map(state::rgb_hex));
                let mut v = vec![MenuItem::new(MenuAction::ClipColor(None), "Track Colour").checked(cur.is_none())];
                v.extend(colour_grid(cur, |c| MenuAction::ClipColor(Some(c))));
                v
            }
            SubMenu::TrackColor(lane) => {
                let cur = self.s.arrangement.with_untracked(|a| a.lanes.get(lane).map(|l| state::rgb_hex(l.color)));
                colour_grid(cur, move |c| MenuAction::TrackColor(lane, c))
            }
            SubMenu::Grid => {
                let (snap, o) = self.s.arrangement.with_untracked(|a| (a.snap, a.grid_opts));
                let mut v: Vec<MenuItem> = state::Snap::ALL.iter().map(|g| MenuItem::new(MenuAction::Grid(*g), g.label()).checked(*g == snap && !o.adaptive)).collect();
                v.push(MenuItem::sep());
                v.push(MenuItem::cmd(Action::GridNarrow));
                v.push(MenuItem::cmd(Action::GridWiden));
                v.push(MenuItem::cmd(Action::GridTriplet).checked(o.triplet));
                v.push(MenuItem::cmd(Action::GridSnap).checked(o.snap));
                v.push(MenuItem::cmd(Action::GridAdaptive).checked(o.adaptive));
                v
            }
        };
        // Beside the menu: on its right, or on its left when there's no room.
        let (w, _) = crate::views::chrome::menu_size_of(&items);
        let main_x = self.s.menu.with_untracked(|m| m.as_ref().map_or(x, |m| m.x));
        let x = if x + w > self.window_logical.0 - 4.0 { (main_x - w + 4.0).max(0.0) } else { x };
        let (x, y) = crate::views::chrome::menu_origin_in(self.window_logical, x, y, &items);
        if crate::dev::enabled() {
            eprintln!("dev: submenu {sub:?} at {x:.0} {y:.0}");
        }
        let sub = Some(Box::new(ContextMenuSpec { x, y, items, sub: None }));
        // Only on a change: the menu is rebuilt when its spec changes.
        if self.s.menu.with_untracked(|m| m.as_ref().is_some_and(|m| m.sub != sub)) {
            self.s.menu.update(|m| {
                if let Some(m) = m {
                    m.sub = sub;
                }
            });
        }
    }

    /// A context-menu row was chosen.
    pub(crate) fn run_menu_action(&mut self, cx: &mut EventContext, action: MenuAction) {
        let sel = self.s.time_sel.get_untracked();
        let span = self.s.arrangement.with_untracked(|a| a.selection_span());
        match action {
            MenuAction::Cmd(Action::LoopSelection) | MenuAction::LoopClips => {
                let range = match (action, sel) {
                    (MenuAction::Cmd(_), Some(ts)) => Some((ts.start, ts.end)),
                    _ => span,
                };
                if let Some((a, b)) = range {
                    self.set_song_loop(a as f64, b as f64, true);
                    self.commit();
                }
            }
            MenuAction::Cmd(Action::InsertClip) => self.insert_clips(),
            MenuAction::Cmd(a) => {
                self.run_action(cx, a, true);
            }
            MenuAction::MakeUnique => self.make_instrument_unique(),
            MenuAction::LaneInstrument => self.use_lane_instrument(),
            MenuAction::SavePreset => self.save_user_preset(),
            MenuAction::Grid(g) => self.s.arrangement.update(|a| {
                a.snap = g;
                a.grid_opts.adaptive = false;
            }),
            MenuAction::ClipColor(_) | MenuAction::TrackColor(..) => {
                // Undo the preview first, then apply the pick and commit once.
                let mut p = std::mem::take(&mut self.colour_preview);
                self.s.arrangement.update(|a| p.pick(a, action));
                self.commit();
            }
            MenuAction::DefaultTrackColour(c) => {
                self.settings.default_track_colour = format!("#{:06X}", state::rgb_hex(c));
                self.save_settings();
            }
            MenuAction::RenameTrack(l) => self.open_rename(RenameTarget::Track(l)),
            MenuAction::PresetRename(i) => self.open_rename(RenameTarget::Preset(i)),
            MenuAction::PresetDelete(i) => {
                let Some(p) = self.preset(i) else { return };
                let text = format!("Delete your preset \"{}\"?\n\nProjects that use it keep their own copy of the instrument.", p.name);
                if self.ask(cx, &text, "Delete", None) == Some(true) {
                    match crate::user_presets::delete(i.saturating_sub(chip::presets::PRESETS.len())) {
                        Ok(()) => {
                            self.s.user_presets.set(crate::user_presets::load());
                            self.s.browser_sel.set(None);
                            self.set_status(format!("Deleted the preset \"{}\".", p.name));
                        }
                        Err(e) => self.set_status(e),
                    }
                }
            }
            MenuAction::PresetReveal(_) => reveal(crate::user_presets::file_path()),
            MenuAction::SpectrumStyle(st) => cx.emit(crate::AppEvent::SetSpectrumStyle(st)),
            MenuAction::Sub(_) | MenuAction::Separator | MenuAction::Info => {}
        }
    }

    /// Hovering a colour previews it on the targets (not an undo step);
    /// `None` puts the colours back.
    pub(crate) fn preview_colour(&mut self, action: Option<MenuAction>) {
        let mut p = std::mem::take(&mut self.colour_preview);
        self.s.arrangement.update(|a| p.preview(a, action));
        self.colour_preview = p;
    }

    /// Puts back the colours a preview changed (the menu closed without a
    /// pick: a click away, Esc).
    pub(crate) fn restore_colours(&mut self) {
        if !self.colour_preview.active() {
            return;
        }
        let mut p = std::mem::take(&mut self.colour_preview);
        self.s.arrangement.update(|a| p.restore(a));
    }

    /// Insert MIDI Clip (Cmd+Shift+M): one clip per open track in the time
    /// selection.
    pub(crate) fn insert_clips(&mut self) {
        let Some(ts) = self.s.time_sel.get_untracked() else { return self.set_status(WHY_NO_TIMESEL.into()) };
        let open = self.lane_open();
        let random = self.settings.new_clip_colour == "random";
        self.s.arrangement.update(|a| {
            let ids: Vec<u32> = (ts.lane0..=ts.lane1.min(a.lanes.len() - 1)).filter(|&l| open(l)).map(|lane| a.add_clip(lane, ts.start, ts.end - ts.start)).collect();
            a.resolve_overlaps(&ids);
            a.colour_new_clips(&ids, random);
            a.selection = ids;
        });
        self.s.time_sel.set(None);
        self.commit();
    }
}

/// Applies a colour action to its targets: every selected clip, or every
/// selected track when the menu's track is one of them (else that track).
pub(crate) fn apply_colour_to(a: &mut state::Arrangement, action: MenuAction) {
    match action {
        MenuAction::ClipColor(c) => {
            for id in a.selection.clone() {
                if let Some(clip) = a.clip_mut(id) {
                    clip.color = c;
                }
            }
        }
        MenuAction::TrackColor(lane, c) => {
            for l in colour_lanes(a, lane) {
                if let Some(l) = a.lanes.get_mut(l) {
                    l.color = c;
                }
            }
        }
        _ => {}
    }
}

/// The tracks a track-colour action colours.
fn colour_lanes(a: &state::Arrangement, lane: usize) -> Vec<usize> {
    if a.track_sel.contains(&lane) {
        a.track_sel.clone()
    } else {
        vec![lane]
    }
}

/// The colours a preview replaced, to put back.
#[derive(Clone, Debug)]
pub(crate) enum ColourBackup {
    Clips(Vec<(u32, Option<Rgba>)>),
    Lanes(Vec<(usize, Rgba)>),
}

/// The Colour grid's hover preview (POLISH.md C2). While it is active the
/// document shows a colour nobody picked, so **nothing may commit it**: the
/// app's per-tick commit waits (`active`), Esc and a click away `restore`, and
/// a pick restores first, then applies and commits once (MAC-CATCHUP.md §8,
/// MC-D15: the per-tick commit used to bake the hovered colour into the undo
/// history, so "hover A, pick B, undo" gave A).
#[derive(Clone, Debug, Default)]
pub(crate) struct ColourPreview {
    backup: Option<ColourBackup>,
}

impl ColourPreview {
    pub(crate) fn active(&self) -> bool {
        self.backup.is_some()
    }

    /// Shows `action`'s colour on its targets (`None`: no preview). The
    /// originals are kept from before the first preview.
    pub(crate) fn preview(&mut self, a: &mut state::Arrangement, action: Option<MenuAction>) {
        self.restore(a);
        let Some(action) = action else { return };
        let backup = match action {
            MenuAction::ClipColor(_) => Some(ColourBackup::Clips(a.selection.iter().filter_map(|id| a.clip(*id).map(|c| (*id, c.color))).collect())),
            MenuAction::TrackColor(lane, _) => Some(ColourBackup::Lanes(colour_lanes(a, lane).iter().filter_map(|l| a.lanes.get(*l).map(|x| (*l, x.color))).collect())),
            _ => None,
        };
        if backup.is_some() {
            self.backup = backup;
            apply_colour_to(a, action);
        }
    }

    /// Puts the original colours back.
    pub(crate) fn restore(&mut self, a: &mut state::Arrangement) {
        let Some(b) = self.backup.take() else { return };
        match &b {
            ColourBackup::Clips(v) => {
                for (id, c) in v {
                    if let Some(clip) = a.clip_mut(*id) {
                        clip.color = *c;
                    }
                }
            }
            ColourBackup::Lanes(v) => {
                for (l, c) in v {
                    if let Some(lane) = a.lanes.get_mut(*l) {
                        lane.color = *c;
                    }
                }
            }
        }
    }

    /// A click on a cell: the preview is undone, then the pick applied (the
    /// caller commits once).
    pub(crate) fn pick(&mut self, a: &mut state::Arrangement, action: MenuAction) {
        self.restore(a);
        apply_colour_to(a, action);
    }
}

/// The 70 palette colours as grid cells, `cur` (0xRRGGBB) marked.
fn colour_grid(cur: Option<u32>, action: impl Fn(Rgba) -> MenuAction) -> Vec<MenuItem> {
    crate::theme::palette::all()
        .map(|c| {
            let hex = state::rgb_hex(c);
            MenuItem { swatch: Some(c), grid: true, ..MenuItem::new(action(c), format!("#{hex:06X}")).checked(cur == Some(hex)) }
        })
        .collect()
}

/// A status message shown whole wraps at about this many characters per row.
const MESSAGE_WRAP: usize = 34;

/// Splits text into rows of at most `width` characters at spaces (and at its
/// own line breaks).
pub fn wrap_lines(text: &str, width: usize) -> Vec<String> {
    let mut out = Vec::new();
    for para in text.lines() {
        let mut line = String::new();
        for w in para.split_whitespace() {
            if !line.is_empty() && line.chars().count() + 1 + w.chars().count() > width {
                out.push(std::mem::take(&mut line));
            }
            if !line.is_empty() {
                line.push(' ');
            }
            line.push_str(w);
        }
        if !line.is_empty() {
            out.push(line);
        }
    }
    out
}

/// Zoom factor of one + / − press, and how many Z steps X remembers.
const ZOOM_STEP: f64 = 1.25;
const MAX_ZOOM_BACK: usize = 32;
/// Rename field geometry (logical px): inset from the row top, the clip
/// field's width limits, and the track field's place and width.
const RENAME_INSET: f32 = 4.0;
const RENAME_MIN_W: f32 = 120.0;
const RENAME_MAX_W: f32 = 240.0;
const RENAME_TRACK_X: f32 = 24.0;
const RENAME_TRACK_W: f32 = 160.0;
/// The value field's width (polish 2 §3: dB and pan typed in place).
const RENAME_VALUE_W: f32 = 64.0;

/// Shows a file in Explorer / Finder (B5).
fn reveal(path: Option<std::path::PathBuf>) {
    let Some(p) = path else { return };
    #[cfg(target_os = "windows")]
    let _ = std::process::Command::new("explorer").arg(format!("/select,{}", p.display())).spawn();
    #[cfg(target_os = "macos")]
    let _ = std::process::Command::new("open").arg("-R").arg(&p).spawn();
    #[cfg(not(any(target_os = "windows", target_os = "macos")))]
    let _ = std::process::Command::new("xdg-open").arg(p.parent().unwrap_or(&p)).spawn();
}

#[cfg(test)]
mod colour_preview_tests {
    use super::*;
    use crate::history::{Doc, History};

    fn setup() -> (state::Arrangement, History) {
        let a = crate::mock::demo_arrangement();
        let h = History::new(Doc::of(&a, 150.0, crate::instruments::default_params()));
        (a, h)
    }

    /// The app commits every pump tick; this is that commit, honouring the
    /// preview gate as `AppState::commit` does.
    fn tick(a: &state::Arrangement, h: &mut History, p: &ColourPreview) {
        if !p.active() {
            h.commit(Doc::of(a, 150.0, crate::instruments::default_params()));
        }
    }

    /// MC-D15 (1): hover a colour, Esc: the old colour is back and nothing
    /// was recorded.
    #[test]
    fn esc_after_hover_restores_the_old_colour() {
        let (mut a, mut h) = setup();
        let orig = a.lanes[2].color;
        let mut p = ColourPreview::default();
        let hover = crate::theme::palette::all().nth(20).unwrap();
        p.preview(&mut a, Some(MenuAction::TrackColor(2, hover)));
        assert_eq!(a.lanes[2].color, hover, "the hover shows the colour");
        tick(&a, &mut h, &p);
        p.restore(&mut a); // Esc
        tick(&a, &mut h, &p);
        assert_eq!(a.lanes[2].color, orig);
        assert!(!h.can_undo(), "a preview is never an undo step");
    }

    /// MC-D15 (2): hover A, pick B, undo: the original colour, for tracks
    /// and clips.
    #[test]
    fn hover_a_pick_b_undo_gives_the_original() {
        let (mut a, mut h) = setup();
        let orig = a.lanes[1].color;
        let (ca, cb) = (crate::theme::palette::all().nth(5).unwrap(), crate::theme::palette::all().nth(40).unwrap());
        let mut p = ColourPreview::default();
        p.preview(&mut a, Some(MenuAction::TrackColor(1, ca)));
        tick(&a, &mut h, &p);
        p.preview(&mut a, Some(MenuAction::TrackColor(1, cb)));
        tick(&a, &mut h, &p);
        p.pick(&mut a, MenuAction::TrackColor(1, cb));
        assert!(!p.active());
        tick(&a, &mut h, &p);
        assert_eq!(a.lanes[1].color, cb);
        let back = h.undo(Doc::of(&a, 150.0, crate::instruments::default_params())).expect("one step");
        assert_eq!(back.lanes[1].color, orig, "undo returns the original, not the hovered A");
        assert!(!h.can_undo(), "exactly one step");

        // Clips the same way.
        let (mut a, mut h) = setup();
        let id = a.clips[0].id;
        a.selection = vec![id];
        let orig = a.clip(id).unwrap().color;
        let mut p = ColourPreview::default();
        p.preview(&mut a, Some(MenuAction::ClipColor(Some(ca))));
        tick(&a, &mut h, &p);
        p.pick(&mut a, MenuAction::ClipColor(Some(cb)));
        tick(&a, &mut h, &p);
        let back = h.undo(Doc::of(&a, 150.0, crate::instruments::default_params())).expect("one step");
        assert_eq!(back.clips.iter().find(|c| c.id == id).unwrap().color, orig);
    }
}
