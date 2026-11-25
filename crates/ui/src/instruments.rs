//! Lane instruments in the UI model: a named instrument per lane (from a
//! preset, maybe edited), kits with their pieces, and the conversion into the
//! engine's flat instrument bank.

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
pub fn default_instruments() -> Vec<LaneInstrument> {
    presets::DEFAULT_LAYOUT.iter().map(|n| LaneInstrument::named(n)).collect()
}

/// The engine's bank: (instruments, per-lane index, audition index).
pub fn bank(lanes: &[LaneInstrument], audition: Option<&LaneInstrument>) -> (Vec<Instrument>, [u8; LANES], u8) {
    let mut bank = Vec::with_capacity(LANES * 2);
    let mut lane_inst = [NONE; LANES];
    for (i, l) in lanes.iter().take(LANES).enumerate() {
        lane_inst[i] = l.push_into(&mut bank);
    }
    let preview = audition.map_or(NONE, |a| a.push_into(&mut bank));
    (bank, lane_inst, preview)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn kits_flatten_into_the_bank() {
        let lanes = default_instruments();
        let (bank, idx, prev) = bank(&lanes, Some(&LaneInstrument::named("Explosion")));
        assert_eq!(prev as usize, bank.len() - 1);
        let kit = bank[idx[3] as usize];
        assert!(kit.is_kit());
        let snare = kit.zone(60).unwrap();
        assert_eq!(bank[snare.inst as usize], presets::find("Snare").unwrap().inst);
        assert!(bank.len() < 255);
    }
}
