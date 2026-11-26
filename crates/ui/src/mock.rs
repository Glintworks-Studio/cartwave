//! Demo data and fake meter / playhead generators.

use crate::notes::Note;
use crate::state::*;
use std::sync::Arc;

fn mk(start: u32, len: u32, pitch: u8) -> Note {
    Note::new(0, start, len, pitch)
}

/// Tiny deterministic xorshift RNG so the demo looks the same every launch.
#[derive(Clone, Debug)]
pub struct Rng(u64);

impl Rng {
    pub fn new(seed: u64) -> Self {
        Self(seed.max(1))
    }

    pub fn next_u32(&mut self) -> u32 {
        let mut x = self.0;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.0 = x;
        (x >> 32) as u32
    }

    /// Uniform float in 0..1.
    pub fn f32(&mut self) -> f32 {
        (self.next_u32() >> 8) as f32 / (1u32 << 24) as f32
    }

    pub fn range(&mut self, lo: u32, hi: u32) -> u32 {
        lo + self.next_u32() % (hi - lo).max(1)
    }
}

/// Generates plausible-looking notes for a lane.
fn notes_for(lane: usize, len: u32, rng: &mut Rng) -> Vec<Note> {
    let six = PPQ / 4;
    let mut notes = Vec::new();
    let mut t = 0;
    match lane {
        // Melodic pulse leads: stepwise melody with varied lengths.
        0 | 1 | 5 | 6 => {
            let mut pitch = 60 + rng.range(0, 12) as u8;
            while t < len {
                let l = six * [1, 2, 2, 3, 4][rng.range(0, 5) as usize];
                if rng.f32() > 0.15 {
                    notes.push(mk(t, l.min(len - t), pitch));
                }
                let step = rng.range(0, 7) as i32 - 3;
                pitch = (pitch as i32 + step).clamp(52, 84) as u8;
                t += l;
            }
        }
        // Triangle bass: longer notes, low register, root-fifth movement.
        2 => {
            let roots = [36u8, 41, 43, 38];
            while t < len {
                let root = roots[((t / TICKS_PER_BAR) % 4) as usize];
                let p = if rng.f32() > 0.6 { root + 7 } else { root };
                let l = six * [2, 4, 4, 8][rng.range(0, 4) as usize];
                notes.push(mk(t, l.min(len - t), p));
                t += l;
            }
        }
        // Noise: drum pattern, pitch encodes kick/snare/hat height.
        3 => {
            while t < len {
                let step = (t / six) % 16;
                let p = match step {
                    0 | 8 => 40,
                    4 | 12 => 60,
                    _ => 76,
                };
                if step.is_multiple_of(2) || rng.f32() > 0.6 {
                    notes.push(mk(t, six, p));
                }
                t += six;
            }
        }
        // DPCM: sparse sample hits.
        4 => {
            while t < len {
                if rng.f32() > 0.45 {
                    notes.push(mk(t, six * 2, 48 + rng.range(0, 16) as u8));
                }
                t += PPQ;
            }
        }
        // Saw bass: driving eighths.
        _ => {
            let roots = [33u8, 38, 40, 36];
            while t < len {
                let root = roots[((t / TICKS_PER_BAR) % 4) as usize];
                let p = root + if (t / (six * 2)) % 4 == 3 { 12 } else { 0 };
                notes.push(mk(t, six * 2 - six / 2, p));
                t += six * 2;
            }
        }
    }
    for (i, n) in notes.iter_mut().enumerate() {
        n.id = i as u32 + 1;
    }
    notes
}

const NAMES: [&[&str]; 8] = [
    &[
        "Intro Lead",
        "Hook A",
        "Hook B",
        "Bridge Lead",
        "Outro Lead",
        "Hook A'",
    ],
    &["Harmony", "Arp Climb", "Counter", "Arp Fall", "Echo Lead"],
    &["Bassline", "Walk Down", "Bass Hold", "Bass B"],
    &["Kick Snare", "Hat Roll", "Fill", "Break", "Groove"],
    &["Sample Hit", "Vox Chop", "Timpani", "Crash"],
    &["Pad Swell", "Bell Arp", "Chord Stab"],
    &["Shimmer", "Chord Stab", "Answer"],
    &["Saw Bass", "Saw Drive", "Saw Lead"],
];

/// A composed, screenshot-friendly arrangement (~44 clips over 32 bars).
pub fn demo_arrangement() -> Arrangement {
    let bar = TICKS_PER_BAR;
    // (lane, start bar, length bars, name index)
    let layout: &[(usize, f32, f32, usize)] = &[
        (0, 0.0, 4.0, 0),
        (0, 4.0, 4.0, 1),
        (0, 8.0, 4.0, 2),
        (0, 13.0, 3.0, 3),
        (0, 16.0, 4.0, 1),
        (0, 20.0, 4.0, 5),
        (0, 26.0, 4.0, 4),
        (1, 4.0, 2.0, 1),
        (1, 6.0, 2.0, 0),
        (1, 8.0, 4.0, 2),
        (1, 16.0, 2.0, 1),
        (1, 18.0, 2.0, 3),
        (1, 22.0, 6.0, 4),
        (2, 0.0, 8.0, 0),
        (2, 8.0, 4.0, 1),
        (2, 12.0, 4.0, 2),
        (2, 16.0, 8.0, 3),
        (2, 24.0, 6.0, 0),
        (3, 2.0, 2.0, 1),
        (3, 4.0, 4.0, 0),
        (3, 8.0, 4.0, 4),
        (3, 12.0, 1.0, 2),
        (3, 13.0, 3.0, 3),
        (3, 16.0, 4.0, 0),
        (3, 20.0, 4.0, 4),
        (3, 24.0, 1.0, 2),
        (3, 25.0, 5.0, 0),
        (4, 3.5, 0.5, 3),
        (4, 8.0, 1.0, 0),
        (4, 12.0, 2.0, 1),
        (4, 16.0, 1.0, 2),
        (4, 24.0, 1.0, 0),
        (5, 8.0, 8.0, 0),
        (5, 20.0, 4.0, 1),
        (5, 24.0, 4.0, 2),
        (6, 10.0, 2.0, 1),
        (6, 12.0, 4.0, 0),
        (6, 20.0, 2.0, 2),
        (6, 24.0, 4.0, 1),
        (7, 0.0, 6.0, 0),
        (7, 8.0, 8.0, 1),
        (7, 16.0, 4.0, 2),
        (7, 20.0, 8.0, 0),
    ];
    let mut rng = Rng::new(0x0C41_DA11);
    let clips = layout
        .iter()
        .enumerate()
        .map(|(i, &(lane, start, len, name))| {
            let len_ticks = (len * bar as f32) as u32;
            Arc::new(Clip::new(
                i as u32 + 1,
                lane,
                (start * bar as f32) as u32,
                len_ticks,
                NAMES[lane][name % NAMES[lane].len()].to_string(),
                notes_for(lane, len_ticks, &mut rng),
            ))
        })
        .collect::<Vec<_>>();
    let next_id = clips.len() as u32 + 1;
    Arrangement {
        clips,
        selection: vec![2],
        next_id,
        ..Arrangement::empty()
    }
}

/// 200 clips packed into the visible range, for the performance test (F4).
pub fn stress_arrangement() -> Arrangement {
    let mut rng = Rng::new(0x5EED);
    let mut clips = Vec::with_capacity(200);
    let per_lane = 25;
    for lane in 0..8 {
        let mut t = 0u32;
        for _ in 0..per_lane {
            let len = PPQ * rng.range(2, 6);
            let id = clips.len() as u32 + 1;
            let name = NAMES[lane][rng.range(0, NAMES[lane].len() as u32) as usize].to_string();
            clips.push(Arc::new(Clip::new(id, lane, t, len, name, notes_for(lane, len, &mut rng))));
            t += len + PPQ / 2 * rng.range(0, 2);
        }
    }
    let next_id = clips.len() as u32 + 1;
    Arrangement {
        clips,
        // Zoomed out so all 200 clips are on screen at 1440 wide.
        view: Viewport {
            px_per_tick: 40.0 / TICKS_PER_BAR as f64,
            scroll_ticks: 0.0,
        },
        next_id,
        ..Arrangement::empty()
    }
}

/// Advances fake meters: random walk driven by whether the channel has a note
/// sounding at the playhead, with exponential decay and peak hold.
pub fn step_meters(
    m: &mut Meters,
    arr: &Arrangement,
    playhead: f64,
    playing: bool,
    dt: f32,
    rng: &mut Rng,
) {
    const DECAY_PER_SEC: f32 = 1.8;
    const PEAK_HOLD_SECS: f32 = 0.9;
    const PEAK_FALL_PER_SEC: f32 = 0.5;
    // The strip light fades over 250 ms (design pass §3.5).
    const LIGHT_DECAY_PER_SEC: f32 = 4.0;

    let ph = playhead as u32;
    for lane in 0..8 {
        // Find a note sounding on this lane at the playhead.
        let mut sounding: Option<(u32, u8)> = None;
        if playing {
            for c in arr.clips.iter().filter(|c| c.lane == lane) {
                if ph >= c.start_ticks && ph < c.end_ticks() {
                    let rel = ph - c.start_ticks;
                    if let Some(n) = c
                        .notes
                        .iter()
                        .find(|n| rel >= n.start && rel < n.start + n.len)
                    {
                        sounding = Some((c.start_ticks + n.start, n.pitch));
                        break;
                    }
                }
            }
        }

        let target = match sounding {
            Some((note_start, pitch)) => {
                // Attack right after note-on, then a mild envelope + jitter.
                let age = (ph.saturating_sub(note_start)) as f32 / PPQ as f32;
                let env = (1.0 - age * 0.5).clamp(0.45, 1.0);
                let base = 0.55 + (pitch as f32 % 12.0) / 40.0;
                (base * env + (rng.f32() - 0.5) * 0.12).clamp(0.0, 1.0)
            }
            None => 0.0,
        };

        // Note-on detection for lights: within the first ~30 ticks of a note.
        if let Some((note_start, _)) = sounding {
            if ph.saturating_sub(note_start) < (PPQ / 16) {
                m.light[lane] = 1.0;
            }
        }
        m.light[lane] =
            (m.light[lane] - LIGHT_DECAY_PER_SEC * dt).max(0.0);

        let lvl = &mut m.level[lane];
        if target > *lvl {
            *lvl += (target - *lvl) * 0.6;
        } else {
            *lvl = (*lvl - DECAY_PER_SEC * dt * (*lvl).max(0.2)).max(target);
        }

        if *lvl >= m.peak[lane] {
            m.peak[lane] = *lvl;
            m.peak_hold[lane] = PEAK_HOLD_SECS;
        } else if m.peak_hold[lane] > 0.0 {
            m.peak_hold[lane] -= dt;
        } else {
            m.peak[lane] = (m.peak[lane] - PEAK_FALL_PER_SEC * dt).max(*lvl);
        }
    }
}

/// Real meters: `levels` are the per-channel peaks the engine measured since
/// the last frame (0..1); `note_on` marks channels whose note-on became
/// audible this frame. Same ballistics as the mock meters.
pub fn step_meters_engine(m: &mut Meters, levels: [f32; 8], note_on: [bool; 8], dt: f32) {
    const DECAY_PER_SEC: f32 = 1.8;
    const PEAK_HOLD_SECS: f32 = 0.9;
    const PEAK_FALL_PER_SEC: f32 = 0.5;
    // The strip light fades over 250 ms (design pass §3.5).
    const LIGHT_DECAY_PER_SEC: f32 = 4.0;
    for lane in 0..8 {
        if note_on[lane] {
            m.light[lane] = 1.0;
        }
        m.light[lane] =
            (m.light[lane] - LIGHT_DECAY_PER_SEC * dt).max(0.0);
        let target = levels[lane].clamp(0.0, 1.0);
        let lvl = &mut m.level[lane];
        if target > *lvl {
            *lvl = target;
        } else {
            *lvl = (*lvl - DECAY_PER_SEC * dt * (*lvl).max(0.2)).max(target);
        }
        if *lvl >= m.peak[lane] {
            m.peak[lane] = *lvl;
            m.peak_hold[lane] = PEAK_HOLD_SECS;
        } else if m.peak_hold[lane] > 0.0 {
            m.peak_hold[lane] -= dt;
        } else {
            m.peak[lane] = (m.peak[lane] - PEAK_FALL_PER_SEC * dt).max(*lvl);
        }
    }
}

/// The macro worst case for benchmarks (`CHIPDAW_SONG=heavy`): the demo's
/// clips with every note held for two beats, on every channel an instrument
/// that runs all four macros (volume, arpeggio, pitch, duty) in loops plus
/// vibrato, so the driver writes registers on every frame for every lane.
pub fn heavy_arrangement() -> Arrangement {
    use chip::instrument::{Instrument, Seq, Vibrato, NONE};
    let mut a = demo_arrangement();
    for c in a.clips.iter_mut() {
        let c = Arc::make_mut(c);
        let len = c.len_ticks;
        let pitch = c.notes.first().map_or(60, |n| n.pitch);
        let step = PPQ * 2;
        c.notes = (0..len / step)
            .map(|i| Note::new(i + 1, i * step, step, pitch.wrapping_add((i % 5) as u8)))
            .collect();
        c.next_note_id = c.notes.len() as u32 + 1;
    }
    let inst = Instrument {
        volume: Seq::new(&[15, 14, 13, 12, 11, 12, 13, 14], 0, NONE),
        arp: Seq::new(&[0, 4, 7, 12, 7, 4], 0, NONE),
        pitch: Seq::new(&[3, 3, -3, -3, -3, -3, 3, 3], 0, NONE),
        duty: Seq::new(&[2, 4, 8, 12, 8, 4], 0, NONE),
        vibrato: Vibrato { depth: 30, period: 6, delay: 0 },
        ..Instrument::PLAIN
    };
    for lane in 0..engine::LANES {
        if lane != 4 {
            a.instruments.set_lane(lane, crate::instruments::LaneInstrument { name: "Heavy Macros".into(), inst, kit: Vec::new(), edited: true });
        }
    }
    let clips = a.clips.clone();
    a.instruments.prune(&clips);
    a
}
