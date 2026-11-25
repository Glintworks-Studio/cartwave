//! The factory preset library: instruments built only from what the chips
//! can do (per-frame volume, arpeggio, pitch and duty macros, plus the
//! channel controls). Names describe the character, not a real recording.
//!
//! Each preset lists the channel kinds it suits (`kinds`). Kits reference
//! other presets by name; the UI resolves them into the song's bank.

use crate::driver::ParamId;
use crate::instrument::{Instrument, Seq, Vibrato, KIT_ZONES, NONE};

/// Channel kinds, as a bit mask.
pub const K_PULSE: u8 = 1;
pub const K_TRI: u8 = 2;
pub const K_NOISE: u8 = 4;
pub const K_DMC: u8 = 8;
pub const K_VPULSE: u8 = 16;
pub const K_SAW: u8 = 32;
/// Every pitched channel.
pub const K_MELODIC: u8 = K_PULSE | K_TRI | K_VPULSE | K_SAW;
/// Channels with a volume control (not the triangle).
pub const K_VOICED: u8 = K_PULSE | K_VPULSE | K_SAW;
pub const K_SQUARE: u8 = K_PULSE | K_VPULSE;

/// The kind bit of a lane (0..8).
pub const fn lane_kind(lane: usize) -> u8 {
    match lane {
        0 | 1 => K_PULSE,
        2 => K_TRI,
        3 => K_NOISE,
        4 => K_DMC,
        5 | 6 => K_VPULSE,
        _ => K_SAW,
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Category {
    Lead,
    Bass,
    Strings,
    Winds,
    Brass,
    Keys,
    Chords,
    Drums,
    Kits,
    Sfx,
}

impl Category {
    pub const ALL: [Category; 10] = [
        Category::Lead,
        Category::Bass,
        Category::Strings,
        Category::Winds,
        Category::Brass,
        Category::Keys,
        Category::Chords,
        Category::Drums,
        Category::Kits,
        Category::Sfx,
    ];

    pub fn label(self) -> &'static str {
        match self {
            Category::Lead => "Leads",
            Category::Bass => "Bass",
            Category::Strings => "Strings",
            Category::Winds => "Winds",
            Category::Brass => "Brass",
            Category::Keys => "Keys & Mallets",
            Category::Chords => "Arpeggios",
            Category::Drums => "Drums",
            Category::Kits => "Drum Kits",
            Category::Sfx => "Sound Effects",
        }
    }
}

/// A kit zone: notes up to `top` play preset `preset` (at `pitch`, or the
/// note's own when `NONE`).
#[derive(Clone, Copy, Debug)]
pub struct Zone {
    pub top: u8,
    pub preset: &'static str,
    pub pitch: u8,
}

#[derive(Clone, Copy, Debug)]
pub struct Preset {
    pub name: &'static str,
    pub category: Category,
    pub kinds: u8,
    pub blurb: &'static str,
    pub inst: Instrument,
    /// Channel controls the preset sets (others keep their values).
    pub params: &'static [(ParamId, i8)],
    pub zones: &'static [Zone],
}

impl Preset {
    pub fn fits(&self, lane: usize) -> bool {
        self.kinds & lane_kind(lane) != 0
    }
}

const N: u8 = NONE;

const fn s(v: &[i8], lp: u8, rel: u8) -> Seq {
    Seq::new(v, lp, rel)
}

const fn vib(depth: u8, period: u8, delay: u8) -> Vibrato {
    Vibrato { depth, period, delay }
}

const fn inst(volume: Seq, arp: Seq, pitch: Seq, duty: Seq, vibrato: Vibrato) -> Instrument {
    Instrument { volume, arp, pitch, duty, vibrato, kit_len: 0, kit: [None; KIT_ZONES] }
}

const E: Seq = Seq::EMPTY;
const NOVIB: Vibrato = vib(0, 0, 0);
const CONSTANT: &[(ParamId, i8)] = &[(ParamId::Envelope, 0), (ParamId::Sweep, 0)];
const NOISE_SET: &[(ParamId, i8)] = &[(ParamId::Envelope, 0), (ParamId::NoisePeriod, -1)];

const fn p(name: &'static str, category: Category, kinds: u8, blurb: &'static str, inst: Instrument) -> Preset {
    Preset { name, category, kinds, blurb, inst, params: CONSTANT, zones: &[] }
}

const fn noise(name: &'static str, category: Category, blurb: &'static str, inst: Instrument) -> Preset {
    Preset { name, category, kinds: K_NOISE, blurb, inst, params: NOISE_SET, zones: &[] }
}

const fn kit(name: &'static str, blurb: &'static str, zones: &'static [Zone]) -> Preset {
    Preset { name, category: Category::Kits, kinds: K_NOISE, blurb, inst: Instrument::PLAIN, params: NOISE_SET, zones }
}

const fn dmc(name: &'static str, blurb: &'static str, params: &'static [(ParamId, i8)]) -> Preset {
    Preset { name, category: Category::Drums, kinds: K_DMC, blurb, inst: Instrument::PLAIN, params, zones: &[] }
}

const fn z(top: u8, preset: &'static str, pitch: u8) -> Zone {
    Zone { top, preset, pitch }
}

pub static PRESETS: &[Preset] = &[
    // ── Leads
    p("Square Lead", Category::Lead, K_VOICED, "50% square, gentle decay to sustain, delayed vibrato",
        inst(s(&[15, 14, 13, 12, 12, 11, 8, 6, 4, 2, 0], N, 5), E, E, s(&[8], N, N), vib(18, 6, 20))),
    p("Bright Lead", Category::Lead, K_SQUARE | K_SAW, "25% pulse with a quick bite and vibrato",
        inst(s(&[15, 13, 12, 11, 11, 10, 7, 4, 2, 0], N, 5), E, E, s(&[2, 4], N, N), vib(20, 6, 16))),
    p("Thin Lead", Category::Lead, K_SQUARE, "Nasal 12.5% pulse lead",
        inst(s(&[14, 12, 11, 10, 6, 3, 0], N, 3), E, E, s(&[2], N, N), vib(15, 5, 18))),
    p("Pluck", Category::Lead, K_VOICED, "Short plucked tone, no sustain",
        inst(s(&[15, 12, 10, 8, 7, 6, 5, 4, 3, 3, 2, 2, 1, 1, 0], N, N), E, E, s(&[4], N, N), NOVIB)),
    p("Echo Lead", Category::Lead, K_SQUARE, "Lead with a fake echo tail (driver-style)",
        inst(s(&[14, 12, 11, 11, 10, 0, 0, 0, 0, 7, 6, 0, 0, 0, 0, 4, 3, 0, 0, 0, 0, 2, 1, 0], N, 4), E, E, s(&[8], N, N), vib(14, 6, 24))),
    p("Portamento Lead", Category::Lead, K_VOICED | K_TRI, "Scoops up into every note",
        inst(s(&[13, 14, 15, 13, 12, 9, 6, 3, 0], N, 4), E, s(&[-48, 8, 8, 8, 8, 8, 8], N, N), s(&[4, 8], N, N), vib(16, 6, 20))),
    p("Hero Lead", Category::Lead, K_SQUARE, "Duty sweeps from thin to full: an adventure theme",
        inst(s(&[15, 14, 13, 12, 9, 6, 3, 0], N, 3), E, E, s(&[2, 2, 4, 4, 8], N, N), vib(22, 6, 14))),

    // ── Bass
    p("Triangle Bass", Category::Bass, K_TRI, "The classic: plain triangle, pure low end",
        inst(E, E, E, E, NOVIB)),
    p("Punch Bass", Category::Bass, K_TRI | K_VOICED, "An octave blip on the attack for punch",
        inst(E, s(&[12, 0], N, N), E, E, NOVIB)),
    p("Pulse Bass", Category::Bass, K_SQUARE, "Round 50% bass with a short decay",
        inst(s(&[15, 13, 12, 11, 10, 10, 6, 0], N, 5), E, E, s(&[8], N, N), NOVIB)),
    p("Saw Bass", Category::Bass, K_SAW, "Fat VRC6 saw bass",
        inst(s(&[15, 14, 13, 12, 11, 11, 7, 3, 0], N, 5), E, E, E, NOVIB)),
    p("Slap Bass", Category::Bass, K_VOICED, "Bright click, then a thin decaying body",
        inst(s(&[15, 12, 10, 9, 8, 7, 7, 6, 6, 5, 5, 4, 0], N, 11), s(&[12, 0], N, N), E, s(&[8, 4], N, N), NOVIB)),

    // ── Strings
    p("Cello", Category::Strings, K_VOICED, "Slow bowed attack, rich 25% tone, warm vibrato, soft release",
        inst(s(&[4, 6, 8, 9, 10, 11, 12, 12, 11, 9, 7, 5, 3, 2, 1, 0], N, 7), E, s(&[-6, 2, 2, 2], N, N), s(&[4, 4, 5, 5], 2, N), vib(22, 6, 10))),
    p("Violin", Category::Strings, K_SQUARE | K_SAW, "Thin bowed tone with expressive vibrato",
        inst(s(&[5, 8, 10, 11, 12, 12, 9, 6, 3, 1, 0], N, 5), E, E, s(&[2, 3, 2, 3], 0, N), vib(28, 5, 8))),
    p("Pizzicato", Category::Strings, K_VOICED, "Plucked strings",
        inst(s(&[14, 10, 7, 5, 4, 3, 2, 1, 1, 0], N, N), E, E, s(&[4], N, N), NOVIB)),
    p("String Pad", Category::Strings, K_VOICED, "Very slow swell with a breathing duty cycle",
        inst(s(&[1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 10, 9, 8, 6, 5, 4, 3, 2, 1, 0], N, 10), E, E, s(&[4, 4, 5, 6, 7, 6, 5], 0, N), vib(12, 8, 30))),

    // ── Winds
    p("Flute", Category::Winds, K_VOICED | K_TRI, "Breathy soft attack, slow vibrato; triangle sounds most like a flute",
        inst(s(&[5, 8, 10, 11, 12, 12, 10, 7, 4, 1, 0], N, 5), E, s(&[-4, 1, 1, 1, 1], N, N), s(&[8], N, N), vib(14, 7, 18))),
    p("Clarinet", Category::Winds, K_SQUARE, "Hollow 50% square: odd harmonics, like a clarinet",
        inst(s(&[8, 11, 12, 12, 11, 8, 4, 0], N, 4), E, E, s(&[8], N, N), vib(10, 7, 24))),
    p("Oboe", Category::Winds, K_SQUARE, "Reedy 12.5% pulse with a quick vibrato",
        inst(s(&[7, 10, 11, 11, 8, 4, 0], N, 3), E, E, s(&[2], N, N), vib(18, 5, 14))),
    p("Ocarina", Category::Winds, K_TRI | K_SQUARE, "Pure, round and wobbly",
        inst(s(&[6, 9, 11, 11, 7, 3, 0], N, 3), E, E, s(&[8], N, N), vib(24, 6, 6))),
    p("Pan Pipe", Category::Winds, K_SQUARE, "Chiff on the attack, then a soft tone",
        inst(s(&[12, 6, 8, 10, 10, 7, 4, 1, 0], N, 4), s(&[12, 0], N, N), E, s(&[8], N, N), vib(10, 7, 20))),

    // ── Brass
    p("Brass", Category::Brass, K_VOICED, "Scoop from below with a brightening attack",
        inst(s(&[9, 12, 14, 15, 14, 13, 13, 10, 6, 2, 0], N, 6), E, s(&[-24, 6, 6, 6, 6], N, N), s(&[2, 4, 4, 8], N, N), vib(14, 6, 24))),
    p("Trumpet", Category::Brass, K_SQUARE | K_SAW, "Punchy and bright",
        inst(s(&[12, 15, 14, 13, 12, 12, 8, 4, 0], N, 5), E, s(&[-12, 4, 4, 4], N, N), s(&[4], N, N), vib(16, 6, 18))),
    p("Horn Section", Category::Brass, K_SAW | K_VPULSE, "Soft swell, mellow",
        inst(s(&[4, 7, 9, 11, 12, 12, 9, 5, 2, 0], N, 5), E, E, s(&[6, 7, 8], N, N), vib(10, 7, 24))),

    // ── Keys & mallets
    p("Organ", Category::Keys, K_VOICED | K_TRI, "Steady, no vibrato",
        inst(s(&[13, 12, 0], N, 1), E, E, s(&[8], N, N), NOVIB)),
    p("Harpsichord", Category::Keys, K_SQUARE, "Thin, quick decay",
        inst(s(&[15, 12, 10, 9, 8, 7, 6, 6, 5, 5, 4, 4, 3, 3, 2, 2, 1, 1, 0], N, N), E, E, s(&[2], N, N), NOVIB)),
    p("Bell", Category::Keys, K_VOICED, "Octave strike, then a long ring",
        inst(s(&[15, 14, 12, 11, 10, 10, 9, 9, 8, 8, 7, 7, 6, 6, 6, 5, 5, 5, 4, 4, 4, 3, 3, 3, 2, 2, 2, 1, 1, 1, 0], N, N), s(&[12, 24, 0], N, N), E, s(&[8, 4], N, N), NOVIB)),
    p("Marimba", Category::Keys, K_VOICED | K_TRI, "Woody mallet",
        inst(s(&[15, 11, 8, 6, 4, 3, 2, 1, 0], N, N), s(&[12, 0], N, N), E, s(&[8], N, N), NOVIB)),
    p("Music Box", Category::Keys, K_SQUARE, "Tiny and twinkly",
        inst(s(&[10, 8, 6, 5, 4, 4, 3, 3, 2, 2, 1, 1, 0], N, N), s(&[12], N, N), E, s(&[2], N, N), vib(8, 4, 0))),

    // ── Arpeggios
    p("Major Arp", Category::Chords, K_MELODIC, "Fast major-chord arpeggio: one channel, a full chord",
        inst(s(&[14, 13, 12, 12, 7, 3, 0], N, 3), s(&[0, 4, 7], 0, N), E, s(&[4], N, N), NOVIB)),
    p("Minor Arp", Category::Chords, K_MELODIC, "Fast minor-chord arpeggio",
        inst(s(&[14, 13, 12, 12, 7, 3, 0], N, 3), s(&[0, 3, 7], 0, N), E, s(&[4], N, N), NOVIB)),
    p("Octave Arp", Category::Chords, K_MELODIC, "Octave flip, a big sound from one channel",
        inst(s(&[13, 12, 11, 7, 3, 0], N, 2), s(&[0, 12], 0, N), E, s(&[8], N, N), NOVIB)),
    p("Power Arp", Category::Chords, K_MELODIC, "Root, fifth, octave",
        inst(s(&[14, 13, 12, 12, 7, 3, 0], N, 3), s(&[0, 7, 12], 0, N), E, s(&[8], N, N), NOVIB)),
    p("Sus Arp", Category::Chords, K_MELODIC, "Suspended, unresolved",
        inst(s(&[14, 13, 12, 12, 7, 3, 0], N, 3), s(&[0, 5, 7], 0, N), E, s(&[4], N, N), NOVIB)),

    // ── Drums (single sounds)
    p("Tri Kick", Category::Drums, K_TRI, "Triangle kick: a fast pitch drop (play around C4)",
        inst(s(&[15, 15, 15, 15, 15, 0], N, N), s(&[0, -6, -12, -18, -24, -24], N, N), E, E, NOVIB)),
    p("Tri Tom", Category::Drums, K_TRI, "Triangle tom: a slower drop",
        inst(s(&[15, 15, 15, 15, 15, 15, 15, 15, 0], N, N), E, s(&[-8], 0, N), E, NOVIB)),
    noise("Kick", Category::Drums, "Low, short noise thump",
        inst(s(&[15, 13, 10, 7, 4, 2, 0], N, N), s(&[0, -2, -4, -5, -6, -6], N, N), E, s(&[0], N, N), NOVIB)),
    noise("Snare", Category::Drums, "Crack plus a noisy tail",
        inst(s(&[15, 14, 12, 10, 9, 8, 7, 6, 5, 4, 3, 3, 2, 2, 1, 1, 0], N, N), s(&[3, 0], N, N), E, s(&[0], N, N), NOVIB)),
    noise("Closed Hat", Category::Drums, "Tight and short",
        inst(s(&[12, 8, 5, 2, 0], N, N), E, E, s(&[0], N, N), NOVIB)),
    noise("Open Hat", Category::Drums, "Longer, sizzling",
        inst(s(&[12, 11, 10, 9, 8, 7, 6, 5, 5, 4, 4, 3, 3, 2, 2, 1, 1, 0], N, N), E, E, s(&[0], N, N), NOVIB)),
    noise("Crash", Category::Drums, "Long cymbal wash",
        inst(s(&[15, 14, 13, 12, 12, 11, 11, 10, 10, 9, 9, 9, 8, 8, 8, 7, 7, 7, 6, 6, 6, 6, 5, 5, 5, 5, 4, 4, 4, 4, 3, 3, 3, 3, 2, 2, 2, 2, 1, 1, 1, 1, 0], N, N), E, E, s(&[0], N, N), NOVIB)),
    noise("Metal Hit", Category::Drums, "Short mode: a tuned, metallic clang",
        inst(s(&[14, 12, 10, 8, 6, 5, 4, 3, 2, 1, 0], N, N), E, E, s(&[1], N, N), NOVIB)),
    dmc("Sample Kick", "DPCM kick sample", &[(ParamId::Sample, 0), (ParamId::Rate, 15), (ParamId::Loop, 0)]),
    dmc("Sample Snare", "DPCM snare sample", &[(ParamId::Sample, 1), (ParamId::Rate, 15), (ParamId::Loop, 0)]),
    dmc("Sample Kit", "Pitch picks the sample (C kick, C# snare, D tom, D# crash)", &[(ParamId::Sample, -1), (ParamId::Rate, 15), (ParamId::Loop, 0)]),

    // ── Kits (noise lane; the note picks the drum)
    kit("Standard Kit", "Low rows kick · then snare · closed hat · open hat · crash at the top", &[
        z(47, "Kick", N), z(64, "Snare", N), z(79, "Closed Hat", N), z(87, "Open Hat", N), z(127, "Crash", N),
    ]),
    kit("Tight Kit", "Short and dry: kick, snare, hat, metal", &[
        z(47, "Kick", N), z(64, "Snare", N), z(83, "Closed Hat", N), z(127, "Metal Hit", N),
    ]),

    // ── Sound effects
    noise("Explosion", Category::Sfx, "Big rumbling blast that sinks and fades",
        inst(s(&[15, 15, 15, 14, 14, 13, 13, 12, 12, 11, 11, 10, 10, 10, 9, 9, 9, 8, 8, 8, 7, 7, 7, 6, 6, 6, 5, 5, 5, 4, 4, 4, 4, 3, 3, 3, 3, 2, 2, 2, 2, 1, 1, 1, 1, 0], N, N),
            s(&[2, 1, 0, 0, -1, -1, -2, -2, -3, -3, -4, -4, -5, -5, -6, -6, -7, -7, -8, -8, -9, -9, -10, -10, -11], N, N), E, s(&[0], N, N), NOVIB)),
    noise("Small Explosion", Category::Sfx, "Short pop",
        inst(s(&[15, 14, 12, 10, 8, 7, 6, 5, 4, 3, 2, 1, 0], N, N), s(&[0, -2, -4, -6, -8, -10], N, N), E, s(&[0], N, N), NOVIB)),
    noise("Hit", Category::Sfx, "Damage / impact",
        inst(s(&[15, 13, 10, 7, 4, 2, 0], N, N), s(&[4, 0, -4], N, N), E, s(&[1, 0], N, N), NOVIB)),
    noise("Thunder", Category::Sfx, "Long, rolling rumble",
        inst(s(&[8, 12, 15, 15, 14, 13, 12, 12, 11, 12, 11, 10, 10, 9, 10, 9, 8, 8, 7, 8, 7, 6, 6, 5, 5, 4, 4, 3, 3, 2, 2, 1, 1, 0], N, N), s(&[-8], N, N), E, s(&[0], N, N), NOVIB)),
    noise("Engine", Category::Sfx, "Looping motor hum (hold the note)",
        inst(s(&[10, 8, 10, 8, 3, 0], 0, 3), s(&[-10, -9], 0, N), E, s(&[1], N, N), NOVIB)),
    noise("Wind", Category::Sfx, "Gusting noise (hold the note)",
        inst(s(&[2, 3, 4, 5, 6, 7, 6, 5, 4, 3, 2, 1, 0], 0, 10), s(&[2, 2, 3, 3, 4, 3, 2, 2, 1, 1], 0, N), E, s(&[0], N, N), NOVIB)),
    p("Laser", Category::Sfx, K_SQUARE | K_SAW, "Zap: pitch dives fast",
        inst(s(&[15, 14, 13, 12, 11, 10, 9, 8, 7, 6, 5, 4, 3, 2, 1, 0], N, N), E, s(&[-28], 0, N), s(&[2], N, N), NOVIB)),
    p("Coin", Category::Sfx, K_SQUARE, "Pick-up chime: two quick notes (try B5)",
        inst(s(&[14, 14, 14, 14, 14, 13, 12, 11, 10, 9, 8, 7, 6, 5, 4, 3, 2, 1, 0], N, N), s(&[0, 0, 0, 0, 5], N, N), E, s(&[8], N, N), NOVIB)),
    p("Jump", Category::Sfx, K_SQUARE | K_TRI, "Quick upward sweep",
        inst(s(&[13, 13, 12, 12, 11, 10, 9, 7, 5, 3, 0], N, N), E, s(&[24], 0, N), s(&[4], N, N), NOVIB)),
    p("Power Up", Category::Sfx, K_SQUARE | K_SAW, "Rising arpeggio ladder",
        inst(s(&[13, 13, 13, 13, 13, 13, 13, 13, 13, 13, 13, 13, 12, 12, 11, 11, 10, 10, 8, 6, 4, 2, 0], N, N),
            s(&[0, 4, 7, 2, 6, 9, 4, 8, 11, 7, 11, 14, 12, 16, 19, 24], N, N), E, s(&[4], N, N), NOVIB)),
    p("1-Up", Category::Sfx, K_SQUARE, "Bright fanfare arpeggio",
        inst(s(&[13, 13, 13, 13, 13, 13, 13, 13, 13, 13, 13, 13, 13, 13, 13, 13, 13, 13, 9, 5, 0], N, N),
            s(&[4, 4, 4, 7, 7, 7, 16, 16, 16, 12, 12, 12, 14, 14, 14, 19, 19, 19], N, N), E, s(&[8], N, N), NOVIB)),
    p("Menu Blip", Category::Sfx, K_SQUARE, "Cursor move",
        inst(s(&[12, 10, 6, 0], N, N), E, E, s(&[4], N, N), NOVIB)),
    p("Siren", Category::Sfx, K_SQUARE | K_SAW, "Wailing pitch loop (hold the note)",
        inst(s(&[12, 0], 0, 0), E, s(&[8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, -8, -8, -8, -8, -8, -8, -8, -8, -8, -8, -8, -8], 0, N), s(&[8], N, N), NOVIB)),
    p("Fall", Category::Sfx, K_SQUARE | K_TRI | K_SAW, "Long falling whistle",
        inst(s(&[12, 12, 12, 12, 12, 12, 12, 12, 11, 11, 10, 10, 9, 9, 8, 8, 7, 7, 6, 6, 5, 5, 4, 4, 3, 3, 2, 2, 1, 1, 0], N, N), E, s(&[-6], 0, N), s(&[8], N, N), NOVIB)),
];

/// Looks up a preset by name.
pub fn find(name: &str) -> Option<&'static Preset> {
    PRESETS.iter().find(|p| p.name == name)
}

/// The default preset per lane in a new song (an authentic 2A03 + VRC6 band).
pub const DEFAULT_LAYOUT: [&str; 8] = [
    "Square Lead",
    "Bright Lead",
    "Triangle Bass",
    "Standard Kit",
    "Sample Kit",
    "Cello",
    "Flute",
    "Saw Bass",
];

/// Channel layouts: a whole band in one click (chip + one preset per lane).
#[derive(Clone, Copy, Debug)]
pub struct Layout {
    pub name: &'static str,
    pub blurb: &'static str,
    /// The VRC6 expansion is on.
    pub vrc6: bool,
    pub lanes: [&'static str; 8],
}

pub static LAYOUTS: &[Layout] = &[
    Layout { name: "Classic 2A03 Band", blurb: "Stock console: two pulses, triangle bass, noise drums, samples", vrc6: false,
        lanes: ["Square Lead", "Bright Lead", "Triangle Bass", "Standard Kit", "Sample Kit", "Square Lead", "Square Lead", "Saw Bass"] },
    Layout { name: "2A03 + VRC6 Band", blurb: "Expansion audio: six melodic voices", vrc6: true, lanes: DEFAULT_LAYOUT },
    Layout { name: "Chamber Strings", blurb: "Cello, violin, flute and pizzicato over a triangle bass", vrc6: true,
        lanes: ["Violin", "Pizzicato", "Triangle Bass", "Tight Kit", "Sample Kit", "Cello", "Flute", "Horn Section"] },
    Layout { name: "Arcade Action", blurb: "Punchy leads, arpeggios and a fat saw", vrc6: true,
        lanes: ["Hero Lead", "Major Arp", "Punch Bass", "Standard Kit", "Sample Kit", "Echo Lead", "Octave Arp", "Saw Bass"] },
    Layout { name: "Sound Effects Kit", blurb: "One SFX per channel, ready to audition", vrc6: true,
        lanes: ["Coin", "Jump", "Fall", "Explosion", "Sample Kit", "Laser", "Power Up", "Siren"] },
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn names_unique_and_references_resolve() {
        for (i, a) in PRESETS.iter().enumerate() {
            assert!(PRESETS[i + 1..].iter().all(|b| b.name != a.name), "duplicate {}", a.name);
            for z in a.zones {
                let t = find(z.preset).unwrap_or_else(|| panic!("{}: missing {}", a.name, z.preset));
                assert!(t.zones.is_empty(), "nested kit");
                assert!(t.kinds & a.kinds != 0, "{}: zone {} doesn't fit", a.name, z.preset);
            }
        }
        for l in LAYOUTS {
            for (lane, n) in l.lanes.iter().enumerate() {
                let p = find(n).unwrap_or_else(|| panic!("{}: missing {n}", l.name));
                assert!(p.fits(lane), "{}: {n} doesn't fit lane {lane}", l.name);
            }
        }
    }

    #[test]
    fn sequences_fit() {
        for p in PRESETS {
            for q in [&p.inst.volume, &p.inst.arp, &p.inst.pitch, &p.inst.duty] {
                assert!(q.values().iter().all(|v| (-127..=127).contains(v)));
            }
            assert!(p.inst.volume.values().iter().all(|v| (0..=15).contains(v)), "{}", p.name);
        }
    }

    /// Every preset, on every lane it fits, makes sound (and stays finite)
    /// through the real driver and synth.
    #[test]
    fn every_preset_sounds() {
        use crate::driver::{Note, Song, LANES};
        use crate::player::Player;
        use std::vec::Vec;
        for pr in PRESETS {
            for lane in (0..LANES).filter(|&l| pr.fits(l)) {
                let mut bank: Vec<Instrument> = pr.zones.iter().map(|z| find(z.preset).unwrap().inst).collect();
                let mut head = pr.inst;
                if !pr.zones.is_empty() {
                    head.kit_len = pr.zones.len() as u8;
                    for (i, z) in pr.zones.iter().enumerate() {
                        head.kit[i] = Some(crate::instrument::KitZone { top: z.top, inst: i as u8, fixed_pitch: z.pitch });
                    }
                }
                bank.push(head);
                let pitch = if lane == 2 { 57 } else { 64 };
                let notes = [Note::new(0, 960, pitch)];
                let mut lanes: [&[Note]; LANES] = [&[]; LANES];
                lanes[lane] = &notes;
                let mut lane_inst = [NONE; LANES];
                lane_inst[lane] = (bank.len() - 1) as u8;
                let song = Song { lanes, instruments: &bank, lane_inst, preview_inst: NONE };
                let mut p = Player::new(48_000.0);
                for (id, v) in pr.params {
                    p.driver.params.set(lane, *id, *v as i32);
                }
                p.play();
                let mut buf = std::vec![0f32; 24_000];
                p.render(&song, &mut buf);
                assert!(buf.iter().all(|x| x.is_finite()));
                let mean = buf.iter().sum::<f32>() / buf.len() as f32;
                let rms = (buf.iter().map(|x| (x - mean) * (x - mean)).sum::<f32>() / buf.len() as f32).sqrt();
                assert!(rms > 0.003, "{} on lane {lane} is silent (rms {rms})", pr.name);
            }
        }
    }
}
