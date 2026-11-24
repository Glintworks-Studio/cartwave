//! Immutable song snapshots, built on the UI thread and handed to the engine.

use chip::driver::{Note, Song, LANES};

/// The engine's own copy of the arrangement: per lane, notes in absolute
/// ticks, sorted and non-overlapping.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct SongSnapshot {
    pub lanes: [Vec<Note>; LANES],
    /// Increases with every edit (for latency measurement and tests).
    pub version: u64,
    /// When the UI sent it (ns since the engine epoch; 0 if unknown).
    pub sent_ns: u64,
}

/// One clip's worth of input: lane, start, length (ticks), and its notes as
/// (start relative to the clip, length, pitch).
pub struct ClipInput<'a> {
    pub lane: usize,
    pub start: u32,
    pub len: u32,
    pub notes: &'a [(u32, u32, u8)],
}

impl SongSnapshot {
    pub fn view(&self) -> Song<'_> {
        Song {
            lanes: core::array::from_fn(|i| self.lanes[i].as_slice()),
        }
    }

    /// Flattens clips into lanes. Notes are cut at their clip's end. Within a
    /// lane, a note that runs into the next one is trimmed to its start, so
    /// the engine never sees overlapping notes on one channel (the UI already
    /// resolves overlapping clips; this is the backstop).
    pub fn build<'a>(clips: impl IntoIterator<Item = ClipInput<'a>>, version: u64) -> Self {
        let mut lanes: [Vec<Note>; LANES] = Default::default();
        for c in clips {
            if c.lane >= LANES {
                continue;
            }
            let clip_end = c.start.saturating_add(c.len);
            for &(s, l, pitch) in c.notes {
                let start = c.start.saturating_add(s);
                let end = start.saturating_add(l).min(clip_end);
                if start < end {
                    lanes[c.lane].push(Note { start, end, pitch });
                }
            }
        }
        for lane in &mut lanes {
            lane.sort_by_key(|n| n.start);
            // Same start twice: keep the later-added one (newer wins).
            lane.dedup_by(|later, earlier| {
                if later.start == earlier.start {
                    *earlier = *later;
                    true
                } else {
                    false
                }
            });
            for i in 1..lane.len() {
                let next = lane[i].start;
                if lane[i - 1].end > next {
                    lane[i - 1].end = next;
                }
            }
        }
        Self {
            lanes,
            version,
            sent_ns: 0,
        }
    }

    pub fn note_count(&self) -> usize {
        self.lanes.iter().map(|l| l.len()).sum()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn flatten_cut_and_trim() {
        let a = [(0, 100, 60), (90, 50, 62), (500, 100, 64)];
        let b = [(0, 30, 70)];
        let s = SongSnapshot::build(
            [
                ClipInput { lane: 0, start: 1000, len: 520, notes: &a },
                ClipInput { lane: 0, start: 1120, len: 30, notes: &b },
            ],
            1,
        );
        let l = &s.lanes[0];
        // 1000..1090 (trimmed by the next), 1090..1120 (trimmed by the other clip's
        // note), 1120..1150, 1500..1520 (cut at clip end).
        assert_eq!(
            l.iter().map(|n| (n.start, n.end, n.pitch)).collect::<Vec<_>>(),
            [(1000, 1090, 60), (1090, 1120, 62), (1120, 1150, 70), (1500, 1520, 64)]
        );
        for w in l.windows(2) {
            assert!(w[0].end <= w[1].start);
        }
    }
}
