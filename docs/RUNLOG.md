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
