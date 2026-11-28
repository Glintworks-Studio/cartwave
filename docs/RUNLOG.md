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
- **A2 UI benchmarks (09:20–09:43; HDMI audio).**
  - **The harnesses were stale since the redesign.** `bench.ps1`/`pacing.ps1` dragged at client
    (300, 160), now a track header, and `bench_pr.ps1`/`pacing.ps1 prdrag` at (98, 591), now an
    empty piano-roll row: those "drag" scenarios dragged nothing since the redesign. They now drag a
    clip (720, 130) and the C4 handle note (88, 535); a mid-drag screenshot confirmed each.
    `make_songs` also silently rewrote the v1 golden song in the current format. It no longer
    touches `docs/songs` (the file was restored from git).
  - Arrangement (`bench.ps1`, 3 runs): idle 0.3/0.4/0.7%, demo 10.5/10.5/10.4%, 200 clips
    9.1/11.6/11.2%, 200-clip drag 12.0/12.8/12.5%. Pacing (30 s): demo, 200 and two of three drag
    runs: 0 > 20 ms, 0 missed vblanks; one drag run had a single 33 ms frame (1 missed vblank).
  - **Demo playback is ~10.2% against the ≤ 10% target.** An alternating A/B with the untouched
    product/v1 build on the same script: old 10.8/9.1%, new 9.5/10.1% (idle 2.1–2.3% on both: a busy
    machine). So nothing in this pass changed it; it has been at the line since the redesign added
    the spectrum, strip glows and browser. Within PERF.md's ±0.5–1 point noise; reported as at
    target.
  - **Regression found and fixed: piano-roll note drag at 32–35% CPU** (target ≤ 25%; PIANOROLL.md
    measured 12.5–16%), with 5 frames > 20 ms. The A/B (with a v2 bench file, because the old build
    rightly refuses v3) showed the old build at 33% too: the redesign made each note two gradient
    shaders and two rounded rects, and the whole 2,000-note layer was re-rendered per pointer move.
    **Fix** (PIANOROLL.md §8's proposed next step): during a note drag the layer is keyed on the
    *other* notes and reused; dragged notes and their volume bars are drawn live on top.
    After: **dragging 7.3/8.1/7.8%**, pacing 8.8–9.0% with **0 > 20 ms, 0 missed vblanks** (×2);
    playing 5.9–6.6%.
- **A3 UI scenarios and A4.4 contrast (09:44–09:50).**
  - `docs/stabilize/tools/scenario_v11.ps1` + `check_v11.py` (synthetic input, a v2 file opened
    and saved as v3 at each stage): draw on all four macro graphs, undo ×4 / redo ×4, drag the
    release marker, wheel the vibrato, browser search, drag onto a clip / an invalid lane / a lane
    header, double-click load, apply a layout. **16/16 checks PASS.**
  - Found on the way and fixed: an unused macro graph could not be drawn on, so the brief's "draw on
    every macro graph" was impossible. **Decision D17:** drawing on an unused graph starts a 16-step
    macro. Also: the inspector title went stale when only the selected lane changed; "SET BY
    INSTRUMENT" was wider than its dial and left a stray letter on partial repaints (now two lines,
    clipped); the CLIP section now caps at 320 px so the instrument editor gets the meters' room;
    the export dialog's group chips overflowed (two rows, short names).
  - **Contrast measured from a screenshot** (informative texts: track preset names, chip badges,
    browser hints and headers, dial labels and reasons, macro labels and step counts, vibrato labels,
    pill labels): every one is **4.62:1 or better** (worst: TEXT_LO on #151515 panels).
  - `screens_v11.ps1`: docs/screens/v1.1 (clock in both formats, knobs and macros, a clip with its
    own instrument, the export dialog, the main window), and **one export driven through the dialog**
    (44.1 kHz / 24-bit / 8 channel stems: 28 files, `smpl` present, PASS). A test-only
    `CHIPDAW_EXPORT_TO` stands in for the native save dialog. The "export done" capture was dropped
    because it showed a local user path.
  - Final suite: **153 tests pass**, clippy clean, release build 0 warnings.
- **09:51** Verified with the product/v1 build's own loader: a v3 file is refused with "This
  project was saved by a newer ChipDAW (file format version 3). This version reads format 2. Update
  ChipDAW to open it." 09:50: final-build matrix started (A, B, C, G at 512, plus H; frozen exe).
- **10:24 Final-build matrix:** A, B, C, G (512) and H, all **0 loopback dropouts**, 0 starved / late,
  0 allocation violations. A 0.22 / 0.6 / 3.3%, B 0.08 / 0.3 / 3.3% (141k edits), C 0.21 / 0.6 /
  0.7%, G −10.34 ms sd 0.11, H 0.35 / 1.4 / 3.1%.
- **Decision D18: all gates pass, so the branch should merge into `main`.** Gate 3: demo playback
  averages ~10.2% against ≤ 10%. That is within PERF.md's ±0.5–1 point noise, equal to the
  pre-pass build in an alternating A/B, and not changed by this pass. Every other UI scenario is
  well inside its target, and the one real regression (piano-roll drag) was fixed.
  **The merge was not done:** the session's automatic permission check blocked merging and pushing
  `main`, and it was not retried or worked around. The merge is a fast-forward, ready for the owner.
- **10:27 End.** The old-build worktree was removed. No ETL traces were made. The default audio
  output was never changed, so there was nothing to restore.


---

# RUNLOG: app-shell/batch-1-2 (unattended run, SHELL_PROMPT.md)

Times are local (CST). Same format as above: every decision taken without the user, with the
options and the reason. The report is SHELL.md.

## 2026-11-25

- **14:20 Start.** VISION.md, the audit (SUMMARY, UX-DRAFT, FINDINGS, FEASIBILITY) and the earlier
  reports read. `cargo` still isn't on PATH (`~/.cargo/bin`).
- **Decision D1: the 5 untracked `*_PROMPT.md` files go to `wip/prompts-2026-11-25`** (local
  commit, not pushed), as the brief says. They are the owner's prompts, so neither discarding
  nor publishing them is right.
- **14:24 Git.** `main` fast-forwarded to `stabilize/v1.1`, then to `docs/ux-audit`; no
  conflicts. 153 tests pass on `main`, golden included. `git push origin main` succeeded
  (`dd590e8..1ca4a75`). `app-shell/batch-1-2` cut from it.
- **14:27 Documents.**
  - **D2:** CLAUDE.md is the proposal without its "proposed" banner. Three standing rules from
    the brief were missing (measure before/after, trust probes, benchmarks prove themselves) and
    are added. It now points to UX.md, SHELL.md and `shortcuts.rs`.
  - **D3:** Follow on macOS is ⌥⇧F, the reference's own Mac key (R135). Decision 3 names the
    Windows key; making it ⌘⇧F would invent a key.
- **Batch 1.**
  - **D4:** dev mode is a debug build or `CHIPDAW_DEV=1`. It gates F3–F7 and every test hook.
    `CHIPDAW_DATA_DIR`, `CHIPDAW_AUDIO=0` and `CHIPDAW_VSYNC=0` stay open: they are configuration
    and change no document. Alternative (gate them too) rejected: harmless, and AT-02 needs a
    throw-away data dir without dev mode.
  - **D5:** the save prompt is rfd's task dialog with Save / Don't Save / Cancel. Its
    `common-controls-v6` feature needs comctl32 v6. A manifest dependency is embedded with MSVC
    linker arguments from `build.rs` (app, and ui for its tests and examples), with no new crate.
    Alternative (Yes / No / Cancel buttons) rejected as unclear.
  - **D6:** the window close is cancelled by the root model consuming `WindowClose`: vizia sends
    it to models before the window view. No fork change.
  - **D7:** "dirty" is the undo document plus the saved fields outside undo (profile, filter,
    loop, mute/solo). A song from a MIDI file and a recovered session start out unsaved.
  - **D8 (a fix):** clip order is not content. A click raises a clip, which reordered the list.
    That made the song "unsaved" and, before this run already, added an undo step. `Doc::same_as`
    now compares by id when the order differs (unit test).
  - **D9:** the text-field rule tracks the field being edited from vizia's
    `TextEvent::StartEdit` / `EndEdit`, which bubble up to the root. In a field, only the field's
    own keys work: no Ctrl+S either, as the brief says ("only Esc, Enter, Tab and the field's own
    text editing").
  - **D10:** Esc is one app action: close the context menu, else close the piano roll, else clear
    the selection. Before, the arrangement and the root each did part of this.
  - **D11:** on dimmed VRC6 lanes the M/S keys stay clickable: muting a silent lane changes no
    content, and hiding them would shift the header layout.
- **Batch 2.**
  - **D12: interface scale is shown, disabled, with its reason.** vizia's winit backend ignores
    `WindowDescription::user_scale_factor`. Applying it is a small fork change, but the fork would
    then need publishing, and the brief allows only the two pushes. Fork change described in
    SHELL.md §7.
  - **D13:** the Warnings tab and its reset work, but no warning can be silenced yet: the native
    dialogs can't hold a "Don't ask again" box. The first one comes with F-47's in-app dialog.
  - **D14:** muda is an optional feature (`menus`, on by default), so a plugin build can leave it
    out. `raw-window-handle` 0.6 is a direct dependency, for the HWND; it was already in the tree
    via winit, rfd and muda. New transitive crates: crossbeam-channel, fdeflate, keyboard-types,
    miniz_oxide, png, muda. All MIT / Apache / Zlib.
  - **D15:** on Windows no accelerator is registered. Items read `label\tkey`, which is exactly
    what muda draws, and keys take our own path. No double fire.
  - **D16:** the window grows by the menu bar's height when it is attached, so the content keeps
    its size and every harness coordinate still holds.
  - **D17:** menu contents: only existing features.
    - Cut / Copy / Paste / Quantize act on notes, and are disabled without the piano roll.
    - Import ▸ holds only Audio as DPCM Sample. Today's replacing MIDI import is File > New from
      MIDI File, and Ctrl+I stays free (decision 3).
    - The Edit menu acts on the editor that last had the mouse.
  - **D18:** the arrangement grid is three items in the right-click menu (Grid: 1/16, 1/4, Bar,
    with a tick) instead of a nested submenu: the in-app context menu has no submenus yet.
  - **D19:** Settings > Audio switches through the engine's rebuild path. A new `ERR_RECONFIG`
    wake reason makes the host drop the stream and rebuild at once. The sample rate is applied to
    the core in between: `Synth::set_sample_rate` rebuilds the band-limited step buffer and the
    filter at the current level, and keeps the chip state (unit test: no level jump, time keeps
    running). Measured gap while playing: 26–34 ms.
  - **D20:** Project Settings needs no format bump: `chip` and `filter` were already in v3.
- **Harnesses.**
  - **D21:** every harness sets `CHIPDAW_DEV=1`. In dev mode the app prints one stderr line per
    hook it applied (`dev: mode on…`, `dev: F4 stress set on`, `dev: CHIPDAW_PROJECT … (n
    clips)`, `dev: CHIPDAW_OPEN_CLIP n (piano roll open: true)`). bench, pacing, bench_pr and
    trace_app check these lines and print "reached" or "NOT REACHED".
  - The screenshot scripts were run, and their images inspected (copies kept in
    `%TEMP%\chipdaw-harness-proof`). The committed images were then restored, since they document
    earlier passes.
  - `trace_app.ps1` was not run: it needs the elevated xperf helper. While editing it, its drag
    coordinates (still the pre-redesign track header) were fixed.
  - **D22:** `pianoroll/scenario.ps1` failed on the **pre-run build too** (pre-redesign
    coordinates, a format-v1 check). It was rewritten for the current layout: grid from the
    context menu, saving through the native Save dialog via the clipboard, because UI Automation
    hangs walking the shell dialog. It passes. `stabilize/screens_v11.ps1` used the removed EXPORT
    key and clock click; it now uses Ctrl+Shift+R and View > Clock Shows Bars. It passes (28-file
    export).
- **Acceptance harness** (`docs/shell/tools`): `at_shell.ps1` drives the app.
  - `dialog.ps1` finds and answers the native dialogs through UI Automation: task-dialog
    buttons are `CCPushButton` panes, file-dialog buttons are `Button` panes. It opens menus
    with their Alt mnemonics, since vizia's accessibility tree hides the menu bar.
  - `pixels.ps1` does the pixel checks (Pillow isn't installed).
  - `check_shell.py` checks obs.json and the saved files.
  - Final run (`%TEMP%\chipdaw-at-final`): **9/9, 49 checks**.
- **Measurements.**
  - **Before/after:** the pre-run build was built in a worktree and benched alternately with the
    branch. No regression (SHELL.md §5).
  - Pacing: 0 frames over 20 ms, 0 stalls.
  - Engine A and C, 120 s: 0 pilot dropouts, on the HDMI endpoint (the MOTU is not connected;
    D5 of the stabilize run applies).
  - An Alt-drag doesn't leave the menu bar focused when Alt is released (checked).
- **15:50 Gates:** all pass (SHELL.md summary). Merge follows.
- **15:46 Merge.** Every gate passes, so `app-shell/batch-1-2` is fast-forwarded into `main` with this commit and pushed (branch pushed too).

---

## Batch 3 run (BATCH3_PROMPT.md), 2026-11-25

Branch `app-shell/batch-3` from `main` @ fb2d632. Unattended; decisions are numbered B3-Dn.

- **Setup.** The pre-run build (fb2d632) is in the worktree `../prerun-b3` (release built) for the
  A/B benchmarks. It also wrote the format-v3 corpus `docs/songs/v3/` (the v2 corpus and the scale
  test, re-saved as v3) and its render/export hashes (`docs/songs/v3/HASHES.txt`), for the v3 → v4
  migration test.
- **B3-D1 (fork branch).** The prompt names the fork's "existing `chipdaw` branch". The fork has no
  `chipdaw` branch; the one the app pins is `chipdaw-perf`. Fork commits go there.
- **B3-D2 (dark menus: report, not build).** Research result: no documented API darkens the stock
  Win32 popup menus. `DwmSetWindowAttribute(DWMWA_USE_IMMERSIVE_DARK_MODE)` only affects the title
  bar; `SetPreferredAppMode` / `AllowDarkModeForWindow` / `FlushMenuThemes` (uxtheme ordinals) are
  undocumented; muda 0.22 (the newest) has no popup theming and its maintainers left popups light
  by choice (muda #97). The documented route is owner-drawn items (`MFT_OWNERDRAW` +
  `WM_MEASUREITEM` / `WM_DRAWITEM`, ~200–300 lines, no new crate), which can't recolour the
  system-drawn popup border, shadow or submenu arrow. Not implemented in this run (the brief: only
  undocumented ones work fully → report the options). **Found while checking:** muda's dark menu
  *bar* (`MenuTheme::Dark`, shipped in Batch 2) itself relies on the undocumented
  `WM_UAHDRAWMENU` / `WM_UAHDRAWMENUITEM` messages. Left as it is and flagged for the owner in
  BATCH3.md (switching to `MenuTheme::Light` is a one-line change).
- **B3-D3 (Space).** Space plays from the insert marker; stopping puts the playhead back on it
  and remembers the stop point, which Shift+Space continues from (R137). While stopped the
  playhead *is* the insert marker. A click on empty lane space or inside a clip (without a
  drag) moves the insert marker (A1); while playing it is drawn as a short line on its track.
- **B3-D4 (clipboard).** Clips (and time) are copied to an in-app clipboard, separate from the
  notes clipboard; nothing is read from the system clipboard, so there is no new parser.
- **B3-D5 (time scope).** The …Time commands act on every track (R46). Cut / Copy / Duplicate /
  Delete with a time selection act on its tracks only and leave the gap (Delete splits at the
  edges, AT-22). Duplicate on a time selection overwrites what follows (the copies win, like
  every newer clip); Duplicate Time inserts.
- **B3-D6 (Insert Silence).** UX.md: "at the time selection"; R47: "the chosen amount at the
  insert marker". Without a dialog for the amount, it inserts the selection's length at its
  start; without a time selection it says so.
- **B3-D7 (consolidate).** Our clips can have their own instrument (Live's can't). Consolidate
  leaves a track alone if its clips play different instruments, and says why.
- **B3-D8 (ruler double-click).** UX.md says Zoom to Fit = whole song; R32's full text: the time
  selection if there is one, else the whole arrangement. Both agree with no selection; it
  follows R32. AT-12 clears the selection (Esc) before the double-click.
- **B3-D9 (draw mode).** In Draw Mode a click on a note erases it (R6). A double-click-created
  note is one grid step long (R3 is unverified: ours).
- **B3-D10 (grid keys).** Cmd+1–5 act on the focused editor. The arrangement grid gained 1/32,
  1/8 and 1/2, triplets, snap off and an adaptive grid. The piano roll has narrow / widen /
  triplet; Snap and Adaptive there say they don't apply (its grid is always on; the no-snap
  modifier places freely).
- **B3-D11 (I2).** F-05 asked whether the triangle's Volume macro does anything: `driver.rs`
  silences the triangle on a step of 0, so it works as a gate. It stays, labelled "GATE
  (0 = SILENT)"; the triangle's Duty and the DPCM's Volume and Duty (ignored by the driver)
  are hidden with the reason.
- **B3-D12 (I3).** The instrument header has no room for three buttons next to its title at
  the minimum window width. Make Unique and Use Track Instrument (they act on the clip) are in
  the CLIP section, Save as Preset in the instrument header.
- **B3-D13 (tooltips).** vizia never hovers a disabled view, so a disabled control's tooltip
  sits on a thin wrapper around it. Tooltips show after 500 ms.
- **B3-D14 (B2).** There is no preview voice during playback; a click on a preset while
  playing says why nothing plays.
- **B3-D15 (F-38).** The CPU pill reads OFF when no output runs (with a tooltip).
- **B3-D16 (F-37).** Notices sit at the bottom right; a click dismisses one; an import report
  stays 30 s and wraps.
- **B3-D17 (format v4).** Track names and colours and clip colours are saved: format 4. A v1–3
  lane `name` was informational, so older files keep the default names (exact migration,
  checked against the v3 corpus hashes). Bad colours are clear errors; names lose control
  characters and are capped at 48 characters.
- **B3-D18 (bug fixed on the way).** A clip split by an overlap lost its own instrument
  (`Clip::trimmed` didn't copy it). Fixed; unit-tested.
- **B3-D19 (F-33).** The profile, the loop brace and the loop switch are undoable (part of the
  undo document). Mute/solo stay out (R141 is unverified).
- **B3-D20 (T5).** Solo is exclusive; Ctrl/Cmd-click adds (R143).
- **B3-D21 (shortcut test).** `no_conflicts` now checks the contexts keys are looked up in
  (Arrangement, PianoRoll, Text, Modal): the arrangement's ← → (insert marker) and the piano
  roll's ← → (nudge notes) may share keys.
- **B3-D22 (interface scale).** Settings > Display: 75–200%, on top of the OS scale, applied at
  start (`Application::user_scale_factor`) and live (`WindowEvent::SetUserScaleFactor`, fork
  5a3b035a). The window keeps its logical size, so at 200% on this 2560×1440 display it is
  2880×1800 px, larger than the screen.
- **B3-D23 (Follow).** One Follow (View menu, Ctrl+Shift+F) pages both editors (F-32).
- **Measurements.**
  - UI A/B against the pre-run build (3 rounds each): demo 9.4 → 9.6%, 200 clips 10.0 → 9.5%,
    drag 11.8 → 11.3%. Pacing 30 s: 0 frames > 20 ms, 0 stalls (demo, 200 clips, drag).
    Piano roll 2,000 notes: playing 6.4 → 6.9%, dragging 6.7 → 6.6%. At 150%: demo 9.2–9.8%,
    200 clips 8.2–9.2%, drag 11.5–11.8%.
  - Engine matrix on the MOTU (frozen build): A, B, C, G, H full length and D at 128/256/512:
    0 loopback dropouts, 0 starved/late, 0 allocation violations. Test E (first time: MOTU →
    HDMI → MOTU default): 2 rebuilds, no crash, 399 ms gap, MOTU default again afterwards.
    Settings > Audio while playing: 23–29 ms to HDMI, ~396 ms back to the MOTU.
  - `trace_app.ps1`: still pending (needs elevation; not attempted).
- **Gates:** all pass on the final build (17/17 Batch 3 ATs, 9/9 AT-01…19, 179 tests, clippy and
  release build clean, contrast ≥ 4.66:1). Merge follows.
- **Merge.** `main` fast-forwarded from fb2d632 to 7d0d18e (`app-shell/batch-3`), plus this note; pushed with the branch. The vizia fork's `chipdaw-perf` was pushed earlier (159bbe55..5a3b035a).

## Design pass (DESIGN_PASS_PROMPT.md, Option D), 2026-11-25

Branch `design/option-d` from `main` 23b5da9. Report: [DESIGNPASS.md](../DESIGNPASS.md); spec:
[docs/design/VISUAL-SPEC.md](design/VISUAL-SPEC.md). Not merged (§6: the CEO approves the
comparison screenshots first). Every decision below was taken alone, as the prompt asked.

- **DP-D1 (rendering the mockup).** `mockup-D.dc.html` loads a `support.js` runtime that wasn't
  supplied. `docs/design/tools/support.js` is a small stand-in (expands `<sc-for>`, `<sc-if>` and
  `{{…}}` from the file's own `renderVals()`); headless Chrome renders it at 1440×400 with the
  Google Fonts the file links. Its `?measure` mode dumps every element's box
  (`mockup-boxes.jsonl`), which is where every position below comes from.
- **DP-D2 (JetBrains Mono 600).** The prompt and the file's CSS both ask for weight 600, but the
  file's font link loads only 500 and 700, so the browser draws 700. We use 600 as specified and
  added `JetBrainsMono-SemiBold.ttf` from the same 2.304 release (its Medium is byte-identical to
  ours); ASSETS.md updated.
- **DP-D3 (file over prompt, logged as asked).** Row pitch 73 (72 + a content-box hairline);
  badge 18 px outer (16 + borders); clips inset 2 px on all four sides (the prompt: top and
  bottom); the key sheen sits on the row inside the border, not on it; the strip's lit look keeps
  the mockup's 10 px spill (§3.5 said "no outer glow"; the old 48 px row spill is gone); the
  selected view icon's glow is 55% (the indicators' 65%); the display's cell group starts at
  17 px (the mockup's 17.5, floored, which is where its dividers land).
- **DP-D4 (not in the mockup: kept).** Clip note previews, clip selection glow, drag shadows,
  the browser's layout, the inspector's layout. **Changed because the mockup shows otherwise:** no
  dimming outside the loop while looping, no shade under the ruler, the four mockup track colours
  (Pulse `#C9D24A`, Triangle `#7F86E0`, Noise `#B5AC98`, VRC6 Pulse `#D9579A`; the Colour ▸ palette's
  first entries follow).
- **DP-D5 (fader).** "Today's lane volume" is the channel Volume (0–15) on the pulses, noise and
  VRC6 pulses, and the saw's accumulator Rate (0–63). The triangle and DPCM have no volume
  control: their faders are disabled with the reason; so is a fader whose Envelope is a decay (the
  register is then the decay speed). Same value as the inspector dial: undoable, saved. The meter
  is the engine's post-fader level on the fader's scale, clamped at the thumb's centre
  (`state::meter_fill`, unit-tested).
- **DP-D6 (indicators).** The format indicators are "active" while the audio output runs; without
  output (CHIPDAW_AUDIO=0, no device) they are dim like a stopped Play.
- **DP-D7 (project format).** Project Settings had no sample rate (the device rate is in Settings >
  Audio), so the project's format gets both: Sample rate (44.1 / 48 kHz) and Bit depth (16 / 24).
  File format **v5** (`rate`, `bits`); v1–4 files open as 48 kHz / 16-bit (the export default they
  had), new projects 48 kHz / 24-bit (the mockup). Migration test: a v4 corpus written by the
  pre-run build (`docs/songs/v4`, hashes in HASHES.txt) renders bit-identically.
- **DP-D8 (export default).** The export dialog now opens on the project's rate and bit depth; its
  other choices are still remembered (E4). AT-28's "remembered" check was changed to that rule.
- **DP-D9 (TAP).** Shortcut **T** (free in the table; ours), in both editors. A tap counts on the
  press. One sequence is one undo step: the second tap commits, later taps amend that step
  (`History::amend`); tempo rounded to 0.01 BPM.
- **DP-D10 (BPM in the display).** Drag 0.25 BPM per px (Shift ×0.1), wheel ±1, double-click types
  in the rename field (a comma works as the decimal point); shown with two decimals.
- **DP-D11 (positions).** The display's POSITION and START are 1-based and unpadded ("17.2.3", the
  mockup), LENGTH 0-based ("2.0.0"); a value too wide for its cell (bar 100+) shrinks to fit.
- **DP-D12 (clock format retired).** View > Clock Shows Bars and Settings > Clock shows did nothing
  once the display shows both (principle 3); removed. The settings field stays so old files read
  the same.
- **DP-D13 (Play / Stop).** Play starts playback and does nothing while playing (the manual's
  restart behaviour is unverified, so not copied); Stop is today's Stop (again: to the start).
- **DP-D14 (MEM).** Private bytes from `K32GetProcessMemoryInfo` (documented), once a second; on
  macOS "--" until it's implemented there (out of scope). The CPU / MEM grid is a fixed 84 px so
  the three groups don't move as the numbers change. AT-02's pixel diff leaves the MEM read-out
  out (it is live, like a clock).
- **DP-D15 (spectrum).** The existing whole-mix spectrum (channel levels spread into bands), now
  neutral in a 216×56 well; styles in settings.json (`spectrum_style`). Its menu opens to the left
  near the window's edge.
- **DP-D16 (Preview key).** Neutral in both states (lime is listed for other places only); 28×16
  to fit the 16 px browser header without changing its layout; remembered (`preview_on_select`).
  ↑/↓ in the preset list move the selection and audition for 600 ms.
- **DP-D17 (lime removed).** Playhead, ruler marker, focus rings, caret and text selection, dial
  arcs, checked buttons, the rename field's border, drag-target outlines, the in-app menu's
  highlight, the split handle's grip, the snap-guide pill, the getting-started title, the dev
  overlay. The lime audit (`docs/screens/design-pass/lime-audit.md`): 0 violations over 9 scenes.
- **DP-D18 (dithering).** Every drawn gradient (`gfx::shaded` and each shader site) sets
  `Paint::dither`; the stylesheet's gradients are dithered by vizia fork commit 60907aad (pin
  updated in all nine `[patch.crates-io]` entries). Dev-only `CHIPDAW_NO_DITHER=1` turns the drawn
  ones off for the A/B.
- **DP-D19 (fork bug fixed on the way).** With the browser closed, the app panicked at start
  ("capacity overflow"): the fork's textbox accessibility code subtracted past zero on the hidden,
  empty search field. Same commit 60907aad (`saturating_sub`). The pre-run build has the same bug.
- **DP-D20 (minimum window).** 1272×920 logical (was 1140×812): the top bar's three groups need
  1269 px; the height keeps the rule "eight full lanes plus the inspector" at the new 73 px pitch.
  Open item: 920 doesn't fit a 1080p display at 125%.
- **DP-D21 (dark menus).** Built by a sub-agent on its own worktree and merged
  (docs/design/DARK-MENUS.md): owner-drawn popups, documented APIs only, live, default off. It
  added no undocumented call; muda's existing dark menu *bar* (WM_UAHDRAWMENU, undocumented,
  cosmetic, falls back to light) is unchanged. `windows-sys` became a direct, optional,
  Windows-only dependency: already in the tree at the same version (0.61.2), so no new crate.
- **DP-D22 (harnesses).** AT coordinates moved to the new layout (top bar 76, ruler 28, rows 73,
  headers 380); the piano-roll bench takes `-NoteY` (591 now, 535 before).
- **DP-D23 (owner, mid-run: clip gaps).** The mockup's 2 px inset at both ends of a clip (DP-D3)
  is reverted at the owner's request: a clip starts on its grid line and ends 1 px short, so
  abutting clips show a 1 px gap, as in the reference DAW. Top and bottom stay 2 px in from the
  row. The pixel comparison lists this as an intended deviation.

## Polish pass (POLISH_PROMPT.md), 2026-11-25

Branch `polish/option-d` from `design/option-d` 23afc3c. Report: [POLISH.md](../POLISH.md).
Every decision below was taken alone, as the prompt asked.

- **PL-D1 (regression gate baseline).** "Before" is `design/option-d` plus one commit of dev
  hooks only (5262ae9: `CHIPDAW_FROZEN_METERS` freezes every level, the spectrum, the strips and
  MEM; `CHIPDAW_PLAYHEAD` pins the playhead), built in `../before-polish`. Fixed states at
  1440 × 920 (`--size`), audio off. The diff compares the client area only (the window frame is
  the OS's) and leaves out the window's rounded bottom corners (Windows 11 shows what's behind
  them; two identical "before" runs differed only there). Threshold: > 2 levels.
- **PL-D2 (browser title).** "BROWSER" already had the inspector titles' class
  (`.section-title`): ink measured identical (11 px caps, `#9A9A9A`). The header row grows to
  24 px for the 24 px Preview key; the browser's top padding drops 12 → 4 so the search field
  and list don't move (the gate allows only the header to change).
- **PL-D3 (minimum window).** 1296 × 720: the top bar's groups need 1269 px; at 1272 they
  touched, so 12 px (their own gap) on each side. 1280 isn't possible without the groups
  touching. 720 keeps five lanes above the inspector. The start size stays 1440 × 920 (the
  design pass's effective size) so nothing else moves.
- **PL-D4 (palette and lime).** The prompt asks both for "today's lane colours stay" and "nothing
  near the toggle lime". Today's pulse colour `#C9D24A` (the approved mockup's) is ΔE 0.012 from
  lime. Existing projects win: it stays, as the one exception (unit-tested and audited as such);
  automatic picks never choose it (`auto_candidates` excludes it). Every other colour is ≥ 0.068
  from lime (threshold 0.06).
- **PL-D5 (palette grid order).** Columns are hues, rows lightness; the 12 legacy colours sit in
  their nearest cells, so two greys (Sand `#B5AC98`, Slate `#8C98A8`) sit in the orange and teal
  columns. Kept: moving them would change existing projects' colours.
- **PL-D6 (clip header text).** Dark `#111111` whenever it reaches 4.5:1 (all of today's colours
  keep it: pixel-identical), else light `#F2F2F2`. The header is flat (VISUAL-SPEC), so its
  lightest and darkest stops are one colour; hover and selection lighten it, so the palette was
  generated to keep ≥ 4.6:1 in every state (tested on all 70).
- **PL-D7 (colour grid).** Built into the existing context menu (`MenuItem::grid`): Colour ▸
  opens the 70 swatches in the menu's swatch material; hovering previews by changing the model
  without an undo step and puts the colours back when the menu closes without a pick; a click
  applies to every selected clip, or every selected track when the menu's track is one of them.
  There is no inspector swatch to wire.
- **PL-D8 (colour settings defaults).** The manual (Live 12 §2.3.2) gives no defaults. Ours:
  Auto-assign on; Default track colour `#7F86E0` (the triangle's periwinkle, neutral-cool and
  far from lime); Reduced off (on: the palette's middle row, 14 hues); **New clip colour: Track
  colour** (today's behaviour, so new clips look as they do now).
- **PL-D9 (picking).** `next_track_color` maximises the smallest OKLab ΔE to the neighbouring
  tracks and the last three picks, prefers unused colours, breaks ties by palette order
  (deterministic). Clips: the same against the clips before and after on the track and the
  track's colour.
- **PL-D10 (START / LENGTH per frame).** The display samples its text on the frame counter (once
  per presented frame, or the 50 ms idle tick) instead of six model bindings.
- **PL-D11 (dark menus on).** As DESIGNPASS.md §9: the default is on; an older settings file
  without the field gets on too; turning it off is kept.
- **PL-D12 (harness).** With dark menus on, an open popup can be the process's "main window":
  `shot.ps1` now picks the titled window (this had made AT-06/30 fail and, by moving the popup,
  AT-18). AT-26 clicks the colour grid. A new suite: PL-01…05.
- **PL-D13 (the drag benchmark measured a resize).** `bench.ps1` / `pacing.ps1` grab a clip at
  (720, 130). In the Option D layout (bars from x 621) that point is 7 px into a clip: its left
  resize zone. The "drag" resized a clip to 2–6 px, each new size a ~12 ms CPU shadow blur;
  DESIGNPASS.md's +1.3-point drag regression was this artifact. Both scripts now take
  `-DragX/-DragY`: 760, 140 grabs the same clip spot in the new layout.
- **PL-D14 (narrow shadows).** Per-size shadow masks for shapes long in one direction and narrow
  in the other (a clip being resized) round up to 4 px and stretch, so a resize reuses a few
  masks. Small fixed shapes (dial knobs) keep exact masks (the regression gate caught a 7-level
  change on the knobs' rims before this distinction).
- **PL-D15 (drag CPU).** The remaining drag cost was the renderer's passes: eight faders each
  dirtied their own small rect (the fork repaints up to six damage rects, each a tree pass).
  Rows now redraw with their meter, so the header column is one contiguous rect again; M/S keys
  blit a per-look image (rendered on a GPU surface of the window's kind, so it matches drawing
  in place); the static pan slider is drawn by its row. 200-clip drag, 5 interleaved rounds:
  11.1% (polish) vs 12.1% (pre-design `main`).
- **PL-D16 (regression hook for dials).** The dials ease towards their value and stop redrawing
  within 0.001 of it, so where they stop depends on frame timing. `CHIPDAW_FROZEN_METERS=1`
  (dev only, the regression screenshots) also turns that easing off; the same commit is in the
  baseline worktree (720f84a: `design/option-d` + dev hooks only).
- **PL-D17 (independent clip colours, owner's request mid-run).** Already in the model since format
  v4 (`Clip.color`, `None` = the track's): the clip menu's Colour ▸ sets it, "Track Colour" clears
  it, and a track colour change leaves clips with their own colour alone. PL-02 now checks it.
- **Merge.** Every gate passed: `pre-polish` tags the old `main` (23b5da9), pushed; `main`
  fast-forwarded to `polish/option-d` (48ee94b, which contains `design/option-d`) and pushed.

## Polish 2 (POLISH2_PROMPT.md), 2026-11-26

- **P2-D1 (pre-flight: the clip gap).** `main` was clean (only untracked prompt files and the
  reference folder). The 1 px gap between adjacent clips is **already committed**: 0020c1d
  "Design pass: caches for per-frame drawing, 1 px clip gaps, …" (RUNLOG DP-D23;
  `views/arrangement.rs`: a clip starts on its grid line and ends 1 px short). Nothing to commit
  on `main`, no wip branch.
- **P2-D2 (pre-flight: clip corner radius).** `theme::tokens::RADIUS_CLIP = 4.0` (logical px),
  used by `views/clip.rs`. Kept.
- **P2-D3 (baseline).** The regression baseline is `main` at 8644ff3, built in the worktree
  `../before-polish2` (it already has the polish pass's dev hooks: frozen meters, pinned
  playhead). The reference pack is copied to `docs/design/polish2-ref/`; its element boxes are
  measured with `docs/polish2/tools/measure_ref.py` (headless Chrome) into `docs/polish2/boxes/`.
- **P2-D4 (track volume is now a real dB gain).** The design pass's header fader moved the
  chip's 4-bit Volume register (0–15; the triangle and DPCM had none): capped at unity and in
  coarse steps, so the §3 scale (−inf … +6 dB) couldn't be built on it. Each track now has a
  volume in dB (`Lane.vol_db`), a gain applied after the chip: `chip::mixer::mix_gained`
  evaluates the 2A03's non-linear groups on the real DAC levels and scales each channel's share
  of its group's output, so a lone channel at +6 dB is exactly ×2 (≤ 2.0). While every gain is
  1.0 the synth runs the old `mix` unchanged (`Synth::set_gains`), so the golden render and every
  migration hash are bit-identical (tested). The engine gets `Command::LaneGains`; the render,
  export and game-export players set the same gains. The register stays the inspector's Volume
  dial; every track (triangle and DPCM too) now has a working fader, so their disabled reasons
  are gone. Wheel: 1 dB per notch. Typed values clamp to −70 … +6 dB; "-inf" is silence.
  Positive values show without a sign ("2.5 dB", as negatives show "−2.2 dB").
- **P2-D5 (format v6).** Row heights, pan and volume are saved per track, so the format goes to
  **version 6** (a v5 build reading v6 fields would have failed on its strict field check
  instead of saying "newer version"). Defaults are written as nothing, v5 files open at
  72 px / C / 0 dB. A v5 corpus (`docs/songs/v5`, the v4 corpus saved by `main` 8644ff3, hashes
  in its HASHES.txt) renders bit-identically (`version_5_files_render_bit_identically_in_version_6`).
- **P2-D6 (pan).** Pan is −50 … +50 (shown 50L … C … 50R; the manual gives no format, P2-R
  "unverified"), editable and saved, not heard until stereo mixing (Batch 4); its hover help
  says so. A render with only pan changed is bit-identical (tested).
- **P2-D7 (the boards' HTML over their PNGs).** Positions come from the HTML boxes. The 1× PNGs
  disagree with their own HTML by 1 px in places (E1-header: M/S at y 9 and the pan well at 59 in
  the PNG; y 10 and 58 in the HTML); the code follows the HTML. The boards draw the M / S letter
  at the key's top-left (a static-HTML button artifact); §3 says the keys are unchanged, so the
  glyph stays centred.
- **P2-D8 (the fader's scale between boards).** E1-header's thumbs don't follow §3's scale
  (−2.2 dB at 5 px, −1.3 dB at 2 px); E1-row-heights does (`round((1 − (dB + 48)/54) · length)`
  for the thumb's centre: −1.3 dB → 15 on a 112 px well, −3.1 → 35 on 208). The code follows the
  formula; its meter stops are §3's 0/45/70/85/89 % (E1-header used px stops).
- **P2-D9 (row heights).** A row's height excludes its hairline (72 = today's 72 + 1). The resize
  grab is the row's bottom 4 px (ns-resize); Alt while dragging sets every track (P2-R20); Alt+wheel
  is 12 px (three 4 px steps) per notch. Heights are part of the document (undo, the unsaved mark).
- **P2-D10 (the header column draws its hairline).** The column paints `#0A0A0A` and the hairline
  on the lanes' side (the old 1 px divider element is gone), so a strip by the clips covers it in
  its row as on the board; the default layout is pixel-identical (regression gate). The width grab
  is 4 px inside the column; the width is saved when the drag ends.
- **P2-D11 (ruler click vs drag, §7.1).** Today a press on the ruler scrubbed while dragging.
  Resolution: a press that moves < 3 px is a click and places the playhead as before; a drag
  navigates (vertical zooms, 1.2 %/px exponential, down = in, anchored at the grabbed bar;
  horizontal scrolls with the bar following the mouse; P2-R7). **Shift+drag scrubs** (the manual's
  Shift-scrub, P2-R10). The loop brace and the double-click (fit) are unchanged.
- **P2-D12 (keys).** W was free: **Fit the Whole Song** (the manual's Optimize Arrangement Width,
  P2-R13; pressed again it goes back, as the manual's toggle). Ctrl+Alt+O / Cmd+Option+O was free:
  **View ▸ Show Overview** (P2-R6). Zoom In / Out (+ / −) existed (B3). Both are in the one
  shortcut table, so the menus and the shortcut sheet list them.
- **P2-D13 (the Overview).** Its extent is the song (the last clip or the loop end, at least four
  bars; the label's "24 BARS"). A press outside the outline jumps there (the view centred on it)
  and keeps dragging from there; inside, it drags. The manual doesn't say what a single click does
  (P2-R5), so the jump is the CEO's spec. Up to 14 tracks get their own row, then they share.
- **P2-D14 (status bar).** Messages stay 8 s (was 6). A click on a message shows it whole (an
  import report no longer fits a one-line bar). Problems: no device, can't open, reconnecting,
  dropouts (cleared 10 s after the last underrun). The warning icon is amber, not lime (not an
  "on" state). "Hardware profile over budget" can't happen yet (Batch 4). The recovery offer stays
  in its capsule (a decision). With the status bar hidden (dev hook), messages fall back to the
  capsule.
- **P2-D15 (hover help scope).** Converted to the status bar: the header controls, transport,
  view keys, display, TAP, ruler, Overview, spectrum, CPU/MEM (§6.5's list). The inspector's,
  browser's and dialogs' tooltips stay tooltips (outside the list; in a dialog the bar is under
  the backdrop); a track row's full-name tooltip stays (content, not help).
- **P2-D16 (the earlier suites).** AT-01…31, DP-01…05 and PL-01…05 click fixed coordinates (rows
  from y 104, lanes down to the window's bottom). They run with the dev hooks
  `CHIPDAW_OVERVIEW=0` and `CHIPDAW_HIDE_STATUS=1` (`docs/polish2/tools/run_old_suites.ps1`); the
  new layout itself is covered by P2-01…11. DP-04 now reads the reasons from the status bar's hover
  help, and the triangle's fader works (P2-D4); DP-05 accepts format ≥ 5.
- **P2-D17 (dev hooks).** `CHIPDAW_HIDE_STATUS=1` (the regression gate), `CHIPDAW_OVERVIEW=0`
  (earlier suites), `CHIPDAW_FAKE_PROBLEM=text` (the status bar's screenshot and P2-10). All
  dev-mode only (`CHIPDAW_DEV=1`), none is a user setting.
- **P2-D18 (harness fixes in the earlier suites).** AT-02 waited 7 s for "Saved" to fade
  (now 8 s): 9 s, and it parks the mouse first (it runs without dev mode, so the Overview shows
  and a track row's tooltip opened under the idle mouse). AT-25 clicks the S keys at their E1
  place (header x 288…312, y 10…34). AT-09 failed once in the full run and passed alone and on
  every later run (a timing flake in the piano roll, which this pass didn't touch).
- **P2-D19 (the header keeps its 1 px divider view).** The first build drew the hairline inside a
  380 px column; at 150% the old divider view rounds to 2 device px, so the lanes started 1 px
  earlier and the regression gate caught every grid line moving. The column is again 379 px plus
  the original `.col-divider` view on the lanes' side, so every scale rounds as before (gate passes
  at 100% and 150%). Consequence: where the board's strip by the clips covers the hairline, ours
  stops 1 px short of the lanes, at the divider (within gate 2's 1 px).
- **P2-D20 (what the new gestures are measured against).** The baseline has no row resize,
  header-width drag, ruler zoom or Overview. Each is compared with the baseline's own gesture
  that does the same kind of work (`docs/polish2/tools/bench_p2.ps1` runs these on both builds):
  ruler zoom, Overview zoom and row resize redraw the whole lane layer every frame, like the
  baseline's **Ctrl+wheel zoom**; the Overview drag scrolls, like the baseline's **time-scrollbar
  drag**; the header-width drag changes the lanes' width every frame, like a **window resize**.
  The POLISH.md scenarios (idle, demo, 200 clips playing, 200-clip drag) are compared directly;
  where 3 interleaved rounds were within noise of the +0.3 budget, a dedicated 5-round A/B
  decided (the polish pass's method).
- **P2-D21 (performance fixes the gate drove).** (1) The OVERVIEW label cell redrew on every
  arrangement change (a separate damage rect, so a renderer pass, per scroll frame): now only when
  the song's length changes. (2) Each fader's height was bound with `map` to the whole
  arrangement, so every scroll re-applied eight heights (eight relayouts and damage rects): now a
  memo. Scrollbar drag A/B: +0.76 → +0.04. (3) The Overview's clip layer waits for the drop
  while clips are dragged, and the Overview redraws only when what it shows changes (not on a
  note edit): piano-roll drag A/B +0.64 → +0.08. (4) The lane layer's surface keeps 256 px of
  slack, so a width drag doesn't reallocate it every frame.
- **Merge.** Every gate passed on the final build: `pre-polish2` tags the old `main` (8644ff3),
  pushed; `main` fast-forwarded to `polish2/header-nav` (e09139c) and pushed.

## Mac catch-up (MAC_CATCHUP_PROMPT.md), 2026-11-26

Mac mini M1, macOS 26.6.2, branch `mac/catchup` from `main` ef2ea19 (Polish 2 merged). Report:
[MAC-CATCHUP.md](../MAC-CATCHUP.md).

- **MC-D1 (start).** `main` fast-forwarded from 24c9d9c to ef2ea19 (74 commits, no conflicts); the
  three untracked prompt files stay untracked. The vizia pin 60907aad resolves from GitHub
  (`Cargo.lock` source line; the release build fetched and compiled it). A baseline build of
  `main` lives in the worktree `../before-mac-catchup` for every before/after in this pass.
- **MC-D2 (audio device).** No USB interface is connected (the MOTU is absent; `coreaudio list`
  and `system_profiler SPAudioDataType` agree). The system default output is a Bluetooth
  speaker (Sound Blaster GS3, 44.1 kHz only). The prompt wants the **built-in output**, and
  changing the system default needs asking, so every run names the device in its own
  `settings.json` (`"audio_device": "Mac mini Speakers"`, Settings ▸ Audio's own field) and the
  system default is never touched.
- **MC-D3 (menu key equivalents vs. F-03 and dialogs; macOS only).** On macOS a menu key
  equivalent (⌘Z, ⌘A, ⌘C, ⌘V, ⌘D, …) is consumed by the menu bar before the window sees the
  key, and `run_menu_cmd` applied none of the key path's context rules. Measured with real key
  events (`docs/mac-catchup/tools/check_text_menu.sh`): typing in the browser search, ⌘Z undid
  the song's last edit, ⌘A selected every clip, ⌘C copied clips, and the field got none of them;
  with Settings open, ⌘Z / ⌘D reached the song. Windows has no accelerator table (its keys go
  through the context-gated path), so it can't happen there. Fix (`#[cfg(target_os = "macos")]`
  `mac_menu_gate`): while a text field is being edited, Cut / Copy / Paste / Select All go to the
  field (the macOS Edit-menu convention); while a field is edited or a dialog is open, editor
  commands do nothing; File, app and Help commands (New, Open, Save, Save As, Export, Settings,
  Project Settings, Quit, About, the shortcut sheet) still work, as in any Mac app (⌘Q quits
  while a field has focus). Undo / Redo in a field do nothing, as on Windows (AT-03).
- **MC-D4 (no app bundle: the menu bar says "app").** The release binary runs unbundled, so
  macOS names the app after the process: the bold menu title reads **"app"**, the save sheet and
  About panel show a generic folder icon, and Finder can't open `.chipdaw` files with it (no
  `CFBundleDocumentTypes`). Renaming the binary would change every platform's build and every
  harness; the real fix is an `.app` bundle (Info.plist: `CFBundleName` ChipDAW, the icon, the
  `.chipdaw` document type, `CFBundleIdentifier`), which belongs with code signing and
  notarisation (a release item, out of scope). Logged as a gap with that proposal.
- **MC-D5 (About on macOS).** The App menu's "About ChipDAW" was muda's predefined item, i.e.
  the system About panel. Unbundled, it showed only a folder icon and "app": no version, commit,
  licence or font credits (the OFL notice), and Help has no About on macOS, so that content was
  unreachable on the Mac. The App menu now opens our own About (the Windows Help ▸ About
  dialog), and "Hide app" reads "Hide ChipDAW". Only `spec(mac = true)` changes, which Windows
  never builds a menu from; `menus::tests::no_placeholders` checks the App menu.
- **MC-D2, update (04:46).** The MOTU M2 was plugged in during the run (not at the start): it
  appeared in `coreaudio list` and `system_profiler`, and macOS made it the default output by
  itself. The built-in output stays the main matrix device (named in each run's settings); the
  M2 gets the matrix too (§5 of the prompt), and the device switch tests use it.
- **MC-D6 (closing the last window quits).** ChipDAW is a single-window app, so closing the
  window quits it, with the same save sheet as ⌘Q (Save / Don't Save / Cancel); Don't Save keeps
  the recovery file. This is today's behaviour, kept: a Mac app that stays running with no
  window would need a way to reopen one (File ▸ New Window, the Dock icon), which is new work.
- **MC-D7 (Dark menus on macOS).** The setting was already hidden outside Windows
  (`cfg!(all(windows, feature = "menus"))`). Settings ▸ Display now says, on macOS only, that menus
  follow the system's appearance. No API is called.
- **MC-D8 (the macOS title bar is 32 pt).** The harness's content coordinates use the measured
  title bar (twice the close button's centre below the window top = 32 pt on macOS 26), not the
  28 pt assumed by `chipbench`; the content layout then matches Windows' client coordinates
  exactly (top bar 0–75, Overview 76–107, ruler 108–135, rows from 136).
- **MC-D9 (Return in a dialog with one clip selected: shared, not fixed).** With the Export
  dialog open and one clip selected, Return reaches the arrangement first (Open Clip, which
  consumes it), so the piano roll opens behind the dialog and the export doesn't start. The key
  path is the same on Windows (vizia delivers keys to the focused view before the root), so it
  isn't a macOS bug; fixing it would change shared code and hold the merge. Logged in
  MAC-CATCHUP.md with the proposed fix (the arrangement and piano-roll key handlers ignore keys
  while a dialog is open). The harness records it as a NOTE and exports with nothing selected.
- **MC-D10 (MEM on macOS).** The top bar's MEM read-out showed "--" on macOS (DESIGNPASS.md §8
  open item: `perf::process_memory` had no macOS branch), so a shipped read-out never worked on
  the Mac. Now `proc_pid_rusage(RUSAGE_INFO_V2).ri_phys_footprint` (libSystem, no new crate),
  `#[cfg(target_os = "macos")]`: the process's own memory, as Activity Monitor and `footprint`
  count it, the counterpart of Windows' private bytes. Three sources agree: the app 94 MB, `top
  -stats mem` 95M, `footprint` 95 MB (one launch, sampled seconds apart).
- **MC-D11 (no window tabs on macOS).** AppKit's automatic window tabbing added View ▸ Show Tab
  Bar and Show All Tabs; Show Tab Bar put a 36 pt tab strip over a one-window app and pushed the
  whole UI down (evidence/tabbar-before.png). The standalone binary now sets
  `NSWindow.allowsAutomaticWindowTabbing = NO` before the window exists, through the
  Objective-C runtime (libobjc; no crate), `#[cfg(target_os = "macos")]` in `crates/app` (a
  plugin's host owns its window, so the library is untouched). After: the View menu ends with
  Draw Mode and Enter Full Screen; the Window menu is unchanged.
- **MC-D12 (2× backing).** Neither display is Retina. For the mockup match at 2× the second
  display was switched for the session to its HiDPI mode (1280 × 720 pt, 2× backing, 60 Hz,
  ioModeID 7) by `docs/mac-catchup/tools/hidpi_mac.py`, which restores ioModeID 18 in a
  `finally` and reads it back (it did: 18). The window (1440 × 920 requested) was held to the
  display at 1440 × 752 pt with its minimum content height; the header boards fit in it.
- **MC-D13 (the dB scale labels at 2×).** E1-row-heights at 2×: the four scale labels of each
  tall row are 1.5 logical px from the @2x board; everything else on every board is within
  1 px. Measured (results/mockup-match.md): the app moves 0.5 px between 1× and 2× (baseline
  rounding), the reference's @2x PNG moves its labels 0.5–1 px up against its own @1x PNG; against
  the HTML box (the spec, P2-D7) the app is within 0.5 px at both scales. Not changed.
- **MC-D14 (⌥ + trackpad swipe over a track; macOS only).** No trackpad here, so trackpad input
  was synthesized (continuous pixel scroll events, 2 px each). ⌥+wheel row height stepped 12 px
  per event whatever its size: a 20-event, 40 px swipe took track 2 from 72 to **300** px (the
  maximum). The fader and pan wheels already accumulate (`input::WheelSteps`, PERF-macos.md §2);
  the row height now does too, `#[cfg(target_os = "macos")]`: the same swipe gives **96** px (two
  lines, two steps), and one mouse notch is still exactly 12 px (`--only mouse`: 84). Windows keeps
  its code (precision touchpads there would benefit from the same accumulator: proposed, not made,
  as it would change shared behaviour). A physical trackpad is still untested (open item).
- **MC-D15 (colour preview: two shared bugs, not fixed).** In the Colour ▸ grid (POLISH.md C2):
  (1) hovering a cell and closing the menu with Esc leaves the previewed colour on the track,
  and it is saved without the unsaved mark: `Action::Escape` clears `s.menu` directly, while the
  `CloseMenu` path calls `restore_colours()` first. Proposed: the Esc branch calls
  `restore_colours()` too (one line). (2) Hovering cell A and then clicking cell B makes ⌘Z restore
  A, not the original colour; going straight to B undoes correctly. The preview must be reaching
  the undo base before the pick; proposed: restore the preview before the pick's commit, and a
  unit test of hover-A/pick-B/undo. Both paths have no platform code, so they are Windows bugs
  too; fixing them would hold the merge. In the harness as NOTE lines (`--only colors`).
- **MC-D16 (the 60 fps cap broke during clip drags: fork commit 14).** The 200-clip drag presented
  **122–126 fps at 32–33% CPU** under the 60 fps cap on the 320 Hz display; PERF-macos.md's build
  (24c9d9c) drags at 60.1 fps, 18.7%. `git bisect` (results/drag_bisect.txt) found 281ee66 (DPCM
  import): the browser binds `s.arrangement` to `Handle::modify` (refresh the sample list), and
  vizia's `modify` emits `WindowEvent::Redraw`, which the winit window answered with
  `request_redraw()` at once, outside the frame limiter's slots. A clip drag changes the
  arrangement on every mouse event (125 Hz), so every event presented a frame. Removing the binding
  (experiment) restored 59.8 fps, which confirmed the path; the fix belongs in the limiter, not
  the binding, because any `modify` binding on a fast signal does the same. **Fork commit 14**
  (9110e982, `chipdaw-perf`, pin updated): on macOS the event sets a pending flag that
  `about_to_wait` requests within the slots. After: **59.8 fps, 20.1%, 538 mW** (was 121.8 fps,
  32.2%, 874 mW); demo and 200 clips unchanged (60.0 fps, the same instructions per frame).
  `#[cfg(target_os = "macos")]` in the fork: Windows keeps the direct request (its swap paces at
  the display rate; on a 144 Hz Windows display the same bypass would show, proposed for the
  Windows pass). Windows never saw this: its 60 Hz swap hides it.
- **MC-D17 (idle +0.25 point from MEM).** Idle reads 1.0% / 10.5 mW against main's 0.8% / 5 mW. With
  `CHIPDAW_FROZEN_METERS=1` (MEM pinned) the two builds are equal (0.81%, 5 mW, 21 wakeups/s); with
  the MEM fix on the old fork it is 1.07% / 11 mW. Cause: MEM now changes about once a second; the
  top bar redraws, and any presented frame keeps the UI pump fast for 250 ms (`adapt_pump`,
  `STAY_FAST`): +25 wakeups/s. Windows has always paid this (MEM was live there). Under the 1-point
  rule; kept.
- **MC-D18 (cursor reloads in drags: fork commit 15).** The Overview drag cost **42.3%** of a core
  (47 M instructions/frame) where Windows measured 8.8%. Time Profiler: 16.3% of a core in the
  winit/AppKit event loop, with `__open` and `__getattrlist` at the top: the Overview sets
  `CursorIcon::AllScroll` on every mouse move, and winit builds a new NSCursor for every
  `set_cursor`, reading the WebKit move cursor's PDF and plist from disk each time. **Fork commit 15**
  (2a146e6b, pin updated), macOS only: the window skips a `SetCursor` that repeats the cursor it
  already has (winit keeps it and re-applies it through cursor rects). After: Overview drag
  **27.0%**, Overview zoom 41.7 → **26.1%**; scrollbar, ruler, row-resize, header-width and clip
  drags unchanged (results/cursor_fix.jsonl). The Overview now costs what the time scrollbar does,
  as on Windows (POLISH2.md §4.6).
- **MC-D19 (full-lane repaints cost ~2.8× the demo on the Mac; not fixed).** Time-scrollbar drag,
  ruler zoom and Overview gestures repaint the whole lane area each frame: 26–28% of a core, 36 M
  instructions per frame, against 16.8% / 18.5 M for the PERF-macos build's scrollbar drag (older,
  lighter lanes) and 9.2% on Windows (1.2× its demo). Every draw pays the GL-on-Metal translation on
  the main thread (PERF-macos.md §7). The fix is in the renderer (Skia's Metal backend, or scrolling a
  cached lane layer by a blit): an architecture change, so per the stop condition it is proposed,
  not made. The scrollbar's overlay thumb makes that benchmark bimodal (14.9% when the press lands
  before the thumb appears and starts a time selection; 27–28% when it scrolls).
- **MC-D20 (display arrangement restored).** Switching the second display's mode back after the 2×
  session (MC-D12) restored the mode but **not its place**: macOS had moved it from above the main
  display (CG origin 0, −1440) to its left (−2560, −720). Found when windows moved to y −1312 were
  clamped onto the main display. Restored with `CGConfigureDisplayOrigin` (`dispmode origin 1 0
  -1440`, for the session; read back: NSScreen frame (0, 1440), as at the start). `hidpi_mac.py` now
  saves and restores the origin too. Runs between the two (the first "60 Hz" perf matrix and pacing
  run) actually ran on the main display; they were discarded (`perf_60_INVALID_on_320.jsonl`) and
  repeated. **The owner may want to glance at System Settings ▸ Displays ▸ Arrange.**
- **MC-D21 (pacing gate on the 60 Hz display: not met, architecture).** Today's build, 60 fps cap,
  demo, 120 s on the 59.953 Hz display: 7161 frames held one vblank, **10 held longer** (2 × 2,
  8 × 3 vblanks), the same isolated holds PERF-macos.md §11 left after the stopgap (6–9 per 120 s,
  cause not found). By the Windows definition (a gap over 1.5 vblanks) they are stalls, so gate 6
  fails on that display. The 320 Hz display passes on the app side (0 frames > 20 ms in demo and
  drag; ideal 5/5/6 cadence). The proposed fix is vblank-paced frames from the display link (with
  Skia's Metal backend: PERF-macos.md §6–7, ENGINE-macos.md §8): a renderer/backend change, which
  the stop condition rules out here. Merge held on this gate (§1.5 and §8 of the prompt).
- **MC-D9, update.** ⌫ is affected too: with Export open and a clip selected, ⌫ deleted the clip
  behind the dialog (4 → 3 clips in the saved file; undoable). Same shared path, same proposed fix.
- **MC-D22 (audio matrix lengths and devices).** Built-in output (the gate): ENGINE.md's A, B, C,
  D, G at 64 / 128 / 256 / 512 frames, A and B at **300 s** (ENGINE.md: 600) to fit the session, C,
  D, G at 120 s. The M2: A, C, G at 120 s. Test E (changing the system default output) is replaced
  by an in-app switch built-in ↔ M2 through Settings ▸ Audio while playing, which needs no system
  change (§0 asks before changing the default device). Ground truth as ENGINE-macos.md §3: CoreAudio
  overloads and the late detector (no loopback cable; tapmon kept as "delivered-stream" evidence).
- **MC-D23 (end of the pass).** Final build: `mac/catchup` with fork 15 (2a146e6b). Tests 207 + 1
  ignored, clippy clean, 0 warnings; scenario harness 84 pass, 0 fail, 3 notes (shared bugs).
  Gates: 6 of 7; gate 6 fails on the 60 Hz display (MC-D21), so **`main` is not merged and no tag
  was made** (§1.5). Every change is macOS-gated, docs, tools or tests: no Windows gate run is needed
  for this branch. Local worktrees left for the next pass: `../before-mac-catchup` (main ef2ea19),
  `../perf-macos-base` (24c9d9c), `../bisect`. The thermal logger ran all session: nominal throughout
  (`docs/mac-catchup/results/thermal.log`).
- **MC-D24 (merge, gate 6 accepted).** The owner accepted gate 6 (the 60 Hz display's isolated
  2–3-vblank holds, MC-D21) as is and asked for the §9 merge. `pre-mac-catchup` tags the old `main`
  (ef2ea19) and is pushed; `main` fast-forwarded to 18b5860 (`--ff-only`) and pushed. `main` is
  checked out in `../before-mac-catchup` (clean), so the fast-forward ran in that worktree rather
  than via `git checkout main`. The 60 Hz holds remain open for the display-link work.

## Batch 4a (BATCH4A_PROMPT.md v2, branch `batch4a/model`, Windows PC)

- **B4A-D1 (pre-flight).** `main` was 19 commits behind `origin/main`; fast-forwarded to 0e669ac.
  **`mac/catchup` is merged** (0e669ac = MC-D24, its last commit; `git log main..origin/mac/catchup`
  is empty). Baseline: `main` 0e669ac, worktree `../before-batch4a`, release built. Branch
  `batch4a/model` from it. **The CEO's `B4a-main-track.*` reference folder was not handed over**:
  not in the repo, `polish2-ref/`, Documents, Downloads or Desktop. The Main track is built from the
  prompt's written spec (§7: colours, gradient, divider, layout rules); the mockup-match gate (§9.3)
  is measured against the prompt's numbers and reported as such, and `docs/design/batch4a-ref/`
  holds a note saying what is missing.
- **B4A-D2 (fixup 1: keys behind dialogs).** `Signals::dialog_open()` (a modal or the Export
  dialog) is the root's `Ctx::Modal` rule; the arrangement's `KeyDown` and the piano roll's
  `on_key` return early on it. Test **B4A-01** (`docs/batch4a/tools/at_b4a.ps1`): a clip
  selected, Export open, Backspace and Delete; Settings open, Return, Backspace, Delete: the saved
  file keeps all 4 clips and the piano roll never opens. On the baseline build the same test fails
  (clip 1 deleted twice over), so it tests what it claims. Return in the Export dialog is its
  Export key (E2) and is not pressed by the test.
- **B4A-D3 (fixup 2: colour previews; the cause).** Reproduced on Windows first (B4A-02, before
  the fix: "hover A, pick B, undo" gave A, `#4DC6FB`). The cause wasn't the pick: the app commits
  **every pump tick** (`idle_housekeeping` → `commit()`), so a hovered colour was recorded as an
  undo step while it showed (1,752 commits in one 20 s run, traced with a backtrace). Fix: the
  preview is a small pure `edit::ColourPreview` (preview / restore / pick); `commit()` treats an
  active preview as busy, like a gesture; a pick restores the originals first, then applies and
  commits once; Esc (`Action::Escape`) restores like `CloseMenu`. The Esc case already passed on
  Windows (the hover-time commit was later overwritten by the restore), but it left two stray
  undo steps; now none. Unit tests `edit::colour_preview_tests` (Esc after hover; hover A / pick B
  / undo for a track and a clip, exactly one step) and B4A-02 pass.
- **B4A-D4 (fixup 3: "▸" isn't in Inter).** `docs/batch4a/tools/cmap_check.py` reads the bundled
  fonts' `cmap` tables (no fontTools on this PC): every Inter face lacks **U+25B8 ▸** and
  **U+1F512 🔒**; it has ▶ › > → − …; JetBrains Mono has ▸ ([`docs/batch4a/cmap.txt`](batch4a/cmap.txt)).
  The three status messages (no device, device won't open, dropouts) now write "Settings > Audio"
  as the clip panel does. `tests/glyphs.rs` fails on either character inside a string literal in
  `crates/ui/src`. Consequence for §5: the lock badge's 🔒 is **drawn** (a procedural glyph), not
  typed.
- **B4A-D5 (fixup 4: selection modifiers, R28).** The manual (Live 12, §42.5 Editing; §42.9
  Zooming, Display and Selections; docs/ux-audit/manual-refs.md R28): Ctrl/Cmd-click adds items one
  at a time, Shift-click selects a contiguous range. Copied: **Ctrl-click (⌘-click) toggles** one
  clip (added on press, removed on release when the press didn't become a drag); **Shift-click
  selects the range** from the last selected clip to the clicked one, every clip on the tracks
  between that overlaps the time between (ours: what "contiguous" means in a two-dimensional
  arrangement; the manual doesn't say). Shift-click used to toggle. On Windows Ctrl is also the
  copy-drag modifier (A6): the copies are now made when the drag starts moving, so a Ctrl-click
  alone no longer stacks invisible copies on the originals (it did: copies were made on press).
  The earlier suites' Shift-clicks select two neighbouring clips, the same with either rule.
  Tests: `selection_tests::shift_click_selects_the_range`, B4A-03 (toggle on, toggle off, range,
  Ctrl-drag still copies).
- **B4A-D6 (the engine: one emulator, two mixes).** Locked tracks drive the existing console
  (`chip::Player`: the full 2A03 + VRC6 emulator, its non-linear mixer, filter and frame-tick
  driver), unchanged, so they are bit-identical by construction. Studio voices (`chip::voice`) are
  the same channel units (`Pulse`, `Triangle`, `Noise`, `Dmc`, the VRC6 pulse and saw) driven by
  the same driver code: `Driver::tick` was split into `tick_init` + `tick_lane` (identical writes,
  identical hashes) and a voice runs `tick_lane` for its one lane. Each voice has its own frame
  counter. **Mixing decision:** each voice through its own channel's DAC curve *alone*
  (`pulse_out(v, 0)`, `tnd_out(t, 0, 0)` …, VRC6 linear and inverted), summed linearly into one
  shared studio step buffer (`Blep::add`, new: order-independent steps) and one output filter, times
  the track's gain. A lone studio pulse note measures within 0.1 dB of the same note on the console
  (`voice::tests::a_lone_voice_matches_the_consoles_pulse`); BATCH4A.md has the comparison. The
  rack (`engine::studio`) is built on the UI thread, sent with `Command::Rack`, adopted on the audio
  thread by swapping or copying the voices (plain data), and the old rack goes back through the
  garbage queue (`Garbage::Rack`). `engine::Mixdown` = console + studio bus + Main gain is used by
  the engine, `render`, `export` and the game export alike; with no studio tracks and Main at 0 dB
  it is `Player::render` exactly (test `no_studio_tracks_is_the_console_bit_for_bit`).
- **B4A-D7 (pool, allocator, stealing).** Pool size = the most notes a studio track's clips sound
  at once ×1.5 + 4, at least 8, in steps of 8, capped by the Voices setting (`pool_size`). At each
  frame tick a voice keeps its note while it sounds; other sounding notes take free voices; a note
  **that has just started** steals the quietest voice not given a note this tick (release tails
  first, then the lowest DAC level, then the oldest). A note that lost its voice doesn't steal it
  back (the first version ping-ponged: 21 steals for one). Tests: chords on 8 voices, stealing the
  volume-4 note of five on four voices, the Voices cap, growth keeping the sounding voices, all in
  `engine::mixdown::tests`; `core::studio_rack_plays_and_grows_without_allocating` runs it inside
  `process` under the allocation checker.
- **B4A-D8 (migration to v7; three decisions).** Every pre-v7 file opens as eight tracks, each
  locked to its channel (`legacy_tracks`). (1) **Profile:** the old chip; a 2A03 + VRC6 file whose
  VRC6 lanes have no clips becomes **NES** (the prompt's rule). Its "auto" filter was the Famicom
  stage, so the filter is kept as an explicit choice: bit-identical, and the user can change it.
  (2) **Empty VRC6 lanes under NES** become studio tracks (names and colours kept): a lock onto a
  channel the profile lacks would show as a problem in every such project. A 2A03 file *with* VRC6
  clips keeps them locked to the VRC6 channels: silent as before, and a status-bar problem. (3) A v6
  track whose **fader wasn't at 0 dB** is locked like the others, so it plays at the chip's level;
  the old value is kept for unlocking and the load report names the track (`a_v6_fader_is_reported_
  and_restored_on_unlock`). Pan isn't heard yet (4b): it is kept for unlocking, without a report.
  The **v6 corpus** (`docs/songs/v6`, written and hashed by the baseline build with
  `v6corpus.rs.txt`): the v5 corpus plus rows/pan/mute, an unused VRC6, 2A03 with VRC6 clips, and
  the −3 dB case. Every hash v2…v6 → v7 and the golden render are unchanged.
- **B4A-D9 (File > New).** VISION.md: studio tracks are the default and authenticity is opt-in, so
  a new project has **no hardware profile** and four studio tracks (Pulse 1, Pulse 2, Triangle 1,
  Noise 1) on the default band's presets and colours; the output filter follows the profile (None:
  Off, UX.md §3). The legacy eight locked lanes remain as `Project::new_legacy()` for the demo
  songs, tools and tests that build a song lane by lane.
- **B4A-D10 (the model in the UI).** A track keeps its id, voice type, lock, Voices setting, the
  fader/pan from before locking and its own channel controls (they moved off `Project.params`);
  the instrument table's per-track defaults are a Vec kept in step with the tracks; mute and solo
  are sets of track ids (they follow a track that moves). Studio clips use a polyphonic note rule
  (a pitch never overlaps itself: `notes::resolve_poly`), set for the length of an edit with
  `notes::rule_scope` so the existing note operations keep their signatures. Identical instruments
  share one engine bank entry, and the table holds 128 (was 32). Stems are per track.
- **B4A-D11 (parameter registry, studio macros).** `ui::params` lists every automatable
  parameter per track kind (studio and locked) and for Main, with id, name, unit, range, default
  and rate class; `docs/batch4a/parameters.md` is generated from it. **Studio macros and channel
  controls are frame-tick**: a studio voice runs the driver's own code, which reads them once per
  frame tick, so automating them sounds like the chip's macros; only a studio track's volume and
  pan and the Main gain are smooth. Locked tracks have no volume/pan parameters (pinned, §5.3).
- **B4A-D12 (a Create menu, not a Track menu).** The prompt asks for "a Track menu"; VISION.md's
  shell lists File, Edit, Create, View, Help with "Create: Track". VISION.md wins: **Create ▸
  Insert Track** (Ctrl+Shift+T / ⇧⌘T, Live's Insert MIDI Track, R84: a studio track of the current
  track's type below it), **Create ▸ Insert Track ▸** Pulse … Saw, Duplicate Track, Delete Track;
  the same in the header's context menu (T3) plus Lock ▸ and Voices ▸. Delete with a track
  selected and no clip deletes it (R86); Ctrl+D likewise duplicates it (R87, generic Duplicate;
  unverified for tracks in the manual). Rename stays Ctrl+R (T2). A copy of a locked track is a
  studio track (its channel is taken). The last track can't be deleted.
- **B4A-D13 (profile warnings: where and how).** The dialog is the native message box the app
  already uses for conflicts; on Windows a task dialog, whose verification box is **Don't ask
  again** (TaskDialogIndirect from the windows-sys already in the tree: no new crate); the answer
  is remembered in `settings.silenced` as `id=choice` and Settings ▸ Warnings resets it. Note edits
  on every track use the polyphonic rule; **`commit()` checks locked tracks for new conflicts**
  (chords/overlaps, notes the channel can't play, DPCM rows without a sample) before recording the
  step: Unlock track / Fit to hardware / Cancel (Cancel puts the last undo step back). One check
  catches the piano roll, pastes, clip moves and imports alike. Conflicts already in the document
  (an old file) never ask; an over-budget project is a status-bar problem and the badge shows a
  warning mark. **Unlock is offered only where it fixes the conflict** (chords): a studio voice
  has the same channel range and DPCM rows, so for range and DPCM only Fit and Cancel are offered
  (nothing pretends). Locking a studio track with chords asks Fit-and-lock / Cancel. Switching to
  a profile without a channel in use lists the tracks: Unlock tracks / Switch anyway (they stay
  locked, dimmed and silent: today's AT-04 behaviour) / Cancel. "A third pulse track" can't arise:
  the lock menu disables taken channels (§5.2) and a duplicated locked track is a studio track.
  `CHIPDAW_WARN_ANSWER` answers the dialog in scripted tests (dev mode only).
- **B4A-D14 (authentic presets are the layouts).** No preset carries a profile today (UX.md B6,
  priority L). The channel layouts are whole hardware bands and already asked before changing the
  profile (decision 5), so they are the authentic presets: loading one puts the project under its
  profile (asking first, the "preset from a different expansion chip" warning: Switch / Load on
  studio tracks / Cancel) and **locks the tracks** it fills, creating a locked track for a channel
  that has none. Single instrument presets load on either kind without changing the lock.
- **B4A-D15 (the Main track as built).** No reference board was provided (B4A-D1), so the row
  follows the prompt's text and the E1 header grid: 72 px + a 1 px `#2E2E2E` divider on top (the
  lanes are 73 px shorter), an 8 px soft shadow (black 0 → 28%) cast on the rows above; the strip
  in the track strip's place (`palette::main_strip`: black → `#1C1C1C` → `#363636`, a `#4A4A4A`
  outer edge, a 6% inner glow), "Main" in the name's place, an OUT badge in the badge's place with
  the device name after it (`#808080`), PEAK in column A (label Inter 600 10 tracked .1em `#808080`
  at y 16, value Inter 600 12 `#E6E6E6` at y 32, dBFS one decimal, held; a click resets it), the
  master fader in column B (the tracks' fader, −inf … +6 dB, wheel 1 dB) with its dB value under
  it. Typing a dB value isn't offered on Main yet (the tracks' rename field is per track). The lane:
  `#0D0D0D`, bar and beat lines, the playhead. Placement: "the bottom end of the Arrangement View"
  (Live 12 manual, ch. 18 Mixing, §18.4 Return Tracks and the Main track); staying put while the
  tracks scroll is ours (the manual doesn't say). The Main gain is saved (`master_db`), applied
  after the whole mix in the engine and every export, written to the export JSON as
  `master_gain_db`, and skipped at 0 dB (bit-identical). It is not an undo step (like the profile's
  filter; it marks the project unsaved).
- **B4A-D16 (black is reserved).** `palette::black_is_reserved_for_main`: no palette colour (so no
  grid cell and no automatic pick, reduced or not) has a brightest channel under `#40`.
- **B4A-D17 (the earlier suites on format 7; harness changes).** All earlier scenarios were run on
  this build (`run_old_suites.ps1`). Six needed attention: **AT-25 and AT-26 found a real bug**:
  the header row captured the mouse on press (for the new header drag), so the M / S keys never
  got their release (AT-25's solo). Fixed: the row captures only once a drag starts. The rest are
  harness updates, as earlier passes made: (1) `docs/batch4a/tools/v7compat.py` lets every checker
  read a format 7 file as the version 6 shape it was written for (`tracks` as `lanes`, `profile`
  as `chip`, default colours left out as v6 did); (2) AT-04 / AT-16 click the new profile label
  "NES (2A03)" and the new warning's "Switch anyway" (the text check accepts the new wording);
  (3) AT-29: fx_basic now opens as NES (no VRC6 clips, B4A-D8), so the layout for "another chip"
  is the 2A03 + VRC6 Band and its answer "Load on studio tracks"; (4) AT-02's pixel diff ignores
  the window's 1 px OS frame (its colour follows focus: every one of the 1,605 differing pixels was
  on it); (5) AT-26 places the window before its gesture: `Shot` moves the window to the top-left,
  and its shot between the hover and the click put the cursor over the track menu's new rows. The
  B4A harness places the window after every launch for the same reason.
- **B4A-D18 (Delete from a track header).** B4A-04 found that Delete with a track selected did
  nothing after a header click: the root's key path matched only the physical key code, and an
  injected (non-extended) Delete arrives as the numpad's. The editors already match the logical
  key for that reason (remote desktop, injected input); the root now does the same. A header click
  also takes the keyboard focus (a text field kept Delete), and so does the end of a header drag.
- **B4A-D19 (autoplay with a test project; a pre-existing bug).** The audio gate's first smoke
  run measured silence: `start_engine()` sent Play, then the test project loaded and
  `load_project` sent Stop, so `CHIPDAW_AUTOPLAY` with `CHIPDAW_PROJECT` never played (earlier
  passes only autoplayed the demo song). The hooks that play now start the transport again after
  the test project loads. Found because the live callback cost (0.8%) disagreed with the offline
  render of the same project (4.2% of real time): a third measurement (a screenshot: time 0:00.00,
  PEAK −inf) settled it.
- **B4A-D20 (the Main row and the piano roll).** With the piano roll open, the split's pane now
  holds the lanes *and* the Main row: the lanes give up its 73 px, the piano roll keeps its height
  (the first build shrank the piano roll instead; the regression gate's piano_roll state caught it).
  PEAK above full scale shows its sign ("+33.6": 512 studio voices summed linearly at full volume;
  the faders and the Main gain are the user's level controls).
- **B4A-D21 (the audio gate's two stress runs).** 30 min of the 64-studio-track project at full
  volume: the engine's detectors 0 starved / 0 late / 0 overruns, but loopback counted 45,285 pilot
  "dropouts", **8–40 in every one of the 1,800 seconds**. Stalls come in bursts; this was steady.
  512 voices at full volume peak near +34 dBFS, so the output clipped and the quiet 23 kHz pilot was
  masked. Re-run with the project's Main gain at −40 dB (applied before the pilot is added):
  **0 pilot dropouts in 30 min**, callback p99 9.7% (the same cost: 5.10% mean both times), 0
  allocation violations. The loopback method needs an unclipped signal; that run is the gate's.
  WASAPI shared mode gave 480-frame callbacks for the 256 requested (ENGINE.md: nominal on Windows).
- **B4A-D22 (the first performance run was invalid; thrown out).** The 64-track bench relaunched
  the app without bringing it to the front, so its injected Ctrl+Shift+T / Delete went to the
  terminal behind it: 54 stray PowerShell tabs (closed afterwards; only shells started during the
  run with no children). That window then held focus, so rounds 2–3 never reached the 200-clip set
  ("stress set NOT REACHED") and pacing measured an obscured window (34 updates/s). Fix in
  `bench_b4a.ps1`: `Focus-App` (an Alt tap lifts the foreground lock, then a check) before each
  scenario; keys are sent only while the app is in front, and every line says "focus held" or
  "FOCUS LOST: INVALID". The bad run is kept in `docs/batch4a/perf/invalid-run1/`, not used.
- **B4A-D23 (the performance gate's regression, found and fixed).** The first valid run had this
  branch 1–1.4 points above the baseline (demo playing: 7.8 → 9.2 % of a core; the gate allows
  +0.3). Attribution, not guesses: pacing's per-phase UI timings (draw +0.07, flush +0.05,
  custom +0.04, events +0.03 ms/frame), an experiment build switching off the Main row, its lane
  and shadow and the track sync one at a time (the Main row explained ~0.03 ms; the rest stayed),
  then the UI thread sampled for both builds (`docs/batch4a/tools/sample_ui.ps1`, `fold_diff.py`).
  The cause: a locked row's disabled controls (§5.3) were drawn through an **offscreen layer per
  row per frame** (pan and fader thumb, 40 % opacity), and every demo track is locked. They are
  now drawn once per scale into a cached image and blitted at 40 % (`draw_disabled`). Also: the
  badge width is measured once per text and scale, the content key is hashed once per pump tick,
  and the Main lane redraws only when its playhead moves a pixel. After: flush and custom equal the
  baseline; draw +0.03, events +0.03 ms/frame.
- **B4A-D24 (the "scrollbar drag" bench missed the scrollbar).** After the first fix the gate still
  showed scrollbar_drag +2.4 in every round, yet the same drag run alone was level (13.4 vs 13.1 %),
  and the baseline's in-sequence 8.5 % was *below* its own standalone 12.5–13.4 %. Screenshots
  before the drag settled it: 200drag clicks a clip, which opens the inspector, and polish 2's
  bench aimed at the window bottom, so its "scrollbar drag" dragged inside the inspector: empty
  space on the baseline, the CLIP section's buttons and tooltip on this branch. The bench now aims
  at the thumb (5 px above the lanes' bottom: client 658 on the baseline, 585 with the Main row
  and the inspector open; screenshot check). The polish 2 report's scrollbar numbers measured the
  inspector, not the scrollbar.
- **B4A-D25 (the last of demo's excess).** After D23/D24 every scenario passed but demo (+1.1 to
  +1.8 in the gate's runs; +0.45 pooled over 31 runs). A 6,000-sample UI-thread profile of each
  build showed `AppState::commit`, `doc()` and the hasher new on this branch: the per-tick commit
  built the document twice (once to see whether anything changed, for the §5.5 conflict check,
  then again to commit). It builds it once; a second time only after an edit whose answer (Fit) may
  change it. Demo after: 7.8 / 7.7 / 6.8 vs the baseline's 7.3 / 7.9 / 8.3.
- **B4A-D26 (the piano roll's note drag and playing).** The dedicated 5-round A/B (polish 2's
  precedent for borderline scenarios) put the piano-roll note drag at **+1.8** (all 5 rounds
  higher). Profiled: every drag frame (1) resolved the edited clip three times (the snapshot,
  its studio lanes, the rack's shape: each clones the arrangement while an edit is in progress),
  (2) ran the rack check although pools only matter once the edit lands, and (3) used a polyphonic
  resolve that was O(pitches × notes). Now: the snapshot shares its resolved clips, `rack_shape`
  returns at once without studio tracks and the rack waits for the edit's commit, and
  `resolve_poly` groups pitches with one stable sort (a test pins it to the old result on 300
  random cases). Playing with the piano roll open was +0.7: the Main lane redrew its fill and ~50
  grid lines every frame for a moving playhead; they are now one cached image (like the lanes'
  retained layer). After (5 rounds each): playing 7.3 vs 7.2, dragging 7.0 vs 7.7. The other
  borderline medians of that A/B (zoom wheel +0.5, scrollbar +0.3, overview drag +0.4, idle +0.4)
  have means of −0.02, −0.16, +0.48, +0.78 (idle: one 2.9 outlier): run-to-run noise, rechecked
  in the final run. The 64-track header drag now scrolls back to the top first (two rounds had
  started on a row's resize edge).
- **B4A-D27 (the polish 2 suite on this build).** P2-01, P2-02, P2-03 and P2-11 type and drag the
  volume and pan of `fx_basic`'s tracks. `fx_basic` is a version 6 file, so it opens with every
  track locked, and a locked track's fader and pan are fixed (§5.3): the scenarios had nothing to
  edit. They now run on `fx_basic_studio`: `fx_basic` opened and saved by this build (the real v7
  migration), then every track unlocked (`docs/batch4a/tools/unlock_all.py`). The checker
  accepts format 6 or 7. P2-11's rename moved from track 7 to track 6: with the inspector open,
  track 7 now sits under the Main row (the lanes are 73 px shorter, §7) and the double-click
  landed on the Main row. **P2-11's typed values are intermittent and were before this batch**:
  5 runs of the baseline's own P2-11 on the baseline build failed once (rc: track 3's dB value),
  this build 2 of 5 (rc track 3's dB, ro track 4's pan); in the failing runs the hover log shows
  the double-click landing on the neighbouring control. Not an app regression; logged, not fixed.

## Docs update: the 2026-11-26 meeting (DOCS_UPDATE_PROMPT.md, branch `docs/meeting-2026-11-26`)

Docs only; no code changed. The rundown is [`DOCS-2026-11-26.md`](../DOCS-2026-11-26.md).

- **DOC-D1 (pre-flight).** `main` = `origin/main` = d3ac25f, the head of `batch4a/model`, so Batch 4a
  is merged (a fast-forward; the owner confirmed the merge when starting this run). BATCH4A.md §1 still said "Merge:
  not done"; a one-line note at its top now records the merge (its gate text is unchanged).
- **DOC-D2 (the reference pack).** `meeting-2026-11-26-ref/` copied to
  `docs/design/meeting-2026-11-26/` (24 files) with a README giving each board's status. The
  original folder in the repo root is left untracked, as handed over.
- **DOC-D3 (the team).** VISION.md has no team section; the roles are named on its byline
  ("Owners: Product (Marcus) and Design (Sam)"). Mika Torres (Sound Design) is added there, not in a
  new section.
- **DOC-D4 (the glint: four or eight points).** The prompt says an eight-point glint; board A's
  caption says "the soft four-point glint". Third source: the board's drawing (glintworks-logo@1x.png)
  has four long curved points and four short diagonal ones. The decision and the drawing win; the
  pack README says so.
- **DOC-D5 (context rows on the boards).** `main-track` and `fixups` show track rows with the old
  cool-to-hot header meters and full-strength controls on locked tracks. They are context for the
  locked Main row and the fixups; the new VISUAL-SPEC rules win (pack README).
- **DOC-D6 (the name).** "ChipDAW" → Cartwave in the living docs' titles and product text: VISION.md,
  README.md, PRODUCT.md (title and byline), CLAUDE.md (header), ASSETS.md (palette line). Kept: UX.md's
  app-menu strings and data folders (they describe what the code shows today; a note says they
  follow the code rename), historical reports (RUNLOG, BATCH*, MAC-CATCHUP, PERF…), and every
  identifier, crate and `.chipdaw`.
- **DOC-D7 (the dropped idea).** "Customisable header block order" is not in IDEAS.md (nor in any doc
  outside the prompts), so nothing was removed. Logged so the drop isn't re-added.
- **DOC-D8 (IDEAS.md rows the meeting touched).** The old roadmap row now points at the new order;
  the Metal backend and MIDI input rows note their roadmap steps; the VRC6 calibration row points
  to the new hardware A/B row; the per-preset marks row points to the Hardware folder.
- **DOC-D9 (where reference listening lives).** The decision says the fan MIDI is git-ignored but
  names no path. This run chose `reference-private/` and added it to `.gitignore` so the rule is
  enforced, not just written (VISION.md, ASSETS.md).
- **DOC-D10 (open issues).** Neither VISION.md nor UX.md had an open-issues section; UX.md §8 is new
  (it is the build spec). K4 (Return/Backspace behind dialogs) and K5 (colour-preview leaks) are
  **closed** by Batch 4a: B4A-D2 / B4A-01 and B4A-D3 / `edit::colour_preview_tests` / B4A-02, with
  B4A-01 failing on the baseline. They were found on the Mac and fixed in shared code, but tested on
  Windows only, so the next Mac run re-checks them.
- **DOC-D11 (gaps against 4a).** Each place the meeting changes what 4a built says "Built in 4a:"
  and "to change in <batch>" (VISION.md, VISUAL-SPEC.md §13, UX.md §4e/§5.4); the list is in
  DOCS-2026-11-26.md §2. Batch names follow the new roadmap: "4b", "the Presets + Browser + device
  panel batch", "the device-chain batch", "the export polish".
- **DOC-D12 (the Pulse colour).** "Lime only for toggles and the centre display" meets the default
  Pulse track colour `#C9D24A` (the mockup's, the palette's one ΔE exception). Recoloring a default
  is a design call, so it's marked **for the owner to decide in 4b**, not "to change".
- **DOC-D13 (the mixer meter colours).** The prompt gives the warm palette in words. VISUAL-SPEC §13
  adds the segment colours read from `meter-colours.html` option 2 (warm white `#E0DCD3`, amber
  `#F5C870`, orange `#FFA757`, red `#FF6C63`, with their edge shades) and leaves the zone
  boundaries to the board.
- **DOC-D14 (open questions).** The meeting's decisions answer VISION.md's open questions 1 (business
  model) and 3 (name) outright, and question 2 (the first effects set) for audio and MIDI effects;
  all three are struck through with a pointer. Principle 9 becomes "Open core".
- **DOC-D15 (the File menu).** VISION.md's File ▸ Import list gains "tracker projects", since the CEO
  allowed format names in the Import menu. UX.md's menu table is unchanged until tracker import is
  built.
- **DOC-D16 (preset provenance).** The rule and the author credit are in ASSETS.md; the 60 existing
  presets still share one row. Writing 60 entries now would invent per-preset history, so they
  arrive with the Presets + Browser batch.
- **DOC-D17 (`cargo test`: a pre-existing debug-only failure).** `cargo test --workspace` (debug) fails
  `ui --test robustness::project_loader_never_panics_or_hangs` deterministically (3 runs): 3 fuzzed
  JSON inputs (#578, #899, #907) panic with "attempt to add with overflow" at `file.rs:790`, where
  `load_clips`' error message formats `c.lane + 1` for a lane index near `usize::MAX`. `cargo test
  --release` passes (7/7; overflow wraps), as in Batch 4a's gate run. No crate differs from `main`,
  so this predates the docs update; it is not fixed here (docs only). Listed as UX.md K6; the fix is
  one `saturating_add` (or reporting the lane as given), for 4b's fixups.

## Batch 4b (BATCH4B_PROMPT.md, branch `batch4b/mixer`, Windows PC), 2026-11-26/27

Pre-flight: `main` = `origin/main` = d4f1afd (Batch 4a merged by the owner, plus the docs update).
Baseline worktree `../before-batch4b` at d4f1afd, release built. The eight reference boards in
`docs/design/meeting-2026-11-26/` are all present.

- **B4B-D1 (roadmap order).** VISION.md's roadmap puts "the authenticity A/B pass" before 4b; the
  owner asked for 4b now. Nothing in 4b depends on the A/B pass (it adds no emulation change), so 4b
  runs; the A/B pass stays next on the roadmap. Logged as a sequencing difference, not a conflict.
- **B4B-D2 (K6).** `load_clips`' message uses `c.lane.saturating_add(1)`: a fuzzed lane index near
  `usize::MAX` no longer overflows in debug. The existing fuzz test (`robustness::
  project_loader_never_panics_or_hangs`) is the test; it passes in debug again.
- **B4B-D3 (K3/G10, the number column).** `HeaderLayout::num_w` = 28 px up to 9 tracks, 48 px for
  10–99 (the `fixups` board), +20 px per digit beyond; the name column starts 12 px after it and
  gives up the width (the control block doesn't move). Numbers stay centred in the column, as on
  the board. The layout follows the track count every tick (`sync_header_layout`), so every row
  (and the Main row's name) moves together. Test `number_column_follows_the_largest_number`.
- **B4B-D4 (the pan law).** Equal power, "−3 dB at centre" read as the standard pan law: a centred
  track is 3 dB below a hard-panned one's side. Gains `√2·cos θ`, `√2·sin θ`, so centre is exactly
  unity in each channel (the mono level, bit for bit) and hard left is +3 dB on the left, silent on
  the right. Locked tracks and the console stay centred (their pan is fixed, as before).
- **B4B-D5 (stereo without breaking the mono path).** The studio bus keeps three step buffers:
  centre, left only, right only. A centred voice puts everything in the centre (the old single
  buffer, so the render is the old one bit for bit); a panned voice puts its quieter side's share
  in the centre and the difference in its louder side (two steps per change). The side buffers are
  read and filtered only once a side step has happened; they take the centre's timing at that
  moment (found by the stem test: an unread buffer's clock stood still). `Mixdown::render` stays the
  mono render; `render_stereo` returns whether the output is stereo. The live engine writes L/R to
  the device's first two channels (others get the centre) and folds to mono on a mono device.
- **B4B-D6 (how exports signal stereo).** By the project: `tracks::channels` = 2 once any studio
  track's pan is off centre, else 1. A mono project writes exactly the old mono WAVs (byte-
  identical; `wav_bytes_ch(.., 1, ..)` = `wav_bytes`). A stereo project writes 2-channel WAVs for the
  mix **and every stem**, so stems line up with the mix sample for sample and channel for channel
  (studio stems add up to the mix within −60 dB, test). The loop JSON's `channels` says which.
  Loudness (EBU R128) measures both channels.
- **B4B-D7 (unison, detune, drift).** Unison 1–4 voices per note, each from the track's pool (the
  pool is sized `polyphony × unison`, and the Voices cap counts them). Detune spreads the voices
  evenly over 0–50 cents (total width, centred on the note). Drift is a slow per-voice wander of
  0–10 cents: two sines (about 0.11 and 0.27 Hz) with per-voice phases, pure arithmetic on the
  frame-tick count (`chip::math`), so renders are the same on every machine. The offset goes into
  the driver's fine pitch (`Driver::detune`), applied at the frame tick like the instrument's own
  fine pitch: the chip's period is an integer, so small detunes on high notes are quantised (as on
  the chip). VISION.md lists "pulse-width drift" as a studio-only control; the prompt specifies
  Drift as pitch wander. They are different controls, so there is no conflict to resolve: Drift
  (pitch) is built here as specified, pulse-width drift arrives with the device panel (VISION.md's
  "Instruments" batch), logged for it.
- **B4B-D8 (the rate class).** Detune and Drift are **frame-tick** (stepped at the driver's tick,
  like every pitch control). Unison is **frame-tick, structural**: a change re-allocates voices at
  the next tick (new notes take the new count; a removed unison voice is released). Registered in
  `params.rs` with that note.
- **B4B-D9 (gain staging).** The scheme: (1) one studio voice is the console's level (+0.000 dB, as
  4a measured); (2) unison trims each voice of a note by `1/√n`, so a unison note keeps about one
  voice's loudness (detuned voices add in power); (3) a **safety limiter on the studio bus**: a
  linked peak limiter at −1 dBFS, instant attack, 150 ms release. It is bit-transparent (gain exactly
  1.0, no multiply) while the bus stays under the ceiling, which one note, and chords of a few notes,
  always do; it bounds chords, unison and many tracks so the output never passes full scale. A
  per-track `1/√polyphony` trim was rejected: drawing a chord would turn the whole track down, a
  surprise (VISION principle 5). The console (locked tracks) is untouched: its own non-linear mixer
  already compresses chords, as the hardware does.
- **B4B-D10 (format v8).** Version 8 = version 7 plus optional `unison`, `detune`, `drift` per track
  (written only when not 1/0/0). v7 files read through the same reader with the defaults. A v7
  corpus (docs/songs/v7, 10 files incl. 4 studio-track projects) was written **by the baseline
  build** with its hashes; every one renders identically in v8 except `v7_studio_panned`, whose pan
  v7 saved but didn't play: it is heard now, and centred again it gives the v7 hash.
- **B4B-D11 (the clip mark).** A track's mark lights when its output passes 0 dBFS: for a studio
  track, the sum of its voices' peaks after its gain and pan (an upper bound of the track's own peak:
  voices are mixed linearly, so the true peak can't exceed it); for a locked track, its channel's DAC
  peak through the channel's curve (a lone channel never reaches full scale). Held by track id (it
  follows a moved track) until a click on the mark resets it. Main's mark is PEAK over 0 dBFS; a
  click on either resets PEAK. The mark is 4 × 2 px at y 1, above the well, clear of the thumb at
  +6 dB.
- **B4B-D12 (no tooltips).** The 12 remaining `.tooltip` sites (instrument dials, Make Unique / Use
  Track Instrument, Save as Preset, the graph hint, Draw Mode, the browser's Preview key, three in
  the export dialog, the default-colour swatch) became status-bar hover help; state-dependent lines
  use a new `HelpExt::help_fn` evaluated on hover. The `tip()` helper is gone. One exception stays,
  as P2-D15 allows: the track row's full-name display (name, kind, preset) when the row truncates.
- **B4B-D13 (G15 colours).** New projects (File > New, New from MIDI File) give their Pulse tracks
  **Sky `#4FA8E0`** (`tokens::NEW_PULSE`), a palette colour far from the toggle lime. The legacy
  layout keeps `#C9D24A`, so every existing project opens with its colours unchanged (bit-identical
  files: the migration tests re-save them). The colour grid's lime cell is now **Sage `#B4C08A`**
  (ΔE 0.10 from the lime, 0.06 from its nearest neighbour), so nothing new can pick it either; the
  browser's pulse marker uses Sky. A dead `FADER_STOPS` token with a lime stop is removed.
- **B4B-D14 (the Main row vs its board).** The `main-track` board differs from 4a's text spec in
  five places, and the board wins (it is "Locked"): a speaker glyph before "OUT" (drawn from the
  board's SVG paths), the badge ink `#A8A8A8` (4a had `#9A9A9A` on Main only), PEAK as a two-line
  block centred in column A in Inter 600 10/12 with "dB" (4a: a 12 px value on a 16 px line), bar
  lines only in the lane in `#222222` (4a drew beat lines), and the CSS shadow `0 −6px 8px −4px`
  black .6 (4a: an 8 px gradient to .28). `mockup_match_main.py` measures board and app with the
  same detectors: 95/95 within 1 px at 100 % and 150 %. The whole Main header (body, strip, text) is
  now one cached image (§3.1).
- **B4B-D15 (MIDI import: when to ask).** VISION principle 6: under no hardware profile nothing
  can conflict, so a drop or File ▸ Import ▸ MIDI File… adds studio tracks without a question.
  Under a profile, every part is checked as a locked track would be (a free channel of its type,
  one note at a time, the channel's range); if all fit, they arrive **locked** with no question
  (the user chose authenticity); if any conflicts, the dialog asks: **Import as studio tracks
  (recommended, the default button)** · **Fit to hardware** · **Fit to hardware, chords as
  arpeggios** · **Cancel**, with "Don't ask again" (a new Settings ▸ Warnings row, "MIDI parts
  that don't fit the hardware profile"). The arpeggio option is a fourth button rather than a
  check box because the task dialog's one check box is "Don't ask again". Fit also fits what the
  profile's own Fit would (a DPCM row with no sample is removed), so the commit doesn't ask a
  second question; parts with no free channel stay studio tracks, so nothing is lost.
- **B4B-D16 (cursor tracking during an OS drag: verified on Windows).** VISION.md's "to verify":
  winit 0.30 reports a file entering the window (`HoveredFile`) and leaving (`HoveredFileCancelled`)
  but no position, and during OLE drag and drop the window gets no mouse messages. The fork now
  forwards those two (`WindowEvent::FileHover` / `FileHoverCancelled`, chipdaw-perf 04596968, pin
  updated in all nine entries), and the app polls the OS cursor (`GetCursorPos` → client px) every
  frame while the drag is over the window. Verified with a real OLE drag (`drag_midi.ps1`, the
  same `DoDragDrop` Explorer uses): the preview followed the cursor across tracks and bars (dev log:
  "midi drag at … -> Onto(1) bar 3"), and the drop landed where the preview last was. macOS isn't
  tested in this batch: there the preview shows without a position (after the last track).
- **B4B-D17 (MIDI import: where the parts go).** Dropped on empty space: new tracks after the last
  one. Dropped onto a track: the first part's clip goes on that track and the other parts get new
  tracks right below it (our reading of the reference's multi-part drop: unverified in the
  manual). Clips start at the drop position snapped down to the bar, each part keeping its own
  offset from the file's start. File ▸ Import ▸ MIDI File… adds new tracks after the last one, at
  the insert marker's bar. Voice types: drums (channel 10) → Noise, a part named DPCM/Samples →
  DPCM, with two or more melodic parts the lowest → Triangle, the rest → Pulse; new tracks take
  automatic colours. A drop of a `.mid` no longer replaces the song (File ▸ New from MIDI File…
  still does, as the prompt says), so it no longer asks to save first: the import is one undo step.
- **B4B-D18 (K1 not reproduced).** The menu-bar flicker was measured with a real sizing-loop drag
  (bottom-right corner and left edge), Desktop Duplication capture at 120 fps and a positive
  control: 0 flash frames before and after, with dark menus on and off (docs/batch4b/flicker). With
  nothing to measure, no code was changed for it; the gate (§9.8) is met as measured, and K1 stays
  open in UX.md for the owner to say where it shows.
- **B4B-D19 (K2).** On Windows too the browser's search field sits 2 px from the window edge and
  1 px from the column divider when focused, so its outer focus ring and 12 px glow were cut at
  both sides (the clipping the Mac run saw). Fixed in shared code: `.field`'s focus ring and glow
  are drawn inside the field (inset). The field doesn't move (the regression gate checks the
  unfocused state); only the focused look changes. Re-check on the next Mac run.
- **B4B-D20 (Tab and the view keys).** The Live 12 manual (ch. 42.1, R77) has Tab flip between the
  two main views; our Mixer view takes the place of the reference's other main view, so Tab toggles
  Arrangement / Mixer (shortcut table `view.mixer`, View ▸ Mixer with a check), and the two view
  keys show which view is up (lit: a toggle on-state). Inside a text field Tab still moves to the
  next field (the text context wins). The view isn't saved in the project.
- **B4B-D21 (the mixer boards' coordinates).** The board HTML positions a strip's parts
  absolutely inside a 1 px border, so they are measured from the **padding box** (1 px in), and
  right- or bottom-anchored parts from the padding width (150 px for a 152 px strip). The mixer
  draws from that (`mixer::INSET = 1`); 4a's Main row already did. Without it every part sat 1 px
  off.
- **B4B-D22 (the strips' number slot).** Two-digit slots (VISION.md "the mixer") with **tabular
  digits** (Inter's `tnum`), so 8 and 18 line up. The board's webfont draws proportional digits:
  its "8" sits 3 px further left than ours (the one residual in `mixer-match/report-100.md`, with
  the name's first glyph "T", 2 px: the same font difference). Logged, not chased: matching it would
  mean the board's font.
- **B4B-D23 (where Unison / Detune / Drift live).** First built as a third dial row in the STUDIO
  section; at the 1296 px minimum window it overflowed the inspector. They are now the
  INSTRUMENT section's **VOICES row** (UNISON / DETUNE / DRIFT value fields: drag, wheel,
  double-click to type; hover help), shown on studio tracks only. The Presets + Browser + device
  panel batch gives them their final home.
- **B4B-D24 (idle voices and the clip mark).** A free voice's power-up DAC step read as a peak
  above 0 dBFS on a quiet triangle track, so its clip mark lit. A voice's peak now starts at its
  power-up value and a free voice reports none (`b4b_idle_voices_report_nothing`).
- **B4B-D25 (the mixer match at 150 %).** The board is 1x; a 150 % shot is sampled at each logical
  pixel's centre (a filtering downscale smears 1 px parts). Keys are found by their dark body (their
  1 px sheen is a hairline that falls between sampled rows). Residuals at 150 %: the two font
  differences of D22 and, on 4 of 9 strips, the knob's lit box 2 px wider at the left: the brushed
  cap's anti-aliased edge crossing the probe's threshold (the zoomed pair is identical in shape).
- **B4B-D26 (the perf method, §3.13).** `perf_b4b.ps1`: 10 rounds, the baseline and this branch
  interleaved, the order alternating each round; both builds run batch 4a's bench with the Main
  geometry (the baseline has the Main row too), the piano-roll bench, and the new mixer bench
  (this branch only: 64 studio tracks playing, the arrangement first for a like-for-like line, then
  the mixer, then the strips scrolled sideways). `perf_summary_b4b.py` judges each scenario on the
  **difference of the means** and reports the **noise floor** (the standard error of the
  round-by-round differences); a scenario passes at Δ ≤ +0.3 or inside 2 × its noise floor.
- **B4B-D27 (the fixups match).** `fixups_shot.ps1` + `mockup_match_fixups.py` against
  `fixups@1x`: the number column's "9" within 1 px, the name start exact, the overhang within 1
  level of 255 on every row of its 48 px, the selection outline 1 px outside the clip (row 1 in
  both). The glow's lift isn't like for like (on the board it falls on the ruler, in the app on the
  row above); its parameters are the board's (white 22 %, a 10 px CSS blur).
- **B4B-D28 (Tab and keyboard focus).** Vizia's window-level Tab navigation runs before any
  shortcut, so Tab moved focus to the next navigable widget (the inspector's Make Unique) *and*
  switched views; the next Space then pressed that button instead of stopping playback (found by
  the lime scenes: the project became dirty). Switching views now gives the keyboard to the
  window (`focus_with_visibility(false)` in `ShowMixer`), as the reference's Tab gives it to the
  shown view. Scenario B4B-T1. Text fields keep Tab (the text context runs no view switch).
- **B4B-D29 (a held PEAK below −120 dB reads −inf).** After File ▸ New the Main row read
  "−792.7 dB": a filter tail's 1e-40 held as a peak. `main_track::PEAK_FLOOR` = 1e-6 (under
  24-bit's floor); the strips' PEAK readouts use the same text.
- **B4B-D30 (the perf gate's judgement).** The 10-round A/B (`docs/batch4b/perf/summary.md`):
  12 of 18 scenarios within +0.3 on the means; idle +0.37, row resize +0.46 and the four 64-track
  scenarios +0.74…+1.84 are over. All six are inside 2 × their noise floor, a rule this run
  introduced, not the prompt's: the gate is judged on the prompt's letter, **not passed**, and
  main is not merged (as in 4a). Attribution so far: offline, the 512-voice render costs 4.6 % of
  real time vs 4.3 % (the stereo bus: three step buffers, the limiter), about 0.3 points of a core;
  the rest of the 64-track excess is UI-side and not yet located.
- **B4B-D31 (DP-04's expectations).** The design pass's DP-04 checked the hover text of two
  disabled-or-silent controls: the Mixer key ("Batch 4") and the pan ("heard with stereo
  mixing"). 4b enables the one and makes the other heard, so their help now says what they do;
  the check expects that text (`at_dp.ps1`, `check_dp.py`). 4/4 pass.
- **B4B-D32 (contrast of the mixer's text, gate 7).** WCAG ratios from the colour constants
  against the darkest and lightest panel colours behind each text: names `#E6E6E6` ≥ 13.4,
  readouts `#C8C8C8` ≥ 10.9, the pan value and locks `#9A9A9A` ≥ 5.9, the over-0 readout
  `#E0483B` 4.74. The labels (PEAK, the scale, VOICES, the badges, the budget bar) were the
  board's `#808080`: 4.22 on the panel's lightest top `#1E1E1E`, so they are `#8A8A8A` (≥ 4.83).
  A locked strip dims its fader and knob only, never its labels. The two-digit number slot
  (`#3A3A3A`) follows the headers' embossed numbers (`NUMBER_INK #222222` since the design
  pass): decoration, the name beside it carries the identity.
- **B4B-D33 (the audio gate at full Main volume).** 30 min of fx_s64u (64 studio tracks, unison 2
  at detune 15: up to 1,024 voices) at Main 0 dB, 256 frames: the engine's detectors read 0
  starved, 0 late, 0 overruns, 0 allocation violations, callback p99 16.4 % (max 21.3 %) of the
  period, max callback gap 10.32 ms (the 480-frame period). The loopback pilot counted **11,118
  dropout blocks of 674,996**, a steady ~6 per second from the second second on, independent of
  the music's position. The two sources disagree, so a third: the same project at Main −40 dB
  (4a's S64M method) for 120 s gives **0 pilot dropouts** with the same callback cost (p99
  16.5 %) and timing. So at full volume the dense mix masks the 23 kHz pilot (1,024 band-limited
  voices leave energy in the pilot's band); the stream doesn't drop. The pilot can't judge
  dropouts at full volume on this project; the gate's letter ("0 dropouts at full Main volume")
  is reported as not met by that measurement, with this evidence. A 30-minute run at −40 dB on
  the final build follows (`audio_b4b.ps1 -Runs S64UM`).
- **B4B-D34 (merge decision).** Two gates are not met by their letter: performance (D30) and the audio gate's full-volume pilot count (D33). Per the prompt, no `pre-batch4b` tag and no merge; `main` stays at d4f1afd. Everything is on `batch4b/mixer` for the owner.
- **B4B-D35 (merge, owner's go-ahead, 2026-11-27).** The CEO accepted D30 (the 64-track performance residual) and D33 (the pilot masked at full volume; engine 0 underruns, 0 pilot dropouts at −40 dB over 30 min) as measured. Per the prompt: `pre-batch4b` tagged on `main` at d4f1afd and pushed, `main` fast-forwarded to `batch4b/mixer` and pushed.

## Rename: ChipDAW → Cartwave by Glintworks (RENAME_PROMPT.md, branch `rename/cartwave`, Windows PC), 2026-11-27

Entries above this heading keep the working title as written. Report: [RENAME.md](../RENAME.md).

- **RN-D1 (GitHub).** `git ls-remote https://github.com/glintworks-studio/cartwave` answered
  (the CEO had transferred and renamed the repo). The first `git remote set-url origin` was
  stopped by the session's automatic permission check; the owner then allowed it mid-run, so
  `origin` is `https://github.com/glintworks-studio/cartwave.git`. After `git fetch`, `main` and
  `origin/main` are both 8e6b4f8 (the old URL showed the same). No repository was created,
  renamed or transferred by this run.
- **RN-D2 (packages and the binary).** No crate or package name contains "chipdaw" (`app`,
  `chip`, `engine`, `ui`). The package `app` keeps its name (every harness builds with
  `-p app`); its executable is renamed with a `[[bin]]` entry: `cartwave`, so
  `target/release/cartwave.exe` on Windows (file names are case-insensitive there; an
  installer may show it as `Cartwave.exe`). Every manifest gains `authors = ["Glintworks"]` and
  the glintworks.studio homepage.
- **RN-D3 (environment aliases).** One module, `engine::env`, reads every variable: the
  `CARTWAVE_*` name first, then its `CHIPDAW_*` alias. The new name wins when both are set. Using
  an alias prints one line per name in dev mode (`dev: CHIPDAW_X is the old name of CARTWAVE_X;
  it still works, but use CARTWAVE_X`). `ui::dev`, the data folder, `CARTWAVE_AUDIO`,
  `CARTWAVE_VSYNC`, the pilot frequency and the fuzz knobs all go through it. Dev mode itself
  is read without the note (the note asks for dev mode), then `CHIPDAW_DEV` is noted once.
  `CARTWAVE_COMMIT` is a build-time name inside the crate (no alias needed).
- **RN-D4 (the project file).** New projects save as `.cartwave`. `.chipdaw` opens forever
  (Open, drop, Open Recent, `CARTWAVE_PROJECT`); **Save keeps the file's own extension**
  (an old file stays where it is), **Save As offers `.cartwave`** (the suggested name's
  extension changes) and still lists `.chipdaw`; Open lists both in one filter. The atomic
  save's temporary file follows the target's extension. **The format version is unchanged (8)
  and so is the `"format": "chipdaw"` field:** it identifies the file format, every file ever
  written has it, and changing it would need a version bump for no user benefit. The "not a
  project" message no longer quotes it (RN-D13).
- **RN-D5 (recovery).** The recovery file is `recovery.cartwave`. A `recovery.chipdaw` in the
  data folder (moved over, or left by an old build in an explicit `CARTWAVE_DATA_DIR`) takes the
  new name, so an unsaved session from before the rename is still offered.
- **RN-D6 (the app-data move).** `%APPDATA%\ChipDAW` → `%APPDATA%\Cartwave` (macOS
  `~/Library/Application Support/Cartwave`, Linux `$XDG_DATA_HOME/cartwave`). At start, before
  anything reads settings, if the new folder doesn't exist and the old one does, the old one is
  **copied** through a staging folder (`Cartwave.moving`, renamed into place at the end), and a
  marker (`migrated-from-chipdaw.txt`: the old path, `pending`) is left. The old folder is the
  backup: the launch that moved it marks the marker `launched` once the UI is built; the next
  launch to get that far removes the old folder and the marker. With an explicit data folder
  nothing moves. A failed copy leaves no new folder and the old one untouched (logged). Unit
  tests: `app_data_folder_moves_and_old_one_is_kept_until_next_launch`,
  `recovery_takes_a_legacy_file_in_its_folder`. The owner's real `%APPDATA%\ChipDAW` (settings,
  a recovery file) was not touched by this run: it moves on the first launch of Cartwave.
- **RN-D7 (export metadata).** Neither export writes a "software" field: WAVs have no
  `LIST/INFO` chunk, and the loop JSON names the software only in `"format": "chipdaw-loop"`.
  That field becomes `"cartwave-loop"` (version 1 unchanged); no field is added (that would be a
  behaviour change). Every WAV is byte-identical (RN-D12). The committed example
  `docs/export-examples/showcase.json` predates Batch 4a (no `master_gain_db`, fewer stems):
  regenerating it would change more than the name, so only its `format` line is edited.
- **RN-D8 (identifiers).** `ui::identity`: `APP_NAME` Cartwave, `COMPANY` Glintworks, `WEBSITE`
  glintworks.studio, `APP_ID` `studio.glintworks.cartwave`. The standalone app sets it as the
  process's Windows AppUserModelID before the window opens (`windows-sys`, already a
  dependency; taskbar grouping and pinning). The Mac bundle id is recorded for later in
  `[package.metadata.bundle]` (there is no bundle yet). No Windows VERSIONINFO resource: that
  needs a resource compiler crate (CLAUDE.md: ask first).
- **RN-D9 (visible text).** Window title (`— Cartwave`), About (`ABOUT CARTWAVE`, `Cartwave
  0.1.0 · build …`, and a new line `Made by Glintworks · glintworks.studio`), the Help menu's
  About item, the macOS app menu (`Cartwave`, Hide / Quit Cartwave), the warning dialogs'
  title, the file-type names in the dialogs, the drop hint (`.cartwave`), the error texts
  ("Not a Cartwave project", "a newer Cartwave"). The shortcut sheet shows the name only in
  the macOS Quit row (Windows: Exit).
- **RN-D10 (not renamed).** The vizia fork's `chipdaw-perf` branch and its pins; the file's
  `format` field (RN-D4); git history; RUNLOG entries before this heading and the historical
  reports (each gets a one-line note at the top); the corpus and fixture `.chipdaw` files
  (they test old files); temp-folder names inside the tools (`%TEMP%\chipdaw-perf-dp` holds
  generated benches the perf tools copy); `make_songs`' `piano_bench.chipdaw` (the perf tools
  copy it by that name). Internal names nothing outside reads (the audio host thread, test
  temp folders) became `cartwave-*`.
- **RN-D11 (harnesses).** 81 scripts under `docs/*/tools` and `tools/`: `CHIPDAW_` →
  `CARTWAVE_`, `app.exe` → `cartwave.exe`, the process name `app` → `cartwave` (`Get-Process`,
  `pkill`, macOS `target/release/cartwave`). `final_gates.ps1`'s `../before-polish2` baseline
  keeps `app.exe` (built before the rename). To shoot a pre-rename build with the shared
  helpers (which find the window by process name), copy its `app.exe` to `cartwave.exe` and run
  it with `CHIPDAW_*` names (`regress_rn.ps1 -Prefix CHIPDAW_`).
- **RN-D12 (gates 1, 2: hashes).** `cargo test` keeps the golden hash and every v2…v7 migration
  hash (those tests are unchanged and pass). Beyond them, `examples/corpus_hashes.rs` rendered
  **all 290 `.chipdaw` files in the repo** (the v2…v7 corpus, the scale test, the stress
  projects, every suite fixture and recovery file) with the baseline (`../before-rename`,
  8e6b4f8) and this branch: render and export hashes **identical** for all 290, 0 load errors
  (252 mono, 38 stereo; `docs/rename/corpus-hashes.tsv`). New test
  `every_corpus_file_round_trips_through_a_cartwave_file`: every corpus version saved as
  `.cartwave` reloads with the same text and sound. Exports: 101 files from 5 projects, every
  WAV byte-identical, each JSON only its `format` field (`docs/rename/export-identity.md`).
- **RN-D13 (one message).** The binary-strings scan found the "not a project" error quoting
  `"format": "chipdaw"`: reworded to "Not a Cartwave project (its format field is missing or
  names another format)." Text only.
- **RN-D14 (gate 4: visual regression).** `docs/rename/tools/regress_rn.ps1` shoots polish 2's
  states plus the ones that show the name (About, the shortcut sheet, Settings ▸ Files, the
  Help menu); `diff_rn.py` allows nothing up front and lists every differing region of the
  whole window (title bar included) for labelling. 150 % needed two harness fixes: the window
  is taller than the work area there, and Windows placed it at a different height in each run
  (the taskbar covered a different band), so the script now pins the window's frame to the
  screen's top-left; and one `solo_mute` shot of this build lost its second click (Mute not
  lit; the known quick-click flake): re-shot alone, it matches. Result at 100 % and 150 %:
  only the title bar's name, the Help menu's About item and the About panel differ, plus a few
  corner pixels outside the window's rounded frame (they differ between states of one build
  too). This build with the old names is pixel-identical to it with the new names.
- **RN-D15 (gate 5: the suites, both spellings).** `docs/rename/tools/run_suites_rn.ps1` on the
  final build. Two harness checks named the old name (the shell suite's `recovery.chipdaw` and
  `ABOUT CHIPDAW`) and `shot.ps1` found the window by a title containing "ChipDAW" (AT-06's
  menu screenshots failed): fixed (shot.ps1 now takes either title). With `CARTWAVE_*` every
  suite passes. The first old-names run copied only the suite scripts with `CHIPDAW_*`; the
  shared helpers they dot-source (`lib.ps1`) still set `CARTWAVE_*`, which win, so B4B-C1 opened
  the previous scenario's project (2 checks failed). That run didn't test the aliases cleanly
  and isn't reported: the runner now rewrites every harness script under `docs/` in place
  (after a commit) and restores them from git. That run: every suite passes (B4B 43/43), and
  65 app logs carry the alias notes.
- **RN-D16 (gate 3 on the real app).** `migrate_e2e.ps1` points `APPDATA` at a temp folder
  holding a pre-rename `ChipDAW` folder and launches the release build three times: copy,
  keep, recovery offered; then the old folder and marker removed and the settings kept; then
  nothing. The first try failed one check, by the fixture: its recent file didn't exist, and the
  app drops recent entries whose file is gone (AT-18's behaviour). With a real file: PASS.
  The owner's real `%APPDATA%\ChipDAW` was checked untouched afterwards.
- **RN-D17 (gate 6).** 268 passed, 0 failed, 2 ignored in debug and release (7 new: the env
  aliases ×2, the identifiers, both extensions, the folder move, the legacy recovery file, the
  corpus round trip). One clippy note (`map_or(true, …)` on the vsync knob, `is_none_or` now);
  clippy clean; a forced release rebuild of every target: 0 warnings.
- **RN-D18 (merge).** Every gate passes. Per the prompt: `rename/cartwave` pushed, `pre-rename`
  tagged on `main` (8e6b4f8) and pushed, `main` fast-forwarded and pushed.

## Batch 4c (BATCH4C_PROMPT.md, branch `batch4c/profile-ab`, Windows PC), 2026-11-27

- **B4C-D1 (setup).** Branch `batch4c/profile-ab` from `main` 5533b5a (= `origin/main`); the
  baseline is `main` exported to `../before-batch4c` and built there (release). `pre-batch4b` is
  d4f1afd, built in `../before-batch4b` (a pre-rename build: `app.exe`, `CHIPDAW_*` names).
  Machine: Windows 11 Pro 26200. The default (and only) output is now the monitor's HDMI endpoint
  (NVIDIA, 480-frame periods, 48 kHz float); the MOTU that ENGINE.md §8 measured on is not
  connected (`set_default_audio.ps1 -List` shows one endpoint).
- **B4C-D2 (the new audio-gate method: the reference match).** Pre-approved by §0.2. Probe first
  (`crates/engine/examples/tapprobe.rs`, `docs/batch4c/audio/tapprobe.txt`): WASAPI shared-mode
  loopback returns the samples we hand the device **bit for bit** (187,200 of 187,200 frames,
  gain 1.000000). So the gate compares the capture with the engine's own copy of its output
  instead of listening for a tone: `Shared::tap` (an `OutputTap`, set once by the harness:
  atomic stores into a preallocated ring, no allocation, no locks; `core::tests::
  the_output_tap_copies_the_first_channel` runs it under the allocation checker) holds the first
  output channel as written; a checker thread (`testkit::Matcher`) locks onto it in the capture
  with 1,024 equal samples and then compares every 128-frame block exactly (tolerance 1e-5). A
  dropout is silence the device inserted: the offset between capture and copy jumps forward by the
  gap's length, which the matcher reports in frames (a sample counter, in effect). A −60 dBFS
  xorshift marker (`Command::TestMarker`) keeps the alignment unique through silence and held
  notes; silence never locks. The music can't mask it: it is subtracted exactly. Test hooks:
  `CARTWAVE_MATCH=1` (app), `--match` (headless). Unit tests: clean stream, gaps of 480/1056/4800
  frames reported at their exact length, a periodic signal with a one-period gap, capture-side
  gaps, foreign sound, silence, tap losses.
- **B4C-D3 (validation by injected stalls).** Headless, 23 stall lengths 5…50 ms, 3 passes each,
  music muted (the pilot can't be masked there, so it is the ground truth) and with music
  (`docs/batch4c/audio/validate_muted.txt`, `validate_music.txt`). **The match, the pilot and the
  starved detector agree on every one of 138 stalls**: 0 false positives, 0 misses. On this
  endpoint stalls ≤ 19 ms never drop out and ≥ 21 ms always do (20 ms: 5 of 6, the same ones in
  all three detectors). ENGINE.md's "≥ 22 ms" is the MOTU's threshold (a 1,126-frame buffer);
  this device's buffer is smaller, so its threshold is lower. The gate's letter (catch every
  ≥ 22 ms stall, none of the shorter ones *that don't drop out*) is met; the shorter ones that do
  drop out here (20, 21 ms) are caught, as they should be. The measured gaps are multiples of the
  device period (8, 18, 28, 38 ms), as expected of a sample-exact count. **In the app, in 4b's
  failing condition** (`fx_s64u`, 1,024 voices, Main at 0 dB, stalls 10/19/30/40 ms every 3 s,
  `validate_app_s64u_full.csv`): the match finds exactly the 12 stalls of 30/40 ms and none of the
  12 at 10/19 ms; the starved detector agrees (12); the pilot reports 460 "dropouts" (masked, as
  4b found). New hook for this: `CARTWAVE_INJECT_STALLS=ms,…` + `CARTWAVE_INJECT_EVERY` (dev mode).
- **B4C-D4 (locating the 4b excess, §2.2).** Tools: `docs/batch4c/tools/threads_b4c.ps1` (per-thread
  TSC cycles from the audio log: UI cycles per presented frame, audio cycles per callback; the
  frame log's phases), `perthread_b4c.ps1` (CPU per thread by start module), `sample_b4c.ps1` (4a's
  sampler for any build and project), `phases_b4c.py`; evidence in `docs/batch4c/perf/`. Scenario:
  `stress64.chipdaw` playing at 1440 × 920. Findings, in order:
  1. **Process CPU is a noisy judge here** (Windows thread times are tick-sampled): the audio
     thread, which runs 100 short bursts a second, read 4.69 / 6.41 / 3.36 % in three runs of one
     build. The cycle counters are precise (sd ≈ 30–70 k cycles per frame) and were used to locate.
  2. **The split** (pre-4b → main): UI **+224 k cycles/frame** (+3.3 %, ≈ 0.3 points), audio
     **+107…130 k cycles/callback** (≈ 0.2–0.3 points: the stereo bus, accepted), the GL driver's
     thread ≈ +0.4 (tick-sampled).
  3. **Where in the UI frame:** the frame log puts all of it in the **events phase: +40 µs per
     frame** (232 → 272 µs); style, layout, draw, flush and every custom draw span are equal.
     With `FROZEN_METERS=1` the UI difference falls from +224 k to +46 k cycles: ≈ 80 % is the
     meter path.
  4. **What, by experiment switches** (one build, features off by `CARTWAVE_B4C_OFF`, two to three
     rounds each; `attribution-switches.patch`, `switches_1/2.txt`), events µs per frame:
     the per-tick commit's document copy **−40** (pre-existing, but it grew with 4b's lanes); the
     held peaks (an extra pass searching the rack per track, then a search per held id) **−22**;
     the clip mark looked up by id in two bindings per track per frame **−18**; reading
     `CARTWAVE_FROZEN_METERS` from the environment every frame (two lookups since the rename's
     aliases) **−10**; `sync_ctls` storing every studio track's controls every tick **−10**;
     Main's L/R meters and VOICES ≈ 0. All off: 212 µs, 20 µs under pre-4b.
- **B4C-D5 (the fixes, §2.3, the code's own patterns).** (1) The commit compares the live
  document in place (`Doc::matches`, tested against `same_as` on a copy) and builds a `Doc` only
  when something changed. (2) The meter tick is one pass over the tracks (a cursor through the
  rack, which is in track order; search only as a fallback), held peaks kept sorted by id (binary
  search), the next state built in a buffer kept between ticks (`Meters::clone_from` reuses the
  vectors), and each track's clip mark computed **once per tick** (`Meters::clip`, read by index
  in the bindings; a reset clears it at once). (3) `dev::frozen_meters()` / `fixed_playhead()`
  read once. (4) `sync_ctls` sends a control only when its inputs changed or it is a new control.
  Unit tests: `matches_agrees_with_comparing_a_copy`, `held_peaks_and_clip_marks`,
  `sync_ctls_sends_only_changes`. The commit's share of UI samples fell from ≈ 0.2 % to 0.07 %.
- **B4C-D6 (the fixes measured, before/after).** 6 interleaved rounds × 30 s, cycle counters
  (`phases_main_b4c.md`, `threads.tsv` tag `long-`): **UI −68 k cycles/frame vs main** (paired
  SE 16 k; events −32.5 µs, draw +18 µs); vs pre-4b the UI is still ≈ +117 k. The draw phase's
  +18 µs isn't in any custom span or row redraw count; same-binary switches (`switches_3.txt`)
  and a same-binary profile (fewer `draw_system` samples on the new path) disagree with the frame
  log about it, so it is reported as measured and not attributed. The gate is judged by the 4b
  method at the end (§6.5).
- **B4C-D7 (K1, §3).** Reproduction first, in the CEO's condition: `menu_flicker_b4c.ps1 -Play`
  (audio on, the song playing), 17 captures of a real sizing-loop drag at up to 120 fps: corner,
  right, bottom and left edges, a fast drag, the 64-track project, dark menus on and off, the top
  of the client area too, before (`main`) and after: **0 flash frames and 0 misdrawn frames in
  every one**; the positive control: 12. The detector was widened twice (a misdrawn-bar test;
  then any new solid light block anywhere in the band) after checking its blind spots; both find
  the control and nothing else (the left edge's moving-window artifact is explained in the
  README). Dark menus on/off can't change the bar: muda draws it either way; the setting is the
  dropdowns'. **No code changed for K1**: it can't be shown fixed without being seen. Gate 6
  (0 flash frames while playing, dark menus on, every condition) reads 0 as measured; UX.md K1
  stays open with the request for a slow-motion phone video (Desktop Duplication can't see a
  glitch after DWM composition, e.g. a swap-chain presentation-mode change). Files:
  `docs/batch4c/flicker/`.
- **B4C-D8 (reference tools, §0.1 / §4.2).** Two, outside the repo: **NSFPlay 2.6** (downloaded
  release zip, `C:\tools\reference\nsfplay26`; its documented command line renders WAV; a
  well-regarded APU core) and **Game_Music_Emu** through the already-installed ffmpeg 8.1's libgme
  (Blargg's cores; an independent second implementation, so the two references can be set
  against each other). Mesen and others were not used (no headless WAV render without more
  installs). VGM was not used: the VRC6 isn't in it; an NSF covers both chips.
- **B4C-D9 (the container and the logs).** `crates/ui/examples/ab.rs`: register writes per frame
  tick from our driver (`Player::tick_writes`, a read-only accessor added to `chip`: no change to
  what is rendered); the logs are proven complete by a bit-identical replay of both songs; an NSF
  we author (6502 player, 4 KiB banked stream at $9000, our DPCM ROM at $C000); ten test patterns;
  per-channel splits of the songs. Our side replays with the NSF player's init writes so both
  start from the same state.
- **B4C-D10 (measurement fixes, found by checking the numbers).** (1) A periodic test tone
  correlates equally at many lags: align on envelopes first. (2) NSFPlay with its high-pass off
  keeps the 2A03's DC, which dominated the envelopes, the noise centroids and a file-wide gain:
  align on the first difference; measure levels, timing and noise after the same 20 Hz high-pass
  on all three; per-segment levels relative to an anchor segment. (3) gme skips leading silence
  (lags up to −2 s) and stops on long rests (not configurable through ffmpeg): ±3 s search and a
  coverage row; NSFPlay's `AUTO_STOP=0`. (4) The strongest harmonic of short-mode noise isn't its
  fundamental: test the LFSR loop by autocorrelation at 93 / 31 / 32767 steps. (5) Waveform error
  per segment with its own fine alignment (an NSF player's writes land a few hundred cycles from
  ours, so each note's phase differs slightly). Noise and the songs' waveform error is ≈ 0 dB by
  construction (differently seeded LFSRs) and is reported as such.
- **B4C-D11 (findings and the pack).** docs/ab/AB.md: NSFPlay matches ours to 0.00 cents and
  0.1 dB in most measurements; the top differences are the tnd group ≈ 1 dB lower against the
  pulses in NSFPlay, the sweep edges (references disagree; ours follows the wiki), the players'
  high-frequency roll-off, gme's polarity (noise, VRC6) and gme's mixer. No engine change
  proposed now except a measured tnd calibration once a hardware recording exists. The pack:
  every pattern in full, the songs as 12 s excerpts (41 MB; the full songs would have added
  17 MB more); listed in ASSETS.md.
- **B4C-D12 (gates 1–3, 6, 7).** Gate 1: 290 corpus files and 252 export files identical to
  `main`, every hash test passes. Gate 2 (`regress_b4c.ps1`, 100 % and 150 %): 0 changed pixels in
  every state apart from the About build stamp (the baseline is a `git archive` build: "build
  unknown") and 13 px outside the rounded corner at 100 % (RN-D14); K1 changed no code, so no K1
  region. Gate 3: every suite without a FAIL line, B4C 3/3, `cargo test` 279/0/2 in debug and
  release, clippy clean, 0 warnings. Gate 6: K1 0 flash frames (D7). Gate 7: AB.md and the pack.
- **B4C-D13 (gate 5: NOT MET, reported as measured).** `perf_b4c.ps1 -Rounds 10`, three builds
  interleaved, 0 runs left out (`docs/batch4c/perf/gate/`). Against `main`, 15 of 21 rows pass;
  six exceed +0.3: 200drag +0.53 (SE 0.46), zoom_wheel +0.97 (0.53), header_width +0.81 (0.57),
  ruler_zoom +3.15 (1.71), overview_drag +2.91 (2.58), s64_insert_delete +0.83 (0.63). Every one is
  within 2 SE of zero, and none is in the code the fixes touched, but the gate is judged on the
  means by its letter (no noise allowance; the summary script follows that). Against pre-batch4b,
  idle −0.11 and row_resize −0.66 pass; the four 64-track rows fail: playing +0.65 (0.61), scroll
  +2.11 (1.32), insert/delete +1.61 (0.97), header drag +0.49 (0.86). `main` itself reads +0.6 /
  +1.9 / +0.8 / +0.8 over pre-batch4b on those rows, so by process CPU the branch hasn't recovered
  4b's excess there. That is consistent with the cycle counters' scale: the UI fix is −68 k
  cycles per frame (≈ 0.1 point of a core at 60 fps), and what is left over pre-batch4b (UI
  +117 k per frame, the unattributed draw +18 µs, and the stereo bus on the audio thread, accepted
  in 4b) adds up to ≈ 0.4 points before the gesture rows' heavier frames. Not re-run in the hope of
  a pass: a second 10-round set would be choosing the sample. **Gate 5 fails; the merge is held**
  (§1.4). What is left to find: the draw phase's +18 µs (D6) and the scroll/insert rows' frames,
  where the excess is largest.
- **B4C-D14 (gate 4 and the merge).** `audio_b4c.ps1 -Secs 1800`: fx_s64u (1,024 voices), Main at
  0 dB, 256 frames, the reference match on. Result: 0 match dropouts, 0 mismatch runs, 0 capture
  gaps / foreign / tap lost, 99.998 % of the run compared; starved 0, late 0, 0 allocation
  violations (`docs/batch4c/audio/gate30/`). Gate 4 passes. **Merge held** (§1.4): gate 5 fails
  (D13); every other gate passes. No tag and no change to `main`. The owner's options are in
  BATCH4C.md §7.

## Presets + Browser + device panel (PRESETS_PROMPT.md, branch `presets/browser`, Windows PC), 2026-11-27

- **PB-D1 (pre-flight).** `main` = `origin/main` = `batch4c/profile-ab` = 1b8a288 (Batch 4c,
  merged by the owner after its held run; `pre-batch4c` exists). Branch `presets/browser` from it.
  Baseline: `main` exported with `git archive` to `../before-presets` and built there (release).
  Machine: Windows 11 Pro 26200 (this session is on Windows, not macOS).
- **PB-D2 (conflicts between the prompt and the board, VISION.md wins).** VISION.md calls the
  `workflow-chip-synth` board "locked for layout"; where the prompt's text and the board differ,
  the board's layout is built and the difference logged:
  (a) the **Voice selector** sits in the OSCILLATOR section's title row (the board), not in the
  device header (the prompt's §4.1 list); the header keeps the colour tab, the name, the
  STUDIO/LOCKED badge and the centred preset field with ◀ ▶;
  (b) the board's **dashed MIDI-effects and device-chain boxes** are not drawn (the prompt §3.3,
  "nothing pretends", wins over the board's placeholders);
  (c) the board's **locked labels** are darker than 4.5 : 1 (`#808080` at about 40 %); the
  prompt's and VISION's ≥ 4.5 : 1 wins, measured and reported in the contrast gate.
- **PB-D3 (Arp Speed is built, not faked).** The board and the prompt put an ARP SPEED knob
  under the envelopes. The engine had no such control, and "nothing pretends", so it is built:
  `Instrument::arp_speed` (frames per arpeggio step, 1–8; 1 = every frame, today's behaviour),
  stepped by the driver like the other macros (a driver-side feature, so allowed on locked
  tracks, as game drivers did). Saved in the instrument JSON: **format 9** (v8 files load with
  speed 1 and render bit-identically; migration hash test with a v8 corpus written by `main`).
- **PB-D4 (the bank's form).** The bank is data: `presets/bank.json`, generated by the committed
  `presets/tools/design_bank.py` (every preset built procedurally from our own macro shapes:
  envelopes, decays, the echo, arpeggios with their speed, duty sweeps, kick-drops, noise kits),
  built into the binary (`include_str!`) and read by `crates/ui/src/bank.rs`, which also builds the
  library's folders. Instrument data uses the project file's instrument JSON (one parser). No new
  crate.
- **PB-D5 (the classic presets are folded in).** The 61 presets of `crates/chip/src/presets.rs`
  (the prompt says 60; the table has 61) stay as they are (new projects' default band, the
  channel layouts, old tests and songs name them) and are listed in the bank in each role's
  **Classic** folder, with their own provenance rows (author, 2026-11-25, hand-written, Batch 1).
  The counts §7.1 asks for are met by the **141 new presets alone** (bank::tests): Leads 15,
  Basses 14, Arps 14, Pads 7, Bells & Keys 7, FX 10, Game Sounds 12 (the twelve named), Drums 4
  kits + 16 hits, Hardware 42 (NES 30, Famicom + VRC6 12). A classic preset keeps its old set of
  voices (`kinds`): on a studio track already on one of them the voice stays; a new preset
  remembers its one voice. Three new names that clashed with classic ones were renamed.
- **PB-D6 (level matching).** `crates/ui/examples/bank_pack.rs` renders every preset playing its
  role's phrase (studio track; authentic presets on a locked track under their profile and
  filter), measures EBU R128 loudness, and writes per-preset gains that `design_bank.py
  --levels` folds into the channel Volume / the saw's Rate, or the volume macro (scaled up only as
  far as its loudest step allows, so envelopes keep their shape). Each category's target is fixed
  from the first pass at the 35th percentile of its new presets (a median target drifted as loud
  presets came down; it was found by watching the passes). Result: **112 of 141 within ±1.5 LU**;
  the 29 others sit at a hardware limit (the triangle and DPCM have no volume control; the
  shortest one-shots and the thinnest duties are at full volume): listed in
  `docs/audio/presets/levels.md`. The classic presets are reported, not changed. A bug found on
  the way: the presets shared one controls dict per voice, so one preset's gain moved them all
  (fixed: each preset gets a copy). No preset clips (no sample at or past full scale).
- **PB-D7 (authentic presets on studio voices: reported, not changed).** §7.3 asks for
  bit-identical renders on locked and studio voices "where the feature set allows". Measured with
  the same profile and filter: **0 of 42 are bit-identical**. The triangle kick and tom and the
  hardware-sweep zap differ only by float rounding (residual below −125 dB); the other pulse,
  triangle and VRC6 presets match in loudness within 0.4 LU but not sample for sample (the
  console's sequencers run freely, a studio voice starts its phase per note: Batch 4a's
  architecture); the noise and DPCM presets are **2.3–3.6 LU louder on a studio voice** than
  through the console's non-linear mixer. That is an engine property (4a/4b), not preset data;
  this batch changes no engine code, and 4c's A/B already proposes calibrating the console's
  triangle/noise/DPCM group from a hardware recording. Flagged in PRESETS.md for the owner.
- **PB-D8 (the listening pack's size).** 202 WAVs, 48 kHz 16-bit mono, the phrase plus its
  tail cut 20 ms after the last sample above −70 dBFS; the one-shot phrases (game sounds, drum
  hits) are one bar. 51 MB (4c's A/B pack is 41 MB).
- **PB-D9 (the device's controls: where each one lives).** The board places Duty, Unison and
  Detune under the oscillator and Vib Depth, Vib Rate, Arp Speed under the envelopes. Every
  instrument control the old inspector had must keep one home (VISION principle 2), so: **Drift**
  (the prompt lists it, the board omits it) joins the OSCILLATOR row; **Vibrato Delay** (an
  existing control) joins the ENVELOPES row (four knobs, `space-around`); the channel's other
  hardware controls get a second row of 28 px knobs: **Sweep and Sweep rate** (pulse) and the
  noise **Period** under the oscillator, the hardware **Envelope** mode under OUTPUT's Volume
  (which reads DECAY when the envelope decays, as the old dial did). Each voice shows only the
  controls its registry row has (`ParamId::for_lane`, the studio trio only on studio tracks).
  Labels are `#8A8A8A`, not the board's `#808080` (4.45 : 1 on the sections' lightest top), as the
  mixer's (B4B-D32). Knob arcs run from the minimum (the board draws them from 12 o'clock, which
  reads as "a lot" at zero). The **DPCM Sampler** shows SAMPLE (the note-picks row, the built-in
  and user slots, Import), PLAYBACK (Rate, Loop, and the sample and its rate in the display) and
  OUTPUT; no envelopes, and no Unison / Detune / Drift: the DMC ignores the macros (only sample,
  rate and loop reach it, `driver.rs`), so drawing them would pretend (their stored values are
  kept). The old inspector's DPCM arpeggio and pitch graphs pretended the same way.
- **PB-D10 (the Detail View).** Live 12 manual §42.1 (R78): Shift+Tab switches the lower panel
  between the clip and the device chain. Shift+Tab was free in the shortcut table; it is
  `ToggleDetail` ("Clip / Device View"), so it shows on the Help sheet (the one-table rule) but
  not in the View menu (it would change a menu outside the allowed regions; the reference doesn't
  list one either). The Clip view is the piano roll; the Device view is the inspector's **CLIP**
  section in the board's first 200 px (where the board drew the MIDI-effects placeholder, so the
  device sits at the board's x 224) and the instrument. Selecting a track header shows its Device
  view; opening a clip its Clip view; Shift+Tab back reopens the clip the Device view replaced.
  The bottom pane is the board's 356 px (it was 232).
- **PB-D11 (the browser's width and the regression gate).** The board's browser is 420 px (a
  150 px sidebar and a 270 px content pane); it was 240. The 180 px moves the headers, lanes and
  mixer right. The regression diff (`docs/presets/tools/diff_pb.py`) counts a pixel unchanged if
  it matches the baseline in place or 180 px to the left; what is left is sorted into the
  browser, the bottom pane and OTHER, every OTHER region listed.
- **PB-D12 (the earlier suites' coordinates).** Their clicks assume the 240 px browser: they run
  with a dev-only hook, `CARTWAVE_BROWSER_W=240` (as polish 2 runs the oldest ones with the
  Overview and status bar hidden, P2-D16), via `docs/presets/tools/run_suites_pb.ps1`.
- **PB-D13 (what "a studio-only preset" is on a locked track).** VISION: "choosing a studio-only
  preset asks first: Unlock track · Cancel". A preset is studio-only for a locked track when it
  uses Unison, Detune or Drift, or plays on a voice the channel isn't. Any other preset loads
  without a question (principle 6: ask only when it matters): the hardware can play it. ◀ ▶ on a
  locked track skips presets it can't play rather than asking at every step.
- **PB-D14 (loading from Hardware).** An authentic preset puts the project under its profile and
  locks the track to a free channel of its type (one undo step with the profile change). From
  None (studio only) the profile is set without a question (nothing can break); a VRC6 preset in a
  NES project asks as a layout does (Switch to Famicom + VRC6 · Load unlocked · Cancel, the
  Expansion warning); no free channel asks (Load unlocked · Cancel, a new "no free channel"
  warning); locking keeps 4a's rules (fader / pan reset asks; chords ask Fit and lock). A track
  locked to another type of channel can't take it (a status-bar message says where to drop it).
- **PB-D15 (Collections, Clips, Samples, Places).** Collections are seven fixed coloured labels
  (Favourites, Red, Yellow, Green, Blue, Purple, Grey; none lime), assigned from any item's context
  menu, kept per user in settings.json by item key; the sidebar always shows Favourites and any
  other collection that has items. **Clips** lists the bank's phrases (the ones the listening
  pack plays): a double-click puts one on the selected track at the insert marker, a drag at the
  dropped bar or (empty space) on a new track. **Samples** lists the four built-in DPCM samples,
  the project's imported ones and Import audio. **User Library**: the user presets. **Current
  Project**: the presets and samples the project's tracks use. Audio Effects, MIDI Effects and
  Imported Projects aren't shown (nothing pretends).
- **PB-D16 (harness changes, gate 4).** The earlier suites, adjusted only where a control moved
  (each change marked "PB (PB-D16)" in the script): **AT-10** right-clicks empty space on lane 3,
  not lane 5 (the 356 px bottom pane puts lane 5 under the Main row while a clip is selected);
  **AT-14** deselects before Help ▸ Getting Started (the pane shrank the guide's area under the
  luma probe); **AT-25** reads the device panel's dev line (the instrument is drawn, not UI text);
  **AT-29** finds the layouts at Hardware ▸ NES (2A03) / Famicom + VRC6 ▸ Bands; **AT-31** saves
  through the picker's Save Preset As… and opens the user preset's menu in User Library (the check
  accepts the new item menu, collections first); **DP-03** previews Sounds ▸ Leads ▸ Square's
  first two presets and finds the Preview key at the new header's x 216. Format 9 in the fixtures'
  version checks: `unlock_all.py` (it refused format 9, so P2-01/02/03/11 ran on locked tracks)
  and P2-01's "saved by this build". One behaviour fixed on the way: a right-click on a track
  header (its menu) no longer closes the open piano roll; only a click shows the Device view (AT-10
  caught it).
- **PB-D17 (the perf gate's coordinates).** Batch 4c's bench clicks main's layout; this branch
  runs those scenarios with the dev hook `CARTWAVE_BROWSER_W=240`, so the coordinates hold and
  the arrangement has main's width (the same work measured); the new scenarios (the device panel
  while playing, the browser searched and scrolled, ◀ ▶ stepping) run at the real 420 px.
  Cycle counters per scenario come from each run's audio log (one row per second) averaged over
  the scenario's window (`bench_pb.ps1` writes them; `perf_summary_pb.py` judges by §2's rule).
- **PB-D18 (a drawing bug the regression gate found).** On a locked track the device's meter
  layer was opened with a `save` before `save_layer`, and closed once: one canvas level stayed
  open, clipped to the meter, so everything drawn after the device in that frame went nowhere.
  The window stopped showing updates (the display kept the old time after a click, the insert
  marker didn't move on screen) until something redrew the whole window. Found by the regression
  gate's "inspector" state (main 0:01.40, this branch 0:00.00), located by replacing the device
  with an empty element (the display updated), fixed (one save or layer, one restore). The suites
  had passed: their checks read dev lines and saved files, and the screenshot tool moves the
  window, which repaints it.
- **PB-D19 (gates 1, 4, 5, 7).** Gate 1 (`identity_pb.sh`): 299 project files, render, export
  and channels identical to `main`; 101 game-export files byte-identical (a first run listed 299
  "can't open" lines, MSYS paths the Windows binaries can't read: identical and useless; caught by
  reading the file, fixed with `pwd -W`). Gate 4: `cargo test` 299 passed, 0 failed, 2 ignored in
  release and debug, clippy clean, 0 warnings. Gate 5 (`audio_pb.ps1`, the demo 600 s at 256
  frames, ▶ every 19 s through 30 presets via the dev hook `CARTWAVE_AUTO_STEP`): 0 match
  dropouts, 0 mismatch runs, 0 capture gaps / foreign / tap lost, 28,798,230 samples compared,
  starved 0, late 0, 0 allocation violations; the 30 steps one undo step. Gate 7: lime 0 in 18
  scenes (a lime-free locked fixture: `fx_board`'s track colours hold lime, so its first run
  counted the project's colours), contrast ≥ 4.61.
- **PB-D20 (final suite run).** On the final build: shell 49, design 23, polish 14, polish 2 92,
  4a 53, 4b 43, PB 13 all pass; Batch 3 77/79 with AT-29 failing once. Its screenshot shows the
  Hardware section open but NES (2A03) not expanded, and the log has no "browser open" line: the
  press on the folder row never reached the browser (a left press on a folder row toggles it and
  logs synchronously, no frame in between, so the app has no path that drops it). Same layout as
  the passing runs (screenshots compared). AT-29 alone 3/3 PASS, then the whole Batch 3 suite
  79/79 PASS on the same build. Recorded as one unexplained input miss in the harness, not hidden:
  both logs are in `docs/presets/suites/`.
- **PB-D21 (perf gate, first run: FAIL, and a bench fault).** The first 10 rounds failed idle
  (+1008 k/frame, SE 276), overview_drag (+3570), overview_zoom (+2836) and s64_insert_delete
  (+1545). Two causes, one in the bench and one in the build:
  1. **The bench aimed at main's places.** 200drag opens the bottom pane, 124 px taller here;
     the scrollbar drag (y 585) then pressed the device's preset field and the header-width drag
     (y 600) dragged inside the device: their -3218 and -9117 meant "nothing scrolled", and the
     overview drags after them ran in that state. `bench_pb.ps1 -PaneGrow 124` moves both up on
     this branch; screenshots of both starts per build in round 1; per-run stderr kept
     (`err-<label>-*.txt`). Two check rounds: scrollbar_drag 13.9 / 13.3 % CPU, header_width
     22.1 / 23.1 %, overview_drag's frames 472 / 491 µs draw: like for like.
  2. **The device's cached image was a CPU raster** of the whole board (912 × 364 px, text,
     gradients): 13–35 ms the frame the selected track changed (main's inspector: 5–7 ms), so
     insert / delete dropped a frame or two each time. Now a GPU surface from the window's canvas,
     as the arrangement's, the Overview's and the piano roll's layers: s64_insert_delete +56 k
     (SE 74) in the check rounds. clippy clean, `cargo test` 299 passed.
  The summary printed its table twice (`set(cyc) | set(cpu) - set(order)`: precedence); fixed.
  The gate reruns in full.
- **PB-D22 (perf gate, second run: PASS; merge).** 10 rounds with the bench fixed (PB-D21): every
  scenario passes the §2 rule. One branch run (pb-5) shows no Overview hover in its kept log (its
  drags were over the top bar's Display: 2.9 ms draw, layout every frame, like the check run's
  outlier); `perf_summary_pb.py` now leaves a run out of overview_drag / overview_zoom when its
  log lacks the hover, and names it. Both figures reported (+1087 SE 1067 with it, +20 SE 47
  without: a pass either way). Idle process CPU 0.5 → 1.3 % (context; the counters are flat).
  After the device's GPU cache: mockup 47/47, lime 0, contrast 4.61, PB 13/13, 4b 43/43,
  `cargo test` 299 passed in release and debug, clippy clean, 0 warnings. All gates pass:
  `pre-presets` tags the old main; main fast-forwarded.

## Batch 5: automation, plus four fixes from the Presets review (BATCH5_PROMPT.md, branch `batch5/automation`, Windows PC), 2026-11-28

- **B5-D1 (pre-flight).** `main` = `origin/main` = `presets/browser` = 4529f1e (the Presets +
  Browser batch merged; `pre-presets` exists). Branch `batch5/automation` from it. Baseline: `main`
  exported with `git archive` to `../before-batch5` and built there (release, with examples).
  Machine: Windows 11 Pro 26200 (not macOS).
- **B5-D2 (conflicts between the prompt and VISION.md; VISION.md wins).**
  (a) **Deleting a breakpoint.** The prompt (B3.1) says "double-click or Delete"; VISION.md
  ("Automation", decided 2026-11-25) and UX.md L4 say a **single click** on a breakpoint deletes it,
  as the reference does (manual 26.7.1, R108). Built: a click on a breakpoint deletes it (and joins
  its neighbours), Delete removes the selected breakpoints / the time selection's envelope, and a
  double-click away from the line adds a point (R106).
  (b) **The private folder.** The prompt (A4) says `private/`; VISION.md ("Reference listening")
  says the fan MIDI lives in **`reference-private/`**, git-ignored. The files go there; both names
  are in `.gitignore` (so a stray `private/` can't be committed either).
- **B5-D3 (A1: the mixer and the bottom pane).** In the mixer view the bottom pane (the Detail
  View) is hidden by default; **Shift+Tab** shows or hides it there (`AppState::mixer_detail`,
  remembered for the session); in the arrangement Shift+Tab still switches the Clip and Device
  views (unchanged). Shown with nothing selected, the status bar says to select a strip.
  Gate (`docs/batch5/tools/regress_b5.ps1`, `diff_b5.py`): the mixer at 100 % and 150 % against
  `pre-presets` (`../before-presets`): **PASS** at both; the only change outside the browser at
  100 % is the strips' scrollbar thumb, shorter because the strips' viewport is 180 px narrower
  (labelled); 150 % has none. A strip clicked in the mixer now leaves the strips' PEAK / VOICES /
  readout row visible (`docs/screens/batch5/mixer-pane-hidden.png`; `pre-presets` opened its
  232 px inspector over them). Reports: `docs/batch5/regression/a1-diff-*/`.
- **B5-D4 (A2: three quiet classic presets).** *How a project stores an instrument:* a full
  copy (`file.rs` `inst_to_file`: the volume, arpeggio, pitch and duty sequences, vibrato and arp
  speed, per entry of the project's instrument table; version 1 files have none and play raw
  channels). Nothing re-reads a preset by name when a file loads, so changing the presets in place
  can't change an old project: A2 goes ahead. Checked: `identity_b5.sh` (every `.chipdaw` /
  `.cartwave` in the repo rendered and exported by `main` and by this build) **299 of 299
  identical**, game export **101 of 101 byte-identical** (`docs/batch5/identity/a2/`); `cargo test` all pass.
  *The change* (the bank's mechanism, `design_bank.py apply_level`: the channel Volume where it has
  room, then the volume macro scaled up only as far as its loudest step allows, so the envelope
  keeps its shape; here both, as each alone falls short): the three presets now set **Volume 15**
  (it was the track's default 12) and their volume macros are scaled to a loudest step of 15
  (Pizzicato ×15/14, Music Box ×1.5, Menu Blip ×1.25, rounded half up). Duty, pitch, arpeggio,
  vibrato and the macros' lengths are unchanged. *Result* (`bank_pack`, studio Pulse voice):
  Pizzicato −33.4 → **−31.6 LUFS** (target −26.3, still −5.3 LU), Music Box −34.7 → **−30.3**
  (−4.0 LU), Menu Blip −37.3 → **−33.9** (Game Sounds −30.1: −3.8 LU). True peaks −15.2 … −15.5
  dBTP, no clipping. **None reaches ±1.5 LU**: each is at its voice's maximum (Volume 15, the
  loudest macro step 15); going further would flatten the envelope against 15 (changing the
  pluck's character) or change the duty or voice. The remaining gaps are reported for Mika (a
  thin 12.5 % / 25 % pulse with a fast decay is quiet by nature). The WAVs, `levels.md` and the
  three PROVENANCE rows (a dated change line) were rewritten by `bank_pack --write`.
  *A tool fault found on the way:* `bank_pack --write` also rewrote `presets/levels.json`, whose
  gains are iterated (each pass multiplies a refinement in, for `design_bank.py --levels` to fold
  in): a plain re-render moved 30-odd gains with nothing folded in. Restored, and the file is now
  written only with `--iterate`. `--measure "A,B"` prints chosen presets' levels.
- **B5-D5 (a mistake, and the clean branch).** A2's first commit (1fea698 on
  `batch5/automation`) also added the identity run's output folder: every game-export WAV of both
  builds, **1.4 GB**, and it was pushed. The rules forbid a force-push or a history rewrite, and a
  fast-forward of `main` would carry those blobs into `main`'s history for good. So
  `batch5/automation` is left exactly as pushed (A1 + that commit; not deleted) and the batch
  continues on **`batch5/automation-2`**, made from `main`: A1 cherry-picked, A2 committed again
  with only the hash lists and a summary. **For the owner:** `origin/batch5/automation` holds the
  1.4 GB commit and nothing else of value; deleting that remote branch lets GitHub drop the blobs.
  Guards: `identity_b5.sh` refuses an out dir inside the repo, `.gitignore` covers
  `docs/batch5/identity/**/main|b5/`, and heavy outputs go to `../b5-work/`.
- **B5-D6 (A3: studio voices' baseline steps).** *Cause (read in the source):* a studio voice
  (`chip::voice::Voice`) adds its channel's DAC level relative to its power-up level, so the
  output is **unipolar**: a pulse swings 0 … +L, carrying a DC offset of duty × L for as long as
  the note sounds (the noise half its level, the saw and triangle their averages, a DPCM sample
  its start level). The studio bus filters it with the project's output stage, which in a
  studio-only project is `FilterMode::Off`: a **5 Hz one-pole DC blocker** (τ ≈ 32 ms), too slow
  to keep a 50 ms mean flat at note starts and stops. Locked tracks go through the console's
  37 / 90 Hz high-pass, hence flat. A per-voice 5–10 Hz blocker (the prompt's other option) would
  still leave about a third of each step in a 50 ms mean, and a voice writes band-limited steps,
  not samples. *Fix:* **bipolar voices.** Each voice subtracts its waveform's own mean
  (`Voice::dc`): the duty's share of the volume (2A03 pulse: ones in its duty row / 8; VRC6 pulse:
  (duty + 1) / 16, or all of it in its constant mode), half the volume for the noise, the 32-step
  average for a running triangle (a halted one holds a step: all of it), the saw's 7-state
  accumulator average for its rate, a DPCM sample's start level while it plays (the level it
  stopped at after). The mean changes only with the volume, duty, rate or gating, so it is one
  more band-limited step there; it is recomputed only after register writes and frame-counter
  clocks (timer clocks move along the waveform only), so the cost is nil: the 64-track studio
  stress song renders in 2.45–2.57 s against `main`'s 2.50–2.53 s (5 alternating runs). An idle or
  halted voice contributes exactly 0 (`Voice::silent`), so a pool still grows without a step.
  Deterministic (plain arithmetic), no allocation. Test: `voice::tests::studio_voices_are_bipolar`.
  *Results* (`docs/batch5/a3-dc.md`, `bank_pack --dc`, every preset on a studio voice): the
  50 ms swing is ≤ 0.005 for **27 → 141** of 202 presets, larger for none; studio worst 0.18 →
  0.041; peak balance median 0.52 → 0.88 (Supersquare −0.09 / +0.34 → −0.12 / +0.30, Square Lead
  0.56 → 0.99); the DC over each whole sound ≤ 0.0006 for every studio preset (was up to 0.0082;
  `dc_overall.py`); no clipping. **Not met for 61 presets (reported):** the 54 over 0.005 and
  outside their locked twin's own swing are basses (41–110 Hz fundamentals), noise drums, noise
  FX and DPCM hits, where a 50 ms window catches the waveform itself: the locked renders of the
  same kinds measure the same way (triangle bass 0.006, VRC6 saw bass 0.017, DPCM 0.011) and their
  DC is ≤ 0.0006. Thin pulses stay lopsided by nature (12.5 %: +7/8, −1/8). **Loudness:** within
  ±0.3 LU for 196 of 202; exceptions Thump Kick −0.56, Noise Explosion (its studio render) +0.35,
  Boom +0.32, Static Crackle, Sampled Snare and Sample Snare −0.30 … −0.31. *Locked tracks
  bit-identical:* all 299 project files with their studio tracks muted (`corpus_hashes`' new
  console column). *Re-baseline* (this commit only): the studio goldens and migration hashes,
  old → new with the reason in `docs/batch5/a3-rebaseline.md`; the listening pack and
  `levels.md` re-rendered.
- **B5-D7 (A4: the private reference render).** `.gitignore` got `/reference-private/` and
  `/private/` before anything was written (B5-D2: VISION.md names the folder
  `reference-private/`). No `.mid` was there, so one fan transcription was downloaded into it
  (its URL in `reference-private/SOURCE.txt`, ignored like everything there). A generic tool,
  `crates/ui/examples/midi_render.rs` (names no song; writes where it is told), imports a `.mid`
  through the app's own MIDI import and renders it: with a profile, **Fit to hardware** and an
  authentic preset per locked track (in turn from the Hardware folder of its voice); with
  `--studio`, studio tracks and Sounds / Drums presets that fit. **A bug found on the way (4b's
  MIDI import):** under 2A03 + VRC6, `channels_for` plans a third and later pulse part onto the
  VRC6 pulses and the saw, but `import` made each one a Pulse track, which can't lock to those
  channels, and the `_ => {}` arm left it a studio track with nothing in the report. Now a Fit
  track takes its channel's voice, and a part that still can't lock says so. Test
  `midi_tracks::tests::fit_puts_extra_pulse_parts_on_the_vrc6`. Before every push: `git status
  --ignored` shows `reference-private/` ignored and `git ls-files` lists nothing under it.
- **B5-D8 (automation: the engine).** `chip::auto`: points (tick, value in the target's units,
  the curve of the segment after it), evaluation (hold before the first and after the last; two
  points on one tick make a jump), a rational bias curve `x / (x + (1 − x)k)` (no `pow`: the
  same on every machine) and quantising to register steps. The driver evaluates each console
  lane's envelopes **once per frame tick** into `LaneAuto` (channel controls in place of
  `params`, macros in place of the instrument's: volume or the triangle's gate, arpeggio, duty,
  pitch, vibrato depth); with no envelope every path is the old one (all goldens unchanged).
  Studio voices get the same `LaneAuto` from the rack. **Studio fader and pan**: evaluated at the
  frame tick, then a **4-step ramp at fixed CPU cycles** (1861 cycles, about 1 ms apart), so a
  render is identical with any audio block size (the prompt says "per block"; a device block
  would make renders depend on the buffer size, which the determinism rule forbids; the frame
  tick is the fixed block). **Main gain**: evaluated at the frame tick, applied from the sample
  that tick falls on (its absolute index) with a fixed 200-sample ramp. Tests: block sizes 64,
  333, 480, 1000 and 4096 give bit-identical renders; `process` with automation on every kind of
  target runs under the allocation checker (debug aborts on any allocation).
- **B5-D9 (automation: the model and format 10).** `ui::automation`: envelopes keyed by owner
  (a track's id, or Main) and the registry id (the one list; `track.active`, "Track On", added
  for studio tracks: the prompt's track activator). Values in the parameter's units (the fader's
  −inf stored as −70 dB). Envelopes are document (undo: `Doc` holds them, shared); the lanes
  each track shows are saved but not undoable (as the reference); overrides are session state.
  What reaches the engine: only parameters the owner has now (a locked track's fader / pan
  envelope is kept, unheard, until it is unlocked; parameters the channel lacks are skipped) and
  never an overridden one. **Format 10**: `automation` (track id or Main, param, points
  `[tick, value, curve]`) and `auto_lanes`; nothing written when empty, so a version 9 file saved
  as 10 differs only in its version. v9 corpus (`docs/songs/v9`, written by `main`'s build in
  `../before-batch5`, `v9corpus.rs.txt`); its hashes from `main` and from this branch differ only
  for the three files with studio tracks (A3), `HASHES.txt` lists both; `migration.rs`
  `version_9_files_render_bit_identically_in_version_10`, `au10_format_10_round_trip`.
- **B5-D10 (the game export: a conflict noted).** The prompt (B5.4) asks for locked-track
  automation as "per-frame register writes" in game exports and a warning for studio lanes.
  Cartwave's game export is **audio** (seamless WAV loops, stems, a JSON sidecar); exporting to
  hardware music formats is out of the first build (VISION.md "Not in the first build"). So:
  locked-track automation reaches the audio *as* the chip's per-frame register writes (the
  console's driver plays them: `au07_locked_lanes_play_the_quantised_envelope_frame_by_frame`
  checks the written registers frame by frame from a project's snapshot), studio automation is
  rendered too, and the export's report (`Report.notes`, the existing pattern) names every
  studio track and lane whose automation has no hardware equivalent (`au09`). The loop stays
  seamless with automation (intro + loop × 3 = a continuous render).
- **B5-D11 (automation: the UI).** *Automation Mode* (A, R104; View ▸ Automation Mode, the
  shortcut table): each track shows its lanes under its row, each one row height (the headers'
  row heights; `Arrangement::row_pitch`, `lane_top`, `lane_at` follow them); a track showing none
  gets its first automated parameter, else its first one. *Lane headers* (a view under each
  `TrackRow`): "Device › Parameter" with a drawn chevron (Inter has no ▸ / ▾: the glyph rule),
  the value at the playhead, − (hide) and + (another parameter); the chooser lists the track's
  registry with "(automated)" marks and **Show Automated Parameters Only**. *Editing*
  (`views/autolanes.rs`, the reference's breakpoint behaviour; B5-D2 for the click-delete): a
  click on the line adds a point, a click on a point deletes it, a drag moves it (grid snap;
  Alt off the grid; Shift fine vertical), Alt-drag on a segment bends it, a drag elsewhere selects
  the lane's time (Ctrl+C / X / V and Delete act on its envelope; a drag inside it moves it), a
  double-click away from the line adds a point; **Draw Mode** (B, the one Draw Mode the app
  already has, R5) draws steps at the grid. Each gesture is one undo step. *Locked lanes* draw the
  frame-quantised staircase the chip plays; studio lanes the line with its curves. *Main*: its
  gain's envelope is drawn and edited over the Main row. *Colours* (tokens.rs; VISUAL-SPEC §14):
  line and dots `#D6D0C4` (1.5 px, 6 px), overridden `#5A5A5A`, a neutral white selection, lanes
  `#111111`, labels `#A8A8A8` (7.9 : 1). *Lock Envelopes*: **Edit ▸ Lock Envelopes** (a check
  item, saved in settings): the reference keeps it in its Options menu, which VISION.md's menu
  bar doesn't have; Batch 4a put the track commands in Create for the same reason. *Transport*:
  **Automation Arm** (a toggle; on, its envelope icon glows warm white: the prompt's "no lime
  anywhere in automation" wins over "lime for toggle on-states") and **Re-Enable Automation**
  (orange `#FF8A1F` while a control is overridden). *Recording* (armed and playing): a control's
  moves write points at the playhead while it's held, replacing what was there since the
  gesture's last point (touch); one undo step per gesture. *Overriding* (not armed): moving an
  automated control overrides it (grey lane, the dot grey, Re-Enable lit) until Re-Enable; the
  manual value plays (the envelope is left out of the snapshot). *Markers*: a 4 px white dot on
  automated device knobs (channel controls, Unison / Detune / Drift) and the header's fader and
  pan; grey when overridden.
- **B5-D12 (harness changes, gate 3).** Every earlier suite runs with a new dev hook,
  `CARTWAVE_NO_AUTO_KEYS=1`: the transport's two automation keys are left out, so the top bar's
  coordinates (the display, TAP) are the ones those suites were written for (`run_suites_b5.ps1`;
  the PB batch did the same for the browser's width, PB-D12). Format 10 in two fixture checks:
  `docs/batch4a/tools/unlock_all.py` refused format 10 (so polish 2's fixtures stayed locked and
  P2-01's "−3 dB" on a pinned fader failed) and `docs/polish2/tools/check_p2.py`'s "saved by this
  build". Results on the final UI: shell 49, Batch 3 79, design 23, polish 14, polish 2 **92** (69
  before the two fixes), Batch 4a 53, Batch 4b 43, PB 13, and the new **AU 18 scenarios**
  (`docs/batch5/tools/at_b5.ps1`: AU-11 … AU-18 in the app: add / move / curve / delete, Draw
  Mode, a lane's time selection, clips carrying their automation with Lock Envelopes off and on,
  arm and touch recording with one undo, override and Re-Enable, locked lanes without fader / pan
  and stepped, the mixer's Shift+Tab), all passing. The AU unit tests (AU-01 … AU-10) are cargo tests.
- **B5-D13 (gates 1, 2, 4, 6).** Gate 1 (`identity_b5.sh`, the final build against `main`): 308
  project files, 242 identical; the 66 that differ all have studio tracks (A3's 63 + the v9
  corpus's 3 studio files); **every file with its studio tracks muted is identical** (locked tracks
  bit-identical everywhere); game export 101 files, all byte-identical to the A3 build (68 to
  `main`, the rest studio stems changed by A3 only). Gate 2 (`regress_b5.ps1`, `diff_b5.py`, 17
  states, 100 % and 150 %, Automation Mode off): **PASS**; the only changes are the top bar (the two
  new keys shift the display, TAP and the meters box), the Edit and View menus' new items, the About
  panel's build line, and the window's frame (its accent colour follows focus; left out); `main`'s
  own 150 % `mixer_selected` capture differed by 2 px from its re-shoot (baseline noise; the
  re-shot pair is identical). Gate 4 (`audio_b5.ps1`, 600 s of `auto32`: the demo band with 32
  automated parameters on 8 tracks, 4 studio and 4 locked, plus Main; a studio fader overridden
  and re-enabled every 19 s, 16 + 15 times): **0 dropouts**, 224,981 blocks matched, 0 mismatched,
  28.8 M samples compared, starved 0, late 0, **0 allocation violations**. Gate 6: lime **0** in 8
  automation scenes (`lime_scenes_b5.ps1`), the new labels **7.94 : 1** (overridden value 4.98).
- **B5-D14 (gate 5, performance: a regression found and fixed; a harness bug).** The first 10-round
  run (`perf_b5.ps1`) **failed** 7 scenarios on the UI thread, all in the arrangement: the 64-track
  ones +513 … +1,217 k cycles/frame (SE 43 … 58), `ruler_zoom` +255 (96), `pr_dragging` +363 (39);
  audio no worse anywhere (−55 … +2). Cause: every track row built an `AutoLaneHeaders` view even
  outside Automation Mode (zero height, but bound to the arrangement and to the transport, so 64
  views redrew on every playhead frame, each with its own height memo), and `automation::mark`
  built a `String` key for every row's draw. Fix: the rows' rebuild key is now (track count,
  Automation Mode) and the lane headers exist only in the mode (outside it the row views are
  `main`'s); `mark` returns at once when the song has no envelopes. One pair after the fix:
  `s64_playing` −64, `s64_scroll` −23, `s64_insert_delete` +117, `s64_header_drag` −373,
  `ruler_zoom` −52. Harness bug: `$plain = if (…) { @("-Plain") }` unrolls to a string and
  splatting it passed "-", "P", … as positional arguments, so the first run's plain (unautomated)
  automation baselines never ran; `[string[]]$plain` fixes it, and `perf_auto_b5.ps1` runs the
  automation scenarios' plain-vs-automated rounds on their own.
  **Second run** (10 rounds, the fixed build): every main-vs-branch scenario passed except
  `pr_dragging`; the automation scenarios failed on the UI thread (`auto_scroll` +1,012,
  `auto_timesel` +1,400, `auto_draw_2000` +450 k/frame). Reading the proof lines then showed the
  harness itself was wrong in four ways, all fixed before the third run:
  1. **The piano roll's song was format 10** (`make_songs` from this branch), so `main` refused it
     and opened its default song: every `pr_*` pair compared two different songs (`main`'s lines
     said "NOT REACHED"). Now `main`'s `make_songs` writes it (format 9, both builds open it).
  2. **The earlier scenarios ran at the wrong coordinates on both builds.** The Presets batch ran
     its branch with the browser at the old 240 px and the pane 124 px taller (`perf_pb.ps1`'s
     `-BrowserW 240 -PaneGrow 124`) so the old scripts' coordinates held; `main` now has both, and
     `perf_b5.ps1` passed neither, so `s64_header_drag` dragged the top bar's Display and no track
     moved ("64-track scenario NOT REACHED" on both sides). Both builds now run with the hooks.
  3. **`perf_summary_b5.py` left unproven runs out of the CPU % columns only**: their cycles still
     made the verdicts. It now drops each unproven scenario's cycle samples and lists them.
  4. **`auto_timesel` was not a time selection on either side**: in the plain song an empty lane's
     press adds a breakpoint, and in the automated song y 214 hit the Duty lane's line (547 → 548
     points). A new dev line (`dev: automation time selection …`) is now the scenario's proof; the
     press moved to the lane's step-0 band (below the line in both songs), and the plain fixture
     keeps one flat envelope on that lane so the same press selects time there.
  Real costs found on the way, fixed: `Envelope::value_at` copied the whole point list into a new
  `Vec` on every call, and the locked lanes' stepped drawing calls it once per pixel column
  (`chip::auto::eval_by` now evaluates the editor's points in place; the engine's `eval` is the same
  code); `content_hash` ran SipHash over every point on every arrangement change (now a
  multiply-fold, `Envelope::points_hash`); each lane's line and dots are cached as paths in
  band-local coordinates, rebuilt only when the envelope, the view or the geometry changes. Two
  pairs after these: `auto_scroll` +1,041 → +8, `auto_playing` +100 → +18. Investigated without a
  fix: with Automation Mode on and the song playing, the automated song still costs ~+50 µs of
  event processing and ~+130 µs of GPU flush per frame against the plain one; switching parts off
  (an env toggle per part, removed) put most of the flush in the lane headers' drawing, but
  neither redrawing them only on a value change, nor at 20 Hz, nor rounding their readout's time
  changed it (all reverted). samply could not be used (its Windows trace never finished).
- **B5-D15 (gate 5 fails on e265df8; a bug and a cost found; one rerun fixed in advance).** The third
  10-round run (build e265df8, `../b5-work/perf3`) **fails `auto_draw_2000`**: UI +267.6 k/frame
  (SE 58.1; limit 235). Every other scenario passes; the proofs hold in all 20 automation runs; the
  one omission is `overview_drag`/`overview_zoom` in b5 round 7 (no Overview hover in its log).
  The excess is in the events phase (+51 us a frame, about 240 k cycles); draw and custom are level.
  Cause: `points_hash` took **6.1 us** on 2,141 points (one serial multiply chain; the "about 1 ns a
  point" in its comment was wrong) and runs several times a frame; `draw_steps` took **13.9 us**
  (copies and a full sort). Now 1.1 us and 1.3 us (`bench_edit_costs`, ignored test).
  **Reading `paste` for this found a correctness bug, not a performance one:** the edge values were
  evaluated after the edge points were removed, so Draw Mode and paste changed the envelope
  *outside* their range (0,100,200 = 0,1,0 pasted at 90..110: tick 45 went 0.45 to 0). The earlier
  AU-02/AU-03 tests only used flat outsides. New test: `au03_paste_and_draw_keep_the_outside`
  (fails on e265df8). As Batch 4c's log says, a second sample of the same build would be choosing
  the sample; this is a new build, so **every gate reruns on it, and the 10 performance rounds run
  once: whatever they say is the result.**
