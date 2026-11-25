# BATCH3: Ableton-parity fixes and the Windows backlog

*2026-11-25 · unattended run of BATCH3_PROMPT.md · branch `app-shell/batch-3` from `main` @ fb2d632 ·
Windows 11, RTX 4070 SUPER, 59.951 Hz display, MOTU M Series connected · decisions B3-D1…D23 in
[docs/RUNLOG.md](docs/RUNLOG.md)*

## Summary

**Gates**

| Gate | Result |
|---|---|
| 1. Acceptance tests | **Batch 3: 17/17 PASS** (AT-07…12, 14, 22 and the new AT-23…31; 79 checks, `docs/batch3/tools/at_b3.ps1`). **AT-01…19: 9/9 PASS** (49 checks, `docs/shell/tools/at_shell.ps1`; AT-03 and AT-18 updated for the in-place rename and the new Help item) |
| 2. `cargo test --release --workspace` | **179 pass** (167 before + 12 new), golden render included. Clippy clean, release build 0 warnings |
| 3. Performance, 60 fps cap | A/B against the pre-run build, 3 rounds each: demo **9.4 → 9.6%** (≤ 10), 200 clips **10.0 → 9.5%** (≤ 20), 200-clip drag **11.8 → 11.3%** (≤ 25): no regression. Pacing (30 s): **0 frames > 20 ms, 0 stalls** in demo, 200 clips and drag. Piano roll, 2,000 notes: playing 6.4 → 6.9%, dragging 6.7 → 6.6%. **At 150% interface scale:** demo 9.2–9.8%, 200 clips 8.2–9.2%, drag 11.5–11.8% |
| 4. Engine matrix on the MOTU | ENGINE_RESULT |
| 5. Contrast of new text | **≥ 4.66:1** everywhere (tooltips 12.9:1, menu reasons 6.1:1, the guide 13.3:1, hints 4.7:1) |

**Built (by UX.md line; each marked done in UX.md)**
- **Arrangement:** copy / cut / paste clips at the insert marker (A9); Duplicate clips or a time
  selection (A10); Delete clips, or a time selection's contents split at its edges (A11, F-22);
  Split Ctrl+E and Consolidate Ctrl+J (A12); rename in place Ctrl+R (A13); zoom Z / X / + / − and
  a double-click on the ruler (A14); ← → move the insert marker or nudge clips, ↑ ↓ select tracks
  (A15); Home and Shift+Space (A16); grid keys Ctrl+1–5 with 1/32…Bar, triplets, snap off and an
  adaptive grid (A17); right-click menus on clips, empty space / selections and the ruler, with
  submenus and disabled items that say why (A18–A20, F-24); Shift-click extends the time
  selection, across tracks (A3); the getting-started guide (A22, F-14).
- **Time commands:** Cut / Copy / Paste / Duplicate / Delete Time (Ctrl+Shift+X/C/V/D/Del, every
  track) and Insert Silence (Ctrl+I).
- **Tracks:** click selects (Shift / Ctrl adds) and the inspector shows the track (T1, I1);
  rename with Ctrl+R or a double-click, Tab to the next track, names saved (T2); right-click menu
  with Rename and Colour ▸ (T3); exclusive solo, Ctrl-click adds (T5).
- **Piano roll:** double-click creates / deletes (P1, P2); a click deselects and a drag selects
  (P3); Draw Mode B (P4); Shift+← → (P7); chop Ctrl+E and join Ctrl+J (P9); right-click menus
  (P10); Z / X / + / − (P11); Alt+← → step through notes (R74).
- **Browser:** Esc clears the search (B1); a click while playing says why it's silent (B2);
  layouts load on double-click or drag (B4); a user preset's menu: Rename, Delete, Show in
  Explorer (B5).
- **Inspector:** unusable macros hidden with the reason (I2); Make Unique / Use Track Instrument
  / Save as Preset as visible buttons (I3); tooltips on every control, disabled ones say why (I4,
  F-34). The clip name is a label (renamed in place).
- **Export dialog:** a backdrop, Esc / Enter, remembered choices, disabled choices say why
  (E1–E4).
- **Menus:** Edit (the time commands, Insert Silence, Split, Consolidate, Rename), View (zoom,
  Draw Mode) and Help (Getting Started), all from the shortcut table; the sheet regenerates.
- **Also:** undoable profile, loop brace and loop switch (F-33); one Follow for both editors
  (F-32); notices docked bottom-right (F-37); the CPU pill reads OFF without audio (F-38); 11 px
  minimum for informative text, track names with an ellipsis, "CHANNEL" instead of "DEVICE"
  (F-39). File format **v4** (track names and colours, clip colours) with an exact v3 → v4
  migration, hash-tested against a v3 corpus written by the pre-run build.

**Backlog**
- **Interface scale:** done. A fork commit (5a3b035a, pushed to `chipdaw-perf`, pin updated)
  applies `user_scale_factor` at window creation and on `ScaleFactorChanged`, and adds
  `WindowEvent::SetUserScaleFactor` for live changes. Settings ▸ Display offers 75–200%,
  applied live. Checked by screenshot at 100, 125 and 150%: the layout holds, hairlines stay
  one device pixel, spacing scales evenly, no text is clipped; PERF scenarios pass at 150%. At
  200% what is on screen renders correctly, but the window (2880×1800 px, logical 1440×900) is
  larger than this 2560×1440 display, so its right and bottom edges are off-screen: 200% needs a
  display of at least that size (or a smaller window, which the minimum size prevents).
- **Dark native menus:** not built. No documented API darkens Win32 popup menus; the documented
  route (owner-drawn items) can't recolour the popup's border, shadow or submenu arrow. Options
  in §5. **Found on the way:** muda's dark menu *bar* (shipped in Batch 2) relies on
  undocumented messages; flagged for the owner.
- **Engine matrix on the MOTU, including test E:** ENGINE_SHORT
- **`trace_app.ps1`:** still pending (it needs elevation; not attempted).

**Decisions taken alone** (B3-D1…D23 in the RUNLOG), the ones that matter most:
- Space plays from the insert marker and Shift+Space continues from the stop point (R137);
  while stopped, the playhead is the insert marker.
- Time commands act on every track (R46); the plain commands on a time selection act on its
  tracks only. Consolidate leaves a track alone if its clips play different instruments.
- Ruler double-click follows R32: the time selection if there is one, else the whole song.
- The triangle's Volume macro stays (the driver uses it as a gate), relabelled; its Duty and
  DPCM's Volume and Duty are hidden.
- The fork's branch is `chipdaw-perf` (the prompt said `chipdaw`, which doesn't exist).

**Merged:** MERGE_LINE

---

## 1. Acceptance tests

`pwsh -File docs\batch3\tools\at_b3.ps1` (`-Only AT-07,AT-11`, `-Scale 150`). Fixtures come
from `fixtures.py` (format v4, known clips and notes); each test saves the project it edited
and `check_b3.py` checks the files. The dev-mode stderr lines prove the harness reached its
scenario: `dev: menu <site>: <items>` for every context menu, `dev: zoom … range … px_per_bar`
for the zooms, `dev: piano roll bounds` to find the piano roll.

| Test | What it proves | Result |
|---|---|---|
| AT-07 (A9) | Harm A + Harm B copied, pasted at bar 9: two new clips at bars 9 and 11 with identical notes | PASS 3/3 |
| AT-08 (A13, T2) | Ctrl+R on a clip ("Verse") and on a track ("Bass Line"); double-click a name, Tab to the next; one undo takes back one rename | PASS 5/5 |
| AT-09 (P1–P4) | Double-click empty C4 bar 1: one 1/16 note; double-click it: none; a click without Draw Mode: none; B + click: one | PASS 5/5 |
| AT-10 (A18–A20, T3, P10) | Menus on a track, the ruler, a note, empty piano-roll space, a clip, empty lane space, and the Grid submenu; items as UX.md §5; disabled with no selection | PASS 14/14 |
| AT-11 (A12) | Ctrl+E at bar 3 splits bars 1–4 into 1–2 and 3–4, notes intact; Ctrl+J joins them | PASS 4/4 |
| AT-12 (A14) | Z on bars 5–6: 389.8 px per bar (view 840 px); X: back to 92; ruler double-click: 0…32 bars | PASS 4/4 |
| AT-14 (A22) | A new project shows the guide (luma 32.6 vs 17.6 after the first clip); Help ▸ Getting Started reopens it, Esc closes | PASS 2/2 |
| AT-22 (A11, A3) | Bars 2–3 time-selected on an empty track, Shift-click extends it over two tracks with clips; Del splits both at bars 2 and 4, no notes left inside | PASS 6/6 |
| AT-23 (A15, A17) | → → nudges 2 × 1/16; Ctrl+2, → one 1/8 more | PASS 2/2 |
| AT-24 (R46, R47) | Ctrl+I inserts a bar on every track; Ctrl+Shift+Del takes it out again, every note back | PASS 3/3 |
| AT-25 (T1, T5, I1, I4) | Header click selects the track (inspector "CHANNEL · PULSE 2"); exclusive solo, Ctrl-click adds; a disabled button's tooltip | PASS 3/3 |
| AT-26 (T3, A18) | Track and clip colour from their menus, saved (#4FA8E0, #E0705A) | PASS 2/2 |
| AT-27 (P7, P9, P11) | Shift+→ lengthens every note by 1/16; Ctrl+E chops 16 → 48 notes; Ctrl+J joins back to 16 | PASS 3/3 |
| AT-28 (E1–E4) | Backdrop, Esc closes, `bits:24` remembered, disabled Selection says why, Enter exports | PASS 5/5 |
| AT-29 (B1, B4) | Esc clears the search; a layout click loads nothing; a double-click asks about the profile, then loads | PASS 5/5 |
| AT-30 (§1) | Edit / View / Help menus (screenshots); the sheet lists Split, Consolidate, Rename, Cut Time, Insert Silence, Zoom to Selection, Draw Mode, Narrow Grid | PASS 11/11 |
| AT-31 (B5, I3) | Save as Preset writes a user preset; its menu: Rename, Delete, Show in Explorer | PASS 2/2 |

Unit tests added: `state::edit_tests` (split + consolidate keep every note, split keeps the
instrument and colour, mixed instruments refused, clear_range, copy/paste with refused tracks,
the time commands, nudge, the adaptive grid, zoom to a range), `notes::tests::batch3_note_ops`,
the undo property test extended with split, consolidate, clear_range, insert / delete time,
paste, rename and colour (40 seeds, undo-all and redo-all exact), `menus::tests` (every Batch 3
command is in the menus, labels from the table), `tests/migration.rs` (the v3 corpus renders
bit-identically under v4; v4 names and colours round-trip), `tests/robustness.rs` (a v4 seed in
the fuzz campaign; bad colours are errors, names cleaned).

## 2. Details

### Context menus (01–07)
A menu is built by the app with each item's state: disabled items are dimmed and the line at
the bottom says why for the hovered one ("Select one or more notes first.", "Copy or cut clips
(or time) first.", …). A ▸ row opens its submenu on hover beside the menu (to the left when
there's no room). Colour ▸ has a 12-colour palette (the lane colours first); the track's
current colour is ticked. Grid ▸ lists 1/32…Bar and Narrow / Widen / Triplet / Snap / Adaptive.

### Draw Mode and the piano roll (05, 06, 08, 09)
Draw Mode is off by default: a click selects, a drag on empty space selects, a double-click
adds a note one grid step long or deletes one (R1, R2; the length is ours, R3). B toggles it
(View ▸ Draw Mode and a Draw button in the piano roll's header show the state); in Draw Mode a
click draws and a click on a note erases it (R6).

### Split, consolidate, time selection (10–15)
Split cuts at the insert marker (a click inside a clip puts it there) or makes a time
selection its own clip(s). Consolidate merges the selected clips per track, or a time
selection's contents (spanning it exactly). Delete on a time selection clears it on its tracks
and splits at the edges; the …Time commands act on every track and close or open the gap.

### Rename (16, 17)
A text field over the clip or the track name; Enter keeps it, Esc cancels, a click elsewhere
keeps it, Tab moves to the next track. Each rename is one undo step and marks the project
unsaved. Names are saved (format v4).

### Empty-project guide (18, 19)
A card over the empty arrangement until the first clip exists; Help ▸ Getting Started shows it
over a song until a click or Esc. Its lines come from the shortcut table.

### Export dialog (20, 21)
Over the same backdrop as the Settings panel; Esc closes, Enter exports, a click on the
backdrop closes. The choices are kept in `settings.json` (`export`) and restored. A disabled
choice says why on hover (tooltips sit on a wrapper: vizia never hovers a disabled view).

### Interface scale (23–26)
| Scale | Client size (px) | Notes |
|---|---|---|
| 100% | 1440×900 | reference |
| 125% | 1800×1125 | piano roll bounds scale 1.25 |
| 150% | 2160×1350 | PERF scenarios pass (gate 3) |
| 200% | 2880×1800 | larger than the 2560×1440 display: the window extends past the screen |
| live 100 → 150% | 2160×1350 | Settings ▸ Display, applied at once, saved (`ui_scale: 150`) |

### Menus (27–30)
The Win32 popups stay light (see §5).

## 3. Remaining Batch 3 findings

| Finding | Status |
|---|---|
| F-07, F-08, F-09, F-10, F-11, F-12, F-14, F-22, F-23 | done (AT-07…12, 14, 22) |
| F-24 disabled items with reasons, platform labels | done (context menus, tooltips) |
| F-25 hints from the table | done (piano-roll hint, guide, tooltips) |
| F-26 export dialog | done (E1–E4); the "split pill" restyle is deferred: the dialog keeps the stock `.btn` toggles |
| F-27 reference keys | done in Batch 2; Ctrl+I is now Insert Silence |
| F-28 Shift+← →, chop / join, Alt+arrows | done |
| F-29 keyboard navigation | done: ← → ↑ ↓, Home, Shift+Space. **Deferred:** Enter-then-arrows edge resize (R55): it needs a clip-edge focus state the arrangement doesn't have yet |
| F-32 Follow in both editors | done |
| F-33 undo for profile, loop brace, loop switch | done; mute/solo stay out (R141 unverified) |
| F-34 tooltips | done |
| F-35 track selection | done |
| F-36 layouts load on double-click; audition while playing | done (it says why; a preview voice is Batch 4 voice-pool work) |
| F-37 notices docked | done |
| F-38 CPU pill without audio | done (OFF) |
| F-39 readability | done: 11 px minimum, ellipses, "CHANNEL" title, header spacing |
| F-13 (tracks: insert, delete, reorder) | Batch 4 (model). Rename and colour are done now |

## 4. Engine matrix (§2.3)

ENGINE_TABLE

## 5. Dark native menus: the options

| Option | Documented? | Dark popups? | Effort |
|---|---|---|---|
| muda 0.22 (newest) | — | No: no popup theming; the maintainers left popups light by choice (muda #97) | — |
| `SetPreferredAppMode` / `AllowDarkModeForWindow` / `FlushMenuThemes` (uxtheme ordinals 135 / 133 / 136) | **No** | Yes, fully native | small; forbidden by the brief |
| `DwmSetWindowAttribute(DWMWA_USE_IMMERSIVE_DARK_MODE)` | Yes | No (title bar only) | — |
| `SetWindowTheme(…, "DarkMode_Explorer")` | the class name isn't | No (controls, not menus) | — |
| `SetMenuInfo(MIM_BACKGROUND)` | Yes | Background only; text stays dark | — |
| Owner-drawn items (`MFT_OWNERDRAW`, `WM_MEASUREITEM` / `WM_DRAWITEM`, set on `WM_INITMENUPOPUP` through our own `SetWindowSubclass`) | Yes | Mostly: background, text, keys, highlight, ticks and separators are ours; the popup's border, shadow and submenu arrow stay system-drawn | ~200–300 lines in `menus.rs`, no new crate |

**Recommendation:** if the rule is strict, the owner-drawn route is the only one; it leaves a
light hairline border around each popup. **Note:** muda's dark menu bar (`MenuTheme::Dark`,
Batch 2) intercepts the undocumented `WM_UAHDRAWMENU` / `WM_UAHDRAWMENUITEM` messages; under the
same rule it would have to become `MenuTheme::Light` (one line). Left unchanged for the owner
to decide.

## 6. How to run
```
cargo build --release
pwsh -File docs\batch3\tools\at_b3.ps1           # Batch 3 acceptance tests (-Only AT-07,AT-11; -Scale 150)
pwsh -File docs\shell\tools\at_shell.ps1         # AT-01…19
pwsh -File docs\batch3\tools\scales.ps1          # interface-scale screenshots and sizes
pwsh -File docs\batch3\tools\ab_bench.ps1        # PERF A/B against ..\prerun-b3
pwsh -File docs\batch3\tools\perf_rest.ps1       # pacing, piano roll, 150%
pwsh -File docs\engine\tools\matrix.ps1 -Tests A,B,C,G,H,D,E -Buffers 128,256,512
```
