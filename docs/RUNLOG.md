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
