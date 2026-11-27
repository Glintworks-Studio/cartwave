//! The project file, `.cartwave` (`.chipdaw` before the rename, which opens
//! forever: RENAME.md) (spec: PIANOROLL.md §5, version 3 changes in
//! STABILIZE.md §B), plus the autosave / recovery files.
//!
//! Versions: 1 (piano roll), 2 (instruments, DPCM samples), 3 (the lane
//! Volume control is a channel volume that scales note volumes; loop start
//! and loop on/off are saved), 4 (track names and colours, clip colours:
//! Batch 3. A version 1–3 lane `name` was informational and is not read, so
//! those files open with the default names, exactly as before), 5 (the
//! project's format, sample rate and bit depth: the design pass. Older files
//! open as 48 kHz / 16-bit, the export default they had), 6 (each track's row
//! height, pan and volume in dB: polish 2. Older files open at 72 px, centre
//! and 0 dB, which renders exactly as before), 7 (Batch 4a: **tracks are
//! instruments**. A list of tracks, each with an id, a voice type, an optional
//! lock to a console channel, a Voices setting, its channel controls and
//! instrument; the hardware profile; the Main track's gain. Older files open
//! with every track locked to its channel (`legacy_tracks`)). Older files are
//! migrated on load so they sound exactly as before (`migrate_v2`).
//!
//! JSON via serde: human-readable, diffable, and easy to migrate. The
//! in-memory model is converted to and from the file structs below, so the
//! format doesn't change by accident when the model does.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use engine::{FilterMode, ParamId, Params, LANES};
use chip::voice::VoiceKind;
use crate::tracks::TrackSet;
use serde::{Deserialize, Serialize};

use crate::instruments::{KitPiece, LaneInstrument};
use crate::notes::{self, Note};
use chip::instrument::{Instrument, Seq, Vibrato, KIT_ZONES, NONE, SEQ_LEN};
use crate::state::{default_lanes, Arrangement, Clip, SoundChip};

/// The `format` field of every project file. It keeps the working title: it
/// identifies the format, not the app, and every file ever saved has it
/// (RENAME.md, RN-D).
pub const FORMAT: &str = "chipdaw";
/// The format version this build writes and reads.
pub const VERSION: u32 = 8;
/// A new project's format (the display's indicators, the export default).
pub const DEFAULT_PROJECT_RATE: u32 = 48_000;
pub const DEFAULT_BIT_DEPTH: u16 = 24;
/// The format version 1–4 files open with (the export default they had).
pub const OLD_BIT_DEPTH: u16 = 16;
/// The rates and depths a project can have (as the export offers).
pub const PROJECT_RATES: [u32; 2] = [44_100, 48_000];
pub const BIT_DEPTHS: [u16; 2] = [16, 24];
/// New projects save as `.cartwave`.
pub const EXTENSION: &str = "cartwave";
/// The extension before the rename: such files open forever, and Save keeps
/// it (Save As offers `.cartwave`).
pub const LEGACY_EXTENSION: &str = "chipdaw";
/// Every extension a project file can have, the current one first.
pub const EXTENSIONS: [&str; 2] = [EXTENSION, LEGACY_EXTENSION];

/// Whether `ext` (any case, no dot) is a project file's extension.
pub fn is_project_extension(ext: &str) -> bool {
    EXTENSIONS.iter().any(|e| e.eq_ignore_ascii_case(ext))
}

/// Everything a project file stores.
#[derive(Clone, Debug, PartialEq)]
pub struct Project {
    pub arrangement: Arrangement,
    pub bpm: f32,
    pub chip: SoundChip,
    pub filter: FilterMode,
    /// The filter follows the chip (the user hasn't picked one).
    pub filter_auto: bool,
    pub loop_end_ticks: u32,
    /// Loop start (version 3; older files: 0).
    pub loop_start_ticks: u32,
    /// The song loop is switched on (version 3; older files: on, the app's default).
    pub loop_on: bool,
    /// Muted and soloed tracks (by id).
    pub mute: TrackSet,
    pub solo: TrackSet,
    /// The Main track's gain in dB (version 7; older files: 0 dB).
    pub master_db: f32,
    /// The project's format (version 5): sample rate and bit depth.
    pub rate: u32,
    pub bits: u16,
    /// What loading changed (migration notes); empty for a current file.
    pub load_report: Vec<String>,
}

impl Project {
    /// The project File > New made before the model change: the legacy eight
    /// locked lanes on the default band, 2A03 + VRC6, the Famicom filter.
    /// Tests, demo songs and tools that build a song lane by lane start here.
    pub fn new_legacy() -> Self {
        Self { arrangement: Arrangement::empty(), chip: SoundChip::WithVrc6, filter: FilterMode::default_for(true), ..Self::new_empty() }
    }

    /// File > New (Batch 4a): studio tracks, no hardware profile (VISION.md
    /// principle 4: authenticity is opt-in), no clips, default settings.
    pub fn new_empty() -> Self {
        Self {
            arrangement: Arrangement::new_project(),
            bpm: 150.0,
            chip: SoundChip::StudioOnly,
            filter: crate::tracks::default_filter(SoundChip::StudioOnly),
            filter_auto: true,
            loop_end_ticks: 32 * crate::state::TICKS_PER_BAR,
            loop_start_ticks: 0,
            loop_on: true,
            mute: TrackSet::default(),
            solo: TrackSet::default(),
            master_db: 0.0,
            rate: DEFAULT_PROJECT_RATE,
            bits: DEFAULT_BIT_DEPTH,
            load_report: Vec::new(),
        }
    }
}

// ───────────────────────────── File structs ─────────────────────────────

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct FileV1 {
    format: String,
    version: u32,
    bpm: f32,
    /// "2A03" or "2A03+VRC6".
    chip: String,
    /// "auto" (follows the chip), or "nes", "nes+440", "famicom", "off".
    filter: String,
    loop_end: u32,
    /// Version 3: the instrument table (each entry has an `id`); lanes and
    /// clips refer to it.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    instruments: Vec<InstV2>,
    /// Version 3: loop start (ticks) and whether the loop is on.
    #[serde(default)]
    loop_start: u32,
    #[serde(default = "yes", rename = "loop")]
    loop_on: bool,
    lanes: Vec<LaneV1>,
    clips: Vec<ClipV1>,
    /// Version 2: imported DPCM samples (hex-encoded delta bytes).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    samples: Vec<SampleV2>,
    /// Version 5: the project's sample rate (Hz) and bit depth.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    rate: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    bits: Option<u16>,
}

/// Version 7: the project file.
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct FileV7 {
    format: String,
    version: u32,
    bpm: f32,
    /// The hardware profile: "none", "nes" or "famicom+vrc6".
    profile: String,
    /// "auto" (follows the profile), or "nes", "nes+440", "famicom", "off".
    filter: String,
    loop_end: u32,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    instruments: Vec<InstV2>,
    #[serde(default)]
    loop_start: u32,
    #[serde(default = "yes", rename = "loop")]
    loop_on: bool,
    /// The Main track's gain in dB (a number, or "-inf"), when not 0 dB.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    master_db: Option<DbValue>,
    tracks: Vec<TrackV7>,
    /// Each clip's `lane` is its track's index in `tracks`.
    clips: Vec<ClipV1>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    samples: Vec<SampleV2>,
    rate: u32,
    bits: u16,
}

/// Version 7: one track.
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct TrackV7 {
    id: u32,
    name: String,
    /// "pulse", "triangle", "noise", "dpcm", "vrc6_pulse" or "saw".
    voice: String,
    /// A locked track's channel: "2a03_p1", "2a03_p2", "2a03_tri",
    /// "2a03_noise", "2a03_dpcm", "vrc6_p1", "vrc6_p2", "vrc6_saw".
    #[serde(default, skip_serializing_if = "Option::is_none")]
    lock: Option<String>,
    /// The Voices setting (most voices at once), when set.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    voices: Option<u16>,
    /// The fader (dB) and pan a locked track had before locking.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    before_lock: Option<(DbValue, i8)>,
    color: String,
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    mute: bool,
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    solo: bool,
    /// Only the controls the voice type has; missing ones are the defaults.
    #[serde(default)]
    params: BTreeMap<String, i32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    height: Option<f32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pan: Option<i8>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    volume_db: Option<DbValue>,
    /// The track's instrument (an id in `instruments`).
    inst: u32,
    /// Version 8 (Batch 4b §5): unison voices per note, detune and drift
    /// (cents), when not the defaults (1, 0, 0). A version 7 file has none.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    unison: Option<u8>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    detune: Option<u8>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    drift: Option<u8>,
}

const VOICE_KEYS: [(VoiceKind, &str); 6] = [
    (VoiceKind::Pulse, "pulse"),
    (VoiceKind::Triangle, "triangle"),
    (VoiceKind::Noise, "noise"),
    (VoiceKind::Dpcm, "dpcm"),
    (VoiceKind::Vrc6Pulse, "vrc6_pulse"),
    (VoiceKind::Saw, "saw"),
];
const CHANNEL_KEYS: [&str; 8] = ["2a03_p1", "2a03_p2", "2a03_tri", "2a03_noise", "2a03_dpcm", "vrc6_p1", "vrc6_p2", "vrc6_saw"];

fn voice_key(k: VoiceKind) -> &'static str {
    VOICE_KEYS.iter().find(|v| v.0 == k).map_or("pulse", |v| v.1)
}

fn profile_key(p: SoundChip) -> &'static str {
    match p {
        SoundChip::StudioOnly => "none",
        SoundChip::Base2A03 => "nes",
        SoundChip::WithVrc6 => "famicom+vrc6",
    }
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct SampleV2 {
    /// User slot 0..4 (DPCM row 5 + slot).
    slot: u8,
    name: String,
    /// DMC rate index 0..=15.
    rate: u8,
    data: String,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct LaneV1 {
    /// The track's name (version 4; before, informational and ignored).
    /// Lanes are identified by their position (0..8).
    name: String,
    /// Version 4: the track's colour, "#RRGGBB", when not the default.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    color: Option<String>,
    #[serde(default)]
    mute: bool,
    #[serde(default)]
    solo: bool,
    /// Only the controls this lane's hardware has; missing ones are the defaults.
    #[serde(default)]
    params: BTreeMap<String, i32>,
    /// Version 6: the row height (logical px), when not 72.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    height: Option<f32>,
    /// Version 6: pan −50 (left) … 50 (right), when not centred.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pan: Option<i8>,
    /// Version 6: the track volume in dB (a number, or "-inf"), when not 0 dB.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    volume_db: Option<DbValue>,
    /// Version 2: the lane's instrument, inline. Missing: the raw channel (v1 sound).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    instrument: Option<InstV2>,
    /// Version 3: the lane's default instrument (an id in `instruments`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    inst: Option<u32>,
}

/// A track volume in the file: dB as a number, or "-inf" for silence (JSON
/// has no infinity).
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(untagged)]
enum DbValue {
    Db(f32),
    Text(String),
}

impl DbValue {
    fn of(db: f32) -> Option<Self> {
        if db == f32::NEG_INFINITY {
            Some(DbValue::Text("-inf".into()))
        } else if db == 0.0 {
            None
        } else {
            Some(DbValue::Db(db))
        }
    }

    fn db(&self) -> Result<f32, String> {
        match self {
            DbValue::Db(v) if v.is_finite() => Ok(v.clamp(crate::state::VOL_DB_MIN_TYPED, crate::state::VOL_DB_MAX)),
            DbValue::Text(t) if t == "-inf" => Ok(f32::NEG_INFINITY),
            other => Err(format!("Bad track volume {other:?} (use dB or \"-inf\").")),
        }
    }
}

/// An instrument. Sequences are text in tracker style: values separated by
/// spaces, `|` before the loop point and `/` before the release point, e.g.
/// `"4 8 | 12 11 / 6 3 0"`.
#[derive(Serialize, Deserialize, Default)]
#[serde(deny_unknown_fields)]
struct InstV2 {
    /// Version 3: the id lanes and clips refer to (table entries only).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    id: Option<u32>,
    name: String,
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    edited: bool,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    volume: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    arpeggio: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pitch: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    duty: String,
    /// [depth cents, period frames, delay frames]
    #[serde(default, skip_serializing_if = "is_zero3")]
    vibrato: [u8; 3],
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    kit: Vec<KitV2>,
}

fn yes() -> bool {
    true
}

fn is_zero3(v: &[u8; 3]) -> bool {
    *v == [0; 3]
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct KitV2 {
    /// Highest pitch of the zone.
    top: u8,
    /// Fixed pitch, if any.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pitch: Option<u8>,
    inst: InstV2,
}

fn seq_text(q: &Seq) -> String {
    let mut out = String::new();
    for (i, v) in q.values().iter().enumerate() {
        if i as u8 == q.loop_at {
            out.push_str("| ");
        }
        out.push_str(&v.to_string());
        out.push(' ');
        if i as u8 == q.release_at {
            out.push_str("/ ");
        }
    }
    out.trim_end().to_string()
}

fn seq_parse(t: &str) -> Result<Seq, String> {
    let (mut vals, mut lp, mut rel) = (Vec::new(), NONE, NONE);
    for tok in t.split_whitespace() {
        match tok {
            "|" => lp = vals.len().min(254) as u8,
            "/" => rel = vals.len().saturating_sub(1).min(254) as u8,
            n => vals.push(n.parse::<i8>().map_err(|_| format!("Bad sequence value \"{n}\" in \"{t}\"."))?),
        }
    }
    if vals.len() > SEQ_LEN {
        return Err(format!("A sequence has {} steps; the most is {SEQ_LEN}.", vals.len()));
    }
    Ok(Seq::new(&vals, lp, rel))
}

fn inst_to_file(name: &str, edited: bool, i: &Instrument) -> InstV2 {
    InstV2 {
        id: None,
        name: name.into(),
        edited,
        volume: seq_text(&i.volume),
        arpeggio: seq_text(&i.arp),
        pitch: seq_text(&i.pitch),
        duty: seq_text(&i.duty),
        vibrato: [i.vibrato.depth, i.vibrato.period, i.vibrato.delay],
        kit: Vec::new(),
    }
}

fn inst_from_file(f: &InstV2) -> Result<Instrument, String> {
    Ok(Instrument {
        volume: seq_parse(&f.volume)?,
        arp: seq_parse(&f.arpeggio)?,
        pitch: seq_parse(&f.pitch)?,
        duty: seq_parse(&f.duty)?,
        vibrato: Vibrato { depth: f.vibrato[0], period: f.vibrato[1], delay: f.vibrato[2] },
        ..Instrument::PLAIN
    })
}

fn lane_inst_to_file(l: &LaneInstrument) -> InstV2 {
    let mut f = inst_to_file(&l.name, l.edited, &l.inst);
    f.kit = l
        .kit
        .iter()
        .map(|k| KitV2 { top: k.top, pitch: (k.pitch != NONE).then_some(k.pitch), inst: inst_to_file(&k.name, false, &k.inst) })
        .collect();
    f
}

/// An instrument as JSON, in the project file's format (user presets).
pub fn instrument_json(l: &LaneInstrument) -> serde_json::Value {
    serde_json::to_value(lane_inst_to_file(l)).unwrap_or_default()
}

/// The inverse of `instrument_json`; errors are user-facing.
pub fn instrument_from_json(v: serde_json::Value) -> Result<LaneInstrument, String> {
    let f: InstV2 = serde_json::from_value(v).map_err(|e| format!("A damaged instrument: {e}"))?;
    lane_inst_from_file(&f)
}

fn lane_inst_from_file(f: &InstV2) -> Result<LaneInstrument, String> {
    let kit = f
        .kit
        .iter()
        .take(KIT_ZONES)
        .map(|k| {
            Ok(KitPiece { top: k.top, name: k.inst.name.clone(), inst: inst_from_file(&k.inst)?, pitch: k.pitch.unwrap_or(NONE) })
        })
        .collect::<Result<Vec<_>, String>>()?;
    Ok(LaneInstrument { name: f.name.clone(), inst: inst_from_file(f)?, kit, edited: f.edited })
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct ClipV1 {
    id: u32,
    lane: usize,
    start: u32,
    len: u32,
    name: String,
    /// Version 3: the clip's own instrument (an id); missing: the lane's.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    instrument: Option<u32>,
    /// Version 4: the clip's own colour, "#RRGGBB"; missing: the track's.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    color: Option<String>,
    notes: Vec<Note>,
}

/// Just the header, read first so a newer file fails with a clear message
/// rather than a field error.
#[derive(Deserialize)]
struct Header {
    format: Option<String>,
    version: Option<u32>,
}

fn param_key(p: ParamId) -> &'static str {
    match p {
        ParamId::Duty => "duty",
        ParamId::Volume => "volume",
        ParamId::Envelope => "envelope",
        ParamId::Sweep => "sweep",
        ParamId::SweepRate => "sweep_rate",
        ParamId::NoiseMode => "noise_mode",
        ParamId::NoisePeriod => "noise_period",
        ParamId::Sample => "sample",
        ParamId::Rate => "rate",
        ParamId::Loop => "loop",
    }
}

fn filter_key(f: FilterMode) -> &'static str {
    match f {
        FilterMode::Off => "off",
        FilterMode::Nes => "nes",
        FilterMode::NesWith440 => "nes+440",
        FilterMode::Famicom => "famicom",
    }
}

/// A colour as file text: "#RRGGBB".
fn hex_text(c: u32) -> String {
    format!("#{c:06X}")
}

/// "#RRGGBB" → a colour; errors are user-facing.
pub fn parse_hex(t: &str) -> Result<crate::theme::tokens::Rgba, String> {
    let h = t.strip_prefix('#').filter(|h| h.len() == 6 && h.bytes().all(|b| b.is_ascii_hexdigit())).ok_or_else(|| format!("Bad colour \"{t}\" (use #RRGGBB)."))?;
    Ok(crate::theme::tokens::Rgba::hex(u32::from_str_radix(h, 16).map_err(|_| format!("Bad colour \"{t}\"."))?))
}

// ───────────────────────────── Save / load ─────────────────────────────

/// The project as file text (pretty-printed JSON, one note per line).
pub fn to_string(p: &Project) -> String {
    let a = &p.arrangement;
    let tracks = a
        .lanes
        .iter()
        .enumerate()
        .map(|(i, l)| TrackV7 {
            id: l.id,
            name: l.name.clone(),
            voice: voice_key(l.voice).into(),
            lock: l.lock.map(|c| CHANNEL_KEYS[c.min(7)].into()),
            voices: (l.voices > 0).then_some(l.voices),
            before_lock: l.before_lock.map(|(db, pan)| (DbValue::of(db).unwrap_or(DbValue::Db(0.0)), pan)),
            color: hex_text(crate::state::rgb_hex(l.color)),
            mute: p.mute.contains(l.id),
            solo: p.solo.contains(l.id),
            params: ParamId::for_lane(l.chan()).iter().map(|&id| (param_key(id).to_string(), l.params[id as usize] as i32)).collect(),
            height: Some(l.height).filter(|h| *h != crate::state::ROW_H_MIN),
            pan: Some(l.pan).filter(|v| *v != 0),
            volume_db: DbValue::of(l.vol_db),
            inst: a.instruments.lane_id(i),
            unison: Some(l.unison.voices).filter(|v| *v != 1),
            detune: Some(l.unison.detune).filter(|v| *v != 0),
            drift: Some(l.unison.drift).filter(|v| *v != 0),
        })
        .collect();
    let clips = a
        .clips
        .iter()
        .map(|c| ClipV1 {
            id: c.id,
            lane: c.lane,
            start: c.start_ticks,
            len: c.len_ticks,
            name: c.name.clone(),
            instrument: c.instrument,
            color: c.color.map(|c| hex_text(crate::state::rgb_hex(c))),
            notes: c.notes.clone(),
        })
        .collect();
    let instruments = a.instruments.entries.iter().map(|e| InstV2 { id: Some(e.id), ..lane_inst_to_file(&e.inst) }).collect();
    let f = FileV7 {
        format: FORMAT.into(),
        version: VERSION,
        bpm: p.bpm,
        profile: profile_key(p.chip).into(),
        filter: if p.filter_auto { "auto".into() } else { filter_key(p.filter).into() },
        loop_end: p.loop_end_ticks,
        instruments,
        loop_start: p.loop_start_ticks,
        loop_on: p.loop_on,
        master_db: DbValue::of(p.master_db),
        tracks,
        clips,
        samples: a
            .samples
            .iter()
            .enumerate()
            .filter_map(|(i, s)| s.as_ref().map(|s| (i, s)))
            .map(|(i, s)| SampleV2 { slot: i as u8, name: s.name.clone(), rate: s.rate, data: crate::dpcm::to_hex(&s.bytes) })
            .collect(),
        rate: p.rate,
        bits: p.bits,
    };
    compact_notes(&serde_json::to_string_pretty(&f).expect("project serialises"))
}

/// without one line per field (a 50k-note song is ~50k lines, not ~300k).
fn compact_notes(pretty: &str) -> String {
    let mut out = String::with_capacity(pretty.len());
    let mut lines = pretty.lines().peekable();
    let mut in_notes = false;
    while let Some(line) = lines.next() {
        let t = line.trim_start();
        if t.starts_with("\"notes\": [") && !t.ends_with("[]") && !t.ends_with("],") {
            in_notes = true;
            out.push_str(line);
            out.push('\n');
            continue;
        }
        if in_notes {
            if t == "{" {
                let indent = &line[..line.len() - t.len()];
                let mut obj = String::from("{");
                // Depth of objects nested inside the note (future effects).
                let mut depth = 0i32;
                for inner in lines.by_ref() {
                    let it = inner.trim();
                    if depth == 0 && (it == "}" || it == "},") {
                        obj.push_str(" }");
                        if it.ends_with(',') {
                            obj.push(',');
                        }
                        break;
                    }
                    depth += it.matches('{').count() as i32 - it.matches('}').count() as i32;
                    obj.push(' ');
                    obj.push_str(it);
                }
                out.push_str(indent);
                out.push_str(&obj);
                out.push('\n');
                continue;
            }
            if t.starts_with(']') {
                in_notes = false;
            }
        }
        out.push_str(line);
        out.push('\n');
    }
    out
}

/// Parses project text. Errors are user-facing messages.
pub fn from_str(text: &str) -> Result<Project, String> {
    let h: Header = serde_json::from_str(text).map_err(|e| format!("Not a Cartwave project: {e}"))?;
    if h.format.as_deref() != Some(FORMAT) {
        return Err("Not a Cartwave project (its format field is missing or names another format).".into());
    }
    match h.version {
        Some(1..=VERSION) => {}
        Some(v) if v > VERSION => {
            return Err(format!(
                "This project was saved by a newer Cartwave (file format version {v}). \
                 This version reads format {VERSION}. Update Cartwave to open it."
            ))
        }
        Some(v) => return Err(format!("Unsupported file format version {v}.")),
        None => return Err("The project file has no format version.".into()),
    }
    // Version 8 is version 7 plus optional unison fields (same reader).
    if matches!(h.version, Some(7 | 8)) {
        return from_v7(text);
    }
    let f: FileV1 = serde_json::from_str(text).map_err(|e| format!("The project file is damaged: {e}"))?;

    let chip = match f.chip.as_str() {
        "2A03" => SoundChip::Base2A03,
        "2A03+VRC6" => SoundChip::WithVrc6,
        other => return Err(format!("Unknown chip \"{other}\".")),
    };
    let (filter, filter_auto) = match f.filter.as_str() {
        "auto" => (FilterMode::default_for(chip == SoundChip::WithVrc6), true),
        key => (
            FilterMode::ALL
                .into_iter()
                .find(|m| filter_key(*m) == key)
                .ok_or_else(|| format!("Unknown filter \"{key}\"."))?,
            false,
        ),
    };
    if f.lanes.len() != LANES {
        return Err(format!("Expected {LANES} lanes, found {}.", f.lanes.len()));
    }
    let mut params = Params::default();
    let (mut mute, mut solo) = (0u8, 0u8);
    for (lane, l) in f.lanes.iter().enumerate() {
        mute |= (l.mute as u8) << lane;
        solo |= (l.solo as u8) << lane;
        for (k, v) in &l.params {
            let id = ParamId::for_lane(lane)
                .iter()
                .copied()
                .find(|p| param_key(*p) == k)
                .ok_or_else(|| format!("Lane {} has no control \"{k}\".", lane + 1))?;
            params.set(lane, id, *v);
        }
    }
    let mut arrangement = Arrangement::empty();
    arrangement.instruments = instrument_table(&f)?;
    // Version 4: track names and colours (older files keep the defaults).
    if h.version.unwrap_or(VERSION) >= 4 {
        for (lane, l) in f.lanes.iter().enumerate() {
            let name: String = l.name.trim().chars().filter(|c| !c.is_control()).take(crate::user_presets::MAX_NAME).collect();
            if !name.is_empty() {
                arrangement.lanes[lane].name = name;
            }
            if let Some(c) = &l.color {
                arrangement.lanes[lane].color = parse_hex(c)?;
            }
        }
    }
    // Version 6: row heights, pan and volume (older files: 72 px, C, 0 dB).
    if h.version.unwrap_or(VERSION) >= 6 {
        for (lane, l) in f.lanes.iter().enumerate() {
            let t = &mut arrangement.lanes[lane];
            if let Some(hgt) = l.height {
                if !hgt.is_finite() {
                    return Err(format!("Lane {} has a bad height.", lane + 1));
                }
                t.height = crate::state::clamp_row_h(hgt);
            }
            if let Some(v) = l.pan {
                t.pan = v.clamp(-crate::state::PAN_MAX, crate::state::PAN_MAX);
            }
            if let Some(v) = &l.volume_db {
                t.vol_db = v.db()?;
            }
        }
    }
    let clips = load_clips(f.clips, &arrangement, h.version.unwrap_or(VERSION))?;
    arrangement.clips = clips;
    arrangement.next_id = arrangement.clips.iter().map(|c| c.id + 1).max().unwrap_or(1);
    arrangement.samples = load_samples(&f.samples)?;
    let loop_end_ticks = f.loop_end.clamp(crate::state::PPQ, crate::state::MAX_SONG_TICKS);
    let mut p = Project {
        arrangement,
        bpm: f.bpm.clamp(crate::state::Transport::MIN_BPM, crate::state::Transport::MAX_BPM),
        chip,
        filter,
        filter_auto,
        loop_end_ticks,
        loop_start_ticks: f.loop_start.min(loop_end_ticks.saturating_sub(crate::state::PPQ)),
        loop_on: f.loop_on,
        mute: TrackSet::default(),
        solo: TrackSet::default(),
        master_db: 0.0,
        // Version 5: the project's format; older files had 48 kHz / 16-bit
        // as their export default.
        rate: match f.rate {
            Some(r) if PROJECT_RATES.contains(&r) => r,
            Some(r) => return Err(format!("Unsupported project sample rate {r} Hz.")),
            None => DEFAULT_PROJECT_RATE,
        },
        bits: match f.bits {
            Some(b) if BIT_DEPTHS.contains(&b) => b,
            Some(b) => return Err(format!("Unsupported project bit depth {b}.")),
            None => OLD_BIT_DEPTH,
        },
        load_report: Vec::new(),
    };
    if h.version.unwrap_or(VERSION) < 3 {
        migrate_v2(&mut p.arrangement, &mut params, &mut p.load_report);
    }
    legacy_tracks(&mut p, &params, mute, solo);
    Ok(p)
}

/// Format 6 and older → 7 (Batch 4a §4.2): the eight lanes become eight
/// tracks, **each locked to its channel**, so the song plays exactly as
/// before. The profile: the old chip setting, except that a 2A03 + VRC6
/// project whose VRC6 lanes have no clips becomes NES (2A03). Its "auto"
/// filter followed the old chip (the Famicom stage), so it is kept as an
/// explicit choice: the render stays bit-identical. Empty VRC6 lanes under
/// the NES profile become studio tracks (a lock onto a channel the profile
/// lacks would show as a problem in every such project). A lane whose fader
/// or pan wasn't at 0 dB / centre (format 6) is locked at 0 dB / centre like
/// any locked track, its values kept for unlocking, and the report says so.
fn legacy_tracks(p: &mut Project, params: &Params, mute: u8, solo: u8) {
    let a = &mut p.arrangement;
    let vrc6_clips = a.clips.iter().any(|c| c.lane >= chip::driver::LANE_V1);
    if p.chip == SoundChip::WithVrc6 && !vrc6_clips {
        p.chip = SoundChip::Base2A03;
        if p.filter_auto {
            p.filter_auto = false;
            p.filter = FilterMode::default_for(true);
        }
    }
    for (i, l) in a.lanes.iter_mut().enumerate() {
        l.params = params.values[i];
        if mute & (1 << i) != 0 {
            p.mute.set(l.id, true);
        }
        if solo & (1 << i) != 0 {
            p.solo.set(l.id, true);
        }
        let in_profile = p.chip.has_channel(i);
        let empty = !a.clips.iter().any(|c| c.lane == i);
        if !in_profile && empty {
            l.lock = None;
            l.short = crate::tracks::short_for(l);
        } else if l.vol_db != 0.0 || l.pan != 0 {
            // Pan isn't heard yet (Batch 4b): only a fader changes the sound.
            if l.vol_db != 0.0 {
                p.load_report.push(format!(
                    "{}: locked to its channel, so it plays at the chip's level now (its fader was at {}); unlock it to get that back.",
                    l.name,
                    crate::state::format_db(l.vol_db)
                ));
            }
            l.before_lock = Some((l.vol_db, l.pan));
            l.vol_db = 0.0;
            l.pan = 0;
        }
    }
}

/// Reads and checks the clips (any version) against the tracks.
fn load_clips(clips: Vec<ClipV1>, arrangement: &Arrangement, version: u32) -> Result<Vec<Arc<Clip>>, String> {
    let tracks = arrangement.lanes.len();
    let mut out = Vec::with_capacity(clips.len());
    let mut seen = std::collections::HashSet::new();
    for c in clips {
        if c.lane >= tracks {
            return Err(format!("Clip {} is on track {}; there are {tracks}.", c.id, c.lane.saturating_add(1)));
        }
        if !seen.insert(c.id) {
            return Err(format!("Two clips have the id {}.", c.id));
        }
        if c.id >= crate::state::MAX_ID {
            return Err(format!("Clip id {} is out of range.", c.id));
        }
        let too_late = |end: u64| end > crate::state::MAX_SONG_TICKS as u64;
        if too_late(c.start as u64 + c.len as u64) {
            return Err(format!("Clip {} ends after bar {}, the end of a song.", c.id, crate::state::MAX_SONG_BARS));
        }
        if c.notes.iter().any(|n| too_late(c.start as u64 + n.start as u64 + n.len as u64)) {
            return Err(format!("A note in clip {} ends after bar {}, the end of a song.", c.id, crate::state::MAX_SONG_BARS));
        }
        if let Some(i) = c.instrument.filter(|i| !arrangement.instruments.contains(*i)) {
            return Err(format!("Clip {} uses instrument {i}, which isn't in the file.", c.id));
        }
        let color = match &c.color {
            Some(t) if version >= 4 => Some(parse_hex(t)?),
            _ => None,
        };
        let mut clip = Clip::new(c.id, c.lane, c.start, c.len.max(1), c.name, c.notes);
        clip.instrument = c.instrument;
        clip.color = color;
        // Hand-edited files: restore the invariants instead of failing.
        clip.notes.retain(|n| n.len > 0);
        for n in &mut clip.notes {
            n.volume = n.volume.map(|v| v.min(notes::MAX_VOLUME));
        }
        let mut ids = std::collections::HashSet::new();
        // Ids at or past MAX_ID are renumbered after the largest valid one.
        let mut next = clip.notes.iter().map(|n| n.id).filter(|&i| i < crate::state::MAX_ID).max().map_or(1, |m| m + 1);
        for n in &mut clip.notes {
            if n.id >= crate::state::MAX_ID || !ids.insert(n.id) {
                n.id = next;
                next += 1;
            }
        }
        // Locked tracks are monophonic (the tracker rule); a studio track
        // keeps its chords.
        if arrangement.lanes[clip.lane].locked() {
            notes::resolve_mono(&mut clip.notes, &[], &mut next);
        } else {
            notes::sort_poly(&mut clip.notes);
        }
        clip.next_note_id = next;
        out.push(Arc::new(clip));
    }
    Ok(out)
}

/// Reads the imported DPCM samples.
fn load_samples(samples: &[SampleV2]) -> Result<Vec<Option<crate::dpcm::UserSample>>, String> {
    let mut out: Vec<Option<crate::dpcm::UserSample>> = vec![None; chip::samples::USER_SLOTS];
    for s in samples {
        let slot = out.get_mut(s.slot as usize).ok_or_else(|| format!("Sample slot {} doesn't exist.", s.slot))?;
        *slot = Some(crate::dpcm::UserSample {
            name: s.name.clone(),
            rate: s.rate.min(15),
            bytes: crate::dpcm::from_hex(&s.data)?.into_iter().take(chip::samples::MAX_SAMPLE_BYTES).collect(),
        });
    }
    if crate::dpcm::used_bytes(&out) > chip::samples::USER_ROM {
        return Err("The project's DPCM samples don't fit in the sample memory.".into());
    }
    if out.iter().all(|s| s.is_none()) {
        out.clear();
    }
    Ok(out)
}

/// A version 7 file.
fn from_v7(text: &str) -> Result<Project, String> {
    use crate::instruments::{Entry, InstrumentTable, MAX_INSTRUMENTS};
    let f: FileV7 = serde_json::from_str(text).map_err(|e| format!("The project file is damaged: {e}"))?;
    let chip = match f.profile.as_str() {
        "none" => SoundChip::StudioOnly,
        "nes" => SoundChip::Base2A03,
        "famicom+vrc6" => SoundChip::WithVrc6,
        other => return Err(format!("Unknown hardware profile \"{other}\".")),
    };
    let (filter, filter_auto) = match f.filter.as_str() {
        "auto" => (crate::tracks::default_filter(chip), true),
        key => (FilterMode::ALL.into_iter().find(|m| filter_key(*m) == key).ok_or_else(|| format!("Unknown filter \"{key}\"."))?, false),
    };
    if f.tracks.is_empty() {
        return Err("The project has no tracks.".into());
    }
    if f.instruments.len() > MAX_INSTRUMENTS {
        return Err(format!("The file has {} instruments; the most is {MAX_INSTRUMENTS}.", f.instruments.len()));
    }
    let mut entries: Vec<Entry> = Vec::with_capacity(f.instruments.len());
    for i in &f.instruments {
        let id = i.id.ok_or("An instrument in the table has no id.")?;
        if id == 0 || id >= crate::state::MAX_ID || entries.iter().any(|e| e.id == id) {
            return Err(format!("Instrument id {id} is out of range or used twice."));
        }
        entries.push(Entry { id, inst: lane_inst_from_file(i)? });
    }
    let mut arrangement = Arrangement::empty();
    arrangement.lanes.clear();
    let (mut mute, mut solo) = (TrackSet::default(), TrackSet::default());
    let mut lanes_inst = Vec::with_capacity(f.tracks.len());
    let mut taken = [false; LANES];
    for (n, t) in f.tracks.iter().enumerate() {
        let voice = VOICE_KEYS.iter().find(|v| v.1 == t.voice).map(|v| v.0).ok_or_else(|| format!("Track {} has an unknown voice type \"{}\".", n + 1, t.voice))?;
        if t.id == 0 || t.id >= crate::state::MAX_ID || arrangement.lanes.iter().any(|l| l.id == t.id) {
            return Err(format!("Track {} has a bad or repeated id {}.", n + 1, t.id));
        }
        let lock = match &t.lock {
            None => None,
            Some(k) => {
                let c = CHANNEL_KEYS.iter().position(|x| x == k).ok_or_else(|| format!("Track {} is locked to an unknown channel \"{k}\".", n + 1))?;
                if chip::voice::VoiceKind::of_lane(c) != voice {
                    return Err(format!("Track {} is a {} track locked to a {} channel.", n + 1, crate::tracks::kind_name(voice), crate::tracks::kind_name(VoiceKind::of_lane(c))));
                }
                if std::mem::replace(&mut taken[c], true) {
                    return Err(format!("Two tracks are locked to {}.", crate::tracks::channel_name(c)));
                }
                Some(c)
            }
        };
        let name: String = t.name.trim().chars().filter(|c| !c.is_control()).take(crate::user_presets::MAX_NAME).collect();
        // An empty name: a locked track's channel name ("Pulse 1"), else the type's.
        let fallback = match lock {
            Some(c) => default_lanes()[c].name.clone(),
            None => crate::tracks::kind_name(voice).into(),
        };
        let mut l = crate::state::Lane::studio(t.id, voice, if name.is_empty() { fallback } else { name }, parse_hex(&t.color)?);
        l.lock = lock;
        l.short = crate::tracks::short_for(&l);
        l.voices = t.voices.unwrap_or(0);
        if let Some((db, pan)) = &t.before_lock {
            l.before_lock = Some((db.db()?, (*pan).clamp(-crate::state::PAN_MAX, crate::state::PAN_MAX)));
        }
        let chan = l.chan();
        let mut row = Params::default();
        for (k, v) in &t.params {
            let id = ParamId::for_lane(chan).iter().copied().find(|p| param_key(*p) == k).ok_or_else(|| format!("Track {} has no control \"{k}\".", n + 1))?;
            row.set(chan, id, *v);
        }
        l.params = row.values[chan];
        if let Some(hgt) = t.height {
            if !hgt.is_finite() {
                return Err(format!("Track {} has a bad height.", n + 1));
            }
            l.height = crate::state::clamp_row_h(hgt);
        }
        l.pan = t.pan.unwrap_or(0).clamp(-crate::state::PAN_MAX, crate::state::PAN_MAX);
        let d = crate::state::Unison::default();
        l.unison = crate::state::Unison { voices: t.unison.unwrap_or(d.voices), detune: t.detune.unwrap_or(d.detune), drift: t.drift.unwrap_or(d.drift) }.clamped();
        if let Some(v) = &t.volume_db {
            l.vol_db = v.db()?;
        }
        if !entries.iter().any(|e| e.id == t.inst) {
            return Err(format!("Track {} uses instrument {}, which isn't in the file.", n + 1, t.inst));
        }
        lanes_inst.push(t.inst);
        mute.set(t.id, t.mute);
        solo.set(t.id, t.solo);
        arrangement.lanes.push(l);
    }
    let next_id = entries.iter().map(|e| e.id).max().unwrap_or(0) + 1;
    arrangement.instruments = InstrumentTable { entries, lanes: lanes_inst, next_id };
    arrangement.clips = load_clips(f.clips, &arrangement, 7)?;
    arrangement.next_id = arrangement.clips.iter().map(|c| c.id + 1).max().unwrap_or(1);
    arrangement.samples = load_samples(&f.samples)?;
    arrangement.insert = (0, 0);
    let loop_end_ticks = f.loop_end.clamp(crate::state::PPQ, crate::state::MAX_SONG_TICKS);
    Ok(Project {
        arrangement,
        bpm: f.bpm.clamp(crate::state::Transport::MIN_BPM, crate::state::Transport::MAX_BPM),
        chip,
        filter,
        filter_auto,
        loop_end_ticks,
        loop_start_ticks: f.loop_start.min(loop_end_ticks.saturating_sub(crate::state::PPQ)),
        loop_on: f.loop_on,
        mute,
        solo,
        master_db: match &f.master_db {
            Some(v) => v.db()?,
            None => 0.0,
        },
        rate: if PROJECT_RATES.contains(&f.rate) { f.rate } else { return Err(format!("Unsupported project sample rate {} Hz.", f.rate)) },
        bits: if BIT_DEPTHS.contains(&f.bits) { f.bits } else { return Err(format!("Unsupported project bit depth {}.", f.bits)) },
        load_report: Vec::new(),
    })
}

/// The instrument table: version 3's `instruments` (lanes refer to ids), or
/// version 2's inline lane instruments, or raw channels (version 1).
fn instrument_table(f: &FileV1) -> Result<crate::instruments::InstrumentTable, String> {
    use crate::instruments::{Entry, InstrumentTable, MAX_INSTRUMENTS};
    if f.instruments.is_empty() {
        if let Some(l) = f.lanes.iter().position(|l| l.inst.is_some()) {
            return Err(format!("Lane {} refers to an instrument, but the file has none.", l + 1));
        }
        return Ok(InstrumentTable::from_lanes(
            f.lanes.iter().map(|l| l.instrument.as_ref().map_or(Ok(LaneInstrument::plain()), lane_inst_from_file)).collect::<Result<_, _>>()?,
        ));
    }
    if f.instruments.len() > MAX_INSTRUMENTS {
        return Err(format!("The file has {} instruments; the most is {MAX_INSTRUMENTS}.", f.instruments.len()));
    }
    let mut entries: Vec<Entry> = Vec::with_capacity(f.instruments.len());
    for i in &f.instruments {
        let id = i.id.ok_or("An instrument in the table has no id.")?;
        if id == 0 || id >= crate::state::MAX_ID || entries.iter().any(|e| e.id == id) {
            return Err(format!("Instrument id {id} is out of range or used twice."));
        }
        entries.push(Entry { id, inst: lane_inst_from_file(i)? });
    }
    let mut lanes = vec![0; LANES];
    for (lane, l) in f.lanes.iter().enumerate() {
        if l.instrument.is_some() {
            return Err(format!("Lane {} has an inline instrument next to an instrument table.", lane + 1));
        }
        let id = l.inst.ok_or_else(|| format!("Lane {} has no instrument.", lane + 1))?;
        if !entries.iter().any(|e| e.id == id) {
            return Err(format!("Lane {} uses instrument {id}, which isn't in the file.", lane + 1));
        }
        lanes[lane] = id;
    }
    let next_id = entries.iter().map(|e| e.id).max().unwrap_or(0) + 1;
    Ok(InstrumentTable { entries, lanes, next_id })
}

/// Version 1/2 → 3. Before version 3 a note's own volume *replaced* the
/// lane's Volume control (V); now the control is a channel volume that
/// scales it (out = note × macro × channel). For a lane that mixes notes with
/// and without their own volume under V < 15, the old sound is kept exactly:
/// notes without a volume get V as their own, and the control goes to 15
/// (which scales by exactly 1). The saw does the same with its Rate control
/// (42 = scale 1); a rate no note volume can express is approximated and
/// reported.
pub fn migrate_v2(a: &mut Arrangement, params: &mut Params, report: &mut Vec<String>) {
    use chip::driver::saw_rate_for_volume;
    const VOLUME_LANES: [usize; 5] = [0, 1, 3, 5, 6];
    const SAW: usize = 7;
    let has: [bool; LANES] =
        std::array::from_fn(|lane| a.clips.iter().any(|c| c.lane == lane && c.notes.iter().any(|n| n.volume.is_some())));
    let has_volumes = |lane: usize| has[lane];
    let mut fill: Vec<(usize, u8)> = Vec::new();
    for lane in VOLUME_LANES {
        let v = params.get(lane, ParamId::Volume) as u8;
        if v != 15 && has_volumes(lane) {
            fill.push((lane, v));
            params.set(lane, ParamId::Volume, 15);
        }
    }
    let r = params.get(SAW, ParamId::Rate) as u8;
    if r != 42 && has_volumes(SAW) {
        let exact = (0..=15u8).find(|&v| saw_rate_for_volume(v) == r);
        let v = exact.unwrap_or_else(|| (0..=15u8).min_by_key(|&v| (saw_rate_for_volume(v) as i32 - r as i32).abs()).unwrap_or(15));
        if exact.is_none() {
            report.push(format!(
                "VRC6 Saw: the Rate control ({r}) can't be kept exactly next to per-note volumes; notes without a volume now play at rate {}.",
                saw_rate_for_volume(v)
            ));
        }
        fill.push((SAW, v));
        params.set(SAW, ParamId::Rate, 42);
    }
    if fill.is_empty() {
        return;
    }
    let ids: Vec<u32> = a.clips.iter().filter(|c| fill.iter().any(|f| f.0 == c.lane)).map(|c| c.id).collect();
    for id in ids {
        let Some(c) = a.clip_mut(id) else { continue };
        let v = fill.iter().find(|f| f.0 == c.lane).map_or(15, |f| f.1);
        for n in &mut c.notes {
            n.volume.get_or_insert(v);
        }
    }
    report.push(format!(
        "Upgraded from an older version: {} lane(s) moved their volume into the notes, so the song sounds the same.",
        fill.len()
    ));
}

/// Writes atomically: a temporary file next to the target, then a rename,
/// so a crash mid-save never leaves a half-written project.
pub fn save(p: &Project, path: &Path) -> Result<(), String> {
    let text = to_string(p);
    // `song.cartwave.tmp` (or `song.chipdaw.tmp`): next to the target.
    let ext = path.extension().and_then(|e| e.to_str()).unwrap_or(EXTENSION);
    let tmp = path.with_extension(format!("{ext}.tmp"));
    std::fs::write(&tmp, text).map_err(|e| format!("Can't write {}: {e}", tmp.display()))?;
    std::fs::rename(&tmp, path).map_err(|e| format!("Can't save {}: {e}", path.display()))
}

pub fn load(path: &Path) -> Result<Project, String> {
    let text = std::fs::read_to_string(path).map_err(|e| format!("Can't open {}: {e}", path.display()))?;
    from_str(&text)
}

/// Adds `.cartwave` when the path has no extension.
pub fn with_extension(path: &str) -> PathBuf {
    let p = PathBuf::from(path.trim().trim_matches('"'));
    if p.extension().is_none() {
        p.with_extension(EXTENSION)
    } else {
        p
    }
}

// ───────────────────────────── Settings ─────────────────────────────

/// App preferences (not part of a project), in `settings.json` next to the
/// recovery file (UX.md §2). Unknown or missing fields fall back to defaults,
/// so an older or newer build never fails on it.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    /// Retired in the design pass (the display shows time and position at
    /// once); kept so older settings files read the same.
    pub clock_bars: bool,
    // ── Audio ──
    /// Output device name (`None`: follow the system default).
    pub audio_device: Option<String>,
    /// Requested buffer size in frames (`None`: the device's default).
    pub buffer_frames: Option<u32>,
    /// Output sample rate, Hz (44 100, 48 000 or 96 000).
    pub sample_rate: u32,
    // ── Display ──
    /// Frame-rate cap: 60 or 120 (0 = off, dev mode only).
    pub frame_cap: u32,
    // ── Editing ──
    /// Default arrangement grid: "1/16", "1/4" or "Bar".
    pub arrangement_grid: String,
    /// Default piano-roll grid: "1/4", "1/8", "1/16", "1/32" (or a triplet).
    pub piano_grid: String,
    /// The piano roll follows the playhead.
    pub follow_playhead: bool,
    // ── Files ──
    /// Autosave to the recovery file every this many seconds (0 = off).
    pub autosave_secs: u32,
    /// Keep the recovery file when the user quits with Don't Save.
    pub offer_recovery: bool,
    /// Open the most recent project at start.
    pub open_last: bool,
    // ── Warnings ──
    /// Ids of the warnings the user turned off with "Don't ask again".
    pub silenced: Vec<String>,
    // ── Not shown ──
    /// Open Recent, newest first (at most `MAX_RECENT`).
    pub recent: Vec<String>,
    pub browser_open: bool,
    /// The export dialog's last choices (E4), as `key:value` strings.
    pub export: Vec<String>,
    /// Interface scale in percent (Settings > Display; 100 = the OS scale).
    pub ui_scale: u32,
    /// The spectrum's style (its right-click menu): "bars", "leds", "line"
    /// or "dots".
    pub spectrum_style: String,
    /// The browser auditions a preset when it is selected (its Preview key).
    pub preview_on_select: bool,
    /// Settings ▸ Editing ▸ Track and Clip Colors (polish pass; the
    /// reference's Theme & Colors, Live 12 manual §2.3.2): new tracks get
    /// an automatic colour, or `default_track_colour` (#RRGGBB) when off.
    pub auto_track_colours: bool,
    pub default_track_colour: String,
    /// Automatic picks use fewer colours (the palette's middle row).
    pub reduced_auto_colours: bool,
    /// New clips: "track" (the track's colour) or "random".
    pub new_clip_colour: String,
    /// Windows: the menu bar's dropdowns are drawn dark (Settings > Display;
    /// on by default since the polish pass, docs/design/DARK-MENUS.md).
    pub dark_menus: bool,
    // ── Look & Feel (polish 2 §5) ──
    /// The track headers' width, logical px (380 … 600; dragging the boundary).
    pub header_width: f32,
    /// Track headers on the right of the lanes (default: left).
    pub headers_right: bool,
    /// The colour strip by the clips (default: at the window's edge).
    pub strip_by_clips: bool,
    /// The browser on the right (default: left).
    pub browser_right: bool,
    /// View ▸ Show Overview (polish 2 §7; on by default).
    pub show_overview: bool,
}

/// Open Recent keeps this many projects.
pub const MAX_RECENT: usize = 10;

impl Default for Settings {
    fn default() -> Self {
        Self {
            clock_bars: false,
            audio_device: None,
            buffer_frames: None,
            sample_rate: 48_000,
            frame_cap: 60,
            arrangement_grid: "1/16".into(),
            piano_grid: "1/16".into(),
            follow_playhead: true,
            autosave_secs: AUTOSAVE_INTERVAL.as_secs() as u32,
            offer_recovery: true,
            open_last: false,
            silenced: Vec::new(),
            recent: Vec::new(),
            browser_open: true,
            export: Vec::new(),
            ui_scale: 100,
            spectrum_style: "bars".into(),
            preview_on_select: true,
            // On by default since the polish pass (the CEO approved it).
            dark_menus: true,
            auto_track_colours: true,
            default_track_colour: "#7F86E0".into(),
            reduced_auto_colours: false,
            new_clip_colour: "track".into(),
            header_width: HEADER_W_MIN,
            headers_right: false,
            strip_by_clips: false,
            browser_right: false,
            show_overview: true,
        }
    }
}

/// The track headers' width range (logical px, polish 2 §5).
pub const HEADER_W_MIN: f32 = 380.0;
pub const HEADER_W_MAX: f32 = 600.0;

/// The interface scales offered (percent).
pub const UI_SCALES: [u32; 6] = [75, 100, 125, 150, 175, 200];

impl Settings {
    /// The interface scale as a factor (an unknown value gives 1.0).
    pub fn ui_scale_factor(&self) -> f64 {
        if UI_SCALES.contains(&self.ui_scale) {
            self.ui_scale as f64 / 100.0
        } else {
            1.0
        }
    }

    /// Puts `path` first in Open Recent (no duplicates, at most `MAX_RECENT`).
    pub fn add_recent(&mut self, path: &str) {
        self.recent.retain(|p| !same_path(p, path));
        self.recent.insert(0, path.to_string());
        self.recent.truncate(MAX_RECENT);
    }

    /// Drops entries whose file no longer exists; returns them.
    pub fn prune_recent(&mut self) -> Vec<String> {
        let (keep, gone): (Vec<String>, Vec<String>) = self.recent.drain(..).partition(|p| Path::new(p).is_file());
        self.recent = keep;
        gone
    }
}

/// Paths compare case-insensitively on Windows and macOS (their usual file
/// systems), exactly elsewhere.
fn same_path(a: &str, b: &str) -> bool {
    if cfg!(any(target_os = "windows", target_os = "macos")) {
        a.eq_ignore_ascii_case(b)
    } else {
        a == b
    }
}

pub fn load_settings() -> Settings {
    data_dir()
        .and_then(|d| std::fs::read_to_string(d.join("settings.json")).ok())
        .and_then(|t| serde_json::from_str(&t).ok())
        .unwrap_or_default()
}

pub fn save_settings(s: &Settings) {
    if let Some(d) = data_dir() {
        let _ = std::fs::create_dir_all(&d);
        if let Ok(t) = serde_json::to_string_pretty(s) {
            let tmp = d.join("settings.json.tmp");
            if std::fs::write(&tmp, t).is_ok() {
                let _ = std::fs::rename(&tmp, d.join("settings.json"));
            }
        }
    }
}

// ───────────────────────────── Recovery ─────────────────────────────

/// Where settings, presets, the recovery file and the session marker live:
/// `CARTWAVE_DATA_DIR`, else `%APPDATA%\Cartwave` (Windows),
/// `~/Library/Application Support/Cartwave` (macOS), `$XDG_DATA_HOME/cartwave`
/// or `~/.local/share/cartwave`.
pub fn data_dir() -> Option<PathBuf> {
    if let Some(d) = engine::env::var_os("CARTWAVE_DATA_DIR") {
        return Some(PathBuf::from(d));
    }
    platform_data_dir("Cartwave", "cartwave")
}

/// The app-data folder before the rename (`%APPDATA%\ChipDAW`, ...).
pub fn legacy_data_dir() -> Option<PathBuf> {
    platform_data_dir("ChipDAW", "chipdaw")
}

fn platform_data_dir(name: &str, unix_name: &str) -> Option<PathBuf> {
    let env = |k| std::env::var_os(k).map(PathBuf::from);
    if cfg!(target_os = "windows") {
        env("APPDATA").map(|d| d.join(name))
    } else if cfg!(target_os = "macos") {
        env("HOME").map(|d| d.join("Library/Application Support").join(name))
    } else {
        env("XDG_DATA_HOME")
            .map(|d| d.join(unix_name))
            .or_else(|| env("HOME").map(|d| d.join(".local/share").join(unix_name)))
    }
}

// ───────────────────────── The app-data move ─────────────────────────
//
// RENAME.md: on the first launch after the rename, the old app-data folder is
// copied to the new one (only if the new one doesn't exist yet). The old
// folder stays as a backup until a later launch has succeeded: the launch
// that moved it marks the move "launched" once it is up, and the next launch
// to come up removes the old folder.

/// The marker in the new folder while the old one is kept as a backup. Its
/// first line is the old folder; the second the state (`pending`, `launched`).
pub const MIGRATION_MARKER: &str = "migrated-from-chipdaw.txt";

/// What [`migrate_data_dir`] did.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum DataMove {
    /// Nothing to move (no old folder, the new one exists, or an explicit
    /// `CARTWAVE_DATA_DIR`).
    Nothing,
    /// The old folder was copied to the new one; the old one is kept.
    Moved,
    /// The copy failed: the new folder was not created, the old one is
    /// untouched. The message is for the log.
    Failed(String),
}

/// Call once at start, before anything reads the data folder.
pub fn migrate_data_dir() -> DataMove {
    if engine::env::var_os("CARTWAVE_DATA_DIR").is_some() {
        return DataMove::Nothing;
    }
    match (legacy_data_dir(), data_dir()) {
        (Some(old), Some(new)) => migrate_dir(&old, &new),
        _ => DataMove::Nothing,
    }
}

/// Copies `old` to `new` when `new` doesn't exist and `old` does (through a
/// staging folder, so a half-finished copy never looks like a real folder),
/// renames a `recovery.chipdaw` to the new recovery name, and leaves the
/// marker.
pub fn migrate_dir(old: &Path, new: &Path) -> DataMove {
    if new.exists() || !old.is_dir() {
        return DataMove::Nothing;
    }
    let staging = new.with_file_name(format!(
        "{}.moving",
        new.file_name().and_then(|n| n.to_str()).unwrap_or("data")
    ));
    let _ = std::fs::remove_dir_all(&staging);
    let result = copy_dir(old, &staging)
        .and_then(|()| {
            let rec_old = staging.join(format!("recovery.{LEGACY_EXTENSION}"));
            let rec_new = staging.join(format!("recovery.{EXTENSION}"));
            if rec_old.exists() && !rec_new.exists() {
                std::fs::rename(&rec_old, &rec_new)?;
            }
            std::fs::write(staging.join(MIGRATION_MARKER), format!("{}\npending\n", old.display()))
        })
        .and_then(|()| std::fs::rename(&staging, new));
    match result {
        Ok(()) => DataMove::Moved,
        Err(e) => {
            let _ = std::fs::remove_dir_all(&staging);
            DataMove::Failed(format!("Can't move {} to {}: {e}", old.display(), new.display()))
        }
    }
}

fn copy_dir(from: &Path, to: &Path) -> std::io::Result<()> {
    std::fs::create_dir_all(to)?;
    for entry in std::fs::read_dir(from)? {
        let entry = entry?;
        let target = to.join(entry.file_name());
        if entry.file_type()?.is_dir() {
            copy_dir(&entry.path(), &target)?;
        } else {
            std::fs::copy(entry.path(), &target)?;
        }
    }
    Ok(())
}

/// Call once the app is up (settings read, the window built). The launch
/// that moved the folder marks the move "launched"; the next launch to come
/// up removes the old folder and the marker. Returns the removed folder.
pub fn confirm_data_migration() -> Option<PathBuf> {
    confirm_dir(&data_dir()?)
}

/// [`confirm_data_migration`] for a given new folder.
pub fn confirm_dir(new: &Path) -> Option<PathBuf> {
    let marker = new.join(MIGRATION_MARKER);
    let text = std::fs::read_to_string(&marker).ok()?;
    let mut lines = text.lines();
    let old = PathBuf::from(lines.next()?.trim());
    match lines.next().map(str::trim) {
        Some("pending") => {
            let _ = std::fs::write(&marker, format!("{}\nlaunched\n", old.display()));
            None
        }
        Some("launched") => {
            // Never remove the folder in use (a marker copied by hand).
            if old.as_os_str().is_empty() || old == new {
                let _ = std::fs::remove_file(&marker);
                return None;
            }
            let _ = std::fs::remove_dir_all(&old);
            let _ = std::fs::remove_file(&marker);
            Some(old)
        }
        _ => None,
    }
}

/// Autosave every 2 minutes (and on quit) to `recovery.cartwave`. A session
/// marker (`session.lock`) exists while the app runs and is removed on a clean
/// quit; finding it at start means the last session didn't close cleanly.
pub struct Recovery {
    dir: PathBuf,
}

pub const AUTOSAVE_INTERVAL: std::time::Duration = std::time::Duration::from_secs(120);

/// The session marker's content after an unsaved quit (else the process id).
const UNSAVED_QUIT: &str = "unsaved-quit";

/// Why the recovery file is offered at start.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LastSession {
    Crashed,
    UnsavedQuit,
}

impl Recovery {
    pub fn new() -> Option<Self> {
        let dir = data_dir()?;
        std::fs::create_dir_all(&dir).ok()?;
        let r = Self { dir };
        r.adopt_legacy_file();
        Some(r)
    }

    pub fn file(&self) -> PathBuf {
        self.dir.join(format!("recovery.{EXTENSION}"))
    }

    /// A recovery file a pre-rename build left in this folder (an explicit
    /// `CARTWAVE_DATA_DIR` that build used too) takes the new name.
    fn adopt_legacy_file(&self) {
        let old = self.dir.join(format!("recovery.{LEGACY_EXTENSION}"));
        if old.exists() && !self.file().exists() {
            let _ = std::fs::rename(old, self.file());
        }
    }

    fn marker(&self) -> PathBuf {
        self.dir.join("session.lock")
    }

    /// Call at start: whether the previous session left a recovery file to
    /// offer (it crashed, or quit with Don't Save while "Offer recovery after
    /// an unsaved quit" was on), then marks this session as running.
    pub fn start_session(&self) -> Option<LastSession> {
        let marker = std::fs::read_to_string(self.marker()).ok();
        let offer = match marker {
            Some(m) if self.file().exists() => Some(if m.trim() == UNSAVED_QUIT { LastSession::UnsavedQuit } else { LastSession::Crashed }),
            _ => None,
        };
        let _ = std::fs::write(self.marker(), std::process::id().to_string());
        offer
    }

    /// Quit with Don't Save, recovery offered: save the unsaved work to the
    /// recovery file and leave a marker that says so.
    pub fn end_session_unsaved(&self, p: &Project) {
        if self.autosave(p).is_ok() {
            let _ = std::fs::write(self.marker(), UNSAVED_QUIT);
        } else {
            let _ = std::fs::remove_file(self.marker());
        }
    }

    pub fn autosave(&self, p: &Project) -> Result<(), String> {
        save(p, &self.file())
    }

    /// Clean quit: save once more and remove the marker.
    pub fn end_session(&self, p: &Project) {
        let _ = self.autosave(p);
        let _ = std::fs::remove_file(self.marker());
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mock;

    fn sample_project() -> Project {
        let mut a = mock::demo_arrangement();
        // Some per-note volumes, a muted and a soloed lane, odd parameters.
        let id = a.clips[0].id;
        let c = a.clip_mut(id).unwrap();
        c.notes[0].volume = Some(7);
        c.notes[1].volume = Some(0);
        let mut params = Params::default();
        params.set(0, ParamId::Duty, 1);
        params.set(3, ParamId::NoisePeriod, 5);
        params.set(7, ParamId::Rate, 50);
        for (i, l) in a.lanes.iter_mut().enumerate() {
            l.params = params.values[i];
        }
        let (mut mute, mut solo) = (TrackSet::default(), TrackSet::default());
        mute.set(a.lanes[2].id, true);
        solo.set(a.lanes[7].id, true);
        Project {
            arrangement: a,
            bpm: 133.5,
            chip: SoundChip::Base2A03,
            filter: FilterMode::NesWith440,
            filter_auto: false,
            loop_end_ticks: 16 * crate::state::TICKS_PER_BAR,
            loop_start_ticks: 4 * crate::state::TICKS_PER_BAR,
            loop_on: true,
            mute,
            solo,
            master_db: -2.5,
            rate: 44_100,
            bits: 16,
            load_report: Vec::new(),
        }
    }

    /// The parts of a project that are saved (view, selection: not).
    type SavedParts = (Vec<Arc<Clip>>, u32, f32, SoundChip, FilterMode, bool, (u32, u32, bool), TrackSet, TrackSet, Vec<crate::state::Lane>, f32, (u32, u16));

    fn saved_parts(p: &Project) -> SavedParts {
        (
            p.arrangement.clips.clone(),
            p.arrangement.next_id,
            p.bpm,
            p.chip,
            p.filter,
            p.filter_auto,
            (p.loop_end_ticks, p.loop_start_ticks, p.loop_on),
            p.mute.clone(),
            p.solo.clone(),
            p.arrangement.lanes.clone(),
            p.master_db,
            (p.rate, p.bits),
        )
    }

    #[test]
    fn save_load_round_trip() {
        let p = sample_project();
        let text = to_string(&p);
        let q = from_str(&text).expect("loads");
        assert_eq!(saved_parts(&p), saved_parts(&q));
        // And through the file system (atomic save).
        // Both extensions: `.cartwave` (new) and `.chipdaw` (opens forever,
        // Save keeps it), byte for byte the same text.
        let dir = std::env::temp_dir().join(format!("cartwave-test-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        for ext in EXTENSIONS {
            let path = dir.join(format!("round.{ext}"));
            save(&p, &path).unwrap();
            assert_eq!(saved_parts(&load(&path).unwrap()), saved_parts(&p), ".{ext}");
            assert!(!path.with_extension(format!("{ext}.tmp")).exists());
            assert_eq!(std::fs::read_to_string(&path).unwrap(), to_string(&p));
        }
        let _ = std::fs::remove_dir_all(dir);
        // Empty project too.
        let e = Project::new_empty();
        assert_eq!(saved_parts(&from_str(&to_string(&e)).unwrap()), saved_parts(&e));
    }

    #[test]
    fn the_file_is_readable_json_with_one_note_per_line() {
        let text = to_string(&sample_project());
        assert!(text.starts_with("{\n  \"format\": \"chipdaw\",\n  \"version\": 8,"));
        assert!(text.contains("{ \"id\": 1, \"start\": 0,"), "{}", &text[..2000]);
        assert!(text.contains("\"volume\": 7 }"));
        assert!(!text.contains("\"fx\""), "empty effects are not written");
        let v: serde_json::Value = serde_json::from_str(&text).expect("still valid JSON");
        assert_eq!(v["tracks"][2]["params"], serde_json::json!({}), "triangle has no controls");
        assert_eq!(v["tracks"][0]["lock"], "2a03_p1");
        assert_eq!(v["profile"], "nes");
    }

    #[test]
    fn newer_version_fails_with_a_clear_message() {
        let text = to_string(&sample_project()).replacen("\"version\": 8,", "\"version\": 9,", 1);
        let err = from_str(&text).unwrap_err();
        assert!(err.contains("newer Cartwave") && err.contains("version 9"), "{err}");
        // A future file with effects on a note is refused by version first;
        // and even with version 1, unknown note fields are rejected.
        let fx = to_string(&sample_project()).replacen("\"volume\": 7 }", "\"volume\": 7, \"fx\": { \"arp\": [0, 4, 7] } }", 1);
        let err = from_str(&fx).unwrap_err();
        assert!(err.contains("damaged") && err.contains("arp"), "{err}");
        assert!(from_str("{\"hello\": 1}").unwrap_err().contains("Not a Cartwave project"));
        assert!(from_str("not json").unwrap_err().contains("Not a Cartwave project"));
    }

    #[test]
    fn instruments_round_trip_and_v1_files_load_raw() {
        let mut p = sample_project();
        let mut cello = LaneInstrument::named("Cello");
        cello.inst.vibrato.depth = 33;
        cello.edited = true;
        p.arrangement.instruments.set_lane(0, cello);
        p.arrangement.instruments.set_lane(3, LaneInstrument::named("Tight Kit"));
        // A clip with its own instrument, shared by two clips.
        let flute = p.arrangement.instruments.add(LaneInstrument::named("Flute")).unwrap();
        let ids: Vec<u32> = p.arrangement.clips.iter().take(2).map(|c| c.id).collect();
        for id in ids {
            p.arrangement.clip_mut(id).unwrap().instrument = Some(flute);
        }
        let clips = p.arrangement.clips.clone();
        p.arrangement.instruments.prune(&clips);
        let text = to_string(&p);
        assert!(text.contains("\"volume\": \"4 6 8 9 10 11 12 12 / 11 9 7 5 3 2 1 0\""), "{text}");
        assert!(text.contains(&format!("\"instrument\": {flute},")), "clip refs are written");
        let back = from_str(&text).unwrap();
        assert_eq!(back.arrangement.instruments, p.arrangement.instruments);
        assert_eq!(back.arrangement.clips.iter().filter(|c| c.instrument == Some(flute)).count(), 2);
        // Broken references are clear errors.
        let bad = text.replacen(&format!("\"instrument\": {flute},"), "\"instrument\": 777,", 1);
        assert!(from_str(&bad).unwrap_err().contains("instrument 777"));
        // A version 1 file (no instruments) plays the raw channels, as it did.
        let v1 = std::fs::read_to_string(std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../docs/songs/scale_test.chipdaw")).unwrap();
        assert!(v1.contains("\"version\": 1"));
        let old = from_str(&v1).unwrap();
        assert!(old.arrangement.instruments.lane_instruments().iter().all(|i| *i == LaneInstrument::plain()));
    }

    #[test]
    fn hand_edited_overlaps_are_repaired_on_load() {
        // A locked track (the legacy layout's Pulse 1): monophonic.
        let mut legacy = Project::new_empty();
        legacy.arrangement = Arrangement::empty();
        legacy.chip = SoundChip::WithVrc6;
        let mut text = to_string(&legacy);
        text = text.replacen(
            "\"clips\": []",
            "\"clips\": [ { \"id\": 4, \"lane\": 0, \"start\": 0, \"len\": 3840, \"name\": \"x\", \"notes\": [\
             { \"id\": 1, \"start\": 0, \"len\": 960, \"pitch\": 60 }, { \"id\": 1, \"start\": 480, \"len\": 960, \"pitch\": 62, \"volume\": 40 } ] } ]",
            1,
        );
        let p = from_str(&text).expect("loads");
        let c = &p.arrangement.clips[0];
        assert!(notes::is_mono(&c.notes));
        assert_eq!(c.notes.len(), 2);
        assert_ne!(c.notes[0].id, c.notes[1].id, "duplicate ids renumbered");
        assert_eq!(c.notes[1].volume, Some(15));
        assert_eq!(p.arrangement.next_id, 5);
        // A studio track keeps a chord; one pitch never overlaps itself.
        let mut text = to_string(&Project::new_empty());
        text = text.replacen(
            "\"clips\": []",
            "\"clips\": [ { \"id\": 4, \"lane\": 0, \"start\": 0, \"len\": 3840, \"name\": \"x\", \"notes\": [\
             { \"id\": 1, \"start\": 0, \"len\": 960, \"pitch\": 60 }, { \"id\": 2, \"start\": 0, \"len\": 960, \"pitch\": 64 },\
             { \"id\": 3, \"start\": 480, \"len\": 960, \"pitch\": 60 } ] } ]",
            1,
        );
        let p = from_str(&text).expect("loads");
        let n = &p.arrangement.clips[0].notes;
        assert_eq!(n.len(), 3);
        assert!(n.iter().any(|x| x.pitch == 64 && x.start == 0 && x.len == 960), "the chord's other note stays: {n:?}");
        let c60: Vec<_> = n.iter().filter(|x| x.pitch == 60).map(|x| (x.start, x.len)).collect();
        assert_eq!(c60, vec![(0, 480), (480, 960)], "the same pitch is cut where it restarts");
    }

    #[test]
    fn extensions() {
        assert_eq!(with_extension("C:/songs/a"), PathBuf::from("C:/songs/a.cartwave"));
        assert_eq!(with_extension("C:/songs/a.chipdaw"), PathBuf::from("C:/songs/a.chipdaw"));
        assert!(is_project_extension("cartwave") && is_project_extension("CHIPDAW"));
        assert!(!is_project_extension("mid"));
    }

    /// RENAME.md: the old app-data folder moves to the new one on the first
    /// launch, and stays as a backup until the next launch has come up.
    #[test]
    fn app_data_folder_moves_and_old_one_is_kept_until_next_launch() {
        let root = std::env::temp_dir().join(format!("cartwave-move-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        let (old, new) = (root.join("ChipDAW"), root.join("Cartwave"));
        // No old folder: nothing happens.
        assert_eq!(migrate_dir(&old, &new), DataMove::Nothing);
        assert!(!new.exists());
        // An old folder with settings, presets, a recovery file and a subfolder.
        std::fs::create_dir_all(old.join("sub")).unwrap();
        let mut st = Settings::default();
        st.add_recent("C:/songs/old.chipdaw");
        std::fs::write(old.join("settings.json"), serde_json::to_string_pretty(&st).unwrap()).unwrap();
        std::fs::write(old.join("user_presets.json"), "[]").unwrap();
        std::fs::write(old.join("recovery.chipdaw"), to_string(&sample_project())).unwrap();
        std::fs::write(old.join("session.lock"), "1234").unwrap();
        std::fs::write(old.join("sub").join("x.bin"), [1u8, 2, 3]).unwrap();
        // Launch 1: copied, old kept.
        assert_eq!(migrate_dir(&old, &new), DataMove::Moved);
        assert!(old.join("settings.json").exists(), "the old folder is the backup");
        assert!(!root.join("Cartwave.moving").exists(), "no staging folder left");
        let read: Settings = serde_json::from_str(&std::fs::read_to_string(new.join("settings.json")).unwrap()).unwrap();
        assert_eq!(read.recent, vec!["C:/songs/old.chipdaw".to_string()]);
        assert_eq!(std::fs::read(new.join("sub").join("x.bin")).unwrap(), vec![1u8, 2, 3]);
        assert_eq!(std::fs::read_to_string(new.join("user_presets.json")).unwrap(), "[]");
        // The recovery file takes the new name and still offers itself.
        assert!(!new.join("recovery.chipdaw").exists());
        let rec = Recovery { dir: new.clone() };
        assert_eq!(rec.file(), new.join("recovery.cartwave"));
        assert_eq!(saved_parts(&load(&rec.file()).unwrap()), saved_parts(&sample_project()));
        assert_eq!(rec.start_session(), Some(LastSession::Crashed), "the old session's marker came along");
        // A second start before launch 1 came up moves nothing again.
        assert_eq!(migrate_dir(&old, &new), DataMove::Nothing);
        // Launch 1 comes up: the old folder stays.
        assert_eq!(confirm_dir(&new), None);
        assert!(old.exists());
        // Launch 2 comes up: the backup goes, and so does the marker.
        assert_eq!(confirm_dir(&new), Some(old.clone()));
        assert!(!old.exists() && !new.join(MIGRATION_MARKER).exists());
        assert!(new.join("settings.json").exists());
        // Later launches: nothing to do.
        assert_eq!(confirm_dir(&new), None);
        assert_eq!(migrate_dir(&old, &new), DataMove::Nothing);
        // The new folder exists already: an old folder found later is left alone.
        std::fs::create_dir_all(&old).unwrap();
        std::fs::write(old.join("settings.json"), "{}").unwrap();
        assert_eq!(migrate_dir(&old, &new), DataMove::Nothing);
        assert!(old.join("settings.json").exists());
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn recovery_takes_a_legacy_file_in_its_folder() {
        let dir = std::env::temp_dir().join(format!("cartwave-legacy-rec-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("recovery.chipdaw"), to_string(&sample_project())).unwrap();
        let r = Recovery { dir: dir.clone() };
        r.adopt_legacy_file();
        assert!(r.file().exists() && !dir.join("recovery.chipdaw").exists());
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn recovery_marks_unclean_sessions() {
        let dir = std::env::temp_dir().join(format!("cartwave-rec-{}", std::process::id()));
        let r = Recovery { dir: dir.clone() };
        std::fs::create_dir_all(&dir).unwrap();
        assert!(r.start_session().is_none(), "first run");
        r.autosave(&Project::new_empty()).unwrap();
        // Simulated crash: the marker is still there.
        assert_eq!(r.start_session(), Some(LastSession::Crashed));
        r.end_session(&Project::new_empty());
        assert!(r.start_session().is_none(), "clean quit");
        // Quit with Don't Save, recovery offered: offered next time, as such.
        r.end_session_unsaved(&Project::new_empty());
        assert_eq!(r.start_session(), Some(LastSession::UnsavedQuit));
        r.end_session(&Project::new_empty());
        assert!(r.start_session().is_none());
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn settings_defaults_and_tolerant_reading() {
        let d = Settings::default();
        assert_eq!((d.frame_cap, d.sample_rate, d.autosave_secs), (60, 48_000, 120));
        assert!(d.offer_recovery && d.follow_playhead && !d.open_last);
        // An old file (only the clock) keeps every other default.
        let old: Settings = serde_json::from_str(r#"{"clock_bars": true}"#).unwrap();
        assert_eq!(old, Settings { clock_bars: true, ..Settings::default() });
        // Unknown fields from a newer build are ignored.
        let newer: Settings = serde_json::from_str(r#"{"frame_cap": 120, "future": [1, 2]}"#).unwrap();
        assert_eq!(newer.frame_cap, 120);
    }

    /// Dark menus are off unless the user turns them on, and the choice
    /// survives a save and load of settings.json.
    #[test]
    fn dark_menus_default_on_and_round_trip() {
        assert!(Settings::default().dark_menus, "on by default (polish pass)");
        let old: Settings = serde_json::from_str(r#"{"ui_scale": 125}"#).unwrap();
        assert!(old.dark_menus, "a file from before the setting gets the default: on");
        let off = Settings { dark_menus: false, ..Settings::default() };
        let text = serde_json::to_string_pretty(&off).unwrap();
        assert!(text.contains("\"dark_menus\": false"));
        let back: Settings = serde_json::from_str(&text).unwrap();
        assert_eq!(back, off, "turning it off is kept");
        // Through the real file functions, in a scratch data dir.
        let dir = std::env::temp_dir().join(format!("cartwave-dark-menus-{}", std::process::id()));
        let _ = std::fs::create_dir_all(&dir);
        std::fs::write(dir.join("settings.json"), &text).unwrap();
        let read: Settings = serde_json::from_str(&std::fs::read_to_string(dir.join("settings.json")).unwrap()).unwrap();
        assert!(!read.dark_menus);
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn open_recent_keeps_ten_newest_first() {
        let mut s = Settings::default();
        for i in 0..12 {
            s.add_recent(&format!("C:/songs/{i}.chipdaw"));
        }
        s.add_recent("C:/songs/5.chipdaw");
        assert_eq!(s.recent.len(), MAX_RECENT);
        assert_eq!(s.recent[0], "C:/songs/5.chipdaw");
        assert_eq!(s.recent[1], "C:/songs/11.chipdaw");
        assert_eq!(s.recent.iter().filter(|p| p.ends_with("/5.chipdaw")).count(), 1);
        // None of these exist: all pruned.
        assert_eq!(s.prune_recent().len(), MAX_RECENT);
        assert!(s.recent.is_empty());
    }
}
