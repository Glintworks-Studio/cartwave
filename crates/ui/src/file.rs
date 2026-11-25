//! The `.chipdaw` project file (spec: PIANOROLL.md §5, version 3 changes in
//! STABILIZE.md §B), plus the autosave / recovery files.
//!
//! Versions: 1 (piano roll), 2 (instruments, DPCM samples), 3 (the lane
//! Volume control is a channel volume that scales note volumes; loop start
//! and loop on/off are saved), 4 (track names and colours, clip colours:
//! Batch 3. A version 1–3 lane `name` was informational and is not read, so
//! those files open with the default names, exactly as before). Older files are migrated on load so they
//! sound exactly as before (`migrate_v2`).
//!
//! JSON via serde: human-readable, diffable, and easy to migrate. The
//! in-memory model is converted to and from the file structs below, so the
//! format doesn't change by accident when the model does.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use engine::{FilterMode, ParamId, Params, LANES};
use serde::{Deserialize, Serialize};

use crate::instruments::{KitPiece, LaneInstrument};
use crate::notes::{self, Note};
use chip::instrument::{Instrument, Seq, Vibrato, KIT_ZONES, NONE, SEQ_LEN};
use crate::state::{default_lanes, Arrangement, Clip, SoundChip};

/// The `format` field of every project file.
pub const FORMAT: &str = "chipdaw";
/// The format version this build writes and reads.
pub const VERSION: u32 = 4;
pub const EXTENSION: &str = "chipdaw";

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
    pub mute: u8,
    pub solo: u8,
    pub params: Params,
    /// What loading changed (migration notes); empty for a current file.
    pub load_report: Vec<String>,
}

impl Project {
    /// File > New: the 8 lanes, no clips, default settings.
    pub fn new_empty() -> Self {
        Self {
            arrangement: Arrangement::empty(),
            bpm: 150.0,
            chip: SoundChip::WithVrc6,
            filter: FilterMode::default_for(true),
            filter_auto: true,
            loop_end_ticks: 32 * crate::state::TICKS_PER_BAR,
            loop_start_ticks: 0,
            loop_on: true,
            mute: 0,
            solo: 0,
            params: crate::instruments::default_params(),
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
    /// Version 2: the lane's instrument, inline. Missing: the raw channel (v1 sound).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    instrument: Option<InstV2>,
    /// Version 3: the lane's default instrument (an id in `instruments`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    inst: Option<u32>,
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
fn parse_hex(t: &str) -> Result<crate::theme::tokens::Rgba, String> {
    let h = t.strip_prefix('#').filter(|h| h.len() == 6 && h.bytes().all(|b| b.is_ascii_hexdigit())).ok_or_else(|| format!("Bad colour \"{t}\" (use #RRGGBB)."))?;
    Ok(crate::theme::tokens::Rgba::hex(u32::from_str_radix(h, 16).map_err(|_| format!("Bad colour \"{t}\"."))?))
}

// ───────────────────────────── Save / load ─────────────────────────────

/// The project as file text (pretty-printed JSON, one note per line).
pub fn to_string(p: &Project) -> String {
    let lanes = (0..LANES)
        .map(|lane| LaneV1 {
            name: p.arrangement.lanes.get(lane).map_or_else(|| default_lanes()[lane].name.clone(), |l| l.name.clone()),
            color: p
                .arrangement
                .lanes
                .get(lane)
                .map(|l| crate::state::rgb_hex(l.color))
                .filter(|c| *c != crate::state::rgb_hex(default_lanes()[lane].color))
                .map(hex_text),
            mute: p.mute & (1 << lane) != 0,
            solo: p.solo & (1 << lane) != 0,
            params: ParamId::for_lane(lane)
                .iter()
                .map(|&id| (param_key(id).to_string(), p.params.get(lane, id)))
                .collect(),
            instrument: None,
            inst: Some(p.arrangement.instruments.lane_id(lane)),
        })
        .collect();
    let clips = p
        .arrangement
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
    let instruments = p
        .arrangement
        .instruments
        .entries
        .iter()
        .map(|e| InstV2 { id: Some(e.id), ..lane_inst_to_file(&e.inst) })
        .collect();
    let f = FileV1 {
        format: FORMAT.into(),
        version: VERSION,
        bpm: p.bpm,
        chip: match p.chip {
            SoundChip::Base2A03 => "2A03",
            SoundChip::WithVrc6 => "2A03+VRC6",
        }
        .into(),
        filter: if p.filter_auto { "auto".into() } else { filter_key(p.filter).into() },
        loop_end: p.loop_end_ticks,
        instruments,
        loop_start: p.loop_start_ticks,
        loop_on: p.loop_on,
        lanes,
        clips,
        samples: p
            .arrangement
            .samples
            .iter()
            .enumerate()
            .filter_map(|(i, s)| s.as_ref().map(|s| (i, s)))
            .map(|(i, s)| SampleV2 { slot: i as u8, name: s.name.clone(), rate: s.rate, data: crate::dpcm::to_hex(&s.bytes) })
            .collect(),
    };
    compact_notes(&serde_json::to_string_pretty(&f).expect("project serialises"))
}

/// Puts each note object on one line: the file stays readable and diffable
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
    let h: Header = serde_json::from_str(text).map_err(|e| format!("Not a ChipDAW project: {e}"))?;
    if h.format.as_deref() != Some(FORMAT) {
        return Err("Not a ChipDAW project (no \"format\": \"chipdaw\" field).".into());
    }
    match h.version {
        Some(1..=VERSION) => {}
        Some(v) if v > VERSION => {
            return Err(format!(
                "This project was saved by a newer ChipDAW (file format version {v}). \
                 This version reads format {VERSION}. Update ChipDAW to open it."
            ))
        }
        Some(v) => return Err(format!("Unsupported file format version {v}.")),
        None => return Err("The project file has no format version.".into()),
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
    let mut seen = std::collections::HashSet::new();
    for c in f.clips {
        if c.lane >= LANES {
            return Err(format!("Clip {} is on lane {}; there are {LANES}.", c.id, c.lane));
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
            Some(t) if h.version.unwrap_or(VERSION) >= 4 => Some(parse_hex(t)?),
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
        notes::resolve_mono(&mut clip.notes, &[], &mut next);
        clip.next_note_id = next;
        arrangement.clips.push(Arc::new(clip));
    }
    arrangement.next_id = arrangement.clips.iter().map(|c| c.id + 1).max().unwrap_or(1);
    arrangement.samples = vec![None; chip::samples::USER_SLOTS];
    for s in &f.samples {
        let slot = arrangement
            .samples
            .get_mut(s.slot as usize)
            .ok_or_else(|| format!("Sample slot {} doesn't exist.", s.slot))?;
        *slot = Some(crate::dpcm::UserSample {
            name: s.name.clone(),
            rate: s.rate.min(15),
            bytes: crate::dpcm::from_hex(&s.data)?.into_iter().take(chip::samples::MAX_SAMPLE_BYTES).collect(),
        });
    }
    if crate::dpcm::used_bytes(&arrangement.samples) > chip::samples::USER_ROM {
        return Err("The project's DPCM samples don't fit in the sample memory.".into());
    }
    if arrangement.samples.iter().all(|s| s.is_none()) {
        arrangement.samples.clear();
    }
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
        mute,
        solo,
        params,
        load_report: Vec::new(),
    };
    if h.version.unwrap_or(VERSION) < 3 {
        migrate_v2(&mut p);
    }
    Ok(p)
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
    let mut lanes = [0; LANES];
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
pub fn migrate_v2(p: &mut Project) {
    use chip::driver::saw_rate_for_volume;
    const VOLUME_LANES: [usize; 5] = [0, 1, 3, 5, 6];
    const SAW: usize = 7;
    let has: [bool; LANES] =
        std::array::from_fn(|lane| p.arrangement.clips.iter().any(|c| c.lane == lane && c.notes.iter().any(|n| n.volume.is_some())));
    let has_volumes = |lane: usize| has[lane];
    let mut fill: Vec<(usize, u8)> = Vec::new();
    for lane in VOLUME_LANES {
        let v = p.params.get(lane, ParamId::Volume) as u8;
        if v != 15 && has_volumes(lane) {
            fill.push((lane, v));
            p.params.set(lane, ParamId::Volume, 15);
        }
    }
    let r = p.params.get(SAW, ParamId::Rate) as u8;
    if r != 42 && has_volumes(SAW) {
        let exact = (0..=15u8).find(|&v| saw_rate_for_volume(v) == r);
        let v = exact.unwrap_or_else(|| (0..=15u8).min_by_key(|&v| (saw_rate_for_volume(v) as i32 - r as i32).abs()).unwrap_or(15));
        if exact.is_none() {
            p.load_report.push(format!(
                "VRC6 Saw: the Rate control ({r}) can't be kept exactly next to per-note volumes; notes without a volume now play at rate {}.",
                saw_rate_for_volume(v)
            ));
        }
        fill.push((SAW, v));
        p.params.set(SAW, ParamId::Rate, 42);
    }
    if fill.is_empty() {
        return;
    }
    let ids: Vec<u32> = p.arrangement.clips.iter().filter(|c| fill.iter().any(|f| f.0 == c.lane)).map(|c| c.id).collect();
    for id in ids {
        let Some(c) = p.arrangement.clip_mut(id) else { continue };
        let v = fill.iter().find(|f| f.0 == c.lane).map_or(15, |f| f.1);
        for n in &mut c.notes {
            n.volume.get_or_insert(v);
        }
    }
    p.load_report.push(format!(
        "Upgraded from an older version: {} lane(s) moved their volume into the notes, so the song sounds the same.",
        fill.len()
    ));
}

/// Writes atomically: a temporary file next to the target, then a rename,
/// so a crash mid-save never leaves a half-written project.
pub fn save(p: &Project, path: &Path) -> Result<(), String> {
    let text = to_string(p);
    let tmp = path.with_extension("chipdaw.tmp");
    std::fs::write(&tmp, text).map_err(|e| format!("Can't write {}: {e}", tmp.display()))?;
    std::fs::rename(&tmp, path).map_err(|e| format!("Can't save {}: {e}", path.display()))
}

pub fn load(path: &Path) -> Result<Project, String> {
    let text = std::fs::read_to_string(path).map_err(|e| format!("Can't open {}: {e}", path.display()))?;
    from_str(&text)
}

/// Adds `.chipdaw` when the path has no extension.
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
    /// The transport clock shows bars.beats.sixteenths instead of m:ss.cc.
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
        }
    }
}

impl Settings {
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

/// Where the recovery file and the session marker live: `CHIPDAW_DATA_DIR`,
/// else `%APPDATA%\ChipDAW` (Windows), `~/Library/Application Support/ChipDAW`
/// (macOS), `$XDG_DATA_HOME/chipdaw` or `~/.local/share/chipdaw`.
pub fn data_dir() -> Option<PathBuf> {
    if let Some(d) = std::env::var_os("CHIPDAW_DATA_DIR") {
        return Some(PathBuf::from(d));
    }
    let env = |k| std::env::var_os(k).map(PathBuf::from);
    if cfg!(target_os = "windows") {
        env("APPDATA").map(|d| d.join("ChipDAW"))
    } else if cfg!(target_os = "macos") {
        env("HOME").map(|d| d.join("Library/Application Support/ChipDAW"))
    } else {
        env("XDG_DATA_HOME")
            .map(|d| d.join("chipdaw"))
            .or_else(|| env("HOME").map(|d| d.join(".local/share/chipdaw")))
    }
}

/// Autosave every 2 minutes (and on quit) to `recovery.chipdaw`. A session
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
        Some(Self { dir })
    }

    pub fn file(&self) -> PathBuf {
        self.dir.join("recovery.chipdaw")
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
        Project {
            arrangement: a,
            bpm: 133.5,
            chip: SoundChip::Base2A03,
            filter: FilterMode::NesWith440,
            filter_auto: false,
            loop_end_ticks: 16 * crate::state::TICKS_PER_BAR,
            loop_start_ticks: 4 * crate::state::TICKS_PER_BAR,
            loop_on: true,
            mute: 0b0000_0100,
            solo: 0b1000_0000,
            params,
            load_report: Vec::new(),
        }
    }

    /// The parts of a project that are saved (view, selection: not).
    type SavedParts = (Vec<Arc<Clip>>, u32, f32, SoundChip, FilterMode, bool, (u32, u32, bool), u8, u8, Params);

    fn saved_parts(p: &Project) -> SavedParts {
        (
            p.arrangement.clips.clone(),
            p.arrangement.next_id,
            p.bpm,
            p.chip,
            p.filter,
            p.filter_auto,
            (p.loop_end_ticks, p.loop_start_ticks, p.loop_on),
            p.mute,
            p.solo,
            p.params,
        )
    }

    #[test]
    fn save_load_round_trip() {
        let p = sample_project();
        let text = to_string(&p);
        let q = from_str(&text).expect("loads");
        assert_eq!(saved_parts(&p), saved_parts(&q));
        // And through the file system (atomic save).
        let dir = std::env::temp_dir().join(format!("chipdaw-test-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("round.chipdaw");
        save(&p, &path).unwrap();
        assert_eq!(saved_parts(&load(&path).unwrap()), saved_parts(&p));
        assert!(!path.with_extension("chipdaw.tmp").exists());
        let _ = std::fs::remove_dir_all(dir);
        // Empty project too.
        let e = Project::new_empty();
        assert_eq!(saved_parts(&from_str(&to_string(&e)).unwrap()), saved_parts(&e));
    }

    #[test]
    fn the_file_is_readable_json_with_one_note_per_line() {
        let text = to_string(&sample_project());
        assert!(text.starts_with("{\n  \"format\": \"chipdaw\",\n  \"version\": 4,"));
        assert!(text.contains("{ \"id\": 1, \"start\": 0,"), "{}", &text[..2000]);
        assert!(text.contains("\"volume\": 7 }"));
        assert!(!text.contains("\"fx\""), "empty effects are not written");
        let v: serde_json::Value = serde_json::from_str(&text).expect("still valid JSON");
        assert_eq!(v["lanes"][2]["params"], serde_json::json!({}), "triangle has no controls");
    }

    #[test]
    fn newer_version_fails_with_a_clear_message() {
        let text = to_string(&sample_project()).replacen("\"version\": 4,", "\"version\": 5,", 1);
        let err = from_str(&text).unwrap_err();
        assert!(err.contains("newer ChipDAW") && err.contains("version 5"), "{err}");
        // A future file with effects on a note is refused by version first;
        // and even with version 1, unknown note fields are rejected.
        let fx = to_string(&sample_project()).replacen("\"volume\": 7 }", "\"volume\": 7, \"fx\": { \"arp\": [0, 4, 7] } }", 1);
        let err = from_str(&fx).unwrap_err();
        assert!(err.contains("damaged") && err.contains("arp"), "{err}");
        assert!(from_str("{\"hello\": 1}").unwrap_err().contains("Not a ChipDAW project"));
        assert!(from_str("not json").unwrap_err().contains("Not a ChipDAW project"));
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
        let v1: String = {
            let mut v: serde_json::Value = serde_json::from_str(&text).unwrap();
            v["version"] = 1.into();
            v.as_object_mut().unwrap().remove("instruments");
            for l in v["lanes"].as_array_mut().unwrap() {
                l.as_object_mut().unwrap().remove("inst");
            }
            for c in v["clips"].as_array_mut().unwrap() {
                c.as_object_mut().unwrap().remove("instrument");
            }
            v.to_string()
        };
        let old = from_str(&v1).unwrap();
        assert!(old.arrangement.instruments.lane_instruments().iter().all(|i| *i == LaneInstrument::plain()));
    }

    #[test]
    fn hand_edited_overlaps_are_repaired_on_load() {
        let mut text = to_string(&Project::new_empty());
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
    }

    #[test]
    fn recovery_marks_unclean_sessions() {
        let dir = std::env::temp_dir().join(format!("chipdaw-rec-{}", std::process::id()));
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
