# UX audit: summary for the team

*2026-11-25 · audited `stabilize/v1.1` @ `c0037d3` against VISION.md · Windows · details:
[INVENTORY](INVENTORY.md) · [FINDINGS](FINDINGS.md) · [FEASIBILITY](FEASIBILITY.md) ·
[UX-DRAFT](UX-DRAFT.md) · [manual references](manual-refs.md) (151 behaviours checked against the
Live 12 manual: 133 verified, 3 contradicted, 15 unverified)*

## The 10 most important findings
1. **F-01 Unsaved work is lost silently.** Quit, New, Open, Import MIDI and file drops replace the song without asking; a clean quit doesn't offer the autosave. *Must, S.*
2. **F-02 F4 swaps the song for the stress test** (F3–F7 debug keys are live in release). *Must, S.*
3. **F-03 Shortcuts fire while typing:** Space in the search box plays, and Ctrl+Z in a text field undoes the document. *Must, S.*
4. **F-06 No app shell:** no menus, Settings, Project Settings, Open Recent, About or shortcut sheet (F-15, F-16, F-18). *Must, M.*
5. **F-04 "Pretends to work":** with CHIP = 2A03 the VRC6 lanes look and edit normally but are silent. *Must, S.*
6. **F-09 / F-23 Piano roll:** double-click doesn't create/delete notes; draw is always on and there's no B toggle. *Must, S.*
7. **F-07 / F-22 Arrangement editing:** no clip copy/paste, and Delete/Duplicate ignore a time selection. *Must, M.*
8. **F-10 / F-08 Context menus and rename:** menus only on clips and empty space; no Ctrl/Cmd+R; tracks can't be renamed. *Must, M.*
9. **F-11 / F-12 Split, consolidate and zoom keys** (Ctrl/Cmd+E, Ctrl/Cmd+J, Z/X, fit) are missing. *Must, S–M.*
10. **F-17 / F-13 The model:** MIDI import replaces the song, and tracks can't be added, deleted, reordered or coloured. Fixing this needs the tracks-as-instruments change. *Must, L.*

## Redundancy: proposed decisions
| Duplicate | Keeps its home in |
|---|---|
| CHIP pill · channel layouts | **Project Settings** (hardware profile); layouts become templates that ask first |
| FILTER pill · F7 | **Project Settings** |
| BPM pill · "tempo" in VISION's Project Settings | **Top bar** (as in Live, R116). *CEO to confirm: differs from VISION.md* |
| NEW/OPEN/SAVE/MIDI IN/EXPORT keys | **Menus + shortcuts** (keys removed) |
| SNAP (top bar) · GRID (piano roll) | Each editor keeps its own grid (as Live); SNAP leaves the top bar |
| Clip name field · Ctrl+R | **In place** (Ctrl/Cmd+R) |
| Clock double-click rewinds · Space · play light | Play light + Space; **Home** rewinds |

## Feasibility answers
- **Native menus: yes.** `muda` (MIT/Apache) works on our fork with no fork change; commands reach the model in < 1 ms. Don't hook accelerators on Windows (they double-fire). The menu's modal loop pauses drawing, not audio.
- **Cursor position during a file drag: not today.** winit reports no positions during a hover on Windows or macOS. A small fork change (forward hover events, poll the cursor each frame) worked in a probe, so the live MIDI preview is buildable.
- **Settings window: use an in-app modal panel.** A second window works on the fork but has title and focus bugs, and baseview (plugins) can't open one.
- **Model change: feasible without a rewrite.** The emulator, macros, snapshot/garbage queue, undo and migrations carry over. Order: M0 spec → M1 UI tracks (8 locked, hash-identical) → M2 engine snapshot → M3 studio voices + pool → M4 profiles. M3 carries the risk. About 6–8 weeks.

## Proposed build order
1. **Safety** (~1 wk): save prompts, debug keys gated, text-field focus, one shortcut table, VRC6-off dimming.
2. **App shell** (~2 wk): menus, Settings, Project Settings, Open Recent, About, shortcut sheet; top bar slimmed.
3. **Ableton-parity fixes** (~2–3 wk): piano-roll double-click/draw mode, zoom, rename, context menus, copy/paste, split/consolidate, keyboard navigation, empty-project guide, tooltips, export dialog.
4. **Model change** (~6–8 wk): tracks as instruments, voice pool, profiles and locked tracks, MIDI import as tracks with a drop preview.
5. **Time signatures and scales** (~2 wk). 6. **Automation** (~3–4 wk), then effects.

## Assumptions
- `main` hadn't been merged, so `stabilize/v1.1` is the newest code.
- Screenshots were taken with audio off and a throw-away data dir; audio-dependent behaviour was read from source.
- macOS was read from source only (muda, winit). The Mac pass must test it.
- The manual check covered the Live 12 online manual; long chapters were read through a summarising fetch, so a few "unverified" items may be documented further down a page.
- Where VISION.md and the reference disagree, this audit flags it rather than choosing:
  - breakpoints are deleted with a single click in Live (R108), not a double-click;
  - in-scale arrow transposition is unverified (R62);
  - Live keeps tempo in the control bar (RD-3).
- Shortcut conflicts were resolved in favour of the reference: Ctrl+I → Insert Silence, Ctrl+U → quantize, Ctrl+Shift+F → Follow.
- Effort sizes assume one engineer who knows the codebase.
