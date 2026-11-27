//! Notes inside a clip, and every editing operation on them (PIANOROLL.md §3).
//!
//! The operations here are pure functions on a clip's note list, so the piano
//! roll, the scripted tests and the undo property test all go through the same
//! code. Times are ticks (960 PPQ) relative to the clip start.
//!
//! **Monophonic rule.** A chip channel plays one note at a time, so a clip never
//! holds overlapping notes. Every operation that places notes (add, move,
//! resize, paste, duplicate, quantize, transpose) calls [`resolve_mono`] with
//! the placed notes as *winners*: each winner trims, splits or removes every
//! other note it overlaps, exactly like the clip overlap rule
//! (`Arrangement::resolve_overlaps`). The engine's snapshot builder keeps its
//! own backstop.

use serde::{Deserialize, Serialize};

use crate::state::PPQ;

/// Shortest note the editor creates by resizing (a 1/64 note).
pub const MIN_NOTE_TICKS: u32 = PPQ / 16;
/// Highest per-note volume (the 4-bit volume of the 2A03 / VRC6 channels).
pub const MAX_VOLUME: u8 = 15;

/// One note. `start` and `len` are ticks relative to the clip start.
///
/// The derive order is the file order (`file.rs` serialises this struct
/// directly; PIANOROLL.md §5 has the spec).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Note {
    pub id: u32,
    pub start: u32,
    pub len: u32,
    /// MIDI note number. Noise and DPCM use the driver's mappings (see
    /// [`RowKind`]).
    pub pitch: u8,
    /// 0–15, or `None` for the channel's default (the lane's Volume control).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub volume: Option<u8>,
    /// Reserved for per-note effects (arpeggio, vibrato, duty or pitch
    /// changes). Empty in format version 1; see [`NoteFx`].
    #[serde(default, skip_serializing_if = "NoteFx::is_empty")]
    pub fx: NoteFx,
}

/// Per-note effects: **the extension point** (PIANOROLL.md §3.2).
///
/// Version 1 has no effects, so this is an empty struct: it costs nothing in
/// memory and is never written. A later phase adds optional fields here
/// (e.g. `arp: Option<Arp>`, `vibrato: Option<Vibrato>`, `duty: Option<u8>`,
/// `pitch_env: Option<PitchEnv>`), each `#[serde(default, skip_serializing_if
/// = "Option::is_none")]`. Files written before then still load unchanged,
/// and notes without effects are still written without an `fx` key. Unknown
/// keys are rejected, so a version-1 build fails clearly on a file that uses
/// effects (which will also carry a higher format version).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NoteFx {}

impl NoteFx {
    pub fn is_empty(&self) -> bool {
        *self == NoteFx::default()
    }
}

impl Note {
    pub fn new(id: u32, start: u32, len: u32, pitch: u8) -> Self {
        Self { id, start, len, pitch, volume: None, fx: NoteFx::default() }
    }

    pub fn end(&self) -> u32 {
        self.start.saturating_add(self.len)
    }
}

/// Piano-roll grid (PIANOROLL.md §2). Straight and triplet values.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Grid {
    Quarter,
    Eighth,
    Sixteenth,
    ThirtySecond,
    QuarterT,
    EighthT,
    SixteenthT,
    ThirtySecondT,
}

impl Grid {
    pub const ALL: [Grid; 8] = [
        Grid::Quarter,
        Grid::Eighth,
        Grid::Sixteenth,
        Grid::ThirtySecond,
        Grid::QuarterT,
        Grid::EighthT,
        Grid::SixteenthT,
        Grid::ThirtySecondT,
    ];

    pub fn ticks(self) -> u32 {
        match self {
            Grid::Quarter => PPQ,
            Grid::Eighth => PPQ / 2,
            Grid::Sixteenth => PPQ / 4,
            Grid::ThirtySecond => PPQ / 8,
            // Three in the time of two.
            Grid::QuarterT => PPQ * 2 / 3,
            Grid::EighthT => PPQ / 3,
            Grid::SixteenthT => PPQ / 6,
            Grid::ThirtySecondT => PPQ / 12,
        }
    }

    /// The grid a label names (settings.json); unknown labels give 1/16.
    pub fn from_label(s: &str) -> Self {
        Self::ALL.into_iter().find(|g| g.label() == s).unwrap_or(Grid::Sixteenth)
    }

    pub fn label(self) -> &'static str {
        match self {
            Grid::Quarter => "1/4",
            Grid::Eighth => "1/8",
            Grid::Sixteenth => "1/16",
            Grid::ThirtySecond => "1/32",
            Grid::QuarterT => "1/4T",
            Grid::EighthT => "1/8T",
            Grid::SixteenthT => "1/16T",
            Grid::ThirtySecondT => "1/32T",
        }
    }

    pub fn next(self) -> Self {
        let i = Self::ALL.iter().position(|g| *g == self).unwrap_or(0);
        Self::ALL[(i + 1) % Self::ALL.len()]
    }

    /// Cmd+1 (finer), Cmd+2 (coarser), Cmd+3 (straight <-> triplet).
    pub fn step(self, a: crate::shortcuts::Action) -> Self {
        use crate::shortcuts::Action;
        let i = Self::ALL.iter().position(|g| *g == self).unwrap_or(2);
        let (row, col) = (i / 4, i % 4);
        match a {
            Action::GridNarrow => Self::ALL[row * 4 + (col + 1).min(3)],
            Action::GridWiden => Self::ALL[row * 4 + col.saturating_sub(1)],
            _ => Self::ALL[(1 - row) * 4 + col],
        }
    }

    /// Nearest grid line.
    pub fn round(self, t: i64) -> i64 {
        let g = self.ticks() as i64;
        (t + g / 2).div_euclid(g) * g
    }

    /// Grid line at or before `t`.
    pub fn floor(self, t: i64) -> i64 {
        let g = self.ticks() as i64;
        t.div_euclid(g) * g
    }
}

// ───────────────────────────── Channel rows ─────────────────────────────

/// How a lane's notes map to piano-roll rows (PIANOROLL.md §6). Rows are
/// numbered bottom-up; `row_of`/`pitch_of` convert.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RowKind {
    /// One row per MIDI note in `lo..=hi`; the rows the channel can play
    /// are `play_lo..=play_hi` (others are shaded and refused).
    Piano { osc: chip::pitch::Osc, lo: u8, hi: u8, play_lo: u8, play_hi: u8 },
    /// 16 rows, one per noise period index. Row r (bottom = 0) is period
    /// index 15 - r, so higher rows sound higher. Stored pitch: the inverse
    /// of the driver's `noise_index_for` (index = (96 - pitch) / 4), i.e.
    /// pitch = 96 - 4 × index.
    Noise,
    /// One row per DPCM sample (Kick, Snare, Tom, Crash). Stored pitch:
    /// 36 + sample (the driver plays sample `pitch % 4`).
    Dpcm,
}

/// Names of the DPCM rows: the built-in drums, then each user slot's sample
/// name ("User n · empty" when nothing is imported there).
pub fn dpcm_row_names(samples: &[Option<crate::dpcm::UserSample>]) -> [String; DPCM_NAMES.len()] {
    std::array::from_fn(|row| match row.checked_sub(chip::samples::BUILTIN_COUNT) {
        None => DPCM_NAMES[row].to_string(),
        Some(slot) => match samples.get(slot).and_then(|s| s.as_ref()) {
            Some(s) if !s.name.trim().is_empty() => s.name.clone(),
            _ => format!("{} · empty", DPCM_NAMES[row]),
        },
    })
}

/// Lowest and highest MIDI note the piano rows show.
pub const PIANO_LO: u8 = 21; // A0
pub const PIANO_HI: u8 = 108; // C8
pub const NOISE_ROWS: u8 = 16;
pub const DPCM_BASE_PITCH: u8 = 36;
pub const DPCM_NAMES: [&str; 8] = chip::samples::NAMES;

impl RowKind {
    pub fn for_lane(lane: usize) -> Self {
        use chip::pitch::Osc;
        let osc = match lane {
            3 => return RowKind::Noise,
            4 => return RowKind::Dpcm,
            2 => Osc::Triangle,
            5 | 6 => Osc::Vrc6Pulse,
            7 => Osc::Saw,
            _ => Osc::Pulse,
        };
        let playable = |p: u8| chip::pitch::period_for(osc, p).is_some();
        let play_lo = (PIANO_LO..=PIANO_HI).find(|p| playable(*p)).unwrap_or(PIANO_LO);
        let play_hi = (PIANO_LO..=PIANO_HI).rev().find(|p| playable(*p)).unwrap_or(PIANO_HI);
        RowKind::Piano { osc, lo: PIANO_LO, hi: PIANO_HI, play_lo, play_hi }
    }

    pub fn rows(self) -> u32 {
        match self {
            RowKind::Piano { lo, hi, .. } => (hi - lo) as u32 + 1,
            RowKind::Noise => NOISE_ROWS as u32,
            RowKind::Dpcm => DPCM_NAMES.len() as u32,
        }
    }

    /// Row (0 = bottom) a pitch is drawn on. Out-of-range pitches clamp.
    pub fn row_of(self, pitch: u8) -> u32 {
        match self {
            RowKind::Piano { lo, hi, .. } => (pitch.clamp(lo, hi) - lo) as u32,
            RowKind::Noise => 15 - chip::pitch::noise_index_for(pitch) as u32,
            RowKind::Dpcm => chip::samples::sample_for_pitch(pitch) as u32,
        }
    }

    /// The pitch stored for a note placed on `row`.
    pub fn pitch_of(self, row: u32) -> u8 {
        let row = row.min(self.rows() - 1);
        match self {
            RowKind::Piano { lo, .. } => lo + row as u8,
            RowKind::Noise => 96 - 4 * (15 - row as u8),
            RowKind::Dpcm => DPCM_BASE_PITCH + row as u8,
        }
    }

    /// Whether the channel can play notes on `row`.
    pub fn playable_row(self, row: u32) -> bool {
        match self {
            RowKind::Piano { lo, play_lo, play_hi, .. } => {
                let p = lo as u32 + row;
                p >= play_lo as u32 && p <= play_hi as u32
            }
            _ => row < self.rows(),
        }
    }

    /// Range of rows notes may be placed on (transpose clamps to it).
    pub fn playable_rows(self) -> (u32, u32) {
        match self {
            RowKind::Piano { lo, play_lo, play_hi, .. } => {
                ((play_lo - lo) as u32, (play_hi - lo) as u32)
            }
            _ => (0, self.rows() - 1),
        }
    }

    /// Rows per octave step (Shift+↑/↓): 12 on piano rows, 4 on noise, all
    /// four DPCM rows.
    pub fn octave_rows(self) -> i32 {
        match self {
            RowKind::Piano { .. } => 12,
            RowKind::Noise => 4,
            RowKind::Dpcm => 4,
        }
    }

    /// A row's label. DPCM rows of imported samples are named by
    /// `dpcm_row_names`; this static version says "User n".
    pub fn row_label(self, row: u32) -> String {
        match self {
            RowKind::Piano { lo, .. } => note_name(lo + row as u8),
            RowKind::Noise => format!("{:>2}", 15 - row),
            RowKind::Dpcm => DPCM_NAMES[row as usize % DPCM_NAMES.len()].to_string(),
        }
    }

    /// Tuning error of the period the driver picks for this row, in cents
    /// (`docs/tuning.md`), and the period. Piano rows only.
    pub fn tuning(self, row: u32) -> Option<(f64, u16)> {
        let RowKind::Piano { osc, .. } = self else { return None };
        let pitch = self.pitch_of(row);
        let t = chip::pitch::period_for(osc, pitch)?;
        Some((chip::pitch::cents_error(osc, pitch, t), t))
    }

    /// Lanes without a volume control get no volume lane: the triangle has
    /// no volume in hardware, the DMC plays samples at a fixed level.
    pub fn has_volume(lane: usize) -> bool {
        !matches!(lane, 2 | 4)
    }
}

pub fn note_name(pitch: u8) -> String {
    const NAMES: [&str; 12] = ["C", "C#", "D", "D#", "E", "F", "F#", "G", "G#", "A", "A#", "B"];
    format!("{}{}", NAMES[(pitch % 12) as usize], pitch as i32 / 12 - 1)
}

// ───────────────────────────── Operations ─────────────────────────────

/// The monophonic rule. Each note in `winners` trims, splits or removes every
/// non-winner it overlaps; split-off right parts get new ids from `next_id`.
/// Winners that overlap each other (e.g. two notes quantized onto one line)
/// are resolved last: sorted by start, an equal start keeps the later winner
/// in `winners` order, and each note is cut at the next one's start. The
/// result is sorted by start and has no overlaps.
/// A studio track's rule (Batch 4a: polyphonic): notes may overlap (chords),
/// but **a pitch never overlaps itself**: `resolve_mono` within each pitch.
/// The result is sorted by start, then pitch.
pub fn resolve_poly(notes: &mut Vec<Note>, winners: &[u32], next_id: &mut u32) {
    // Group by pitch with a stable sort: each group keeps the notes' order
    // (as a filter per pitch would), in O(n log n), not O(pitches × n): this
    // runs every frame of a note drag (B4A-D26).
    let mut by_pitch = std::mem::take(notes);
    by_pitch.sort_by_key(|n| n.pitch);
    let mut out = Vec::with_capacity(by_pitch.len());
    let mut rest = by_pitch.as_slice();
    while let Some(first) = rest.first() {
        let n = rest.iter().position(|x| x.pitch != first.pitch).unwrap_or(rest.len());
        let mut group = rest[..n].to_vec();
        resolve_mono(&mut group, winners, next_id);
        out.extend(group);
        rest = &rest[n..];
    }
    out.sort_by_key(|n| (n.start, n.pitch));
    *notes = out;
}

/// `resolve_poly` with no winners: a loaded studio clip's invariant.
pub fn sort_poly(notes: &mut Vec<Note>) {
    let mut next = notes.iter().map(|n| n.id.saturating_add(1)).max().unwrap_or(1);
    resolve_poly(notes, &[], &mut next);
}

/// Locked tracks are monophonic, studio tracks polyphonic: the right rule.
pub fn resolve(poly: bool, notes: &mut Vec<Note>, winners: &[u32], next_id: &mut u32) {
    if poly {
        resolve_poly(notes, winners, next_id)
    } else {
        resolve_mono(notes, winners, next_id)
    }
}

thread_local! {
    /// The overlap rule the note operations below end with: monophonic (the
    /// default: locked tracks and every older caller) or polyphonic (a studio
    /// track's clip). Set for the length of one edit with `rule_scope`.
    static POLY: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
}

/// While the guard lives, note operations on this thread use the
/// polyphonic rule if `poly` (a studio track), else the monophonic one.
pub fn rule_scope(poly: bool) -> RuleScope {
    RuleScope(POLY.with(|p| p.replace(poly)))
}

/// Puts the previous rule back when dropped.
pub struct RuleScope(bool);

impl Drop for RuleScope {
    fn drop(&mut self) {
        POLY.with(|p| p.set(self.0));
    }
}

/// Whether the polyphonic rule is in force.
pub fn poly_rule() -> bool {
    POLY.with(|p| p.get())
}

/// The rule in force (`rule_scope`).
fn resolve_rule(notes: &mut Vec<Note>, winners: &[u32], next_id: &mut u32) {
    resolve(poly_rule(), notes, winners, next_id)
}

pub fn resolve_mono(notes: &mut Vec<Note>, winners: &[u32], next_id: &mut u32) {
    let spans: Vec<(u32, u32)> = notes
        .iter()
        .filter(|n| winners.contains(&n.id))
        .map(|n| (n.start, n.end()))
        .collect();
    for (ws, we) in spans {
        let mut out = Vec::with_capacity(notes.len() + 1);
        for n in notes.drain(..) {
            if winners.contains(&n.id) || n.end() <= ws || n.start >= we {
                out.push(n);
                continue;
            }
            if n.start < ws {
                out.push(Note { len: ws - n.start, ..n });
            }
            if n.end() > we {
                let id = if n.start < ws {
                    let id = *next_id;
                    *next_id += 1;
                    id
                } else {
                    n.id
                };
                out.push(Note { id, start: we, len: n.end() - we, ..n });
            }
        }
        *notes = out;
    }
    // Winners among themselves (and the invariant for everything else).
    let rank = |id: u32| winners.iter().position(|w| *w == id);
    notes.sort_by(|a, b| a.start.cmp(&b.start).then(rank(a.id).cmp(&rank(b.id))));
    // Equal starts: keep the last of the run (the later winner).
    let mut i = 0;
    while i + 1 < notes.len() {
        if notes[i].start == notes[i + 1].start {
            notes.remove(i);
        } else {
            i += 1;
        }
    }
    for i in 1..notes.len() {
        let next = notes[i].start;
        if notes[i - 1].end() > next {
            notes[i - 1].len = next - notes[i - 1].start;
        }
    }
}

/// Adds a note (the newest edit: it wins) and returns its id.
pub fn add(notes: &mut Vec<Note>, next_id: &mut u32, start: u32, len: u32, pitch: u8) -> u32 {
    let id = *next_id;
    *next_id += 1;
    notes.push(Note::new(id, start, len.max(1), pitch));
    resolve_rule(notes, &[id], next_id);
    id
}

pub fn delete(notes: &mut Vec<Note>, ids: &[u32]) {
    notes.retain(|n| !ids.contains(&n.id));
}

/// Shifts the rows of `ids` by `rows`, clamped so that every note stays in
/// `kind`'s playable range (the intervals are kept). Returns the shift used.
pub fn transpose(notes: &mut Vec<Note>, ids: &[u32], rows: i32, kind: RowKind, next_id: &mut u32) -> i32 {
    let sel: Vec<u32> = notes.iter().filter(|n| ids.contains(&n.id)).map(|n| kind.row_of(n.pitch)).collect();
    let (Some(&lo), Some(&hi)) = (sel.iter().min(), sel.iter().max()) else { return 0 };
    let (plo, phi) = kind.playable_rows();
    let rows = rows.clamp(plo as i32 - lo as i32, phi as i32 - hi as i32);
    // A selection that already sits partly outside the range doesn't move
    // further out.
    let rows = if lo < plo || hi > phi { 0 } else { rows };
    if rows == 0 {
        return 0;
    }
    for n in notes.iter_mut().filter(|n| ids.contains(&n.id)) {
        n.pitch = kind.pitch_of((kind.row_of(n.pitch) as i32 + rows) as u32);
    }
    resolve_rule(notes, ids, next_id);
    rows
}

/// Moves `ids` by `dt` ticks (clamped at the clip start), keeping their
/// spacing. Returns the shift used.
pub fn nudge(notes: &mut Vec<Note>, ids: &[u32], dt: i64, next_id: &mut u32) -> i64 {
    let Some(min) = notes.iter().filter(|n| ids.contains(&n.id)).map(|n| n.start).min() else {
        return 0;
    };
    let dt = dt.max(-(min as i64));
    if dt == 0 {
        return 0;
    }
    for n in notes.iter_mut().filter(|n| ids.contains(&n.id)) {
        n.start = (n.start as i64 + dt) as u32;
    }
    resolve_rule(notes, ids, next_id);
    dt
}

/// Snaps each start in `ids` to the nearest grid line. Lengths are kept.
pub fn quantize(notes: &mut Vec<Note>, ids: &[u32], grid: Grid, next_id: &mut u32) {
    for n in notes.iter_mut().filter(|n| ids.contains(&n.id)) {
        n.start = grid.round(n.start as i64).max(0) as u32;
    }
    resolve_rule(notes, ids, next_id);
}

/// Copies of `ids`, with starts relative to the earliest one: the clipboard.
pub fn copy(notes: &[Note], ids: &[u32]) -> Vec<Note> {
    let mut out: Vec<Note> = notes.iter().filter(|n| ids.contains(&n.id)).copied().collect();
    out.sort_by_key(|n| n.start);
    if let Some(first) = out.first().map(|n| n.start) {
        for n in &mut out {
            n.start -= first;
        }
    }
    out
}

/// Inserts `clip` (from [`copy`]) at `at`, with new ids; the pasted notes win.
/// Returns the new ids. Pitches are clamped into `kind`'s playable rows.
pub fn paste(notes: &mut Vec<Note>, next_id: &mut u32, clip: &[Note], at: u32, kind: RowKind) -> Vec<u32> {
    let (plo, phi) = kind.playable_rows();
    let mut ids = Vec::with_capacity(clip.len());
    for n in clip {
        let id = *next_id;
        *next_id += 1;
        ids.push(id);
        let row = kind.row_of(n.pitch).clamp(plo, phi);
        notes.push(Note { id, start: at + n.start, pitch: kind.pitch_of(row), ..*n });
    }
    resolve_rule(notes, &ids, next_id);
    ids
}

/// The span the selection covers: (earliest start, latest end).
pub fn span(notes: &[Note], ids: &[u32]) -> Option<(u32, u32)> {
    let sel = notes.iter().filter(|n| ids.contains(&n.id));
    let lo = sel.clone().map(|n| n.start).min()?;
    let hi = sel.map(|n| n.end()).max()?;
    Some((lo, hi))
}

/// Ctrl+D: copies of the selection placed right after it. Returns the new
/// ids (they become the selection).
pub fn duplicate_right(notes: &mut Vec<Note>, next_id: &mut u32, ids: &[u32], kind: RowKind) -> Vec<u32> {
    let Some((lo, hi)) = span(notes, ids) else { return Vec::new() };
    let clip = copy(notes, ids);
    // The copy of a selection starting mid-span keeps its offset to `lo`.
    let first = notes.iter().filter(|n| ids.contains(&n.id)).map(|n| n.start).min().unwrap_or(lo);
    paste(notes, next_id, &clip, hi + (first - lo), kind)
}

/// Sets the volume of `ids` (None = the channel default).
pub fn set_volume(notes: &mut [Note], ids: &[u32], v: Option<u8>) {
    for n in notes.iter_mut().filter(|n| ids.contains(&n.id)) {
        n.volume = v.map(|v| v.min(MAX_VOLUME));
    }
}

/// Shift+← / Shift+→ (P7, R63): lengthens (`dt` > 0) or shortens the notes
/// of `ids` by `dt` ticks; never shorter than `min`. Longer notes win what
/// they now overlap.
pub fn resize(notes: &mut Vec<Note>, ids: &[u32], dt: i64, min: u32, next_id: &mut u32) {
    for n in notes.iter_mut().filter(|n| ids.contains(&n.id)) {
        n.len = (n.len as i64 + dt).max(min.max(1) as i64) as u32;
    }
    resolve_rule(notes, ids, next_id);
}

/// Cmd+E in the piano roll (P9, R73: chop on the grid): each note of `ids`
/// longer than one grid step becomes notes of `grid` ticks (the last one
/// keeps the rest). Returns the ids of all the pieces.
pub fn chop(notes: &mut Vec<Note>, ids: &[u32], grid: u32, next_id: &mut u32) -> Vec<u32> {
    let grid = grid.max(1);
    let mut out = Vec::with_capacity(notes.len());
    let mut pieces = Vec::new();
    for n in notes.drain(..) {
        if !ids.contains(&n.id) || n.len <= grid {
            if ids.contains(&n.id) {
                pieces.push(n.id);
            }
            out.push(n);
            continue;
        }
        let mut t = 0;
        while t < n.len {
            let id = if t == 0 {
                n.id
            } else {
                let id = *next_id;
                *next_id += 1;
                id
            };
            pieces.push(id);
            out.push(Note { id, start: n.start + t, len: grid.min(n.len - t), ..n });
            t += grid;
        }
    }
    *notes = out;
    pieces
}

/// Cmd+J in the piano roll (P9, R73: join): runs of notes of `ids` that
/// follow each other in time on the same pitch become one note, from the
/// first start to the last end. Returns the ids that remain.
pub fn join(notes: &mut Vec<Note>, ids: &[u32]) -> Vec<u32> {
    let mut sel: Vec<Note> = notes.iter().filter(|n| ids.contains(&n.id)).copied().collect();
    sel.sort_by_key(|n| n.start);
    let mut gone = Vec::new();
    let mut grow: Vec<(u32, u32)> = Vec::new();
    let mut i = 0;
    while i < sel.len() {
        let mut j = i;
        while j + 1 < sel.len() && sel[j + 1].pitch == sel[i].pitch {
            j += 1;
        }
        if j > i {
            grow.push((sel[i].id, sel[j].end() - sel[i].start));
            gone.extend(sel[i + 1..=j].iter().map(|n| n.id));
        }
        i = j + 1;
    }
    notes.retain(|n| !gone.contains(&n.id));
    for (id, len) in grow {
        if let Some(n) = notes.iter_mut().find(|n| n.id == id) {
            n.len = len;
        }
    }
    let mut next = notes.iter().map(|n| n.id + 1).max().unwrap_or(1);
    let keep: Vec<u32> = ids.iter().copied().filter(|id| !gone.contains(id)).collect();
    resolve_rule(notes, &keep, &mut next);
    keep
}

/// Alt+← / Alt+→ (R74): the note before / after the selection (by start),
/// among the notes starting before `len`.
pub fn neighbour(notes: &[Note], ids: &[u32], forward: bool, len: u32) -> Option<u32> {
    let audible = notes.iter().filter(|n| n.start < len);
    let Some((lo, hi)) = span(notes, ids).map(|(_, _)| {
        let starts = notes.iter().filter(|n| ids.contains(&n.id)).map(|n| n.start);
        (starts.clone().min().unwrap_or(0), starts.max().unwrap_or(0))
    }) else {
        // Nothing selected: the first (or last) note.
        return if forward { audible.min_by_key(|n| n.start) } else { audible.max_by_key(|n| n.start) }.map(|n| n.id);
    };
    if forward {
        audible.filter(|n| n.start > hi).min_by_key(|n| n.start).map(|n| n.id)
    } else {
        audible.filter(|n| n.start < lo).max_by_key(|n| n.start).map(|n| n.id)
    }
}

/// True when no two notes overlap and the list is sorted (the invariant).
pub fn is_mono(notes: &[Note]) -> bool {
    notes.windows(2).all(|w| w[0].end() <= w[1].start && w[0].start < w[1].start)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// B4A-D26: the O(n log n) `resolve_poly` gives exactly what the per-pitch
    /// filter version gave (ids, order, lengths, new ids), on random note sets
    /// with overlaps, equal starts and winners.
    #[test]
    fn resolve_poly_matches_the_per_pitch_version() {
        fn old(notes: &mut Vec<Note>, winners: &[u32], next_id: &mut u32) {
            let mut pitches: Vec<u8> = notes.iter().map(|n| n.pitch).collect();
            pitches.sort_unstable();
            pitches.dedup();
            let mut out = Vec::with_capacity(notes.len());
            for p in pitches {
                let mut group: Vec<Note> = notes.iter().filter(|n| n.pitch == p).cloned().collect();
                resolve_mono(&mut group, winners, next_id);
                out.extend(group);
            }
            out.sort_by_key(|n| (n.start, n.pitch));
            *notes = out;
        }
        let mut seed = 0x2545_F491_u32;
        let mut rnd = |m: u32| {
            seed ^= seed << 13;
            seed ^= seed >> 17;
            seed ^= seed << 5;
            seed % m
        };
        for case in 0..300 {
            let n = 1 + rnd(60);
            let v: Vec<Note> = (0..n).map(|i| Note::new(i + 1, rnd(16) * PPQ / 4, PPQ / 4 + rnd(8) * PPQ / 4, 58 + rnd(6) as u8)).collect();
            let winners: Vec<u32> = (0..rnd(4)).map(|_| 1 + rnd(n)).collect();
            let (mut a, mut b) = (v.clone(), v);
            let (mut na, mut nb) = (n + 1, n + 1);
            old(&mut a, &winners, &mut na);
            resolve_poly(&mut b, &winners, &mut nb);
            assert_eq!(a, b, "case {case}");
            assert_eq!(na, nb, "case {case}: next id");
        }
    }

    /// P7, P9 [unit]: shorten / lengthen, chop on the grid, join, neighbours.
    #[test]
    fn batch3_note_ops() {
        let mut v = vec![Note::new(1, 0, PPQ * 4, 60), Note::new(2, PPQ * 4, PPQ, 62)];
        let mut next = 3;
        resize(&mut v, &[2], PPQ as i64, MIN_NOTE_TICKS, &mut next);
        assert_eq!(v[1].len, PPQ * 2);
        resize(&mut v, &[2], -(PPQ as i64) * 5, MIN_NOTE_TICKS, &mut next);
        assert_eq!(v[1].len, MIN_NOTE_TICKS, "never shorter than the minimum");
        // A lengthened note wins what it now covers.
        resize(&mut v, &[1], PPQ as i64, MIN_NOTE_TICKS, &mut next);
        assert!(is_mono(&v));
        assert_eq!(v.len(), 1, "note 2 was covered");
        let pieces = chop(&mut v, &[1], PPQ, &mut next);
        assert_eq!(pieces.len(), 5);
        assert!(is_mono(&v) && v.iter().all(|n| n.len == PPQ && n.pitch == 60));
        let keep = join(&mut v, &pieces);
        assert_eq!(keep, [1]);
        assert_eq!(v.len(), 1);
        assert_eq!((v[0].start, v[0].len), (0, PPQ * 5));
        // Joining keeps different pitches apart.
        let mut w = vec![Note::new(1, 0, PPQ, 60), Note::new(2, PPQ, PPQ, 60), Note::new(3, PPQ * 2, PPQ, 64)];
        join(&mut w, &[1, 2, 3]);
        assert_eq!(w.len(), 2);
        assert_eq!(neighbour(&w, &[1], true, PPQ * 8), Some(3));
        assert_eq!(neighbour(&w, &[3], false, PPQ * 8), Some(1));
        assert_eq!(neighbour(&w, &[], true, PPQ * 8), Some(1));
    }

    fn n(id: u32, start: u32, len: u32, pitch: u8) -> Note {
        Note::new(id, start, len, pitch)
    }

    fn spans(notes: &[Note]) -> Vec<(u32, u32, u32, u8)> {
        notes.iter().map(|n| (n.id, n.start, n.end(), n.pitch)).collect()
    }

    #[test]
    fn new_note_trims_end_and_start() {
        let mut v = vec![n(1, 0, 400, 60), n(2, 600, 400, 62)];
        let mut next = 3;
        let id = add(&mut v, &mut next, 300, 400, 64);
        assert_eq!(id, 3);
        assert_eq!(spans(&v), [(1, 0, 300, 60), (3, 300, 700, 64), (2, 700, 1000, 62)]);
        assert!(is_mono(&v));
    }

    #[test]
    fn note_inside_another_splits_it() {
        let mut v = vec![n(1, 0, 960, 60)];
        let mut next = 2;
        add(&mut v, &mut next, 240, 240, 67);
        assert_eq!(spans(&v), [(1, 0, 240, 60), (2, 240, 480, 67), (3, 480, 960, 60)]);
        assert_eq!(next, 4, "the right part got a new id");
    }

    #[test]
    fn covered_note_is_removed() {
        let mut v = vec![n(1, 100, 50, 60), n(2, 200, 50, 61)];
        let mut next = 3;
        add(&mut v, &mut next, 0, 960, 40);
        assert_eq!(spans(&v), [(3, 0, 960, 40)]);
    }

    #[test]
    fn same_start_newer_wins() {
        // Like the clip rule: the older note keeps the part after the new one.
        let mut v = vec![n(1, 0, 240, 60)];
        let mut next = 2;
        add(&mut v, &mut next, 0, 120, 70);
        assert_eq!(spans(&v), [(2, 0, 120, 70), (1, 120, 240, 60)]);
        // Same span: replaced.
        add(&mut v, &mut next, 0, 240, 72);
        assert_eq!(spans(&v), [(3, 0, 240, 72)]);
    }

    #[test]
    fn move_selection_overwrites_and_keeps_spacing() {
        let mut v = vec![n(1, 0, 240, 60), n(2, 480, 240, 62), n(3, 960, 240, 64)];
        let mut next = 4;
        // Move 1 and 2 right by 600: 1 -> 600..840 (overlaps 3? no), 2 -> 1080..1320 overlaps 3 (960..1200).
        let d = nudge(&mut v, &[1, 2], 600, &mut next);
        assert_eq!(d, 600);
        assert_eq!(spans(&v), [(1, 600, 840, 60), (3, 960, 1080, 64), (2, 1080, 1320, 62)]);
        // Can't move before the clip start: clamped by the earliest note.
        let d = nudge(&mut v, &[1, 2], -10_000, &mut next);
        assert_eq!(d, -600);
        assert!(is_mono(&v));
    }

    #[test]
    fn resize_by_editing_len_then_resolving() {
        let mut v = vec![n(1, 0, 240, 60), n(2, 480, 240, 62)];
        let mut next = 3;
        v[0].len = 600; // drag the right edge over note 2
        resolve_mono(&mut v, &[1], &mut next);
        assert_eq!(spans(&v), [(1, 0, 600, 60), (2, 600, 720, 62)]);
    }

    #[test]
    fn transpose_clamps_to_the_playable_range_and_keeps_intervals() {
        let kind = RowKind::for_lane(0); // 2A03 pulse: lowest is A1 (33)
        let RowKind::Piano { play_lo, play_hi, .. } = kind else { panic!() };
        assert_eq!(play_lo, 33);
        let mut v = vec![n(1, 0, 240, 35), n(2, 240, 240, 47)];
        let mut next = 3;
        assert_eq!(transpose(&mut v, &[1, 2], -12, kind, &mut next), -2);
        assert_eq!((v[0].pitch, v[1].pitch), (33, 45));
        let up = transpose(&mut v, &[1, 2], 200, kind, &mut next);
        assert_eq!(v[1].pitch, play_hi);
        assert_eq!(v[1].pitch - v[0].pitch, 12);
        assert_eq!(up, (play_hi - 45) as i32);
    }

    #[test]
    fn noise_and_dpcm_rows_round_trip_through_the_driver_mapping() {
        let noise = RowKind::for_lane(3);
        for row in 0..16 {
            let p = noise.pitch_of(row);
            assert_eq!(noise.row_of(p), row);
            assert_eq!(chip::pitch::noise_index_for(p) as u32, 15 - row);
        }
        let dpcm = RowKind::for_lane(4);
        for row in 0..4 {
            assert_eq!(dpcm.pitch_of(row) % 4, row as u8, "driver plays sample pitch % 4");
            assert_eq!(dpcm.row_of(dpcm.pitch_of(row)), row);
        }
        let mut v = vec![n(1, 0, 240, noise.pitch_of(14))];
        let mut next = 2;
        assert_eq!(transpose(&mut v, &[1], 4, noise, &mut next), 1, "clamped at the top row");
    }

    #[test]
    fn quantize_snaps_starts_and_resolves_collisions() {
        let mut v = vec![n(1, 10, 100, 60), n(2, 230, 100, 62), n(3, 250, 100, 64)];
        let mut next = 4;
        quantize(&mut v, &[1, 2, 3], Grid::Sixteenth, &mut next);
        // 10 -> 0; 230 and 250 both -> 240: the later winner (3) keeps it.
        assert_eq!(spans(&v), [(1, 0, 100, 60), (3, 240, 340, 64)]);
        assert!(is_mono(&v));
    }

    #[test]
    fn triplet_grid() {
        assert_eq!(Grid::EighthT.ticks(), 320);
        assert_eq!(Grid::SixteenthT.round(170), 160);
        assert_eq!(Grid::QuarterT.ticks() * 3, 2 * PPQ);
    }

    #[test]
    fn clipboard_copy_paste_and_duplicate() {
        let kind = RowKind::for_lane(0);
        let mut v = vec![n(1, 480, 240, 60), n(2, 960, 240, 64)];
        let mut next = 3;
        let clip = copy(&v, &[1, 2]);
        assert_eq!(spans(&clip), [(1, 0, 240, 60), (2, 480, 720, 64)]);
        let ids = paste(&mut v, &mut next, &clip, 1200, kind);
        assert_eq!(ids, [3, 4]);
        assert_eq!(spans(&v)[2..], [(3, 1200, 1440, 60), (4, 1680, 1920, 64)]);
        // Paste over existing notes: the pasted ones win.
        let ids = paste(&mut v, &mut next, &clip, 960, kind);
        assert!(is_mono(&v));
        assert_eq!(v.iter().find(|n| n.id == ids[0]).map(|n| (n.start, n.pitch)), Some((960, 60)));
        // Duplicate: right after the selection's end.
        let mut w = vec![n(1, 0, 240, 60), n(2, 240, 240, 62)];
        let mut next = 3;
        let d = duplicate_right(&mut w, &mut next, &[1, 2], kind);
        assert_eq!(spans(&w), [(1, 0, 240, 60), (2, 240, 480, 62), (3, 480, 720, 60), (4, 720, 960, 62)]);
        assert_eq!(d, [3, 4]);
    }

    #[test]
    fn paste_clamps_pitch_into_the_target_channel() {
        let tri = RowKind::for_lane(2);
        let mut v = Vec::new();
        let mut next = 1;
        let clip = [n(9, 0, 240, 127)];
        paste(&mut v, &mut next, &clip, 0, tri);
        let RowKind::Piano { play_hi, .. } = tri else { panic!() };
        assert_eq!(v[0].pitch, play_hi);
    }

    #[test]
    fn volume_is_optional_and_clamped() {
        let mut v = vec![n(1, 0, 240, 60), n(2, 240, 240, 62)];
        set_volume(&mut v, &[1, 2], Some(99));
        assert_eq!((v[0].volume, v[1].volume), (Some(15), Some(15)));
        set_volume(&mut v, &[2], None);
        assert_eq!(v[1].volume, None);
    }
}
