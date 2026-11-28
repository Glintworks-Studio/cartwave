# BATCH4C.md: the 4b excess profiled and fixed, K1 while playing, the authenticity A/B pass

Branch **`batch4c/profile-ab`** (from `main` 5533b5a), pushed. Prompt: `BATCH4C_PROMPT.md`. Decisions:
[`docs/RUNLOG.md`](docs/RUNLOG.md) **B4C-D1…D14**. Baseline: `main` in `../before-batch4c`; also
`pre-batch4b` (d4f1afd) in `../before-batch4b`. Machine: Windows 11 Pro 26200, Ryzen 7 9800X3D,
RTX 4070 SUPER, the monitor's HDMI audio (the MOTU of ENGINE.md §8 is not connected).

## 1. Summary (one screen)

**Merge: HELD.** Gate 5 (performance) fails by its letter; every other gate passes. `main` is untouched, no `pre-batch4c` tag (§7).

| # | Gate | Result |
|---|---|---|
| 1 | Bit-identical audio | **PASS**: golden render, every migration hash v2…v8, the stereo golden hash (`cargo test`); all **290** project files in the repo render and export with identical hashes in `main` and this branch; **252** game-export files byte-identical over 3 projects ([identity](docs/batch4c/identity/)) |
| 2 | Visual regression 100 % / 150 % vs `main` | **PASS**: 0 changed pixels in all 16 states at both scales (the rename's 15 + the mixer), apart from the About panel's build stamp (the baseline was built from a `git archive`: "build unknown") and 13 px outside the window's rounded corner at 100 % (RN-D14's artifact) ([100](docs/batch4c/regression/report-100.md), [150](docs/batch4c/regression/report-150.md)). No K1 region: K1 changed no code |
| 3 | Tests | **PASS**: every suite (shell/AT 49, Batch 3 79, DP 23, PL 14, P2 92, B4A 53, B4B 43; no FAIL line); **B4C 3/3** (the new audio gate catches injected stalls, a clean run, the K1 capture with its control); `cargo test` **279 passed, 0 failed, 2 ignored** in release and debug; clippy clean; release build 0 warnings |
| 4 | Audio, the new method | **Validation PASS**: the match, the pilot (music muted) and the starved detector agree on all **138** injected stalls; in the app at full volume on the 1,024-voice project it catches exactly the stalls that drop out and none of the others, where the pilot reported 460 false dropouts. **30 min at full Main volume: PASS**: 0 match dropouts, 0 mismatch runs, 0 capture gaps / foreign / tap lost, 86,398,612 of 86,400,000 samples compared (99.998 %); starved 0, late 0, 0 allocation violations ([gate30](docs/batch4c/audio/gate30/)) |
| 5 | Performance (10 rounds) | **NOT MET**: vs `main` 15 of 21 rows within +0.3; six over it (200drag +0.53, zoom_wheel +0.97, header_width +0.81, ruler_zoom +3.15, overview_drag +2.91, s64_insert_delete +0.83), each within 2 SE of zero. vs pre-batch4b idle −0.11 and row resize −0.66 pass; the four 64-track rows fail (+0.65, +2.11, +1.61, +0.49; `main` reads about the same over pre-4b). 0 runs left out ([summary](docs/batch4c/perf/gate/perf_summary.md)) |
| 6 | K1 | **PASS as measured**: 0 flash frames while playing in every resize condition (17 captures before and after, dark menus on); the positive control is caught every time. K1 does not reproduce here (§3) |
| 7 | A/B report and pack | **PASS**: [`docs/ab/AB.md`](docs/ab/AB.md) with every §4.3 metric; [`docs/audio/ab/`](docs/audio/ab/README.md) (36 files) |

**What the excess was, and the fix.** Located with per-thread cycle counters (precise; process
CPU is tick-sampled and its audio-thread share swung 3.4–6.4 % between runs of one build), the
frame log's phases and experiment switches (§2). Of 4b's 64-track excess, the audio thread's part
(≈ 0.2–0.3 points) is the stereo bus (accepted); the UI's (+224 k cycles per frame) was all in
the **events phase (+40 µs per frame)**, ≈ 80 % of it downstream of the meters: the clip mark looked
up by id in two bindings per track per frame (18 µs), held peaks found by searching the rack per
track (22 µs), an environment variable read every frame (10 µs; two lookups since the rename), every
studio track's controls stored every tick (10 µs), plus the commit's per-tick document copy (40 µs,
older, grown with 4b's lanes). Fixed with the code's patterns (work once per tick, nothing sent or
copied unless it changed). **Before/after (cycle counters, 6 rounds): UI −68 k ± 16 k cycles per
frame vs `main`; events −32.5 µs per frame.** Idle was never a real excess: 0 frames are presented in
every build and precise process cycles read 16.7 (pre-4b), 18.4 (`main`), 16.2 (4c) M/s.

**K1.** Not reproduced in the CEO's condition (playing while resizing): 17 captures at up to
120 fps, four edges, a fast drag, the 64-track project, dark menus on and off, before and after:
0 flash frames, 0 misdrawn bars; the control: 11–12. No code changed. Desktop Duplication can't see
a glitch after composition (e.g. a swap-chain presentation-mode change); UX.md K1 asks for a
slow-motion phone video.

**Reference tools.** **NSFPlay 2.6** (2023-07-03; no named licence, its readme: "distributed
freely", reuse "without restriction";
https://github.com/bbbradsmith/nsfplay/releases/download/2.6/nsfplay26.zip) in
`C:\tools\reference\nsfplay26`; **Game_Music_Emu** (LGPL-2.1+) as linked in **ffmpeg 8.1**
(gyan.dev full build, GPL-3.0; already installed, from the Windows package manager). Both run only
as separate programs; no source of theirs was read.

**Top 5 A/B differences** (AB.md §1): (1) the triangle/noise/DMC group ≈ 1 dB lower against the
pulses in NSFPlay: the wiki's tnd formula is an approximation; proposal: calibrate from a hardware
recording, then one constant in `mixer.rs`. (2) Sweep edges: NSFPlay rings 1–6 frames longer before
the overflow mute and once starts a step late; gme and the wiki agree with us: no change.
(3) Both players roll off above 8 kHz with their filters off; ours is flat: no change.
(4) gme inverts the noise and the VRC6; ours, NSFPlay and the wiki agree: no change. (5) gme's
mixer levels (quiet volumes, pairs, DPCM steps) differ by ≈ 1 dB; ours and NSFPlay follow the
wiki's formulas: no change. Everything else matches NSFPlay: pitch 0.00 cents on 57 notes, levels
within 0.1–0.3 dB, envelopes ±0.12 frame, LFSR loops identical.

## 2. The 4b excess (§2)

Tools (`docs/batch4c/tools/`): `threads_b4c.ps1` (per-thread TSC cycles and the frame log's
phases), `perthread_b4c.ps1`, `sample_b4c.ps1` (4a's sampler for any build), `phases_b4c.py`,
`idle_b4c.ps1` (process cycles), `perf_b4c.ps1` + `perf_summary_b4c.py` (the gate). Evidence:
`docs/batch4c/perf/`.

| Step | Finding | Evidence |
|---|---|---|
| Split by thread | UI +224 k cycles/frame (+3.3 %); audio +107…130 k/callback (the stereo bus); process CPU's own noise is the tick sampling | `threads.tsv`, `perthread_s64.csv` |
| Split by phase | events +40 µs/frame; style, layout, draw, flush, every custom draw span equal | `phases_pre4b_main.md` |
| Meters frozen | UI difference +224 k → +46 k: ≈ 80 % is the meter path | `threads.tsv` (`frozen-`) |
| Switches | commit −40, held peaks −22, clip lookups −18, env read −10, sync_ctls −10 µs; Main L/R, VOICES ≈ 0 | `switches_1.txt`, `switches_2.txt`, `attribution-switches.patch` |
| After the fixes | events −32.5 µs, UI −68 k ± 16 k cycles/frame vs `main` (6 rounds) | `phases_main_b4c.md` |
| Idle | 0 frames in every build; 16.7 / 18.4 / 16.2 M cycles/s | `idle_cycles.txt` |

The fixes (B4C-D5): `Doc::matches` (compare in place; build a `Doc` only on a change); the meter tick
in one pass with reused buffers and held peaks sorted by id; `Meters::clip` (each track's clip mark
once per tick, read by index); `dev::frozen_meters()` / `fixed_playhead()` read once; `sync_ctls`
sends only changes. One thing is reported, not attributed: the frame log's draw phase reads
+18 µs on the branch while the custom spans, the row redraws and a same-binary profile show no
extra drawing (B4C-D6).

### The gate: 10 rounds, three builds interleaved (B4C-D13)

Rounds: 10; runs left out (focus lost / scenario not reached): 0.
Δ = this branch's mean − the other build's (CPU % of one core); SE = the noise floor (standard error of the paired differences). Limits (§6.5): vs main +0.3 everywhere; vs pre-batch4b the 64-track rows +0.4, idle +0.1, row resize +0.3.

| scenario | n | pre-4b | main | 4c | Δ vs main (SE) | ok | Δ vs pre-4b (SE) | limit | ok |
|---|---|---|---|---|---|---|---|---|---|
| idle | 10 | 0.5 | 0.5 | 0.4 | -0.08 (0.28) | yes | -0.11 (0.18) | 0.1 | yes |
| demo | 10 | 8.2 | 7.3 | 7.6 | +0.28 (0.41) | yes | -0.67 (0.54) | – | – |
| 200 | 10 | 7.9 | 7.9 | 7.9 | +0.03 (0.24) | yes | +0.00 (0.17) | – | – |
| 200drag | 10 | 10.6 | 10.7 | 11.2 | +0.53 (0.46) | **no** | +0.67 (0.43) | – | – |
| zoom_wheel | 10 | 17.4 | 16.8 | 17.7 | +0.97 (0.53) | **no** | +0.39 (0.74) | – | – |
| scrollbar_drag | 10 | 15.0 | 14.7 | 14.8 | +0.12 (0.64) | yes | -0.16 (0.79) | – | – |
| window_resize | 10 | 37.0 | 37.0 | 36.6 | -0.39 (1.17) | yes | -0.40 (0.83) | – | – |
| row_resize | 10 | 13.9 | 13.5 | 13.2 | -0.29 (0.45) | yes | -0.66 (0.44) | 0.3 | yes |
| header_width | 10 | 24.9 | 24.4 | 25.2 | +0.81 (0.57) | **no** | +0.25 (0.42) | – | – |
| ruler_zoom | 10 | 16.6 | 14.0 | 17.1 | +3.15 (1.71) | **no** | +0.51 (2.32) | – | – |
| overview_drag | 10 | 8.5 | 8.2 | 11.1 | +2.91 (2.58) | **no** | +2.54 (2.56) | – | – |
| overview_zoom | 10 | 9.0 | 8.5 | 8.6 | +0.09 (0.44) | yes | -0.41 (0.56) | – | – |
| s64_playing | 10 | 17.3 | 17.9 | 18.0 | +0.08 (0.46) | yes | +0.65 (0.61) | 0.4 | **no** |
| s64_scroll | 10 | 36.2 | 38.1 | 38.3 | +0.21 (0.83) | yes | +2.11 (1.32) | 0.4 | **no** |
| s64_insert_delete | 10 | 26.4 | 27.2 | 28.0 | +0.83 (0.63) | **no** | +1.61 (0.97) | 0.4 | **no** |
| s64_header_drag | 10 | 18.8 | 19.6 | 19.3 | -0.34 (0.70) | yes | +0.49 (0.86) | 0.4 | **no** |
| pr_playing | 10 | 6.7 | 7.6 | 6.9 | -0.77 (0.47) | yes | +0.21 (0.37) | – | – |
| pr_dragging | 10 | 8.2 | 8.2 | 8.0 | -0.11 (0.23) | yes | -0.15 (0.39) | – | – |
| s64_arrangement | 10 | – | 17.1 | 16.7 | -0.34 (0.64) | yes | – | 0.4 | – |
| s64_mixer | 10 | – | 11.6 | 10.8 | -0.75 (0.61) | yes | – | 0.4 | – |
| s64_mixer_scroll | 10 | – | 18.9 | 18.4 | -0.53 (0.36) | yes | – | 0.4 | – |

**vs main: NOT MET; vs pre-batch4b: NOT MET**

The gate is judged on the means by its letter, with no noise allowance, so it **fails**. The rows over the limit against `main` are all within 2 SE of zero, and none is in the code the fixes touched. Against pre-batch4b, the 64-track rows show 4b's excess is still there by process CPU. That matches the cycle counters' scale: the fix is ≈ 0.1 point of a core; what is left (UI +117 k cycles per frame, the unattributed draw +18 µs, the stereo bus) is ≈ 0.4 points before the scroll and insert rows' heavier frames. It was not re-run to fish for a pass. Still to find: the draw phase's +18 µs and the scroll/insert frames.

## 3. K1 (§3)

[`docs/batch4c/flicker/README.md`](docs/batch4c/flicker/README.md). Capture: a real mouse drag
(Windows' sizing loop), Desktop Duplication at up to 120 fps, the bar band plus 60 px of the client
area. Detectors: 4b's light test; a misdrawn-bar test (the labels' part against the steady bar);
any new solid light block anywhere in the band. Before and after, playing: 0 / 0 / 0 in every
condition; the control 11–12. Mechanism tested (our presents during the size loop interleaving
with the non-client repaint): no composed frame in ~11,500 resizing frames shows it. The dark-menus
setting doesn't touch the bar (muda draws it dark either way).

## 4. Audio: the new gate (§0.2, gate 4)

ENGINE.md §7a. The engine copies its first output channel, as written to the device, into a
wait-free tap; a checker aligns the WASAPI loopback capture (bit-exact here: 187,200 / 187,200
frames) with it sample for sample. A dropout is the capture's offset jumping forward by the
inserted silence; a −60 dBFS marker keeps the alignment unique. Validation:
[`docs/batch4c/audio/`](docs/batch4c/audio/).

| Run | Result |
|---|---|
| Headless, 23 stall lengths × 3, music muted | match = pilot = starved on every stall: ≤ 19 ms never, ≥ 21 ms always (20 ms: 3/3) |
| Same with music | the same (20 ms: 2/3, identical in all three) |
| App, `fx_s64u` (1,024 voices) at full volume, stalls 10/19/30/40 ms | match: the 12 stalls of 30/40 ms, none of the 12 shorter; starved 12; pilot 460 (masked) |
| B4C-1 / B4C-3 (final build) | 6 of 6 long stalls caught, 0 wrong; a clean demo run 0 / 0, 99.7 % of it compared |
| **30 min, `fx_s64u`, Main 0 dB, 256 frames** | **PASS**: 179,998 callbacks; match 0 dropouts, 0 mismatch runs, 1 lock, 99.998 % of the run compared; starved 0, late 0, max callback gap 10.32 ms; callback cost mean 10.2 %, p99 16.4 %, max 23.0 % of the period; 0 allocation violations, 0 device rebuilds |

## 5. The A/B pass (§4)

[`docs/ab/AB.md`](docs/ab/AB.md); tools `crates/ui/examples/ab.rs`, `ab_compare.rs`,
`docs/ab/tools/render_ab.ps1`, `compare_ab.ps1`; results `docs/ab/results.tsv`; the pack
`docs/audio/ab/`. The logs are proven complete (bit-identical replays of both songs); the NSF
container and every pattern are ours; no ROM, rip or copyrighted music; the private MIDI unused.

## 6. Decisions taken alone (RUNLOG B4C-D*)

1. **The gate method** (D2, D3): the reference match, not a new pilot; ENGINE.md's 22 ms is the
   MOTU's threshold, measured 20–21 ms on this endpoint, agreed by three detectors.
2. **Judging perf with cycle counters while locating** (D4); the gate itself by 4b's method.
3. **The fixes** (D5) are behaviour-neutral (unit-tested equivalences); no visual or sound change.
4. **K1: no code change** without a reproduction (D7).
5. **Reference tools** (D8): NSFPlay + gme (two independent cores); the container and logs (D9);
   the measurement corrections found by checking the numbers (D10); the pack's song excerpts (D11).

## 7. Merge

**Held.** §1.4 merges only if every gate in §6 passes, and gate 5 doesn't (B4C-D13):

- **Against `main`:** six rows are over +0.3 (200drag, zoom_wheel, header_width, ruler_zoom, overview_drag, s64_insert_delete). Each is within 2 SE of zero.
- **Against pre-batch4b:** all four 64-track rows are over +0.4.

Gates 1, 2, 3, 4, 6 and 7 pass. The branch is pushed. `main` and the tags are unchanged.

**The owner's options:**
- (a) Accept gate 5 as noise against `main` and as 4b's known excess against pre-batch4b, then merge by hand: `git tag pre-batch4c main`, then fast-forward `main` to `batch4c/profile-ab`.
- (b) Keep the branch open for a follow-up that finds the draw phase's +18 µs and the scroll/insert rows' cost.
- (c) Re-set the limits with a noise allowance (4b's rule, B4B-D30). Under it, every row against `main` would pass.
