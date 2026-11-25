//! Immutable song snapshots, built on the UI thread and handed to the engine.

use chip::driver::{Note, Song, LANES};
use std::sync::Arc;

use chip::instrument::{Instrument, NONE};
use chip::samples::{SampleRef, USER_ROM, USER_SLOTS};

/// Imported DPCM samples: the user part of the sample ROM and where each
/// sample is. Shared (`Arc`) so a snapshot costs a pointer; the engine copies
/// it into the ROM only when `version` changes.
#[derive(Clone, Debug, PartialEq)]
pub struct DpcmBank {
    pub data: Vec<u8>,
    pub refs: [SampleRef; USER_SLOTS],
    pub version: u64,
}

impl Default for DpcmBank {
    fn default() -> Self {
        Self { data: vec![0; USER_ROM], refs: [SampleRef::default(); USER_SLOTS], version: 0 }
    }
}

/// The engine's own copy of the arrangement: per lane, notes in absolute
/// ticks, sorted and non-overlapping.
#[derive(Clone, Debug, PartialEq)]
pub struct SongSnapshot {
    pub lanes: [Vec<Note>; LANES],
    /// The instrument bank (kits reference other entries by index).
    pub instruments: Vec<Instrument>,
    /// Each lane's bank index (`NONE`: plain channel controls).
    pub lane_inst: [u8; LANES],
    /// Bank index for audition notes (`NONE`: the lane's instrument).
    pub preview_inst: u8,
    /// Imported DPCM samples.
    pub dpcm: Arc<DpcmBank>,
    /// Increases with every edit (for latency measurement and tests).
    pub version: u64,
    /// When the UI sent it (ns since the engine epoch; 0 if unknown).
    pub sent_ns: u64,
}

/// One note of a [`ClipInput`]: (start relative to the clip, length, pitch,
/// volume or `None` for the lane default).
pub type NoteInput = (u32, u32, u8, Option<u8>);

/// One clip's worth of input: lane, start, length (ticks), its instrument
/// (bank index, `NONE` = the lane's) and its notes.
pub struct ClipInput<'a> {
    pub lane: usize,
    pub start: u32,
    pub len: u32,
    pub inst: u8,
    pub notes: &'a [NoteInput],
}

impl Default for SongSnapshot {
    fn default() -> Self {
        Self::build(core::iter::empty(), 0)
    }
}

impl SongSnapshot {
    pub fn view(&self) -> Song<'_> {
        Song {
            lanes: core::array::from_fn(|i| self.lanes[i].as_slice()),
            instruments: &self.instruments,
            lane_inst: self.lane_inst,
            preview_inst: self.preview_inst,
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
            for &(s, l, pitch, volume) in c.notes {
                let start = c.start.saturating_add(s);
                let end = start.saturating_add(l).min(clip_end);
                if start < end {
                    lanes[c.lane].push(Note { start, end, pitch, volume, inst: c.inst });
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
            instruments: Vec::new(),
            lane_inst: [NONE; LANES],
            preview_inst: NONE,
            dpcm: Arc::new(DpcmBank::default()),
            version,
            sent_ns: 0,
        }
    }

    pub fn with_dpcm(mut self, dpcm: Arc<DpcmBank>) -> Self {
        self.dpcm = dpcm;
        self
    }

    /// Sets the instrument bank and the lanes' instruments.
    pub fn with_instruments(mut self, bank: Vec<Instrument>, lane_inst: [u8; LANES], preview_inst: u8) -> Self {
        self.instruments = bank;
        self.lane_inst = lane_inst;
        self.preview_inst = preview_inst;
        self
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
        let a = [(0, 100, 60, None), (90, 50, 62, None), (500, 100, 64, Some(7))];
        let b = [(0, 30, 70, None)];
        let s = SongSnapshot::build(
            [
                ClipInput { lane: 0, start: 1000, len: 520, inst: NONE, notes: &a },
                ClipInput { lane: 0, start: 1120, len: 30, inst: NONE, notes: &b },
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
