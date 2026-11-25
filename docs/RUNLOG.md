# RUNLOG: stabilize/v1.1 (unattended run, STABILIZE_PROMPT.md)

Times are local (CST). Each decision taken without the user lists the options and the reason.

## 2026-11-25

- **07:44 Start.** Branch `stabilize/v1.1` cut from `product/v1` (281ee66). Disk: 834 GB free.
  All project docs read.
- **Decision D1: no push until A1 is done.** `product/v1` was never pushed. Pushing the branch
  publishes its assets in the public repo, so A1 (provenance) runs first and the first push
  follows it. Alternative: push at once (rejected: it would publish an unverified sample).
- **Environment.** `cargo` isn't on this session's PATH; every call prefixes
  `~/.cargo/bin`. No effect on results.
- **A1 start (07:44).**
  - `hey.wav` and `showcase.mid`: no generator in the repo. A search of the product-pass session
    transcript found the generator, `make_assets.py`, written by that session. Its final version was
    still in that session's scratch folder. Running it reproduces **both files byte for byte**, and
    the app's render of them reproduces `docs/screens/product/showcase.wav` byte for byte. Three
    sources agree (transcript, script, output), so the files are provably ours.
  - **Decision D2: keep the files and commit the generator** (`tools/assets/make_assets.py`),
    instead of replacing them. Replacing them was the fallback for "not provably ours", which no
    longer applies.
  - Fonts: name tables confirm Inter 4.001 and JetBrains Mono 2.304, with OFL files present.
  - Names: "Ableton" removed from code comments, DESIGN.md and PRODUCT.md.
  - **Decision D3: leave `CLAUDE.md` unchanged.** It is the owner's brief. It names "Konami VRC6"
    and "Ableton-style" descriptively, and lists the forbidden names as a rule. Rewording it is
    the owner's call; logged in ASSETS.md.
  - **Decision D4: keep "NES", "Famicom", "2A03" and "VRC6".** They name the emulated hardware
    (nominative use), as the brief's own channel names ("VRC6 Pulse 1") do.
- **07:46 A1 done**, pushed (first push of the branch).
- **A2 start (07:47).**
  - Added `CHIPDAW_SONG=heavy` (`mock::heavy_arrangement`): the demo's clips with 2-beat notes,
    and every lane except DPCM on an instrument that loops all four macros plus vibrato. Added
    test **H** (heavy song, 600 s) and an `-AppExe` parameter to `matrix.ps1`, so the matrix runs a
    frozen copy of the build while the repo keeps compiling.
  - **The MOTU is disconnected** (PnP status "Unknown"; the only active output is the monitor's
    HDMI endpoint, VX3211-2K). **Decision D5:** run on the HDMI endpoint (480-frame callbacks),
    as PIANOROLL.md did, and say so in every table. Alternatives: wait for the user (not possible),
    or skip the audio tests (rejected). Test E (default-device switch) needs two outputs, so it
    is **skipped**. Nothing to restore: the default output wasn't changed.
  - **First finding: 58 loopback "dropouts" in a 20 s smoke run of the heavy song**, with 0 starved
    or late callbacks and a max callback cost of 3.7%. The two detectors disagreed. Third source:
    the detector's own comment (ENGINE-macos.md §3) says a 19 kHz pilot can be masked by music when
    the Famicom filter (no 14 kHz low-pass) is active. That filter has been the default with VRC6
    since the macOS pass, after ENGINE.md's matrix was run. Re-running the same build and song with
    the pilot at **23 kHz** (above the synthesis passband, 21.6 kHz) gave **0**; the demo also gave 0.
    **Validation of the 23 kHz detector:** the headless harness with injected audio-thread stalls
    of 30 ms and 15 ms alternating every 3 s flagged exactly the five 30 ms stalls and none of the
    15 ms ones. That matches ENGINE.md §8.1 (stalls ≥ 22 ms drop out). **Decision D6:** this
    pass's matrix uses a 23 kHz pilot, and `PILOT_HZ` now defaults to 23 kHz.
  - 07:50 Full matrix started in the background (A, B, C, G at full length × 3 buffer sizes; D; H).
    **Decision D7:** keep coding during it, with builds at below-normal priority. A dropout would
    then mean "even under a parallel build"; a clean run is stronger evidence, not weaker.
- **A3 parsers (07:51–07:59).** `crates/ui/tests/robustness.rs`: seeded campaigns for `.mid`,
  `.wav` and `.chipdaw` (valid seeds of every kind, byte mutations, JSON-aware mutations, noise;
  20 s budget per input on its own thread, so a hang fails instead of blocking). Inputs that parse
  go on through arrange / convert / snapshot / a short render / save-and-reload.
  - **Found by the campaign (debug build, 1000 JSON mutations):** 4 panics, 2 kinds. A clip or note
    id of 4294967295 overflowed `id + 1` (`state.rs`), and a note at the end of time overflowed
    `start + len` (`notes.rs`).
  - **Found while reading the parsers for the harness:** `from_hex` sliced the sample string at byte
    offsets (non-ASCII → panic); `free_bytes` underflowed when a file's samples overflow the ROM;
    an export sized its buffer from `loop_end` (u32::MAX → terabytes → abort); WAV floats could
    carry NaN/∞ into the encoder; a whole WAV was read into memory although DPCM holds ≤ 7.8 s;
    the MIDI clip end overflowed near u32::MAX ticks; notes with no note-off were dropped, and so
    were zero-length notes (drum triggers).
  - **Fixes:** one song limit, `MAX_SONG_TICKS` (9999 bars) and `MAX_ID`, checked on load (clear
    errors) and on MIDI import (dropped and reported); saturating ends; byte-wise hex; the ROM total
    is checked on load; export is capped at 1 h; WAV reading stops at 10 s, with non-finite samples
    set to 0; a silent WAV is refused with a message; unclosed MIDI notes end with their track, and
    zero-length ones last one tick; tempo changes and octave-folded notes are reported.
  - **Decision D8: fold out-of-range MIDI notes by octaves** into the channel's playable range,
    instead of importing silent notes. Reported per lane.
  - After the fixes: 0 failures at 3000 inputs per parser (debug, overflow checks on) and at the
    default 2000 (release). The showcase still imports and renders byte-identically.
  - `crates/ui/tests/import.rs`: 18 behaviour tests (MIDI formats 0/1, running status, tempo
    changes, many tracks, chords, out-of-range, zero-length, drums, DPCM track, truncated/corrupt;
    WAV 6 formats × 3 rates × mono/stereo, silence, clipping, long, corrupt, ROM full; export
    length and tails; v1/v2 files, atomic save under a simulated failure, crash recovery).
- **A2 interim (08:09).** A_128 (600 s): 0 loopback dropouts, 0 starved/late, callback mean 0.21%
  / p99 0.6% / max 3.5%, with low-priority builds running beside it.
- **A4 (08:00–08:12).**
  - **A4.1 One source of truth.** The lane Volume control is now a **channel volume**:
    out = note volume (or 15) × volume macro × channel. `scale_volume` is symmetric with 15 as
    identity, so notes without their own volume sound exactly as before. In the hardware decay
    envelope modes V is a decay speed, not a level; there the dial reads **"Decay"**. Duty,
    Envelope and the noise Mode show **"SET BY INSTRUMENT"** (or "SET BY KIT" when every drum
    overrides them), drawn faded, and ignore input. The values are kept and come back when the
    instrument stops overriding them. Saw Rate scales the note's rate (42 = as is; above 42
    overdrives into the wrap). Undo and saving already covered params.
  - **Decision D9: bump the file format to version 3 now**, not in Part B, because the volume
    semantics changed. Part B adds its instrument table to the same version 3 before any merge.
    Migration v1/v2 → v3 (`file::migrate_v2`): on lanes that mix notes with and without their own
    volume under V < 15, notes without one get V and the control goes to 15. This is exact. The
    saw does the same when its Rate is a value a note volume can express; otherwise it is
    approximated and reported. **Evidence:** the v1 golden song still renders to
    `0xaaaca2df8157ca6a`.
  - **A4.6 Loop start** and the loop on/off light are saved (v3 `loop_start`, `loop`).
  - **A4.2 Clock:** `m:ss.cc`. A click without a drag toggles to bars.beats.sixteenths,
    remembered in `settings.json` in the app-data folder. A double-click still returns to the
    start and cancels its first click's toggle.
  - **A4.3 Vibrato** moved from the CLIP section to the INSTRUMENT section, now editable: depth,
    rate and delay fields (drag or wheel, undoable).
  - **A4.5 Duplicate meters. Decision D10: keep the top-bar spectrum; remove the inspector's
    CHANNELS meters.** The spectrum is in the designer's prototype and always visible. Per-channel
    activity also shows on the track strips (they flash on note-ons). The inspector width goes to
    the instrument editor, which Part B extends. The minimum window shrinks from 1460 to 1140 px
    wide, the width the top bar was verified at (DESIGN.md §8). `views/meter.rs` was deleted.
  - **A4.7 Sample rows** are named after the imported sample ("User n · empty" when the slot is
    empty) in the piano roll rows, the hover read-out and the import message. The browser shows
    "name · slot n".
  - **A4.4 Contrast (by token; screenshots follow after the matrix).** WCAG ratios computed from
    the tokens: TEXT_LO #5C was 2.7–3.0:1 on every panel; TEXT_GHOST (chip badges, step counts,
    "not used", the browser's counts and hints, the field placeholder) was 1.6–1.7:1. TEXT_MID #88
    was 5.2:1 on panels but 3.9:1 on key faces (#2D). **Decision D11:** TEXT_LO → #808080 (≥ 4.56:1
    on #151515), TEXT_MID → #9A9A9A (≥ 4.5:1 on key faces), with the stylesheet variables, the
    placeholder and the browser hint to match. Every informative TEXT_GHOST use moved to TEXT_LO;
    TEXT_GHOST is decoration only. The hierarchy HI > MID > LO is kept.
- **Migration corpus (08:12–08:16).** A worktree of `product/v1` (281ee66, untouched) built
  `docs/songs/v2/v2corpus.rs.txt`, which saved 10 version-2 files (the demo band; mixed per-note
  volumes under channel controls < 15 with decay envelopes; raw channels; all 5 layouts; the
  showcase with its DPCM sample; one deliberately inexact saw case). It printed each file's hashes
  from *that* build: `render::render` and `export::render`, the latter including the DPCM samples.
  `crates/ui/tests/migration.rs`: **all 9 exact files render bit-identically after migration to v3
  (both hashes)**, and again after a v3 save and reload. The inexact case is reported in
  `load_report` and does differ. This was checked before the Part B refactor, so B must keep it.
- **Part B (08:17–08:26).** Started before the A2 UI benchmarks and the A3 UI scenarios, which need
  the matrix finished (the matrix's app windows take focus). **Decision D12:** do B's offline code
  in that time; A5's must-fix check is re-run once the A2/A3 runs are in, and anything found is
  fixed before C ships.
  - Model: `InstrumentTable` (stable ids, lane defaults, `Clip::instrument` overrides, shared by
    reference, `prune`, max 32 so a table full of kits still fits the engine's u8 bank index).
  - **Engine: the driver now copies the instrument at note-on.** Before, a held note kept a bank
    *index*. Per-clip instruments reorder the bank whenever clips change, so a new snapshot could
    have switched a sounding note to another instrument. Test
    `clip_instruments_switch_at_note_on_and_never_mid_note` fails on the index version and passes
    on the copy. No allocation, no locks (the copy is a fixed-size `Copy` struct).
  - v3: top-level `instruments` (with ids), `lanes[].inst`, `clips[].instrument`. Broken references
    are clear errors. Migration identity still holds after the refactor (all 9 v2 files, both hashes).
  - UI: drop a preset on a clip (the clip is outlined during the drag; the ghost says "→ this
    clip"); on the lane header it sets the lane default as before. The clip header shows
    "name · instrument" when the clip has its own. The inspector title says whose instrument it is
    and how many clips play it. The clip menu has Make Instrument Unique, Use Lane Instrument and
    Save Instrument as Preset (`user_presets.json` in the app-data folder, listed in the browser
    under "User Presets"). In the macro editor, loop and release markers can be dragged;
    Shift-click sets the loop point, Alt-click the release point, a double-click removes one.
  - **Decision D13:** a preset dropped on a *clip* sets only the instrument, not the preset's
    channel controls: those belong to the lane, and changing them would alter every other clip
    on the lane.
  - Undo: the property test now mixes 5 instrument operations (lane default, clip override, make
    unique, shared edit incl. loop point, back to lane) with the old ones, and checks every
    reference stays valid. 40 seeds pass. Unused instruments are pruned at each undo commit.
- **Part C core (08:27–08:32).** `crates/ui/src/game_export.rs`, `tests/export_loop.rs`.
  - **Decision D14: how a loop is made exactly periodic.** An emulator never repeats itself: its
    60.1 Hz driver clock, the LFSR and oscillator phases run on, so pass 3 of a free-running render
    differs from pass 2. The export renders the intro and pass 1 normally, **clones the whole
    emulator at the start of pass 2** (`Player: Clone`; never on the audio thread) and plays every
    later pass from that clone. `intro.wav` therefore runs up to pass 2 (intro + pass 1), because
    pass 1's start follows the intro and can't carry the loop-end tails; `loop.wav` is pass 2. The
    single file and the JSON give the same points. Alternative: a pure-intro file. Rejected: its
    junction to the loop would be wrong by construction (cut intro tails, wrong loop-end tails).
  - **Gate C1 passes:** intro + loop × 3 read back from the files equals one continuous render
    sample for sample. Covered: the showcase (8-bar intro + 8-bar loop) and the test song (whole
    song), 44.1 and 48 kHz, 16 and 24 bit. The single file's data and `smpl` points match, and so
    does the JSON.
  - **44.1 kHz renders natively** (the step synthesis and filters take the rate). Pulse A4 counts
    the period formula's frequency ±1 Hz at both rates. No `rubato`.
  - **Measured:** free-running playback (the app's live loop) vs the exported loop over passes 3–4:
    +3.1 dB RMS. The showcase loop is 824.21 driver frames, so each live pass shifts the frame phase
    by 0.21 frame and some notes land one frame (16.6 ms) apart from pass to pass. The export
    keeps pass 2's timing. Reported as a known gap, not changed in the engine.
  - **C2 stems** (showcase, 48 kHz): 8 channel stems summed vs the mix: **−11.3 dB RMS** relative
    to the mix, peak error **−20.3 dBFS**. Groups melody/bass/drums: **−11.5 dB RMS**, peak
    **−19.3 dBFS**. This is the non-linear 2A03 mixer. Every stem has the loop's length and points.
  - Loudness: authentic (default), peak to −1 dBFS, or LUFS integrated via `ebur128` 0.1.11 (MIT,
    pre-approved) with a true-peak ceiling. One gain from the mix, applied to the mix and every
    stem, so stems keep their balance and the loop stays exact.
- **08:38 Robustness, final:** 10,000 inputs per parser, in release (9 s) and debug with overflow
  checks (58 s): 0 panics, 0 hangs. Snapshot build with instruments in the snapshot: 1k notes
  0.027 ms (was 0.007), 10k 0.128 (0.124), 50k 0.593 (0.602): a fixed ~0.02 ms for the
  instrument bank. DPCM with all 4 user slots filling the ROM plus the 4 built-ins: every row
  sounds, no overlap in the ROM (`dpcm_all_four_user_slots_full_play_with_the_built_ins`).
- **09:18 Matrix done** (build of 07:50, i.e. after A3 but before A4/B/C; HDMI endpoint, 480-frame
  callbacks, 23 kHz pilot). **All 16 runs: 0 loopback dropouts, 0 starved, 0 late, 0 allocation
  violations, 0 dropped commands.** A (600 s ×3): cost mean 0.21% / p99 0.6% / max 3.5–3.8%.
  B (600 s ×3, 141k edits each): 0.08% / 0.3% / 1.3–3.1%, edit → frame tick ≤ 16.64 ms. C (120 s ×3,
  60 stalls each): 0.21% / 0.6% / 0.9–3.3%. G: A/V −10.29 … −10.33 ms, sd 0.10. D (resize churn):
  A/V sd 0.43–0.58 ms, **no crash** from the external SetWindowPos resizes DESIGN.md §8 reported.
  H (heavy macros, 600 s): 0.35% / 1.3% / 3.7%. E skipped (one output device).
  **Decision D16:** Part B changed the driver after this build, so A, B, C and G are re-run on the
  final build before the merge decision (one buffer size each: WASAPI ignores the request, so the
  three sizes are the same run).
