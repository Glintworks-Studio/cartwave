# BATCH4B.md: visual fixups, stereo, unison, MIDI import as tracks, the mixer view

> *Historical report, kept as written under the working title ChipDAW. Since the rename (2026-11-27, [RENAME.md](RENAME.md)) the app is Cartwave by Glintworks: projects save as `.cartwave`, the executable is `cartwave`, and environment variables are `CARTWAVE_*` (the `CHIPDAW_*` names still work).*

Branch **`batch4b/mixer`** (from `main` d4f1afd), pushed. Prompt: `BATCH4B_PROMPT.md`. Decisions:
[`docs/RUNLOG.md`](docs/RUNLOG.md) **B4B-D1…D32**. Spec: [`VISION.md`](VISION.md), [`UX.md`](UX.md)
§4f and §8; [`docs/design/VISUAL-SPEC.md`](docs/design/VISUAL-SPEC.md) §13. Vizia fork:
`chipdaw-perf` 04596968 (forwards the OS file-hover events), pinned in all nine entries.

## 1. Summary (one screen)

**Merge: done on the owner's go-ahead (2026-11-27, B4B-D35).** The CEO accepted the two gates
below as measured; `main` was tagged `pre-batch4b` at d4f1afd and fast-forwarded to this branch.
At the end of the unattended run the merge was held, because two gates are not met by their letter: performance (§9.6: +0.3 on the 10-round means; §3) and the audio
gate's "0 dropouts at full Main volume" as the loopback pilot measures it (§4: the dense mix
masks the pilot; the engine reads 0 underruns and the same project at −40 dB reads 0 pilot
dropouts). Every other gate passes.

| # | Gate | Result |
|---|---|---|
| 1 | Bit-identical audio and exports | **PASS**: golden render and v2…v7 hashes unchanged (`tests/migration.rs`, the v7 corpus written by the baseline); the game export without pan **byte-identical** in 74 files over 3 projects ([report](docs/batch4b/export-identity.md)); new **stereo golden hash** `0x3e54_3873_23b0_f321` (`tests/batch4b.rs`) |
| 2 | Visual regression, 100 % and 150 % | **PASS**: 0 px outside the allowed regions in 11 states at each scale ([100](docs/batch4b/regression/report-100.md), [150](docs/batch4b/regression/report-150.md)); every allowed region is a change the batch asked for |
| 3 | Mockup match within 1 px | Main row **95/95** at 100 % and 150 % ([report](docs/batch4b/main-match/report-100.md)). Mixer strips + Main strip **142/144** at 100 %, **138/144** at 150 % ([100](docs/batch4b/mixer-match/report-100.md), [150](docs/batch4b/mixer-match/report-150.md)): the misses are font rendering (the board's webfont vs bundled Inter: a name's first glyph 2 px, a proportional "8" 3 px) and, at 150 %, a knob's anti-aliased edge (D22, D25). Fixups: every measured part within 1 px / 1 level ([report](docs/batch4b/fixups-match/report-100.md)) |
| 4 | Earlier suites, AT-17, B4B | **PASS**: AT-01…31, DP-01…05 (DP-04's text updated to 4b's controls, D31), PL-01…05, P2-01…11 (P2-11 deterministic now, 5/5), B4A-01…09; **B4B: 43/43** (M1 = AT-17, M2…M4, X1, X2, U1, C1, T1) |
| 5 | `cargo test`, clippy, warnings | **PASS**: **261 passed, 0 failed, 2 ignored** in release and in debug (263 listed) ([list](docs/batch4b/test-list.txt), [release run](docs/batch4b/test-run-release.txt), [debug run](docs/batch4b/test-run-debug.txt)); K6's fuzz test passes in debug; clippy clean; release build 0 warnings |
| 6a | Audio | **Engine: PASS** at full Main volume: 30 min, 64 studio tracks with unison 2 (up to 1,024 voices), 256 frames: 0 starved, 0 late, **0 allocation violations**, callback p99 **16.4 %**. **Pilot: not met by the letter** at full volume (11,118 masked blocks of 674,996); the same project at Main −40 dB for 30 min on the final build: **0 pilot dropouts**, p99 16.6 % (§4, D33). The demo: 0 / 0 |
| 6b | Performance | **NOT PASSED** by the letter: 12 of 18 scenarios within +0.3 on 10-round means; idle +0.37, row resize +0.46, the 64-track scenarios +0.74…+1.84 (each inside 2 × its noise floor). The new mixer: 64 strips playing **11.2 %** of a core (the arrangement 16.3 %), scrolling sideways 18.5 % ([summary](docs/batch4b/perf/summary.md)) |
| 7 | Lime audit, contrast | **PASS**: 0 violations in 7 scenes (arrangement, selection, playing, the mixer, 64 strips, File ▸ New in both views) ([report](docs/batch4b/lime/report.md)); new text ≥ **4.74 : 1** (D32) |
| 8 | Menu-bar flicker (K1) | **PASS**: 0 flash frames before and after, in 8 conditions, and again on the final build (dark menus on and off); the capture's positive control finds 12 ([README](docs/batch4b/flicker/README.md)). K1 doesn't reproduce on this PC (D18) |

**What changed.**
- **Fixups (§3).** The Main row cached whole and redrawn from its board (95/95). K6 fixed. K2's
  focus ring drawn inset. The number column follows the largest track number (K3/G10). The
  display's overhang eases out over 48 px (G11). Selection is neutral (white outline + glow, G12).
  Header meters are the pan grey with a white clip mark that holds until clicked (G8). No
  tooltips anywhere (G9). Lime: new projects' Pulse tracks are Sky, the colour grid's lime cell
  is Sage, existing files keep their colours bit for bit (G15). P2-11 is deterministic. K1 was
  measured and doesn't reproduce.
- **Stereo (§4).** Equal-power pan on studio tracks (a centred track unchanged); locked tracks and
  the console stay centred; nothing panned renders bit-identical. Exports are mono until a studio
  track is panned, then the mix and every stem are stereo (stems sum to the mix).
- **Unison / Detune / Drift (§5).** Studio tracks, 1–4 voices per note, ±0–50 cents, a 0–10 cent
  wander, all in the parameter registry (frame-tick rate class), saved in format 8. Gain staging:
  one note at the console's level, unison trimmed 1/√n, a −1 dBFS safety limiter on the studio
  bus, bit-transparent below it. The listening pack: [docs/audio/batch4b](docs/audio/batch4b/levels.md).
- **MIDI import as tracks (§6).** Cursor tracking during an OS drag verified on Windows (D16, the
  fork forwards `HoveredFile`); a live ghost preview with the conflicts named while dragging; the
  dialog only on conflict (Studio / Fit / Fit with chords as arpeggios / Cancel / Don't ask
  again); File ▸ Import ▸ MIDI File…; AT-17 passes.
- **The mixer (§7).** Tab or View ▸ Mixer; 152 px strips, Main 176 px pinned right; machined knob
  (option A), mixer M/S keys, the plastic cap, engraved scale, stereo LED meters in the warm
  palette, PEAK (held, click resets), VOICES, the dB readout (double-click types); the channel-
  budget bar; one model with the headers (B4B-X1); locked strips follow the lock rules (X2);
  static parts cached.

## 2. Decisions taken alone (RUNLOG B4B-D*)

1. **Roadmap order** (D1): the authenticity A/B pass is listed before 4b in VISION.md; the owner's
   prompt starts 4b now. Logged, not built against.
2. **Pan law** (D4): equal power, centre at unity (so mono projects stay bit-identical). Stereo is
   three step buffers (centre, left, right) so the mono path is untouched (D5).
3. **Exports** (D6): channels follow the project (stereo once any studio track is panned); the
   game export's JSON says `"channels"`.
4. **Unison** (D7–D9): 4 voices max, voices from the track's pool; frame-tick rate class; the gain
   scheme and the bus limiter. **Format 8** (D10) adds `unison`, `detune`, `drift`.
5. **The clip mark** (D11): past 0 dBFS of the track's output; held until clicked. Idle voices
   report no level (D24: a triangle's power-up step read as a clip).
6. **MIDI import** (D15, D17): ask only on conflict; parts on new tracks after the last, or onto
   the hovered track at the hovered bar.
7. **Tab** (D20, D28): Tab toggles Arrangement / Mixer, the shown view's key is lit; switching
   gives the keyboard to the window (vizia's Tab navigation had focused a button, and the next
   Space pressed it: found by the lime scenes, fixed, scenario T1).
8. **Where the voices controls live** (D23): the INSTRUMENT section's VOICES row (the STUDIO dial
   row overflowed at the minimum window).
9. **Board geometry** (D21): the boards position parts from the padding box (1 px in).
10. **Readouts** (D29): a held PEAK under −120 dB reads −inf (File ▸ New had shown −792.7 dB).
11. **Contrast** (D32): the mixer's labels `#8A8A8A`, not the board's `#808080` (4.22 : 1).
12. **The perf method** (D26) and its judgement (D30).

## 3. Why there is no merge: the performance gate

Method (§3.13, D26): `docs/batch4b/tools/perf_b4b.ps1`. 10 rounds, the baseline and this branch
interleaved with the order alternating each round, batch 4a's bench (both builds with the Main
row geometry), the piano-roll bench and the new mixer bench; nothing else running; every run's
proof reached and focus held (0 runs left out). Judged on the difference of the means; the noise
floor is the standard error of the paired round-by-round differences.

| Scenario | Baseline | 4b | Δ | Noise floor |
|---|---|---|---|---|
| demo / 200 clips / 200-clip drag | 8.0 / 8.6 / 11.2 | 7.9 / 8.4 / 11.0 | −0.11 / −0.13 / −0.26 | 0.35 / 0.26 / 0.48 |
| zoom wheel / scrollbar drag / window resize | 16.9 / 15.0 / 36.2 | 16.7 / 15.1 / 35.2 | −0.18 / +0.12 / −0.96 | |
| header width / ruler zoom / overview drag / zoom | | | −0.76 / +0.08 / −0.91 / +0.14 | |
| piano roll playing / dragging | 6.9 / 8.2 | 7.1 / 8.3 | +0.25 / +0.12 | 0.47 / 0.36 |
| **idle** | 0.1 | 0.5 | **+0.37** | 0.19 |
| **row resize** | 12.9 | 13.4 | **+0.46** | 0.38 |
| **64 tracks playing** | 17.2 | 18.4 | **+1.14** | 0.63 |
| **64 tracks scrolling** | 37.1 | 38.0 | **+0.92** | 1.08 |
| **64 tracks insert / delete** | 26.6 | 28.4 | **+1.84** | 0.93 |
| **64 tracks header drag** | 19.2 | 20.0 | **+0.74** | 0.68 |

Four of the six over-limit rows are the 64-studio-track project, which is where 4b's new audio
work runs (stereo bus, unison bookkeeping). Offline, the same 512 voices render at **4.6 %** of
real time against the baseline's **4.3 %**: about 0.3 points of a core are the stereo bus (three
step buffers and the limiter). The rest of the 64-track excess (≈0.5–1.5 points) is UI-side and
**not located yet**; candidates are the per-tick meter work (held peaks, L/R Main meters and
voices per track for 64 tracks) and the header clip marks. A 2 × noise-floor rule would pass all
six, but that rule is this run's, not the prompt's, so the gate is reported as not passed (D30).

**Owner's call.** Merge as is (the 64-track excess is ≈1 point on a stress project, inside the
noise band, and partly the price of audible stereo), or have the UI-side excess profiled first.

## 4. Audio

Method: batch 4a's (`docs/batch4b/tools/audio_b4b.ps1`; ENGINE.md): the release app plays with a
23 kHz pilot, captures its own output through WASAPI loopback and counts pilot dropouts, next to
the engine's own detectors, NoAllocGuard and the callback cost. Logs: `docs/batch4b/audio/`.

| Run | Length | Underruns (starved / late) | Alloc violations | Callback p99 / max | Pilot dropouts |
|---|---|---|---|---|---|
| fx_s64u, Main 0 dB (full volume) | 30 min | 0 / 0 | 0 | 16.4 % / 21.3 % | **11,118** of 674,996 blocks |
| fx_s64u, Main −40 dB | 120 s | 0 / 0 | 0 | 16.5 % / 20.5 % | 0 of 44,988 |
| fx_s64u, Main −40 dB, final build | 30 min | 0 / 0 | 0 | 16.6 % / 22.1 % | **0** of 674,992 |
| the demo (locked tracks, the console path) | 120 s | 0 / 0 | 0 | 0.6 % / 3.1 % | 0 |

**Reading (D33).** At full volume the pilot count disagrees with the engine. The third source is the
same project at −40 dB: identical callback cost and timing, and 0 dropouts. At full volume the
count is a steady ~6 blocks per second from the second second on, unrelated to the music's
position. So 1,024 band-limited voices put energy into the pilot's band and mask it; the stream
doesn't drop. The bus limiter keeps the full-volume mix at −1.0 dBFS sample peak
(`mixer.png`: Main reads −1.0 dB). The prompt asked for full volume specifically (4a was judged at
−40 dB). The pilot method can't give that number on this project, so the gate is reported as not
met by its letter. A pilot outside the mix's band, or a sample-counter check in the loopback
capture, would measure it directly. That is a tool change for the owner to approve, not
something to slip into this run.

## 5. Screenshots (`docs/screens/batch4b/`)

| File | What |
|---|---|
| `mixer.png` | 64 studio tracks with unison 2, playing: lit meters, PEAK, VOICES, Main held at −1.0 dB by the bus limiter |
| `mixer-locked.png` | the board's locked tracks: dimmed faders and knobs, locks in the readouts, the budget bar |
| `mixer-mockup-match.png`, `main-mockup-match.png` | the boards and the app side by side, measured parts marked |
| `fixups-before-after.png` | selection, meters, overhang, Main row, tooltip: before and after; the 12-track number column |
| `midi-import-preview.png` | a real OS drag: the ghost clips under the cursor, conflicts named |
| `midi-import-dialog.png` | the question on conflict: Studio / Fit / Fit with arpeggios / Cancel / Don't ask again |
| `unison.png` | the inspector's VOICES row (UNISON 3, DETUNE 15) |
| `regression-diff.png` | the `inspector` state's diff: allowed regions red, nothing magenta |

## 6. Tools (`docs/batch4b/tools/`)

`at_b4b.ps1` + `check_b4b.py` (scenarios), `drag_midi.ps1` (real OLE drags), `fixtures_b4b.py`,
`mockup_match_main.py` / `mockup_match_mixer.py` / `mockup_match_fixups.py` with their shot
scripts, `diff_b4b.py` (regression), `lime_scenes_b4b.ps1` + `lime_audit_b4b.py`,
`menu_flicker.ps1` + `flicker.py` (K1), `perf_b4b.ps1` + `bench_mixer.ps1` +
`perf_summary_b4b.py`, `audio_b4b.ps1`; `crates/ui/examples/listening_b4b.rs` (the listening pack).
