//! The automatable-parameter registry (BATCH4A_PROMPT.md §6), ready for
//! Batch 5's automation. No UI: one table of every parameter a track kind
//! (and the Main track) can automate, with its id, name, unit, range,
//! default and **rate class**:
//!
//! - **frame-tick**: stepped at the driver's frame tick (60.0988 Hz), as game
//!   sound drivers did: the chip registers and instrument macros on locked
//!   tracks (VISION.md: "Hardware-locked tracks automate authentically").
//!   Studio tracks' channel controls and macros are frame-tick too: a studio
//!   voice runs the same driver code, which reads them once per frame tick
//!   (RUNLOG B4A-D11).
//! - **smooth**: per audio block, de-zippered (a ramp over the block): a
//!   studio track's gain and pan, and the Main gain.
//!
//! Documented in `docs/batch4a/parameters.md`.

use chip::driver::ParamId;
use chip::voice::VoiceKind;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Rate {
    FrameTick,
    Smooth,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Owner {
    /// A track of this voice type; `locked`: bound to a real channel.
    Track { kind: VoiceKind, locked: bool },
    Main,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Param {
    /// Stable id ("track.gain", "chip.duty", "macro.volume", "main.gain").
    pub id: &'static str,
    pub name: &'static str,
    pub unit: &'static str,
    pub min: f32,
    pub max: f32,
    pub default: f32,
    /// Discrete steps (register values) or continuous.
    pub stepped: bool,
    pub rate: Rate,
}

fn chip_param(p: ParamId, lane: usize) -> Param {
    let (lo, hi, def) = p.range(lane);
    let (id, name, unit) = match p {
        ParamId::Duty => ("chip.duty", "Duty", "step"),
        ParamId::Volume => ("chip.volume", "Volume", "register (0–15)"),
        ParamId::Envelope => ("chip.envelope", "Envelope", "mode"),
        ParamId::Sweep => ("chip.sweep", "Sweep", "shift"),
        ParamId::SweepRate => ("chip.sweep_rate", "Sweep rate", "period"),
        ParamId::NoiseMode => ("chip.noise_mode", "Noise mode", "mode"),
        ParamId::NoisePeriod => ("chip.noise_period", "Noise period", "index"),
        ParamId::Sample => ("chip.sample", "Sample", "slot"),
        ParamId::Rate => ("chip.rate", "Rate", "index"),
        ParamId::Loop => ("chip.loop", "Loop", "on/off"),
    };
    Param { id, name, unit, min: lo as f32, max: hi as f32, default: def as f32, stepped: true, rate: Rate::FrameTick }
}

/// The instrument macros (VISION.md: volume, arpeggio, pitch, duty, vibrato):
/// automating one sets its value for the frame (a held step).
fn macros(kind: VoiceKind) -> Vec<Param> {
    let mut v = vec![
        Param { id: "macro.volume", name: "Volume macro", unit: "step (0–15)", min: 0.0, max: 15.0, default: 15.0, stepped: true, rate: Rate::FrameTick },
        Param { id: "macro.arpeggio", name: "Arpeggio", unit: "semitones", min: -12.0, max: 12.0, default: 0.0, stepped: true, rate: Rate::FrameTick },
        Param { id: "macro.pitch", name: "Pitch", unit: "cents", min: -100.0, max: 100.0, default: 0.0, stepped: true, rate: Rate::FrameTick },
        Param { id: "macro.vibrato", name: "Vibrato depth", unit: "cents", min: 0.0, max: 100.0, default: 0.0, stepped: true, rate: Rate::FrameTick },
    ];
    // Duty only where the channel has one (pulse, VRC6 pulse; the noise's
    // "duty" is its mode).
    if matches!(kind, VoiceKind::Pulse | VoiceKind::Vrc6Pulse | VoiceKind::Noise) {
        v.push(Param { id: "macro.duty", name: "Duty macro", unit: "step", min: 0.0, max: 15.0, default: 0.0, stepped: true, rate: Rate::FrameTick });
    }
    // The triangle has no volume (its macro only gates it on and off).
    if kind == VoiceKind::Triangle {
        v.retain(|p| p.id != "macro.volume");
        v.push(Param { id: "macro.gate", name: "Gate", unit: "on/off", min: 0.0, max: 1.0, default: 1.0, stepped: true, rate: Rate::FrameTick });
    }
    v
}

/// Every automatable parameter of `o`, in display order.
pub fn registry(o: Owner) -> Vec<Param> {
    match o {
        Owner::Main => vec![Param { id: "main.gain", name: "Main gain", unit: "dB", min: f32::NEG_INFINITY, max: crate::state::VOL_DB_MAX, default: 0.0, stepped: false, rate: Rate::Smooth }],
        Owner::Track { kind, locked } => {
            let lane = kind.lane();
            let mut v = Vec::new();
            // A locked track's fader and pan are pinned (§5.3): not automatable.
            if !locked {
                v.push(Param { id: "track.gain", name: "Volume", unit: "dB", min: f32::NEG_INFINITY, max: crate::state::VOL_DB_MAX, default: 0.0, stepped: false, rate: Rate::Smooth });
                v.push(Param { id: "track.pan", name: "Pan", unit: "L/R", min: -(crate::state::PAN_MAX as f32), max: crate::state::PAN_MAX as f32, default: 0.0, stepped: false, rate: Rate::Smooth });
            }
            v.extend(ParamId::for_lane(lane).iter().map(|&p| chip_param(p, lane)));
            v.extend(macros(kind));
            v
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn all_owners() -> Vec<Owner> {
        let mut o = vec![Owner::Main];
        for kind in VoiceKind::ALL {
            o.push(Owner::Track { kind, locked: false });
            o.push(Owner::Track { kind, locked: true });
        }
        o
    }

    #[test]
    fn ids_are_unique_and_defaults_in_range() {
        for o in all_owners() {
            let r = registry(o);
            let mut ids: Vec<&str> = r.iter().map(|p| p.id).collect();
            ids.sort();
            let n = ids.len();
            ids.dedup();
            assert_eq!(ids.len(), n, "{o:?}");
            for p in &r {
                assert!(p.min <= p.default && p.default <= p.max, "{o:?} {}", p.id);
                assert!(!p.name.is_empty() && !p.unit.is_empty());
            }
        }
    }

    #[test]
    fn rate_classes_follow_the_rules() {
        // Locked: every parameter is on the frame tick; no gain or pan.
        for kind in VoiceKind::ALL {
            let locked = registry(Owner::Track { kind, locked: true });
            assert!(locked.iter().all(|p| p.rate == Rate::FrameTick && p.stepped), "{kind:?}");
            assert!(!locked.iter().any(|p| p.id == "track.gain" || p.id == "track.pan"));
            // Studio: gain and pan smooth, the chip and macros on the frame tick.
            let studio = registry(Owner::Track { kind, locked: false });
            for p in &studio {
                let smooth = p.id == "track.gain" || p.id == "track.pan";
                assert_eq!(p.rate == Rate::Smooth, smooth, "{kind:?} {}", p.id);
            }
            // Every hardware control of the channel is there.
            for &pid in ParamId::for_lane(kind.lane()) {
                assert!(studio.iter().any(|p| p.name == chip_param(pid, kind.lane()).name), "{kind:?} {pid:?}");
            }
        }
        let main = registry(Owner::Main);
        assert_eq!(main.len(), 1);
        assert_eq!((main[0].id, main[0].rate), ("main.gain", Rate::Smooth));
    }

    #[test]
    fn the_triangle_has_a_gate_not_a_volume() {
        let t = registry(Owner::Track { kind: VoiceKind::Triangle, locked: true });
        assert!(t.iter().any(|p| p.id == "macro.gate") && !t.iter().any(|p| p.id == "macro.volume" || p.id == "chip.volume"));
    }
}

/// The registry as Markdown tables (docs/batch4a/parameters.md is this,
/// written by `cargo test -p ui write_parameters_md -- --ignored`).
pub fn markdown() -> String {
    let mut out = String::new();
    let fmt = |v: f32| if v == f32::NEG_INFINITY { "−inf".to_string() } else if v.fract() == 0.0 { format!("{v:.0}") } else { format!("{v}") };
    let mut owners = vec![("Main track".to_string(), Owner::Main)];
    for kind in VoiceKind::ALL {
        owners.push((format!("{} track, studio", crate::tracks::kind_name(kind)), Owner::Track { kind, locked: false }));
        owners.push((format!("{} track, locked", crate::tracks::kind_name(kind)), Owner::Track { kind, locked: true }));
    }
    for (title, o) in owners {
        out += &format!("\n### {title}\n\n| Id | Name | Unit | Range | Default | Rate |\n|---|---|---|---|---|---|\n");
        for p in registry(o) {
            let rate = match p.rate {
                Rate::FrameTick => "frame-tick",
                Rate::Smooth => "smooth",
            };
            out += &format!("| `{}` | {} | {} | {} … {} | {} | {rate} |\n", p.id, p.name, p.unit, fmt(p.min), fmt(p.max), fmt(p.default));
        }
    }
    out
}

#[cfg(test)]
mod doc {
    #[test]
    #[ignore]
    fn write_parameters_md() {
        let head = "# Automatable parameters (Batch 4a §6)\n\nThe registry Batch 5's automation builds on: `crates/ui/src/params.rs` (unit-tested). No UI yet.\n\n**Rate classes.** *frame-tick*: stepped at the driver's frame tick (60.0988 Hz), as game sound drivers did: on locked tracks the chip registers and the instrument macros (values quantised to the register's real steps, VISION.md \"Automation\"). A studio track's channel controls and macros are frame-tick too (RUNLOG B4A-D11): a studio voice runs the same driver code, which reads them once per frame tick, so automation there sounds like the chip's own macros. *smooth*: per audio block with de-zippering (a ramp over the block, as the engine does for track gains and the Main gain today): a studio track's volume and pan, and the Main gain.\n\nA locked track has no volume or pan parameters: its fader is pinned at 0 dB and its pan at centre (BATCH4A_PROMPT.md §5.3); its level comes from the chip's Volume register (`chip.volume`) and the volume macro.\n";
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../docs/batch4a/parameters.md");
        std::fs::write(path, format!("{head}{}", super::markdown())).unwrap();
    }
}
