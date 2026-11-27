# POLISH2.md: track header, layout and navigation

> *Historical report, kept as written under the working title ChipDAW. Since the rename (2026-11-27, [RENAME.md](RENAME.md)) the app is Cartwave by Glintworks: projects save as `.cartwave`, the executable is `cartwave`, and environment variables are `CARTWAVE_*` (the `CHIPDAW_*` names still work).*

Branch **`polish2/header-nav`** (from `main` 8644ff3). Prompt: `POLISH2_PROMPT.md`. Decisions:
[`docs/RUNLOG.md`](docs/RUNLOG.md) P2-D1…D21. Spec: [`docs/design/VISUAL-SPEC.md`](docs/design/VISUAL-SPEC.md)
§4, §4a, §10, §11; [`UX.md`](UX.md) §1, §2, §4c, §4d, §5.2, §6. Reference pack:
[`docs/design/polish2-ref/`](docs/design/polish2-ref/). Manual citations:
[`docs/polish2/manual-refs-p2.md`](docs/polish2/manual-refs-p2.md) (P2-R1…R44).

## 1. Summary (one screen)

**Gates: all pass.** Merged into `main` (see §7).

| Gate | Result |
|---|---|
| Pre-flight | The 1 px clip gap was **already committed** (0020c1d, DP-D23); nothing uncommitted, **no wip branch**. Clip corner radius: **4** (`RADIUS_CLIP`), kept |
| 1. Visual regression | **PASS** at 100% and 150%, 9 states + View menu + Look & Feel: changes only in the track headers, the Settings panel's new tab and the View menu's popup (Overview and status bar hidden) |
| 2. Mockup match | **PASS**: every header element within 1 px on all boards (E1, row heights 72/144/240, the four side × order layouts, widths 520 and 380 right, Overview on); 96 px rows and 600 px by the rules |
| 3. Hit tests | **PASS** in all four layouts (P2-11) |
| 4. Tests | **PASS**: P2-01…11; AT-01–04, 06, 15, 16, 18, 19; AT-07–12, 14, 22–31; DP-01…05; PL-01…05; `cargo test` 208 (golden render, migration hashes v2…v6); clippy clean; 0 warnings |
| 5. Lime audit | **0 violations**; the Overview's loop line only while the loop is on |
| 6. Performance | **PASS**: POLISH.md scenarios within +0.3 (200-clip drag −0.16 in a 5-round A/B); new gestures below their baseline counterparts; piano roll +0.16 / +0.08; 60 fps, 0 stalls |
| 7. Contrast | **PASS**: every new text ≥ 4.6:1 |

**What changed** (§2): the E1 header (dB fader, typed values, editable pan), track volume as a
real gain (+6 dB, unity bit-identical), format v6, row heights 72–300, header width 380–600,
Settings ▸ Look & Feel (headers, strip, browser sides), the status bar (messages, problems, hover
help), ruler zoom/scroll, W, the Overview.

**Decisions taken alone** (§5, RUNLOG P2-D1…D21): volume as a post-chip gain (the old fader was a
4-bit register); format v6; the boards' HTML over their PNGs and §3's fader formula over E1's
thumbs; Shift+drag scrubs the ruler (a drag now navigates); W and Ctrl+Alt+O; the earlier suites
run with dev hooks for their coordinates; the header keeps its 1 px divider view (150% rounding);
performance compared against the baseline's like-for-like gestures.

## 2. What changed

- **The E1 track header** (§3): a 120 px control block at the edge toward the clips, two columns
  16 px apart. Column A: M / S (unchanged), the pan value and an editable pan slider. Column B: a
  vertical volume meter-fader **in dB** (−inf … +6, position `(dB + 48)/54`, meter stops
  0/45/70/85/89 %, hot at 0 dB) and its dB value. **Double-click a value to type it** (dB: `-3`,
  `-3 dB`, `−3`, `+2.5`, `-inf`; pan: `C`, `0`, `25L`, `30R`, `-25`, `+25`): Enter commits, Esc
  cancels, a click away commits; clamped; one undo step; the unsaved mark.
- **Track volume is a real gain** (P2-D4): the old fader moved the chip's 4-bit Volume register,
  capped at unity. Each track now has a dB volume applied after the chip (`chip::mixer::mix_gained`);
  at 0 dB the old mix runs unchanged, so the golden render and every migration hash are
  bit-identical. Every track has a working fader (the triangle and DPCM too). The register stays
  the inspector's Volume dial.
- **Pan** is editable and saved, heard once stereo mixing arrives (Batch 4); its help says so.
- **Format v6** (P2-D5): row heights, pan and volume per track; v5 files open unchanged
  (`docs/songs/v5` corpus, hashes equal).
- **The strip** is toned down ~20% (white mixes .28/.55, glow .40, spill .25) in the palette's
  `strip` recipe; the palette sheet is regenerated.
- **Row heights** (§4): 72 … 300 px in 4 px steps: drag a track's bottom edge (Alt: every track)
  or Alt+wheel. Taller rows keep the 72 px band at the top; only the fader stretches; dB scale marks
  from 96 px.
- **Header width, side and order** (§5): the boundary drags 380 … 600 (extra width to the name);
  **Settings ▸ Look & Feel**: Track headers Left/Right, Colour strip Outside/By the clips, Browser
  Left/Right. The four orders of the table; the TRACKS and OVERVIEW cells move with the headers.
- **The status bar** (§6): messages (8 s, then "Saved · name"), persistent problems (an icon until
  resolved; a click lists them), hover help on the right. Every toast moved there
  ([`docs/polish2/notices.md`](docs/polish2/notices.md): 62 message sites); dialogs stay for decisions.
- **Navigation** (§7): the ruler zooms (vertical drag) and scrolls (horizontal drag); a click still
  places the playhead; Shift+drag scrubs; **W** fits the song (again: back); **the Overview** (32 px,
  cached clip layer, drag/zoom/jump/double-click, View ▸ Show Overview, Ctrl+Alt+O).

## 3. Pre-flight

- **The clip gap:** already committed on `main`: 0020c1d "Design pass: caches for per-frame
  drawing, 1 px clip gaps, …" (RUNLOG DP-D23). `main` had no uncommitted changes to tracked
  files, so nothing was committed there and **no `wip/` branch** was needed (P2-D1).
- **The clip corner radius:** `RADIUS_CLIP = 4.0` (`theme/tokens.rs`), kept (P2-D2).
- **Baseline:** `main` 8644ff3, built in `../before-polish2` (P2-D3).

## 4. The gates

### 4.1 Visual regression (gate 1): PASS at 100% and 150%

`docs/polish2/tools/regress.ps1` + `diff.py`: the polish pass's method (1440 × 920, meters and
dials frozen, playhead pinned, audio off, threshold > 2 levels), with the **Overview hidden**
(settings) and the **status bar hidden** (dev hook `CHIPDAW_HIDE_STATUS=1`). States: the nine
(stopped, playing, loop off, solo/mute, browser closed, inspector, piano roll, Settings, the Edit
menu) plus the **View menu** and **Look & Feel** (new build only). Every changed pixel is inside
an allowed region: the track headers' rows, the Settings panel's new tab, the View menu's popup.
Reports: [`regression/report-100.md`](docs/screens/polish2/regression/report-100.md),
[`report-150.md`](docs/screens/polish2/regression/report-150.md);
[`regression-diff.png`](docs/screens/polish2/regression-diff.png) (red = changed in an allowed
region; magenta would be a violation: none).

What the gate caught: the first build drew the header's hairline inside a 380 px column; at 150%
the old 1 px divider view rounds to 2 device px, so every grid line moved 1 px. The column is
again 379 px + the original divider view (P2-D19).

### 4.2 Mockup match (gate 2): PASS

`docs/polish2/tools/mockup_match.py`: each header element of the boards (HTML boxes,
`docs/polish2/boxes/`) found in the app's screenshot by template matching of its non-background
pixels (best offset within ±1 px); the strip and the fader's well by their edges; the fader's
thumb against §3's formula (the boards disagree with each other there, P2-D8). Test projects
(`docs/polish2/songs/`) carry the boards' names, pan and dB values.

| Board / layout | Rows | Result |
|---|---|---|
| E1-header (4 tracks, 72 px) | 4 | PASS |
| E1-row-heights (72 / 144 / 240) | 3 | PASS |
| Order: left · outside, left · by the clips, right · by the clips, right · outside | 2 each | PASS ×4 |
| Width 520 (left), 380 (right) | 2 each | PASS ×2 |
| E1-overview (Overview on) | 4 | PASS |

Rows at 96 px and the 600 px width have no board; they follow §4's rules (unit tests
`scale_marks_skip_crowded_labels`, `width_goes_to_the_name`; screenshots). Reports per board:
[`mockup-match/`](docs/screens/polish2/mockup-match/); side by side (board | app | difference):
[`mockup-match.png`](docs/screens/polish2/mockup-match.png).

One 1 px deviation by design: where the board's strip by the clips covers the header's hairline,
ours stops at the divider (P2-D19).

### 4.3 Hit tests (gate 3): PASS

P2-11, in all four layouts: M, S, the dB value (type), the pan value (type), the fader, a row's
bottom edge, the name (rename), the width boundary, the ruler and the Overview each hit what they
should (saved files, settings.json, dev lines). The audit of left-side assumptions (§5.4):

| Place | Found | Fixed |
|---|---|---|
| Browser drop targets (`browser::lane_at`) | accepted only points right of a header row | a point over the row or over the lanes, whichever side |
| Rename field over a track name | fixed x 24 | from the layout (mirrored too) |
| Value fields (dB, pan) | new | from the layout |
| Header width edge | new | on the lanes' side, either side |
| Strip spill | always to the right | toward the header's inside |
| TRACKS / OVERVIEW cells | in the column | move with it, label 16 px in |
| Playhead, loop brace, scrolling, overlay scrollbars, zoom anchoring, rubber band, clip drag and drop preview | relative to the arrangement view | nothing to fix (checked; P2-11 clicks the ruler and Overview in every layout) |
| Piano roll | its own key column, not aligned with the headers | nothing to fix |
| OS file drop | adds tracks at the insert marker, no position | nothing to fix |
| Lanes rows | fixed 73 px pitch everywhere (`lane_h`) | per-row heights (`Arrangement::lane_top / lane_at`) in every hit test and drawing |

### 4.4 Tests (gate 4): PASS

- **P2-01…P2-11** (new, `docs/polish2/tools/at_p2.ps1` + `check_p2.py`): all pass
  ([`docs/polish2/at_p2-result.txt`](docs/polish2/at_p2-result.txt), 92 checks).
- **The earlier suites**, run by `docs/polish2/tools/run_old_suites.ps1` (with the Overview and the
  status bar hidden by dev hooks, P2-D16):
  - `docs/shell/tools/at_shell.ps1`: **AT-01, AT-02, AT-03, AT-04, AT-06, AT-15, AT-16, AT-18, AT-19**: pass;
  - `docs/batch3/tools/at_b3.ps1`: **AT-07, AT-08, AT-09, AT-10, AT-11, AT-12, AT-14, AT-22, AT-23,
    AT-24, AT-25, AT-26, AT-27, AT-28, AT-29, AT-30, AT-31**: pass;
  - `docs/design/tools/at_dp.ps1`: **DP-01, DP-02, DP-03, DP-04, DP-05**: pass;
  - `docs/polish/tools/at_polish.ps1`: **PL-01, PL-02, PL-03, PL-04, PL-05**: pass.

  These are every scenario the suites contain (UX.md's AT-13 and AT-17 wait for Batch 4's model
  change; there are no AT-05, AT-20 or AT-21). Harness updates: AT-02 waits 9 s (the
  8 s message) and parks the mouse; AT-25 clicks the E1 S keys; DP-04 reads the reasons from the
  status bar; DP-05 accepts format ≥ 5 (P2-D16, D18).
- **`cargo test --workspace --release`**: 208 tests, all pass (the full list:
  [`docs/polish2/cargo-tests.txt`](docs/polish2/cargo-tests.txt)), including the golden render,
  every migration hash (v2→v3, v3→v4, v4→v5, **v5→v6**), the palette, the order table, the dB
  scale round trip, value parsing and the gained mix.
- **`cargo clippy --workspace --all-targets`**: clean. **`cargo build --release`**: 0 warnings.

### 4.5 Lime audit (gate 5): 0 violations

`docs/polish2/tools/lime_scenes_p2.ps1` + `lime_audit_p2.py`: stopped, playing, loop off, loop on,
solo, mute, inspector, Project Settings, headers right with the strip by the clips, browser right.
The Overview's loop line is allowed only while the loop is on (loop off: none found). Report:
[`lime-audit.md`](docs/screens/polish2/lime-audit.md).

### 4.6 Performance (gate 6): PASS

`docs/polish2/tools/perf_p2.ps1` (bench_p2.ps1, pacing.ps1, bench_pr.ps1): the baseline (`main`
8644ff3) and this branch interleaved, 3 rounds × 12 s, process CPU in % of one core, the 60 fps
cap, default settings (this branch shows the Overview and the status bar). Final build:

| Scenario (POLISH.md's) | Baseline | Polish 2 | Δ |
|---|---|---|---|
| Idle | 0.30 | 0.37 | +0.07 |
| Demo playing | 8.13 | 7.83 | −0.30 |
| 200 clips playing | 8.43 | 8.53 | +0.10 |
| 200 clips, dragging (3 rounds: +0.43; **dedicated 5-round A/B**) | 11.94 | 11.78 | **−0.16** |

| New gesture (200 clips playing) | Polish 2 | Measured against (baseline, same kind of work, P2-D20) | Baseline |
|---|---|---|---|
| Row-resize drag | 14.17 | Ctrl+wheel zoom (whole lane layer per frame) | 17.23 |
| Header-width drag | 25.77 | Window-width resize | 34.90 |
| Ruler zoom drag | 14.93 | Ctrl+wheel zoom | 17.23 |
| Overview drag | 8.77 | Time-scrollbar drag (Polish 2: 9.17, A/B +0.04) | 9.17 |
| Overview zoom drag | 9.10 | Ctrl+wheel zoom | 17.23 |

**Piano roll, its own row** (2,000 notes; dedicated 5-round A/B on the final build): playing
**6.50% vs 6.34%** (+0.16), dragging **6.76% vs 6.68%** (+0.08). The polish pass's 6.2% vs 5.9%
does not grow here.

**Pacing** (this branch, 30 s each): demo, 200 clips and the drag all present 60.0 updates/s with
**0 stalls**; max gap 17.5–18.5 ms.

The gate drove four fixes (P2-D21): the OVERVIEW cell no longer redraws per scroll; the faders'
heights are memos (scrollbar drag +0.76 → +0.04); the Overview's clip layer waits for a drop and
the Overview ignores note edits (piano-roll drag +0.64 → +0.08); the lane layer keeps slack
during width drags.

### 4.7 Contrast (gate 7): PASS

| New text | Colours | Ratio |
|---|---|---|
| Pan / dB values | `#9A9A9A` on `#0A0A0A` (selected row `#121212`) | 7.04 (6.66) : 1 |
| dB scale labels | `#808080` on `#0A0A0A` | 5.01 : 1 |
| Status bar message | `#9A9A9A` on `#0F0F0F` | 6.81 : 1 |
| Status bar hover help | `#808080` on `#0F0F0F` | 4.85 : 1 |
| OVERVIEW label, "24 BARS" | `#808080` on `#0F0F0F` | 4.85 : 1 |
| Look & Feel notes | `#808080` on `#151515` | 4.62 : 1 |

## 5. Decisions taken alone

RUNLOG **P2-D1…D21**, in short:

- **Volume became a real gain** (D4): the fader used to move the 4-bit chip register; now a dB
  gain after the chip, unity bit-identical; every track has a fader. Positive values show
  without a sign; wheel 1 dB; typed values clamp to −70 … +6.
- **Format v6** (D5) for heights, pan and volume; a v5 corpus with equal hashes.
- **Pan** −50 … +50, saved, not heard (D6).
- **The boards' HTML over their PNGs** (D7); **§3's fader formula over E1-header's thumbs** (D8).
- **Row resize:** the bottom 4 px; Alt+drag sets every track (P2-R20); Alt+wheel 12 px a notch;
  undoable (D9).
- **Ruler:** a click still places the playhead, a drag navigates, **Shift+drag scrubs** (D11).
- **Keys:** W = Fit the Whole Song (toggles back), Ctrl+Alt+O = Show Overview (D12).
- **Overview:** the song is its extent; a click outside the frame jumps there (D13).
- **Status bar:** 8 s, a click shows a message whole, amber problem icon, dropouts clear after
  10 s; the recovery offer stays a capsule (D14). Hover help for §6.5's list; other tooltips stay
  (D15).
- **The earlier suites** run with dev hooks for their coordinates (D16, D17, D18).
- **The header keeps its divider view** (D19); performance comparisons and fixes (D20, D21).

## 6. Screenshots

[`docs/screens/polish2/`](docs/screens/polish2/): `header-e1.png`, `row-heights.png`,
`header-widths.png`, `layouts-4.png`, `status-bar.png` (a message, the problem icon, hover help,
the problem list), `overview-on-off.png`, `ruler-zoom.png` (a frame sequence: bar 5 stays under
the mouse), `strip-before-after.png`, `palette-reference-sheet.png`, `mockup-match.png`,
`regression-diff.png`. Tools: `shots_p2.ps1` → `compose_p2.py`.

## 7. Merge

All gates passed (on the final build: regression 100% and 150%, P2-01…11, the 36 earlier
scenarios, tests, clippy, the A/Bs), so:

```
git tag pre-polish2 main
git push origin pre-polish2
git checkout main
git merge --ff-only polish2/header-nav
git push origin main
```

**Done 2026-11-26:** `pre-polish2` tags the old `main` (8644ff3) and is pushed; `main` was fast-forwarded to `polish2/header-nav` (e09139c) and pushed. To go back: `git checkout -b back-to-pre-polish2 pre-polish2`.

