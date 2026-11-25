# PIANOROLL: the piano roll, undo and save/load (Windows)

**Machine:** Windows 11 Pro 26200 · Ryzen 7 9800X3D · RTX 4070 SUPER (driver 610.88) · ViewSonic
VX3211-2K at 59.951 Hz · release builds · rustc 1.100.0. The default audio output during this pass
was the monitor's HDMI endpoint (480-frame callbacks), not the MOTU used in ENGINE.md.

---

## Part 0: quick checks

### 0.1 Fork commit 9 on Windows: built, measured, **one regression found and fixed** (fork commit 10)

**Build.** `main` was already current (fork pin `75e643a0` resolves). `cargo build --release` and
`cargo clippy --workspace --release` were clean, with no changes. The Windows half of commit 9
(`dwm_refresh_rate()`) compiled as written.

**The DWM rate, against a second and third source:**

| Source | Value |
|---|---|
| App (fork `dwm_refresh_rate()`, logged once a second via `CHIPDAW_AUDIOLOG`) | 59.950–59.953 Hz |
| Raw `DwmGetCompositionTimingInfo` (PowerShell probe, 10 readings) | `rateRefresh` 10000000 / 166802…166804 = 59.9506–59.9513 Hz |
| `QueryDisplayConfig` target mode (`docs/perf/tools/dpi2.ps1`) | 59951/1000 = **59.951 Hz** |
| winit's `refresh_rate_millihertz` (logged by the commit-8 build) | **59.000** (Windows' integer `dmDisplayFrequency`) |

DWM agrees with DisplayConfig and PERF.md §1 within 3 mHz (50 ppm). Unlike CoreVideo's fixed
nominal period on macOS, DWM's `rateRefresh` is a *measured* estimate that moves by ±1 in its
100 ns denominator between readings. winit's rounded rate is 59, not 60. That matters: without the
precise rate, the slot rule would lock the 60 cap to 59.000 Hz slots, slower than the display.

**The slot-rate rule picks the precise rate.** F3 now has a `slots` line. On this display it reads
`slots 59.951 Hz 1:1, swap-paced`, and the `display` line shows the rate to 3 decimals.

**Pacing (PERF.md's DXGI method, 30 s, cap 60):**

| Build | Scenario | Missed vblanks (display) | Max display gap | Submit > 20 ms | `present` per frame | CPU |
|---|---|---|---|---|---|---|
| fork 8 (60.000 Hz slots) | drag ×2 | 0, 0 | 17.5 ms | 0, 0 | 8.9, 13.2 ms | 12.3, 11.2% |
| **fork 9** | demo | 0 | 17.6 ms | 0 | 0.02 ms | 10.3% |
| **fork 9** | 200 clips | 0 | 17.5 ms | 1 | 0.03 ms | 10.1% |
| **fork 9** | drag ×3 | **11**, 0, 0 | **33.9**, 17.5, 17.5 ms | 2, 1, 0 | **1.35**, 6.48, 15.29 ms | 11.7, 12.6, 11.0% |
| **fork 10** | demo ×3 | 0, 0, 0 | 17.4–17.5 ms | 0, 0, 0 | 15.5 ms | 9.5–9.7% |
| **fork 10** | 200 clips ×3 | 0, 0, 0 | 17.4–17.5 ms | 0, 0, 0 | 15.6 ms | 9.6–11.0% |
| **fork 10** | drag ×3 | 0, 0, 0 | 17.6 ms | 1, 2, 0 | 15.3 ms | 11.3–11.9% |

- **What regressed, and why.** With commit 9 the slots run at the display's own rate, so their
  *phase* against vblank is whatever it was when the limiter started. It then drifts only by
  DWM's ~30 ppm jitter: about one period every ten minutes, so it is effectively fixed for a
  30 s run. The `present` column shows the phase: the time the swap blocks before vblank. In the
  bad run it was 1.35 ms, so the slot sat just before vblank and frames drawn at it missed the
  refresh 11 times. With 60.000 Hz slots (commit 8) the slots always run slightly ahead and the
  vsync'd `SwapBuffers` gates every frame, so that can't happen.
- **The fix (fork commit 10,
  [`1693f265`](https://github.com/jeffreyfrederick/vizia/commit/1693f265)).** On Windows, when
  vsync is on and the slots would lock 1:1 to the display, no slots: the swap paces the frames,
  as PERF.md measured before the limit existed. macOS keeps its slots, because `flushBuffer`
  doesn't lock to vblank there. Non-integer ratios (60 on 144 Hz) are unchanged. 9/9 runs then
  had 0 missed vblanks, and `present` was back to ~15.5 ms blocked in the swap. App-side,
  1–2 submit intervals of 20.1–22.6 ms appeared in two drag runs with no display-side miss.
  ENGINE.md recorded one such interval (20.7 ms) before any of this, so they are not new.
- **Not tested:** a 120 Hz or 144 Hz Windows display (none here). With cap 60 on 119.88 Hz the
  slots lock 1:2 with a free phase, the same risk as above; `SwapBuffers` can't help there. That
  needs the DWM `qpcVBlank` phase anchor, the Windows counterpart of the macOS display-link
  anchor. It's written up here, not built.

### 0.2 The "200 clips playing" UI budget (bug 3): **the original Windows row was correct**

The brief suspected that ENGINE.md §7's "200 clips playing" row was measured with the engine
stopped, because F4 didn't send `Play` before `fbc2309`. Before re-running, I checked that
suspicion with a probe.

- `bench.ps1` presses **Space** (which sends `Play`) **before** F4. So on Windows the engine was
  already playing when F4 loaded the 200 clips. The macOS `chipbench` "200" scenario starts with
  F4 alone, which is where bug 3 showed.
- **Probe:** I ran the pre-fix build (`4027a25`) through `bench.ps1`'s exact sequence with
  `CHIPDAW_AUDIOLOG`. It logged **0 playhead frames per second on the local clock** for the whole
  200-clip phase. The playhead only reads the audio clock while the engine reports
  `playing`, so the engine was playing.
- **Re-measured anyway** (current build, 3 runs; F3's new `clock` line read `audio 60/s local 0/s`):

| Scenario | ENGINE.md §7 (original) | Re-measured |
|---|---|---|
| Idle | 0.0–0.4% | 0.1 / 0.4 / 0.7% |
| Demo playing | 9.6 / 9.8 / 10.3% | 8.9 / 9.1 / 10.0% |
| 200 clips playing | 10.1 / 10.5 / 10.8% | 9.2 / 9.5 / 9.8% |
| 200-clip drag | 10.6 / 10.7 / 12.1% | 10.2 / 10.8 / 10.8% |

ENGINE.md §7 now has this column and a correction note. **Side finding:** during a drag, 4–21
frames per second take the *local* clock, in both builds. Each drag step sends a new snapshot,
and `audible_position_at` returns `None` while a command is in flight (ENGINE.md §5, by design).
The playhead keeps integrating smoothly, so nothing visible happens. It's worth knowing for the
piano roll, which sends snapshots while notes are dragged.

**Verdict: no open problem from Part 0.** The one regression was fixed per brief step 4, so Part 1
continues.

---

## Part 1 summary

The piano roll is in. A person can now write a song in the app: create clips, draw and edit notes
per channel, audition them, undo anything, and save and load `.chipdaw` files. Everything below
was measured or tested on the Windows machine; macOS mappings are implemented but untested.

| Requirement | Status |
|---|---|
| Create / open clips, File > New | Done (§2) |
| Data model, mono rule, effects extension point | Done (§3), unit-tested |
| Channel-aware rows (piano + range + tuning, noise, DPCM, triangle without volume) | Done (§6) |
| Editing (draw, select, marquee, move, resize, duplicate, transpose, nudge, clipboard, quantize, grid, volume lane, zoom/scroll, follow, audition, loop clip) | Done (§2) |
| Global undo / redo, gesture = one step, memory cap, property test | Done (§4) |
| Save / load, versioned JSON, autosave + recovery, native dialogs (`rfd`, user-approved) | Done (§5) |
| Golden render test, test song | Done (§9, `docs/songs/scale_test.chipdaw`) |
| Snapshot scaling 1k / 10k / 50k notes | 0.007 / 0.12 / 0.60 ms: no redesign needed (§7) |
| Performance with the piano roll open, 2,000 visible notes, dragging | All budgets met, 0 missed vblanks (§8) |
| Scripted UI scenario | `docs/pianoroll/tools/scenario.ps1`: PASS (§9) |

Two vizia bugs were found and fixed in the fork on the way: commit 10 (Windows pacing, Part 0) and
**commit 11 (`159bbe55`): `modify_timer` / `query_timer` spun forever when the timer wasn't the
next one due**. In the app this froze the UI at 100% of a core when typing in a text field after
the animation pump had slowed down, because the focused text field's caret timer was due first.
Both commits are pushed with upstream-PR descriptions and a regression test, and the pin is updated.

## 2. What was built, and the keyboard and mouse map

**Opening clips.** Double-click empty lane space to create a 1-bar clip at the snapped position. The
new clip wins any overlap. Double-click a clip, or press Enter with one clip selected, to open it.
The piano roll replaces the device panel. While it is open the arrangement lanes go compact
(36 px instead of 64) so the piano roll gets the room, and the arrangement stays visible and
editable above it. Esc or **Close** returns to the device panel.

**Duplicated clips are independent copies** (Ctrl-drag in the arrangement, Ctrl+D in the piano
roll). There are no linked clips yet: editing a copy never changes the original.

Shortcuts go through `crates/ui/src/input.rs`, which has both platform columns. Only Windows was
tested in this pass.

| Where | Action | Windows / Linux | macOS |
|---|---|---|---|
| Global | Undo | Ctrl+Z | Cmd+Z |
| | Redo | Ctrl+Shift+Z or Ctrl+Y | Cmd+Shift+Z or Cmd+Y |
| | New project (empty, 8 lanes) | Ctrl+N or **New** | Cmd+N |
| | Open (native dialog) | Ctrl+O or **Open** | Cmd+O |
| | Save (asks for a name the first time) | Ctrl+S or **Save** | Cmd+S |
| | Save as (native dialog) | Ctrl+Shift+S or **Save As** | Cmd+Shift+S |
| | Save to a typed path | type in FILE, Enter | same |
| | Play / stop | Space | Space |
| Arrangement | New 1-bar clip | double-click empty lane space | same |
| | Open clip in the piano roll | double-click clip, or Enter (one selected) | same |
| | Select, move, resize, delete, copy-drag, no-snap drag (as before) | see FINDINGS.md; Ctrl-drag copies, Alt = no snap | Option-drag copies, Cmd = no snap |
| Piano roll | Draw a note (current grid length) | click an empty cell | same |
| | Draw with a length | click and drag right | same |
| | Select / toggle | click / Shift-click a note | same |
| | **Marquee** (adds to the selection) | **Shift-drag** on empty space | Shift-drag |
| | Select all notes in the clip | Ctrl+A | Cmd+A |
| | Move (time and pitch) | drag a note | same |
| | Move or draw without snapping | Alt held | Cmd held |
| | Resize | drag a note's left or right edge (resize cursor) | same |
| | Copy-drag | Ctrl-drag | Option-drag |
| | Duplicate the selection to the right | Ctrl+D | Cmd+D |
| | Delete | Delete or Backspace | Backspace (fn+Delete) |
| | Transpose one row (semitone) / one octave | Up, Down / Shift+Up, Shift+Down | same |
| | Nudge one grid step | Left, Right | same |
| | Copy / cut / paste | Ctrl+C / X / V | Cmd+C / X / V |
| | Quantize starts to the grid | Q | Q |
| | Grid (1/4, 1/8, 1/16, 1/32, 1/4T, 1/8T, 1/16T, 1/32T) | **GRID** button cycles | same |
| | Volume of one note, or of the whole selection when the bar's note is selected | drag in the volume lane; dragging across unselected notes paints them | same |
| | Volume back to the channel default | double-click a volume bar | same |
| | Audition (transport stopped only) | press a key, or click/draw/drag a note | same |
| | Move the playhead | click the piano roll's ruler | same |
| | Scroll horizontally / vertically / zoom | wheel / Shift+wheel / Ctrl+wheel | wheel / Shift+wheel / Cmd+wheel |
| | Follow the playhead while playing | **Follow** toggle (on by default) | same |
| | Loop playback over this clip | **Loop clip** toggle | same |
| | Close | Esc or **Close** | same |

- **Marquee choice:** Shift-drag on empty space. A plain click on empty space draws a note, Ctrl
  and Alt are taken by copy-drag and snap bypass, and Shift already means "add to the selection",
  so Shift-drag adds a rectangle of notes to it. There's no separate select mode.
- **Paste position:** after the selection when there is one, so repeated Ctrl+V chains copies.
  Otherwise at the playhead if it is inside the clip, else at the clip start.
- **Clicking a selected note** in a multi-selection and releasing without moving collapses the
  selection to that note, as in the arrangement.
- **Hover read-out** (header line): on piano rows the note name, the driver's period and its tuning
  error in cents, the same numbers as `docs/tuning.md`. On noise rows the period index, with a note
  when the Period control overrides the row. On DPCM rows the sample.
- **Notes past the clip end** are kept but neither drawn nor played. The area past the end is
  shaded, and drawing or moving notes into it is refused. Lengthening the clip brings them back.

## 3. The data model (permanent)

```rust
// crates/ui/src/notes.rs
pub struct Note {
    pub id: u32,             // unique within its clip
    pub start: u32,          // ticks (960 PPQ) relative to the clip start
    pub len: u32,            // ticks
    pub pitch: u8,           // MIDI note; noise and DPCM use the driver's mappings (§6)
    pub volume: Option<u8>,  // 0..=15, None = the lane's Volume control
    pub fx: NoteFx,          // reserved: per-note effects (empty in v1)
}
// crates/ui/src/state.rs
pub struct Clip {
    pub id: u32, pub lane: usize, pub start_ticks: u32, pub len_ticks: u32, pub name: String,
    pub notes: Vec<Note>,    // sorted, never overlapping (the mono rule)
    pub next_note_id: u32,
    pub rev: Rev,            // revision stamp, not content (see below)
}
pub struct Arrangement { pub clips: Vec<Arc<Clip>>, /* lanes, selection, view, ... */ }
```

- **Monophonic rule** (`notes::resolve_mono`). Every operation that places notes (add, move,
  resize, paste, duplicate, quantize, transpose, nudge) names the placed notes as **winners**. Each
  winner trims, splits or removes every other note it overlaps, exactly like the clip rule:
  - a note that starts earlier is cut at the winner's start;
  - a note that runs past the winner keeps its tail (a note covering a winner on both sides is
    split, and the right part gets a new id);
  - a covered note is removed.

  Winners that collide with each other (two notes quantized onto one grid line) are resolved by
  start, and the later winner keeps an equal start. During a drag the model isn't trimmed: the
  dragged notes are flagged (`Arrangement::editing_notes`), the engine snapshot hears them winning,
  and the rule is applied to the model on release, just as clip drags do. `SongSnapshot::build`'s
  backstop is unchanged.
- **Per-note volume.** 2A03 pulse, noise and VRC6 pulse use it as V. The saw maps it to its
  accumulator rate `round(v * 42 / 15)`, so 15 = 42 is the loudest rate that doesn't wrap. The
  triangle and DMC have no volume, so their notes keep `None` and get no volume lane. A volume
  change on a sounding note is applied at the next frame tick without retriggering it (driver
  test `per_note_volume_overrides_the_lane_and_updates_while_held`).
- **The per-note effects extension point** (`NoteFx`). Today it is an empty struct (zero bytes),
  never written to files. The plan: add optional fields to it, e.g. `arp: Option<Arp>`,
  `vibrato: Option<Vibrato>`, `duty: Option<u8>`, `pitch_env: Option<PitchEnv>`, each
  `#[serde(default, skip_serializing_if = "Option::is_none")]`, and carry them into the engine
  (`chip::driver::Note` gains the same fields; the driver applies them on frame ticks, where
  chiptune effects naturally live). Version-1 files load unchanged in that build, and notes
  without effects are still written without an `fx` key. A file that uses effects will say
  `version: 2`, and a version-1 build refuses it with a clear message. Even with a forged
  `version: 1`, unknown fields are rejected (`deny_unknown_fields`, tested).
- **Arc clips and revision stamps.** Clips are shared `Arc`s, and the only way to change one is
  `Arrangement::clip_mut`. It copies the clip if anything else shares it (an undo snapshot) and
  stamps a fresh `Rev` from a process-wide counter, which is never reused, unlike an `Arc`
  address. `content_key` (which decides when the engine gets a new snapshot) is a hash of
  `(id, rev)` per clip. The old key hashed only each clip's first and last notes, so **it would
  have missed every edit in the middle of a clip**; that was fixed before the piano roll could hit
  it.

## 4. Undo / redo

- **Design: snapshot-based.** A step stores the document as it was: `Doc { clips, next_id, bpm,
  params }`. With `Arc` clips a snapshot costs one pointer per clip plus the clips that actually
  changed, so the usual edit (one clip) costs one clip copy.
- **Why snapshots, not commands.** Undo replaces the document with a stored one, so it can't drift
  from the model the way a wrong inverse command can. Every operation, including future ones, is
  undoable without writing an inverse. The monophonic rule's trims and splits make inverses
  awkward. Memory is the cost, hence the cap.
- **What it covers:**
  - Covered: clips, notes (including per-note volume), clip names, tempo, and every lane's
    hardware controls (dials), across the arrangement and the piano roll in one history.
  - Not covered (deliberately): mute/solo, chip, filter, selection, view, the loop toggle.
  - File > New, Open and Recover start a fresh history.
- **Steps.**
  - A gesture is one step: a drag, resize, marquee move, volume-lane drag, dial drag or BPM scrub.
    The views emit `GestureBegin` / `GestureEnd`, and nothing is committed while a gesture or a
    note/clip drag is in progress.
  - Discrete edits (keys, double-click create, typed clip name on submit) commit when they happen,
    and every pump tick commits anything left over.
  - Commits that change nothing (e.g. a drag that ends where it started) are dropped by a content
    comparison.
  - Each wheel notch on a dial or the BPM field is its own step.
- **Engine.** Undo and redo write the document back into the model. The changed `Rev`s change the
  content key, and the existing snapshot path sends the song at the next pump tick (≤ 4 ms).
  Tempo and parameter changes are sent as commands, so playback reflects the undo immediately.
- **Memory cap:**
  - The history keeps at most **1,000 steps** and **64 MB**, dropping the oldest steps first.
  - Each step is charged the bytes it doesn't share with the next state (its clip list plus the
    clips that differ).
  - Tested: a 20,000-note clip changed 400 times keeps ~200 steps under 64 MB; 1,010 small edits
    on the 200-clip set keep 1,000 steps in < 8 MB.
- **Property test** (`history::tests::undo_all_restores_the_original_and_redo_all_the_final`):
  - 40 seeds, each 20–80 random edits through the real model operations: add/move/delete clips,
    tempo, lane parameters, and note add/delete/transpose/nudge/quantize/paste/volume.
  - It checks the mono invariant after every note edit.
  - It undoes everything, checking each intermediate state and that the model equals the
    original, then redoes everything to the final state.

## 5. The `.chipdaw` format, version 1

UTF-8 JSON (serde), written atomically (temporary file + rename). Notes are written one per line so
the file stays readable and diffable. The test song is 51 KB; 50,000 notes are 3.2 MB.

```json
{
  "format": "chipdaw",
  "version": 1,
  "bpm": 150.0,
  "chip": "2A03+VRC6",
  "filter": "auto",
  "loop_end": 30720,
  "lanes": [
    { "name": "Pulse 1", "mute": false, "solo": false,
      "params": { "duty": 2, "envelope": 0, "sweep": 0, "sweep_rate": 3, "volume": 12 } },
    ...
  ],
  "clips": [
    { "id": 1, "lane": 0, "start": 0, "len": 15360, "name": "P1 scale",
      "notes": [
        { "id": 1, "start": 0, "len": 240, "pitch": 60, "volume": 4 },
        ...
      ] }
  ]
}
```

| Field | Meaning |
|---|---|
| `format` | Always `"chipdaw"`. Read first; anything else is "not a ChipDAW project" |
| `version` | `1`. A higher version fails with "saved by a newer ChipDAW (file format version N) ... Update ChipDAW to open it"; checked before any other field |
| `bpm` | Tempo, clamped to 40–300 on load |
| `chip` | `"2A03"` or `"2A03+VRC6"` |
| `filter` | `"auto"` (follows the chip), `"nes"`, `"nes+440"`, `"famicom"`, `"off"` |
| `loop_end` | Song loop end, ticks (960 PPQ). The "loop clip" toggle is not saved |
| `lanes` | Exactly 8, by position (Pulse 1, Pulse 2, Triangle, Noise, DPCM, VRC6 Pulse 1, VRC6 Pulse 2, VRC6 Saw). `name` is informational. `mute`/`solo` default false. `params` holds only the controls that lane's hardware has (`duty`, `volume`, `envelope`, `sweep`, `sweep_rate`, `noise_mode`, `noise_period`, `sample`, `rate`, `loop`): missing ones are defaults, unknown ones are an error |
| `clips[]` | `id` (unique), `lane` (0–7), `start`/`len` (ticks), `name`, `notes` |
| `notes[]` | `id`, `start`, `len` (ticks relative to the clip), `pitch` (MIDI; §6 for noise/DPCM), optional `volume` (0–15), reserved `fx` (omitted in v1) |

- **Strict where it matters:** unknown fields are rejected at every level, so a typo or a newer
  feature can't be silently dropped.
- **Lenient where a hand edit is harmless:** on load, overlapping notes are resolved by the mono
  rule, duplicate note ids are renumbered, volumes are clamped and zero-length notes dropped.
- **Native dialogs:** `rfd` 0.18 (MIT), with default features only (Linux uses xdg-portal and
  Wayland, not GTK, so no LGPL library). Its tree for all targets is 53 crates, all
  MIT/Apache/Zlib/ISC/Unlicense/Unicode-3.0. The dialogs are modal on the UI thread; the audio
  thread keeps playing.
- **Autosave and recovery:**
  - Every 2 minutes, if the document changed, the project is serialised on a background thread
    to `recovery.chipdaw`. It is saved again on quit.
  - The folder is `%APPDATA%\ChipDAW` (Windows), `~/Library/Application Support/ChipDAW`
    (macOS), `$XDG_DATA_HOME/chipdaw` (Linux), or `CHIPDAW_DATA_DIR`.
  - A `session.lock` marker exists while the app runs and is removed on a clean quit.
  - If it's there at start (with a recovery file), the transport bar offers **Recover** /
    **Discard**. Autosave is paused until the user answers, so the old recovery file isn't
    overwritten first.

## 6. Channel-row rules

| Lane | Rows | Unplayable rows | Stored pitch | Volume lane |
|---|---|---|---|---|
| Pulse 1/2 (2A03) | Piano A0–C8 | Shaded and refused: below **A1** (period > $7FF) and above the top note with period ≥ 8 (the sweep unit mutes shorter periods) | MIDI | Yes (V) |
| Triangle | Piano A0–C8 | Shaded outside the triangle's period range | MIDI | **No** (the hardware has no volume) |
| VRC6 Pulse 1/2 | Piano A0–C8 | Shaded outside the 12-bit period range | MIDI | Yes (V) |
| VRC6 Saw | Piano A0–C8 | Shaded outside the saw's period range | MIDI | Yes (accumulator rate = round(v * 42 / 15)) |
| Noise | 16 rows labelled with the **period index**, 0 (shortest, highest) at the top to 15 at the bottom. The mode (long/short) is the panel's Mode control | — | The inverse of the driver's existing mapping `index = (96 - pitch) / 4`: **pitch = 96 - 4 × index** (index 0 → 96, 15 → 36). Any pitch maps to a row by the driver's formula, so older demo notes (40, 60, 76) land on rows 14, 9, 5 | Yes (V) |
| DPCM | 4 rows: Kick, Snare, Tom, Crash | — | The existing note → sample map (`pitch % 4`): stored as 36 + sample (C2, C#2, D2, D#2) | No (samples play at a fixed level) |

- Range ends come from `chip::pitch::period_for`, the function the driver uses, so the editor and
  the hardware can't disagree. The driver still refuses out-of-range notes as before.
- Transpose clamps so that the whole selection stays inside the playable rows and keeps its
  intervals. A selection already partly outside the range (e.g. a clip dragged to another lane keeps its pitches) isn't transposed. On noise rows Up/Down is
  one period index and Shift moves 4; on DPCM rows it is one sample.
- When the noise **Period** control is set to a fixed index, the rows don't change the sound; the
  hover read-out says so. The same holds for a fixed DPCM **Sample**, but the read-out doesn't
  mention it (known gap).

## 7. Snapshot scaling (`cargo run --release -p ui --example snapshot_bench`)

Generated projects with 100 notes per 4-bar clip across 8 lanes. Median of 21–51 runs on the UI
thread (Ryzen 7 9800X3D):

| Notes | Clips | `content_key` (every pump tick) | `song_snapshot` (every edit) | Snapshot while dragging a clip | Undo commit | Save | Load | File |
|---|---|---|---|---|---|---|---|---|
| 1,000 | 10 | 0.000 ms | **0.007 ms** | 0.007 ms | 0.001 ms | 0.97 ms | 0.19 ms | 0.1 MB |
| 10,000 | 100 | 0.000 ms | **0.124 ms** | 0.121 ms | 0.002 ms | 4.2 ms | 1.7 ms | 0.6 MB |
| 50,000 | 500 | 0.002 ms | **0.602 ms** | 0.582 ms | 0.011 ms | 18.3 ms | 8.0 ms | 3.2 MB |

- **The full rebuild scales linearly (~12 ns per note) and is far under the 2 ms threshold at
  10k notes**, so ENGINE.md's design stays as it is. At most one snapshot is built per pump
  tick, so a drag on a 50k-note song costs ~0.6 ms per frame for the snapshot.
- **Save** is the slowest operation at 50k notes (18 ms). Ctrl+S is a one-off, and autosave runs
  it on a background thread.
- **If it is ever needed** (e.g. 500k notes), the incremental approach would be:
  - Keep a per-clip flattened note list keyed by `(clip id, Rev)`, which is already unique per
    content.
  - On an edit, rebuild only the lanes whose clips changed, merging their cached lists.
  - Send per-lane snapshots so the engine swaps lanes independently. That keeps the wait-free,
    allocation-free audio side as it is.

## 8. Performance (Windows, cap 60, PERF.md methods)

**The piano roll** (`docs/pianoroll/tools/bench_pr.ps1`, and `pacing.ps1 -Scenario pr|prdrag`;
the project is `piano_bench.chipdaw`). It is open at 32 px per bar with **2,001 visible notes**
while the song plays; the drag moves the C4 "handle" note in a sine pattern (pitch and time):

| Scenario | CPU, 3 runs (12 s) | Pacing, 30 s (DXGI) | UI thread per frame |
|---|---|---|---|
| Piano roll open, playing | **6.8 / 6.6 / 6.4%** | 1800 frames, p99 16.88 ms, max 17.3 ms, **0 > 20 ms, 0 missed vblanks** | draw 0.33 ms, piano roll 0.06 ms (layer reused) |
| Dragging notes, playing | **16.0 / 13.6 / 12.5%** | p99 16.88 ms, max 19.5 ms, **0 > 20 ms, 0 missed** | draw 1.32 ms, custom 1.05 ms (layer re-rendered per move) |
| *Targets (PERF.md)* | *≤ 20% playing, ≤ 25% dragging* | *0 > 20 ms* | |

- **Layering, as in the arrangement:**
  - Rows, grid, keys, ruler, notes and volume bars go into one retained GPU layer, keyed by
    clip `Rev`, selection, view and size.
  - The playhead, hovered row and marquee are drawn live on top, so playback redraws one image
    and a line.
  - Notes outside the view are culled.
  - Notes and bars are axis-aligned rects (`draw_rect`), with no paths, so there is no
    per-frame path-identity churn (PERF.md H3).
- **During a note drag the layer is re-rendered per move.** At 2,000 notes that is ~1 ms per
  frame, well within budget. If larger clips ever need it, the next step is the arrangement's
  approach: draw dragged notes live and keep the rest cached.
- The piano roll scenario uses *less* CPU than the arrangement scenarios, because the compact
  arrangement above it draws fewer pixels.

**No regression in the arrangement scenarios** (final build, `bench.ps1` 3 runs, `pacing.ps1` 30 s):

| Scenario | PERF.md / ENGINE.md | Final build | Pacing (final) |
|---|---|---|---|
| Idle | 0.0–0.7% | 1.0 / 1.6 / 1.0% (see note) | no frames |
| Demo playing | 8.9–10.3% | 8.8 / 9.0 / 9.8% | 0 > 20 ms, 0 missed, max gap 17.5 ms |
| 200 clips playing | 9.2–10.8% | 9.8 / 10.2 / 10.0% | 0 > 20 ms, 0 missed |
| 200-clip drag | 10.2–12.1% | 11.8 / 11.6 / 11.4% | 0 missed vblanks; one app-side 33 ms submit interval with no missed display refresh |

**Idle note:** idle read higher in these runs, so I A/B'd the final build against the Part 0 build
(before any piano-roll work), alternating, 12 s each. Old build 1.69% then 0.13%; new build 2.08%
then 0.13%. Both builds swing together, so this is background noise on the machine, not a
regression. No frames are drawn while idle (the frame log stays empty).

## 9. Tests

| Test | Where | What it proves |
|---|---|---|
| 13 note-operation tests | `ui/src/notes.rs` | Mono trimming (end, start, split, removal, equal starts), move/nudge with clamping, resize, transpose clamped to the playable range keeping intervals (2A03 pulse ≥ A1), noise/DPCM row ↔ driver mapping round trip, quantize (incl. collisions), triplet grid, copy/cut/paste/duplicate, paste clamping into another channel, volume |
| 6 arrangement tests | `ui/src/state.rs` | Clip overlap rule, live snapshot during drags, content key ignores selection/view |
| 4 undo tests | `ui/src/history.rs` | **Property test** (40 seeds, undo all = original, redo all = final, each intermediate state); a gesture is one step and no-op commits are dropped; a new edit drops redo; memory and step caps |
| 5 file tests | `ui/src/file.rs` | **Save → load round trip** (memory and disk, atomic, empty project); readable one-note-per-line JSON; **newer version fails clearly**, unknown fields rejected; hand-edited overlaps repaired; recovery marker logic |
| 1 song test | `ui/src/songs.rs` | Generated songs are monophonic and every note is playable on its channel |
| **Golden render** | `ui/tests/golden.rs` | `docs/songs/scale_test.chipdaw` renders identically twice, every channel sounds solo, and the 12.8 s render hashes to `0xaaaca2df8157ca6a` (the engine is deterministic across platforms, ENGINE-macos.md §1) |
| 3 driver/player tests | `chip` | Per-note volume overrides the lane and updates a held note without retrigger; preview only while stopped, retriggers, ignores mute, refuses unplayable notes; loop range |
| 1 engine test | `engine/src/core.rs` | `Preview` through `Core::process` under the no-allocation checker (ENGINE.md §1 rules) |
| vizia fork test | `vizia_core` | `modify_timer` / `query_timer` on a timer that isn't next due (hung before commit 11) |
| **Scripted UI scenario** | `docs/pianoroll/tools/scenario.ps1` | Synthetic input: New, Bar snap, double-click to create, double-click to open, draw 3 notes (one by drag), undo/redo the drag as one step, Ctrl+A, Ctrl+D, Shift+Up and undo, set a volume, Esc, type a file name, Enter. Then it checks the saved file: 1 clip at bar 2, pitches 60 64 67 60 64 67, duplicates offset by the span, monophonic, only the first note has a volume. **PASS** |

`cargo test --release --workspace`: chip 62, engine 10, ui 29 + golden 1, all passing. `cargo
clippy --workspace --release --all-targets` is clean; `cargo build --release` has no warnings.

Screenshots (`docs/pianoroll/tools/screens.ps1`), in `docs/screens/pianoroll/`:
- `open-clip.png`: the demo's "Hook A" open, with the hover tuning read-out.
- `marquee.png`: a Shift-drag marquee in progress.
- `volume-lane.png`: the test song's Pulse 1 swell, one note selected.
- `noise-rows.png`: the 16 period rows.
- `dpcm-rows.png`: the 4 sample rows.

## 10. Known gaps

- **macOS is untested.** The mappings are implemented in `input.rs`, and `rfd` uses the native
  open/save panels. The Mac pass should run the scenario and the benchmarks.
- **Windows 120/144 Hz displays are untested.** With a 60 cap on 119.88 Hz the frame slots lock
  1:2 with a free phase (Part 0).
- **During edits while playing, the playhead runs on the UI clock** for frames where a snapshot
  command is in flight (Part 0 side finding). Smooth and harmless, but not audio-locked during
  those frames.
- **Undo** doesn't cover mute/solo, chip, filter or the loop toggle, and New/Open/Recover clear
  the history. Each wheel notch on a dial or BPM is its own step.
- **Native dialogs are modal** on the UI thread: the UI pauses while one is open; audio continues.
- **A double-click right after a double-click** at the same spot counts as a triple-click in
  vizia, so "create clip, then immediately open it" needs a short pause. A real user rarely hits
  it; the scenario waits 0.7 s.
- **Notes past the clip end** are hidden with no count or indicator.
- **The mono rule keeps tails**: drawing a short note at the start of a long one leaves the long
  note's tail as a new note, which retriggers. That is consistent with the clip rule, but some
  users may expect the old note to vanish.
- **Audition** needs the transport stopped (as specified) and ignores mute/solo. A fixed DPCM
  Sample control isn't mentioned in the hover read-out.
- **The arrangement lanes are 36 px** while the piano roll is open. The chip name under each lane
  name is hidden, and clips show only their header strip, not the mini note preview.
- **Recovery** is only offered if an autosave happened (first one after 2 minutes, or on a clean
  quit). A crash in the first two minutes of a session leaves nothing to recover.
- **The look is placeholder**, as the brief asked: flat rects, stock buttons, plain labels.

## 11. How to write a song in it (for the team)

1. `cargo run --release -p app`. **New** (Ctrl+N) gives an empty project with the 8 channels.
   Pick the chip (2A03, or with VRC6) and the tempo (drag or wheel the BPM field).
2. Set **SNAP** to *Bar*, then **double-click** in a lane to make a 1-bar clip. Drag its right
   edge to make it longer.
3. **Double-click the clip** (or select it and press Enter) to open the piano roll.
4. Pick a **GRID** value. Click to draw notes, and drag while drawing for longer ones. Shaded rows
   are notes that channel can't play. Hover a key to see how far the chip's pitch is from true.
   Click a key or a note to hear it (while stopped).
5. Shape with the keyboard: select (click, Shift-click, Shift-drag, Ctrl+A), then Up/Down or
   Shift+Up/Down to transpose, Left/Right to nudge, **Q** to quantize, Ctrl+D to repeat to the
   right, Ctrl+C/V to copy. Drag in the **VOLUME** lane for dynamics; double-click a bar to go
   back to the channel's default.
6. Turn on **Loop clip** and press Space to hear the clip over and over while you edit. **Follow**
   keeps the playhead in view.
7. Esc goes back to the device panel to set the channel's sound: duty, envelope, sweep, noise
   mode, DPCM rate, saw rate. These are undoable too.
8. Reuse clips by Ctrl-dragging them in the arrangement. Copies are independent: editing one
   doesn't change the others.
9. Ctrl+S saves a `.chipdaw` file (it asks for a name the first time). Ctrl+Z / Ctrl+Shift+Z
   undo and redo anything. If the app crashes, the next start offers to recover the last autosave.

`docs/songs/scale_test.chipdaw` is a reference for what each channel can do. It's a test song, not
a demo.
