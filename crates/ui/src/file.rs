//! The `.chipdaw` project file, format version 1 (spec: PIANOROLL.md §5),
//! plus the autosave / recovery files.
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
pub const VERSION: u32 = 2;
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
    pub mute: u8,
    pub solo: u8,
    pub params: Params,
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
            mute: 0,
            solo: 0,
            params: crate::instruments::default_params(),
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
    /// Informational; lanes are identified by their position (0..8).
    name: String,
    #[serde(default)]
    mute: bool,
    #[serde(default)]
    solo: bool,
    /// Only the controls this lane's hardware has; missing ones are the defaults.
    #[serde(default)]
    params: BTreeMap<String, i32>,
    /// Version 2: the lane's instrument. Missing: the raw channel (v1 sound).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    instrument: Option<InstV2>,
}

/// An instrument. Sequences are text in tracker style: values separated by
/// spaces, `|` before the loop point and `/` before the release point, e.g.
/// `"4 8 | 12 11 / 6 3 0"`.
#[derive(Serialize, Deserialize, Default)]
#[serde(deny_unknown_fields)]
struct InstV2 {
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

// ───────────────────────────── Save / load ─────────────────────────────

/// The project as file text (pretty-printed JSON, one note per line).
pub fn to_string(p: &Project) -> String {
    let lanes = (0..LANES)
        .map(|lane| LaneV1 {
            name: default_lanes()[lane].name.to_string(),
            mute: p.mute & (1 << lane) != 0,
            solo: p.solo & (1 << lane) != 0,
            params: ParamId::for_lane(lane)
                .iter()
                .map(|&id| (param_key(id).to_string(), p.params.get(lane, id)))
                .collect(),
            instrument: p.arrangement.instruments.get(lane).map(lane_inst_to_file),
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
            notes: c.notes.clone(),
        })
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
        Some(1 | VERSION) => {}
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
    arrangement.instruments = f
        .lanes
        .iter()
        .map(|l| l.instrument.as_ref().map_or(Ok(LaneInstrument::plain()), lane_inst_from_file))
        .collect::<Result<_, _>>()?;
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
        let mut clip = Clip::new(c.id, c.lane, c.start, c.len.max(1), c.name, c.notes);
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
    Ok(Project {
        arrangement,
        bpm: f.bpm.clamp(crate::state::Transport::MIN_BPM, crate::state::Transport::MAX_BPM),
        chip,
        filter,
        filter_auto,
        loop_end_ticks: f.loop_end.clamp(crate::state::PPQ, crate::state::MAX_SONG_TICKS),
        mute,
        solo,
        params,
    })
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

    /// Call at start: whether the previous session crashed with a recovery
    /// file left, then marks this session as running.
    pub fn start_session(&self) -> bool {
        let crashed = self.marker().exists() && self.file().exists();
        let _ = std::fs::write(self.marker(), std::process::id().to_string());
        crashed
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
            mute: 0b0000_0100,
            solo: 0b1000_0000,
            params,
        }
    }

    /// The parts of a project that are saved (view, selection: not).
    type SavedParts = (Vec<Arc<Clip>>, u32, f32, SoundChip, FilterMode, bool, u32, u8, u8, Params);

    fn saved_parts(p: &Project) -> SavedParts {
        (
            p.arrangement.clips.clone(),
            p.arrangement.next_id,
            p.bpm,
            p.chip,
            p.filter,
            p.filter_auto,
            p.loop_end_ticks,
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
        assert!(text.starts_with("{\n  \"format\": \"chipdaw\",\n  \"version\": 2,"));
        assert!(text.contains("{ \"id\": 1, \"start\": 0,"), "{}", &text[..2000]);
        assert!(text.contains("\"volume\": 7 }"));
        assert!(!text.contains("\"fx\""), "empty effects are not written");
        let v: serde_json::Value = serde_json::from_str(&text).expect("still valid JSON");
        assert_eq!(v["lanes"][2]["params"], serde_json::json!({}), "triangle has no controls");
    }

    #[test]
    fn newer_version_fails_with_a_clear_message() {
        let text = to_string(&sample_project()).replacen("\"version\": 2,", "\"version\": 3,", 1);
        let err = from_str(&text).unwrap_err();
        assert!(err.contains("newer ChipDAW") && err.contains("version 3"), "{err}");
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
        p.arrangement.instruments[0] = LaneInstrument::named("Cello");
        p.arrangement.instruments[0].inst.vibrato.depth = 33;
        p.arrangement.instruments[0].edited = true;
        p.arrangement.instruments[3] = LaneInstrument::named("Tight Kit");
        let text = to_string(&p);
        assert!(text.contains("\"volume\": \"4 6 8 9 10 11 12 12 / 11 9 7 5 3 2 1 0\""), "{text}");
        let back = from_str(&text).unwrap();
        assert_eq!(back.arrangement.instruments, p.arrangement.instruments);
        // A version 1 file (no instruments) plays the raw channels, as it did.
        let v1: String = {
            let mut v: serde_json::Value = serde_json::from_str(&text).unwrap();
            v["version"] = 1.into();
            for l in v["lanes"].as_array_mut().unwrap() {
                l.as_object_mut().unwrap().remove("instrument");
            }
            v.to_string()
        };
        let old = from_str(&v1).unwrap();
        assert!(old.arrangement.instruments.iter().all(|i| *i == LaneInstrument::plain()));
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
        assert!(!r.start_session(), "first run");
        r.autosave(&Project::new_empty()).unwrap();
        // Simulated crash: the marker is still there.
        assert!(r.start_session());
        r.end_session(&Project::new_empty());
        assert!(!r.start_session(), "clean quit");
        let _ = std::fs::remove_dir_all(dir);
    }
}
