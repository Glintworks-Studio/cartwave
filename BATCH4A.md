# BATCH4A.md: tracks are instruments, hardware profiles and locks, the Main track

Branch **`batch4a/model`** (from `main` 0e669ac), pushed. Prompt: `BATCH4A_PROMPT.md`. Decisions:
[`docs/RUNLOG.md`](docs/RUNLOG.md) **B4A-D1…D27**. Spec: [`VISION.md`](VISION.md),
[`UX.md`](UX.md); [`docs/design/VISUAL-SPEC.md`](docs/design/VISUAL-SPEC.md) §4b.

## 1. Summary (one screen)

> **2026-11-26: merged.** `main` is at d3ac25f, this branch's head (the owner merged it; RUNLOG
> DOC-D1). The text below is the run's report as written, including its performance-gate finding.

**Merge: not done.** `main` stays at 0e669ac and no `pre-batch4a` tag was made: the performance
gate (§9.6) does not pass by its letter. Every other gate passes. See §3.

| # | Gate | Result |
|---|---|---|
| 1 | Bit-identical audio and exports | **PASS**: golden render and every v2–v6 → v7 migration hash equal; the showcase game export's 11 WAVs byte-identical (the JSON gains `master_gain_db`, §7) |
| 2 | Visual regression, 100 % and 150 % | **PASS**: 0 px outside the allowed regions in 11 states at each scale ([report-100](docs/batch4a/regression/report-100.md), [report-150](docs/batch4a/regression/report-150.md)) |
| 3 | Main row within 1 px | **PASS 45/45** in five layouts, measured against the prompt's §7 text: **the CEO's reference board was not provided** (B4A-D1, D15) |
| 4 | Earlier tests + AT-13 + B4A | **PASS**: AT/DP/PL suites, P2 suite (P2-11 has a pre-existing intermittent failure, B4A-D27), B4A-01…09 |
| 5 | `cargo test`, clippy, warnings | **PASS**: 238 passed, 0 failed, 2 ignored ([full list](docs/batch4a/test-list.txt), [run](docs/batch4a/test-run.txt)); clippy clean; release build 0 warnings |
| 6a | Audio | **PASS**: 30 min, 64 studio tracks / 512 voices at 256 frames: 0 starved, 0 late, **0 pilot dropouts** at Main −40 dB, callback p99 **9.7 %**, 0 allocation violations (B4A-D21) |
| 6b | Performance | **NOT PASSED** by the letter: core scenarios pass (demo +0.0, 200 clips −0.1, drag −1.5), but piano roll playing **+1.0**, zoom wheel +0.5, window resize +2.0 (medians of 5); 60 fps, 0 stalls everywhere |
| 7 | Lime audit, contrast | **PASS**: lime 0 violations (10 scenes + the new UI's shots); new text ≥ **4.85 : 1** |

**Pre-flight.** `main` = `origin/main` 0e669ac, with `mac/catchup` merged. The baseline was built
from it in `../before-batch4a`.

**What changed.**
- **Fixups (§3):** keys never reach the song behind a dialog (MC-D9). Colour previews never reach
  undo. The "▸" glyph is gone (Inter lacks it, and the lock is drawn). Ctrl-click toggles a clip
  and Shift-click selects a range (R28).
- **Tracks are instruments (§4).** Each track is either studio or locked:
  - **Studio:** its own voice pool. Tracks and polyphony are unlimited, with a Voices cap and
    quietest-voice stealing; the pool grows from the UI thread.
  - **Locked:** one real channel through the full emulator. The last note wins, and the fader and
    pan are fixed.
  - Studio voices are the chip's own channel units driven by the driver's lane code, so there is
    no second emulator.
  - **Format v7.** Older files open with their tracks locked and render bit-identically.
- **Profiles and locks (§5):**
  - Profiles: None, NES (2A03), Famicom + VRC6.
  - The channel budget shows in Project Settings and in the TRACKS cell, red when over.
  - A badge opens the lock menu; taken channels are disabled and name their owner.
  - "Reset and lock"; the locked header state.
  - Profile warnings offer Unlock / Fit (or Switch anyway) / Cancel and "Don't ask again" (a real
    Windows dialog).
  - An old file over budget is a status-bar problem.
- **Tracks:** Create > Insert Track (Ctrl+Shift+T) and Insert Track > a type, Duplicate, Delete,
  rename, drag a header to reorder, Voices >. Each is one undo step.
- **Parameter registry (§6)** for Batch 5: [`docs/batch4a/parameters.md`](docs/batch4a/parameters.md).
- **Main track (§7):**
  - Pinned at the bottom: a black strip, OUT + device, PEAK, a dark lane.
  - Its master gain applies after the whole mix: saved, written to the export JSON, and
    bit-identical at 0 dB.

**Studio vs locked:** one note sounds the same (+0.000 dB, measured). Chords differ by the
console's non-linear mixer: two pulses at full volume are 1.26 dB louder on studio voices.

## 2. Decisions taken alone (RUNLOG B4A-D*)

1. **Track commands in a Create menu,** not a new "Track" menu (D12). VISION.md's menu set wins
   over the prompt; the conflict is logged.
2. **Migration (D8):**
   - A 2A03+VRC6 file with no VRC6 clips opens as NES and keeps its filter.
   - Empty VRC6 lanes under NES become studio tracks.
   - A v6 fader that isn't 0 dB is locked like any other. It is reported, and its old value is
     kept in `before_lock`.
3. **File > New is studio-only** (D9), and studio macros run on the frame tick (D11).
4. **Unlock is offered only where it fixes the conflict** (D13); layouts are the authentic presets
   (D14).
5. **The Main row is built from the prompt's text** (D15), because its board is missing. Black is
   reserved for it (D16).
6. **Delete from a track header** (D18): it acts on the logical key, so numpad Delete works.

Process decisions (all in RUNLOG):
- D19: autoplay with a test project never played. This bug predates the batch; it is fixed.
- D21: at full volume, 512 voices clip the output and the pilot tone is lost in it, so the audio
  gate was judged at Main −40 dB.
- D22–D26: the performance work (§3).
- D27: the polish 2 suite now runs on unlocked tracks.

## 3. Why there is no merge: the performance gate

§9.6 asks for "the POLISH2 scenarios within +0.3 points of the baseline". The final run
([`docs/batch4a/perf/final/summary.md`](docs/batch4a/perf/final/summary.md)) is 5 interleaved
rounds per build. Every proof was reached, focus held throughout, and pacing was 60.0 updates/s
with 0 stalls.

| Scenario | Baseline | Batch 4a | Δ median | Δ mean |
|---|---|---|---|---|
| idle / demo / 200 clips / 200-clip drag | 0.0 / 7.8 / 8.3 / 11.8 | 0.0 / 7.8 / 8.2 / 10.3 | +0.0 / +0.0 / −0.1 / −1.5 | |
| row resize / header width / ruler zoom / overview drag / overview zoom | | | −0.7 / −1.8 / +0.2 / −0.4 / −0.3 | |
| **zoom wheel** | 16.5 | 17.0 | **+0.5** | +0.92 |
| **scrollbar drag** | 14.9 | 15.5 | **+0.6** | +0.08 |
| **window resize** | 35.7 | 37.7 | **+2.0** | +0.48 |
| **piano roll, playing** | 6.0 | 7.0 | **+1.0** | +0.84 |
| **piano roll, dragging** | 7.7 | 8.2 | **+0.5** | +0.32 |

**These numbers are noisy.** A single scenario swings 2–7 points between runs of the same build
(window resize, baseline: 33.1–40.8), and the same scenarios came out negative in earlier runs.
Even so, the five rows above lean positive across repeated runs. My reading: a real residual of
about 0.3–0.8 points remains in whole-view redraws, from the Main row and the busier header rows
drawing every frame. That is a judgement, not a pass.

**What the gate found and fixed along the way** (each one profiled, not guessed):
- **D23:** each locked row drew its disabled controls through an offscreen layer every frame.
  Now cached images.
- **D25:** the per-tick commit built the document twice. Now once.
- **D26:**
  - A note drag resolved the edited clip three times per frame. The snapshot now shares its
    resolved clips.
  - The rack check now waits for the edit to finish.
  - `resolve_poly` was O(pitches × notes). Now O(n log n), pinned to the old output by a test.
  - The Main lane redrew its grid every frame. Now a cached image.

Two harness faults also invalidated earlier numbers:
- **D22:** injected keys went to a terminal, and 54 stray shell tabs were closed afterwards.
- **D24:** the "scrollbar drag" was dragging inside the inspector, in polish 2's bench too.

**To merge:** either accept this residual (a decision for the owner), or give the gate a budget
the measurement can resolve, for example +0.3 on a 10-round mean. A next step on the code side
would be caching the Main row's header and meter the way the track rows are cached.

## 4. Gate details

- **Audio** ([`docs/batch4a/audio`](docs/batch4a/audio)):

  | Run | Length | Dropouts | Callback cost | Other |
  |---|---|---|---|---|
  | S64 at Main −40 dB | 30 min | 0 | mean 5.10 %, p99 9.7 %, max 13.0 % | 0 allocation violations |
  | DEMO | 2 min | 0 | p99 0.6 % | |
  | S64 at full volume | 30 min | 45,285 pilot dropouts | | 0 starved, 0 late |

  - The full-volume dropouts came 8–40 in *every* second: steady clipping, not stall bursts.
  - Offline, 512 voices render at 4.2 % of real time (`crates/ui/tests/stress64.rs`).
  - WASAPI shared mode gave 480-frame callbacks for the 256 requested.
- **64-track scenarios** (new, this build, % of one core, median of 5): playing 18.0, scrolling
  38.4, inserting and deleting tracks 29.0, dragging a header 20.9.
- **Visual regression:** `docs/batch4a/tools/diff_b4a.py`. Allowed regions: badges, locked
  controls, the TRACKS cell, the Main row and the lanes' scrollbar (the lanes are 73 px shorter).
  [`regression-diff.png`](docs/screens/batch4a/regression-diff.png).
- **Main row:** [`docs/batch4a/mockup-match.md`](docs/batch4a/mockup-match.md),
  [`mockup-match.png`](docs/screens/batch4a/mockup-match.png).
- **Lime:** [`docs/batch4a/lime-audit.md`](docs/batch4a/lime-audit.md) (10 scenes) and
  [`lime-audit-new-ui.md`](docs/batch4a/lime-audit-new-ui.md). The 14 flagged pixels are the
  allowed loop line, displaced by the desktop crop of the dialog shot.
- **Contrast:** [`docs/batch4a/contrast.md`](docs/batch4a/contrast.md), minimum 4.85 : 1.
- **Suites:** [`docs/batch4a/suites`](docs/batch4a/suites).
  - The old suites: 165 checks pass.
  - P2: 91 pass. P2-11 failed once on an intermittent typed value and passed on rerun; the
    baseline fails the same way 1 run in 5 (D27).
  - B4A-01…09 all pass.

## 5. Screenshots

[`docs/screens/batch4a/`](docs/screens/batch4a/): fixups, insert-track, lock-menu,
lock-reset-dialog, locked-header, budget, profile-warning, main-track, mockup-match,
regression-diff.

`lock-reset-dialog.png` was first committed (7dee8aa) as a full-desktop capture, including part of
the terminal. It has since been cut to the app and its dialog. The earlier version stays in
history, since history is never rewritten.
