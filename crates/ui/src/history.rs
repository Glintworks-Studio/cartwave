//! Global undo / redo (PIANOROLL.md §4).
//!
//! **Snapshot-based.** The undoable document ([`Doc`]) is the clips (with
//! their notes), the next clip id, the tempo and the lane parameters. Clips
//! are `Arc`s and every edit copies only the clip it changes
//! (`Arrangement::clip_mut`), so a snapshot costs one pointer per clip plus
//! the clips that really changed. Undo replaces the document with the
//! previous snapshot, which can't get out of step with the model the way an
//! inverse command can: every operation, present and future, is undoable
//! without writing an inverse for it.
//!
//! **Steps.** The app commits the document when an edit is finished: at the
//! end of a gesture (mouse release; one drag = one step) or after a discrete
//! edit (a key, a typed value). Commits that change nothing are dropped.
//!
//! **Memory cap.** Each undo step is charged the bytes it doesn't share with
//! the step after it. The history keeps at most [`MAX_STEPS`] steps and
//! [`MAX_BYTES`]; the oldest steps go first.

use crate::instruments::LaneInstrument;
use std::collections::VecDeque;
use std::sync::Arc;

use engine::Params;

use crate::notes::Note;
use crate::state::{Arrangement, Clip};

/// Most undo steps kept.
pub const MAX_STEPS: usize = 1000;
/// Most memory the undo steps may hold (the clips they don't share).
pub const MAX_BYTES: usize = 64 * 1024 * 1024;

/// Everything undo covers.
#[derive(Clone, Debug, PartialEq)]
pub struct Doc {
    pub clips: Vec<Arc<Clip>>,
    pub next_id: u32,
    pub bpm: f32,
    pub params: Params,
    pub instruments: Vec<LaneInstrument>,
}

impl Doc {
    pub fn of(a: &Arrangement, bpm: f32, params: Params) -> Self {
        Self { clips: a.clips.clone(), next_id: a.next_id, bpm, params, instruments: a.instruments.clone() }
    }

    /// Writes the document into an arrangement; selection and view stay,
    /// minus clips that no longer exist.
    pub fn apply_to(&self, a: &mut Arrangement) {
        a.clips = self.clips.clone();
        a.next_id = self.next_id;
        a.instruments = self.instruments.clone();
        a.editing.clear();
        a.editing_notes = None;
        let ids: Vec<u32> = a.clips.iter().map(|c| c.id).collect();
        a.selection.retain(|id| ids.contains(id));
    }

    /// Equal content. Shared clips compare by pointer, the rest by value.
    pub fn same_as(&self, o: &Doc) -> bool {
        self.next_id == o.next_id
            && self.bpm == o.bpm
            && self.params == o.params
            && self.instruments == o.instruments
            && self.clips.len() == o.clips.len()
            && self.clips.iter().zip(&o.clips).all(|(a, b)| Arc::ptr_eq(a, b) || a == b)
    }
}

fn clip_bytes(c: &Clip) -> usize {
    std::mem::size_of::<Clip>() + c.notes.capacity() * std::mem::size_of::<Note>() + c.name.capacity()
}

/// What keeping `old` costs while `newer` exists: its clip list plus every
/// clip `newer` doesn't share.
fn unique_bytes(old: &Doc, newer: &Doc) -> usize {
    let shared: std::collections::HashSet<*const Clip> =
        newer.clips.iter().map(Arc::as_ptr).collect();
    old.clips.len() * std::mem::size_of::<Arc<Clip>>()
        + old
            .clips
            .iter()
            .filter(|c| !shared.contains(&Arc::as_ptr(c)))
            .map(|c| clip_bytes(c))
            .sum::<usize>()
}

struct Step {
    doc: Doc,
    bytes: usize,
}

pub struct History {
    undo: VecDeque<Step>,
    redo: Vec<Step>,
    /// The last committed document.
    current: Doc,
    bytes: usize,
}

impl History {
    pub fn new(doc: Doc) -> Self {
        Self { undo: VecDeque::new(), redo: Vec::new(), current: doc, bytes: 0 }
    }

    /// Forgets everything (File > New, Open).
    pub fn reset(&mut self, doc: Doc) {
        *self = Self::new(doc);
    }

    pub fn can_undo(&self) -> bool {
        !self.undo.is_empty()
    }

    pub fn can_redo(&self) -> bool {
        !self.redo.is_empty()
    }

    pub fn steps(&self) -> (usize, usize) {
        (self.undo.len(), self.redo.len())
    }

    pub fn bytes(&self) -> usize {
        self.bytes
    }

    /// Records `now` as one step if it differs from the last commit.
    /// Returns whether a step was added.
    pub fn commit(&mut self, now: Doc) -> bool {
        if now.same_as(&self.current) {
            return false;
        }
        let old = std::mem::replace(&mut self.current, now);
        let bytes = unique_bytes(&old, &self.current);
        self.bytes += bytes;
        self.undo.push_back(Step { doc: old, bytes });
        for s in self.redo.drain(..) {
            self.bytes -= s.bytes;
        }
        while self.undo.len() > MAX_STEPS || (self.bytes > MAX_BYTES && self.undo.len() > 1) {
            if let Some(s) = self.undo.pop_front() {
                self.bytes -= s.bytes;
            }
        }
        true
    }

    /// Commits `now` (an edit not yet committed is its own step), then
    /// returns the document before the last step.
    pub fn undo(&mut self, now: Doc) -> Option<Doc> {
        self.commit(now);
        let prev = self.undo.pop_back()?;
        self.bytes -= prev.bytes;
        let cur = std::mem::replace(&mut self.current, prev.doc);
        let bytes = unique_bytes(&cur, &self.current);
        self.bytes += bytes;
        self.redo.push(Step { doc: cur, bytes });
        Some(self.current.clone())
    }

    /// The document after the last undone step. An uncommitted edit since
    /// the undo drops the redo steps (as any new edit does).
    pub fn redo(&mut self, now: Doc) -> Option<Doc> {
        if self.commit(now) {
            return None;
        }
        let next = self.redo.pop()?;
        self.bytes -= next.bytes;
        let cur = std::mem::replace(&mut self.current, next.doc);
        let bytes = unique_bytes(&cur, &self.current);
        self.bytes += bytes;
        self.undo.push_back(Step { doc: cur, bytes });
        Some(self.current.clone())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mock::{self, Rng};
    use crate::notes::{self, Grid, RowKind};
    use crate::state::{PPQ, TICKS_PER_BAR};
    use engine::ParamId;

    /// One random edit through the same model operations the UI uses.
    fn random_edit(a: &mut Arrangement, bpm: &mut f32, params: &mut Params, r: &mut Rng) {
        let pick = |a: &Arrangement, r: &mut Rng| -> Option<u32> {
            (!a.clips.is_empty()).then(|| a.clips[r.range(0, a.clips.len() as u32) as usize].id)
        };
        match r.range(0, 14) {
            0 => {
                let lane = r.range(0, 8) as usize;
                a.add_clip(lane, r.range(0, 32) * TICKS_PER_BAR, TICKS_PER_BAR);
            }
            1 => {
                if let Some(id) = pick(a, r) {
                    if let Some(c) = a.clip_mut(id) {
                        c.start_ticks = r.range(0, 120) * (PPQ / 4);
                    }
                    a.resolve_overlaps(&[id]);
                }
            }
            2 => {
                if let Some(id) = pick(a, r) {
                    a.selection = vec![id];
                    a.delete_selected();
                }
            }
            3 => *bpm = 60.0 + r.range(0, 200) as f32,
            4 => {
                let lane = r.range(0, 8) as usize;
                if let Some(&p) = ParamId::for_lane(lane).first() {
                    let (lo, hi, _) = p.range(lane);
                    params.set(lane, p, lo + r.range(0, (hi - lo + 1) as u32) as i32);
                }
            }
            _ => {
                // A note edit inside a clip.
                let Some(id) = pick(a, r) else { return };
                let Some(c) = a.clip_mut(id) else { return };
                let kind = RowKind::for_lane(c.lane);
                let mut next = c.next_note_id;
                let ids: Vec<u32> = c.notes.iter().filter(|_| r.f32() < 0.3).map(|n| n.id).collect();
                match r.range(0, 8) {
                    0 | 1 => {
                        let (lo, hi) = kind.playable_rows();
                        let pitch = kind.pitch_of(r.range(lo, hi + 1));
                        notes::add(&mut c.notes, &mut next, r.range(0, 64) * (PPQ / 8), PPQ / 4 * r.range(1, 5), pitch);
                    }
                    2 => notes::delete(&mut c.notes, &ids),
                    3 => {
                        notes::transpose(&mut c.notes, &ids, r.range(0, 25) as i32 - 12, kind, &mut next);
                    }
                    4 => {
                        notes::nudge(&mut c.notes, &ids, (r.range(0, 9) as i64 - 4) * 120, &mut next);
                    }
                    5 => notes::quantize(&mut c.notes, &ids, Grid::ALL[r.range(0, 8) as usize], &mut next),
                    6 => {
                        let clip = notes::copy(&c.notes, &ids);
                        notes::paste(&mut c.notes, &mut next, &clip, r.range(0, 16) * PPQ, kind);
                    }
                    _ => notes::set_volume(&mut c.notes, &ids, Some(r.range(0, 16) as u8)),
                }
                c.next_note_id = next;
                assert!(notes::is_mono(&c.notes), "mono rule broken: {:?}", c.notes);
            }
        }
    }

    #[test]
    fn undo_all_restores_the_original_and_redo_all_the_final() {
        for seed in 1..=40u64 {
            let mut r = Rng::new(seed);
            let mut a = mock::demo_arrangement();
            let mut bpm = 150.0f32;
            let mut params = Params::default();
            let original = Doc::of(&a, bpm, params);
            let mut h = History::new(original.clone());
            let mut states = vec![original.clone()];
            let n = 20 + r.range(0, 60) as usize;
            for _ in 0..n {
                random_edit(&mut a, &mut bpm, &mut params, &mut r);
                let d = Doc::of(&a, bpm, params);
                if h.commit(d.clone()) {
                    states.push(d);
                }
            }
            let fin = Doc::of(&a, bpm, params);
            assert_eq!(h.steps().0, states.len() - 1);
            // Undo everything, checking each intermediate state.
            let mut now = fin.clone();
            for want in states.iter().rev().skip(1) {
                now = h.undo(now).expect("undo step");
                assert_eq!(&now, want, "seed {seed}");
            }
            assert!(h.undo(now.clone()).is_none());
            assert_eq!(now, original, "seed {seed}: undo all");
            // Redo everything.
            for want in states.iter().skip(1) {
                now = h.redo(now).expect("redo step");
                assert_eq!(&now, want, "seed {seed}");
            }
            assert_eq!(now, fin, "seed {seed}: redo all");
            assert!(h.redo(now).is_none());
        }
    }

    #[test]
    fn a_gesture_is_one_step_and_noop_commits_are_dropped() {
        let mut a = mock::demo_arrangement();
        let mut h = History::new(Doc::of(&a, 150.0, Params::default()));
        let id = a.clips[0].id;
        // 30 drag moves, committed once at release.
        for i in 0..30 {
            a.clip_mut(id).unwrap().start_ticks = i * 10;
        }
        assert!(h.commit(Doc::of(&a, 150.0, Params::default())));
        // A "change" back to the same values (new Arc, same content): no step.
        let s = a.clip(id).unwrap().start_ticks;
        a.clip_mut(id).unwrap().start_ticks = s;
        assert!(!h.commit(Doc::of(&a, 150.0, Params::default())));
        assert_eq!(h.steps(), (1, 0));
    }

    #[test]
    fn new_edit_after_undo_drops_redo() {
        let mut a = mock::demo_arrangement();
        let p = Params::default();
        let mut h = History::new(Doc::of(&a, 150.0, p));
        a.add_clip(0, 40 * TICKS_PER_BAR, TICKS_PER_BAR);
        h.commit(Doc::of(&a, 150.0, p));
        let d = h.undo(Doc::of(&a, 150.0, p)).unwrap();
        d.apply_to(&mut a);
        assert!(h.can_redo());
        a.add_clip(1, 40 * TICKS_PER_BAR, TICKS_PER_BAR);
        assert!(h.redo(Doc::of(&a, 150.0, p)).is_none(), "the new edit wins");
        assert!(!h.can_redo());
        assert_eq!(h.steps().0, 1);
    }

    #[test]
    fn memory_is_capped() {
        // One clip with 20k notes; each step changes it, so each step holds a
        // full copy (~20k x 16 B = 320 kB). 64 MB keeps ~200 of them.
        let mut a = Arrangement::empty();
        let id = a.add_clip(0, 0, 64 * TICKS_PER_BAR);
        {
            let c = a.clip_mut(id).unwrap();
            c.notes = (0..20_000).map(|i| Note::new(i + 1, i * 12, 12, 60)).collect();
            c.next_note_id = 20_001;
        }
        let p = Params::default();
        let mut h = History::new(Doc::of(&a, 150.0, p));
        for i in 0..400 {
            a.clip_mut(id).unwrap().notes[0].pitch = 60 + (i % 2) as u8 + 1;
            h.commit(Doc::of(&a, 150.0, p));
        }
        assert!(h.bytes() <= MAX_BYTES, "{}", h.bytes());
        assert!(h.steps().0 < 400 && h.steps().0 > 100, "{:?}", h.steps());
        // Steps that share everything but one small clip are cheap.
        let mut b = mock::stress_arrangement();
        let mut h = History::new(Doc::of(&b, 150.0, p));
        let small = b.clips[0].id;
        for i in 0..MAX_STEPS + 10 {
            b.clip_mut(small).unwrap().start_ticks = (i % 7) as u32 * 10 + 1;
            h.commit(Doc::of(&b, 150.0, p));
        }
        assert_eq!(h.steps().0, MAX_STEPS, "step cap");
        assert!(h.bytes() < 8 * 1024 * 1024, "{}", h.bytes());
    }
}
