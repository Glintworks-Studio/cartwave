//! Instruments in the UI model: the project's instrument table (named
//! instruments from presets, maybe edited, and kits with their pieces), each
//! lane's default, each clip's optional override, and the conversion into
//! the engine's flat instrument bank.

use chip::instrument::{Instrument, KitZone, KIT_ZONES, NONE};
use chip::presets::{self, Preset};
use engine::LANES;

/// One drum of a kit: notes up to `top` play `inst` (at `pitch` unless NONE).
#[derive(Clone, Debug, PartialEq, Hash)]
pub struct KitPiece {
    pub top: u8,
    pub name: String,
    pub inst: Instrument,
    pub pitch: u8,
}

#[derive(Clone, Debug, PartialEq, Hash)]
pub struct LaneInstrument {
    /// The preset it came from, or a user name.
    pub name: String,
    pub inst: Instrument,
    /// Non-empty: a kit (the note picks the piece).
    pub kit: Vec<KitPiece>,
    /// Changed since it was loaded from the preset.
    pub edited: bool,
}

impl LaneInstrument {
    /// No macros: the channel's own controls only.
    pub fn plain() -> Self {
        Self { name: "Raw Channel".into(), inst: Instrument::PLAIN, kit: Vec::new(), edited: false }
    }

    pub fn from_preset(p: &Preset) -> Self {
        let kit = p
            .zones
            .iter()
            .filter_map(|z| {
                let piece = presets::find(z.preset)?;
                Some(KitPiece { top: z.top, name: z.preset.into(), inst: piece.inst, pitch: z.pitch })
            })
            .take(KIT_ZONES)
            .collect();
        Self { name: p.name.into(), inst: p.inst, kit, edited: false }
    }

    pub fn named(name: &str) -> Self {
        presets::find(name).map_or_else(Self::plain, Self::from_preset)
    }

    pub fn is_kit(&self) -> bool {
        !self.kit.is_empty()
    }

    /// The kit piece that plays `pitch`.
    pub fn piece_for(&self, pitch: u8) -> Option<&KitPiece> {
        self.kit.iter().find(|k| pitch <= k.top)
    }

    /// Name shown in the UI ("Cello*" once edited).
    pub fn label(&self) -> String {
        if self.edited {
            format!("{}*", self.name)
        } else {
            self.name.clone()
        }
    }

    /// Appends this instrument (and its kit pieces) to `bank`; returns its index.
    fn push_into(&self, bank: &mut Vec<Instrument>) -> u8 {
        let mut head = self.inst;
        if self.is_kit() {
            head = Instrument::PLAIN;
            head.kit_len = self.kit.len().min(KIT_ZONES) as u8;
            for (i, k) in self.kit.iter().take(KIT_ZONES).enumerate() {
                let idx = bank.len() as u8;
                bank.push(k.inst);
                head.kit[i] = Some(KitZone { top: k.top, inst: idx, fixed_pitch: k.pitch });
            }
        }
        let idx = bank.len() as u8;
        bank.push(head);
        idx
    }
}

/// The new-song band (an authentic 2A03 + VRC6 layout).
pub fn default_instruments() -> InstrumentTable {
    InstrumentTable::from_lanes(presets::DEFAULT_LAYOUT.iter().map(|n| LaneInstrument::named(n)).collect())
}

/// Stable id of an instrument in a project's table (never reused while the
/// project is open, so undo snapshots and clip references stay valid).
pub type InstId = u32;

/// Most instruments a project holds. With kits (up to 7 bank entries each)
/// and an audition preset this keeps the engine's bank under its 255-entry
/// index (`u8`, `NONE` = 255).
pub const MAX_INSTRUMENTS: usize = 32;

#[derive(Clone, Debug, PartialEq, Hash)]
pub struct Entry {
    pub id: InstId,
    pub inst: LaneInstrument,
}

/// The project's instruments, as in trackers: each lane has a default, and a
/// clip may override it (`Clip::instrument`). Instruments are shared by
/// reference: editing one changes every clip that plays it. Entries no lane
/// or clip uses are dropped by `prune`.
#[derive(Clone, Debug, PartialEq, Hash)]
pub struct InstrumentTable {
    pub entries: Vec<Entry>,
    /// Each lane's default instrument.
    pub lanes: [InstId; LANES],
    pub next_id: InstId,
}

impl InstrumentTable {
    /// One entry per lane (ids 1..=8), each lane's default.
    pub fn from_lanes(v: Vec<LaneInstrument>) -> Self {
        let mut v = v;
        v.resize(LANES, LaneInstrument::plain());
        let entries: Vec<Entry> = v.into_iter().take(LANES).enumerate().map(|(i, inst)| Entry { id: i as InstId + 1, inst }).collect();
        Self { entries, lanes: std::array::from_fn(|i| i as InstId + 1), next_id: LANES as InstId + 1 }
    }

    pub fn get(&self, id: InstId) -> Option<&LaneInstrument> {
        self.entries.iter().find(|e| e.id == id).map(|e| &e.inst)
    }

    pub fn get_mut(&mut self, id: InstId) -> Option<&mut LaneInstrument> {
        self.entries.iter_mut().find(|e| e.id == id).map(|e| &mut e.inst)
    }

    pub fn contains(&self, id: InstId) -> bool {
        self.entries.iter().any(|e| e.id == id)
    }

    /// The lane's default instrument.
    pub fn lane(&self, lane: usize) -> Option<&LaneInstrument> {
        self.get(*self.lanes.get(lane)?)
    }

    pub fn lane_id(&self, lane: usize) -> InstId {
        self.lanes.get(lane).copied().unwrap_or(0)
    }

    /// The instrument a clip plays: its own, else its lane's default.
    pub fn id_for(&self, clip: &crate::state::Clip) -> InstId {
        clip.instrument.filter(|id| self.contains(*id)).unwrap_or_else(|| self.lane_id(clip.lane))
    }

    /// Adds an instrument; `None` when the table is full.
    pub fn add(&mut self, inst: LaneInstrument) -> Option<InstId> {
        if self.entries.len() >= MAX_INSTRUMENTS {
            return None;
        }
        let id = self.next_id;
        self.next_id += 1;
        self.entries.push(Entry { id, inst });
        Some(id)
    }

    /// Makes `inst` the lane's default (a new entry; the old one is dropped
    /// by `prune` if nothing else plays it). `false` when the table is full.
    pub fn set_lane(&mut self, lane: usize, inst: LaneInstrument) -> bool {
        if lane >= LANES {
            return false;
        }
        // The only user of the old default is this lane: replace it in place
        // (so a table that's full can still swap a lane's instrument).
        let old = self.lanes[lane];
        let alone = self.lanes.iter().filter(|&&l| l == old).count() == 1;
        match self.add(inst.clone()) {
            Some(id) => {
                self.lanes[lane] = id;
                true
            }
            None if alone => {
                if let Some(e) = self.get_mut(old) {
                    *e = inst;
                }
                true
            }
            None => false,
        }
    }

    /// How many clips play instrument `id` (their own, or as their lane's default).
    pub fn users(&self, id: InstId, clips: &[std::sync::Arc<crate::state::Clip>]) -> usize {
        clips.iter().filter(|c| self.id_for(c) == id).count()
    }

    /// Drops entries no lane default and no clip refers to.
    pub fn prune(&mut self, clips: &[std::sync::Arc<crate::state::Clip>]) {
        let lanes = self.lanes;
        self.entries.retain(|e| lanes.contains(&e.id) || clips.iter().any(|c| c.instrument == Some(e.id)));
    }

    /// Lane defaults in lane order (for the old per-lane views and files).
    pub fn lane_instruments(&self) -> Vec<LaneInstrument> {
        (0..LANES).map(|l| self.lane(l).cloned().unwrap_or_else(LaneInstrument::plain)).collect()
    }
}

/// Channel controls for a new song: the defaults, with each default
/// preset's own settings applied (e.g. constant volume under a volume macro).
pub fn default_params() -> engine::Params {
    let mut p = engine::Params::default();
    for (lane, name) in presets::DEFAULT_LAYOUT.iter().enumerate() {
        if let Some(pr) = presets::find(name) {
            for &(id, v) in pr.params {
                p.set(lane, id, v as i32);
            }
        }
    }
    p
}

/// The engine's bank built from the table: (instruments, each lane's
/// default index, audition index, table id -> bank index).
pub struct Bank {
    pub bank: Vec<Instrument>,
    pub lane_inst: [u8; LANES],
    pub preview: u8,
    pub index: Vec<(InstId, u8)>,
}

impl Bank {
    /// The bank index of table entry `id` (`NONE`: plain channel).
    pub fn of(&self, id: InstId) -> u8 {
        self.index.iter().find(|e| e.0 == id).map_or(NONE, |e| e.1)
    }
}

/// Bank entries one instrument takes (a kit: its head and every piece).
fn bank_size(l: &LaneInstrument) -> usize {
    1 + if l.is_kit() { l.kit.len().min(KIT_ZONES) } else { 0 }
}

/// Flattens the table (in table order) and the audition preset into the
/// engine's bank. An instrument that wouldn't fit under the 255-entry index
/// plays plain (it can't happen within `MAX_INSTRUMENTS`).
pub fn bank(table: &InstrumentTable, audition: Option<&LaneInstrument>) -> Bank {
    let mut bank = Vec::with_capacity(table.entries.len() * 2);
    let mut index = Vec::with_capacity(table.entries.len());
    let fits = |bank: &Vec<Instrument>, l: &LaneInstrument| bank.len() + bank_size(l) < NONE as usize;
    for e in &table.entries {
        if fits(&bank, &e.inst) {
            index.push((e.id, e.inst.push_into(&mut bank)));
        }
    }
    let of = |id: InstId| index.iter().find(|e: &&(InstId, u8)| e.0 == id).map_or(NONE, |e| e.1);
    let lane_inst = std::array::from_fn(|l| of(table.lanes[l]));
    let preview = audition.filter(|a| fits(&bank, a)).map_or(NONE, |a| a.push_into(&mut bank));
    Bank { bank, lane_inst, preview, index }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn kits_flatten_into_the_bank() {
        let lanes = default_instruments();
        let b = bank(&lanes, Some(&LaneInstrument::named("Explosion")));
        assert_eq!(b.preview as usize, b.bank.len() - 1);
        let kit = b.bank[b.lane_inst[3] as usize];
        assert!(kit.is_kit());
        let snare = kit.zone(60).unwrap();
        assert_eq!(b.bank[snare.inst as usize], presets::find("Snare").unwrap().inst);
        assert!(b.bank.len() < 255);
    }

    #[test]
    fn a_full_table_of_kits_still_fits_the_bank() {
        let mut t = InstrumentTable::from_lanes(vec![LaneInstrument::named("Standard Kit"); LANES]);
        while t.add(LaneInstrument::named("Standard Kit")).is_some() {}
        assert_eq!(t.entries.len(), MAX_INSTRUMENTS);
        let b = bank(&t, Some(&LaneInstrument::named("Standard Kit")));
        assert!(b.bank.len() < NONE as usize);
        assert!(t.entries.iter().all(|e| b.of(e.id) != NONE), "every instrument has a bank entry");
        assert_ne!(b.preview, NONE);
    }

    #[test]
    fn lanes_clips_and_pruning() {
        use crate::state::Clip;
        use std::sync::Arc;
        let mut t = default_instruments();
        let cello = t.add(LaneInstrument::named("Cello")).unwrap();
        let mut c = Clip::new(1, 5, 0, 960, "a".into(), Vec::new());
        assert_eq!(t.id_for(&c), t.lane_id(5), "no override: the lane's default");
        c.instrument = Some(cello);
        assert_eq!(t.id_for(&c), cello);
        let clips = vec![Arc::new(c.clone()), Arc::new(Clip::new(2, 5, 960, 960, "b".into(), Vec::new()))];
        assert_eq!(t.users(cello, &clips), 1);
        assert_eq!(t.users(t.lane_id(5), &clips), 1);
        // A new lane default replaces the old entry, which nothing else uses.
        let old = t.lane_id(0);
        assert!(t.set_lane(0, LaneInstrument::named("Flute")));
        t.prune(&clips);
        assert!(!t.contains(old) && t.contains(cello) && t.lane(0).unwrap().name == "Flute");
        // A dangling override falls back to the lane.
        c.instrument = Some(9999);
        assert_eq!(t.id_for(&c), t.lane_id(5));
    }
}
