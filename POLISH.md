# POLISH.md: the Option D polish pass

Branch **`polish/option-d`** (from `design/option-d`, which it contains). Prompt:
`POLISH_PROMPT.md`. Decisions: [`docs/RUNLOG.md`](docs/RUNLOG.md) PL-D1…D17. Spec:
[`docs/design/VISUAL-SPEC.md`](docs/design/VISUAL-SPEC.md) §2, §6–8.

## 1. Summary (one screen)

**Gates: all pass.**

| Gate | Result |
|---|---|
| 1. Visual regression | **PASS** at 100% and 150%: 9 states × 2 scales. Every changed pixel lies in an allowed region: the display well (the overhang), the browser header, the Edit menu's popup (dark menus), or the window's rounded OS corners. 0 pixels outside. [`regression-diff.png`](docs/screens/polish/regression-diff.png), reports in `docs/screens/polish/regression/` |
| 2. Tests | AT-01…19, AT-07…31, DP-01…05 and the new **PL-01…05** all pass; `cargo test --workspace` (golden render, migration hashes, 77 ui unit tests incl. the palette's) passes; clippy clean; 0 warnings |
| 3. Lime audit | 0 violations: 9 app scenes, the colour popup and the palette reference sheet ([`lime-audit.md`](docs/screens/polish/lime-audit.md), [`lime-palette.md`](docs/screens/polish/lime-palette.md)) |
| 4. Performance | Demo 9.0% (design 9.4), 200 clips 9.3% (10.2), **200-clip drag 11.6% vs 11.6% pre-design** (target ≤ +0.3); pacing: 0 stalls, ≤ 1 frame over 20 ms per 30 s run; piano roll level with design |
| 5. Contrast | New text: section title 6.6:1, labels 13:1, notes 4.7:1. Clip headers ≥ 4.6:1 on all 70 colours, in every hover/selection state (unit test) |

**What changed.**
- **CEO tweaks:**
  - the display's overhang toned to about 80%;
  - the browser header: 24 px row and 24 px Preview key; the title already used the inspector's style;
  - **dark menus on by default**;
  - **lanes always scroll** and the **minimum window is 1296 × 720**, so a 1080p laptop at 125% fits;
  - START/LENGTH sampled once per frame.
- **Colours:**
  - our own 70-colour palette and the recipes every coloured surface uses;
  - the **colour grid** in the track and clip menus: hover preview, multi-select, one undo step;
  - clip text picks dark or light by contrast;
  - Settings ▸ Editing ▸ **Track and Clip Colors** (auto-assign + default colour, reduced colours, new clip colour);
  - random new-clip colours;
  - `next_track_color` ready for Batch 4.
  - Clips keep their own colour independently of their track (owner's request; already in the model, now tested).
- **IDEAS.md:** custom-drawn menu bar (parked).

**Decisions taken alone** (RUNLOG):
- **The legacy pulse colour.** `#C9D24A` is kept although it sits near the toggle lime, so existing projects don't change. It is the one exception, and automatic picks never choose it.
- **Defaults** (the manual gives none): new clips use the **track colour**; reduced colours **off**; default track colour `#7F86E0`.
- **Minimum window 1296 × 720.** At 1280 the top bar's groups touch.
- **Browser padding.** Its top padding shrinks so the list doesn't move.
- **Regression baseline.** `design/option-d` plus dev-only hooks.
- **Drag benchmark fix.** The drag benchmark was found to resize a clip in the new layout (PL-D13). That artifact also explains DESIGNPASS.md's "+1.3" drag figure.

**Merge:** §7.

## 2. The CEO's tweaks

| | Before → after |
|---|---|
| Display overhang | black 78/50/28/12/0 % → **62/40/22/10/0 %** over the same 34 px; inset shadow .95 → **.8**. [`display-shadow-before-after.png`](docs/screens/polish/display-shadow-before-after.png) |
| Browser header | 16 px row, 28 × 16 key → **24 px row, 24 × 24 key centred**, 8 px to the shortcut, 12 px in like the list; the browser's top padding 12 → 4 so the search field and list stay put. "BROWSER" already had the inspector titles' style (measured identical). [`browser-header-before-after.png`](docs/screens/polish/browser-header-before-after.png) |
| Dark menus | default **on** (DESIGNPASS.md §9; the test now checks on-by-default and that off is kept) |
| Window | lanes scroll whenever they don't fit (above the inspector too); minimum 1272 × 920 → **1296 × 720**; start size stays 1440 × 920. [`min-window-1536x816.png`](docs/screens/polish/min-window-1536x816.png), [`min-window-minimum.png`](docs/screens/polish/min-window-minimum.png) |
| START / LENGTH | the display samples its text once per presented frame (the frame counter) instead of on six model changes |

## 3. Colours

- **Palette** (`crates/ui/src/theme/palette.rs`, generator `docs/polish/tools/make_palette.py`):
  - 70 base colours, 5 lightness rows × 14 OKLCh hues; ours, listed in ASSETS.md;
  - the 12 colours already in use sit in their nearest cells;
  - every other colour is ≥ ΔE 0.068 from the lime (threshold 0.06).
- **Recipes:** `strip`, `clip`, `marker`, `graph_bar`/`graph_fill` and `swatch`, the only way a surface gets a palette colour's shades. A unit test runs all 70 colours through all of them.
- **Reference sheet:** [`palette-reference-sheet.png`](docs/screens/polish/palette-reference-sheet.png). Each colour as a lit strip, an unlit strip and a clip, drawn by the app's own code, with its header contrast.
- **Grid:** [`color-grid.png`](docs/screens/polish/color-grid.png). Built into the existing context menu.
  - The current colour has a 2 px neutral ring.
  - Hovering previews the colour on the selection; leaving the menu puts the old colours back.
  - A click applies to every selected clip or track, as one undo step; the project is marked unsaved and the colour is saved.
- **Settings:** [`color-settings.png`](docs/screens/polish/color-settings.png).
- **Reference behaviour:** Live 12 manual §2.3.2 "Theme & Colors" (cited in UX.md §4a). The behaviour is copied; the palette and look are not.

## 4. The regression gate

Method (`docs/polish/tools/regress.ps1`, `diff.py`):
- **Before** is `design/option-d` plus dev-only hooks (`../before-polish`, 720f84a); **after** is this branch.
- **States:** stopped, playing, loop off, solo/mute, browser closed, inspector, piano roll, Settings, the Edit menu.
- **Conditions:** 1440 × 920; meters, spectrum, strips, MEM and dial easing frozen; playhead pinned; audio off; 100% and 150%.
- **Diff:** client area only, threshold > 2 levels. The OS's rounded window corners are left out; two identical "before" runs differed only there.

What the gate caught and fixed along the way:
- the cached M/S keys were first rendered on a CPU surface (antialiasing off by up to 7 levels); they now render on the window's GPU surface;
- shadow masks rounded for resize speed changed the dial knobs' rims; small fixed shapes now keep exact masks.

## 5. Tests

- **PL-01:** a track colour from the grid; marked unsaved; one Ctrl+Z restores it; the hover preview is put back.
- **PL-02:** two clips at once; then the track's colour changes and the clips keep theirs.
- **PL-03:** a random colour for a clip inserted with Ctrl+Shift+M.
- **PL-04:** the lanes scroll at 1536 × 816 with the inspector open.
- **PL-05:** an 800 × 600 request opens at exactly 1296 × 720 logical.
- **Unit:** palette (70 distinct, legacy kept, nothing near lime, every recipe, header contrast), pickers (deterministic, distinct, reduced), dark-menus default on.
- **Harness fixes:**
  - `shot.ps1` picks the titled window when a menu popup is the process's "main window". With dark menus on, this had broken AT-06/30's screenshots, and AT-18 because the popup got moved.
  - AT-26 clicks the grid.
  - The shell and Batch 3 coordinates are unchanged, because the start size is kept.

## 6. Performance

`docs/polish/tools/perf_polish.ps1`: three builds interleaved, 3 rounds; CPU % of one core, mean:

| Scenario | pre-design (`main`) | `design/option-d` | polish |
|---|---|---|---|
| Idle | 0.4 | 0.2 | 0.1 |
| Demo playing | 9.4 | 9.4 | 9.0 |
| 200 clips playing | 10.3 | 10.2 | 9.3 |
| 200 clips, dragging | 11.6 | 12.6 | **11.6** |

- **Pacing** (polish, 30 s each): demo, 200 clips and drag all run at 60.0 updates/s with 0 stalls; frames over 20 ms: 1, 0, 1.
- **Piano roll**, 2,000 notes, 6 interleaved rounds:
  - playing 6.2% polish vs 5.9% design;
  - dragging 7.3% vs 7.2%.
- **How the drag got there:**
  1. The benchmark grabbed a clip's left resize zone in the new layout, so each frame re-blurred a 2–6 px wide shadow; it now grabs the same spot in each layout.
  2. Narrow-shape shadow masks are reused instead of re-rendered.
  3. Eight per-fader damage rects became one column (rows redraw with their meter).
  4. M/S keys blit cached looks, and the static pan slider is drawn by its row.

  A dedicated 5-round drag A/B: 11.1% (polish) vs 12.1% (pre-design).
- **New dev tools:**
  - `CHIPDAW_FRAMELOG` also writes per-view spans;
  - `threads_drag.ps1` and `sample_drag.ps1` attribute CPU by thread and by stack.

## 7. Merge

All gates passed, so:

```
git tag pre-polish main
git push origin pre-polish
git checkout main
git merge --ff-only polish/option-d
git push origin main
```

**Done 2026-11-26:** `pre-polish` tags the old `main` (23b5da9) and is pushed; `main` was
fast-forwarded to `polish/option-d` (48ee94b: the design pass and this pass) and pushed. To go
back: `git checkout -b back-to-pre-polish pre-polish`.
