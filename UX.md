# UX.md: the build spec

*Adopted 2026-11-25 from the UX audit's [UX-DRAFT.md](docs/ux-audit/UX-DRAFT.md). **Every priority
was accepted by the CEO as proposed** (✓ in the CEO column). [VISION.md](VISION.md) still wins on
any conflict; this page is the build spec under it. Each line keeps its acceptance test (§6).
Background: [findings](docs/ux-audit/FINDINGS.md), [feasibility](docs/ux-audit/FEASIBILITY.md),
[manual references](docs/ux-audit/manual-refs.md).*

Conventions: **Cmd** = Ctrl on Windows, ⌘ on macOS. Keys follow the Live 12 manual where the audit
verified them (R-numbers → [manual-refs.md](docs/ux-audit/manual-refs.md)); keys marked **(ours)**
are not from the reference. Priority: **M** must · **S** should · **L** later.

## CEO decisions (2026-11-25)

1. **Tempo lives in the top bar** (R116), with the time signature next to it when it arrives. It is
   not in Project Settings.
2. **A single click on an automation breakpoint deletes it** (R108). VISION.md was updated.
3. **Reference shortcuts:** Quantize = **Cmd+U** (Q is retired); **Cmd+I = Insert Silence** (it no
   longer imports MIDI); **Follow = Ctrl+Shift+F** (⌥⇧F on macOS, R135).
4. **The top bar slims down:** the NEW / OPEN / SAVE / MIDI IN / EXPORT keys, CHIP, FILTER and SNAP
   leave it, for the menus, Project Settings and the editors.
5. **Channel layouts become templates** that ask before they change the hardware profile.
6. **The new CLAUDE.md is adopted.**

**Correction to the draft:** the frame-rate cap defaults to **60 fps** (PERF-macos.md §6,
ENGINE-macos.md §1), not "Display rate". The options are **60** and **120**; **Off** exists only in
dev mode (`CHIPDAW_DEV=1`).

---

## 1. Menu structure

Native menus (docs/ux-audit/FEASIBILITY.md §1). On macOS the App menu holds About, Settings… (⌘,), Hide, Quit.
On Windows, About goes under Help and Settings… under Edit (Windows convention), Exit under File.

### File
| Item | Windows | macOS | Does | Pri | CEO |
|---|---|---|---|---|---|
| New | Ctrl+N | ⌘N | New empty project (asks to save if dirty) | M | ✓ |
| New from MIDI File… | — | — | Today's "replace with MIDI" behaviour | S | ✓ |
| Open… | Ctrl+O | ⌘O | Open a `.chipdaw` (asks to save if dirty) | M | ✓ |
| Open Recent ▸ | — | — | Last 10 projects; "Clear Menu" | M | ✓ |
| Save | Ctrl+S | ⌘S | Save | M | ✓ |
| Save As… | Ctrl+Shift+S | ⌘⇧S | Save under a new name | M | ✓ |
| Import ▸ MIDI File… | — | — | Adds the file's parts as new tracks at the insert marker (F-17) | M | ✓ |
| Import ▸ Audio as DPCM Sample… | — | — | WAV → DPCM slot | M | ✓ |
| Export Audio… | Ctrl+Shift+R | ⌘⇧R | Export dialog (R124) | M | ✓ |
| Export MIDI Clip… | Ctrl+Shift+E | ⌘⇧E | Selected clip as `.mid` (R125) | L | ✓ |
| Project Settings… | — | — | Profile, filter (§3) | M | ✓ |
| Exit (Windows) | Alt+F4 | (App menu: Quit ⌘Q) | Quit (asks to save if dirty) | M | ✓ |

### Edit
| Item | Windows | macOS | Does | Pri | CEO |
|---|---|---|---|---|---|
| Undo | Ctrl+Z | ⌘Z | Undo (R21) | M | ✓ |
| Redo | Ctrl+Y (also Ctrl+Shift+Z, ours) | ⌘⇧Z | Redo (R22; Ctrl+Shift+Z on Windows is not in the manual, R23) | M | ✓ |
| Cut / Copy / Paste | Ctrl+X / C / V | ⌘X / C / V | Clips, notes, or the time selection (R24) | M | ✓ · **done** (B3) |
| Duplicate | Ctrl+D | ⌘D | Clips/notes/time, placed right after (R25) | M | ✓ · **done** (B3) |
| Delete | Del | ⌫ / Del | Clips/notes/time (R26) | M | ✓ · **done** (B3) |
| Cut / Copy / Paste / Duplicate / Delete Time | Ctrl+Shift+X/C/V/D/Del | ⌘⇧ + same | Time commands (R46) | S | ✓ · **done** (B3) |
| Insert Silence | Ctrl+I | ⌘I | At the time selection (R47; decision 3: no longer MIDI import) | S | ✓ · **done** (B3) |
| Split | Ctrl+E | ⌘E | Clips at the insert marker or selection edges; notes in the piano roll (R38, R73) | M | ✓ · **done** (B3) |
| Consolidate | Ctrl+J | ⌘J | Selected clips on a track → one clip (R39) | M | ✓ · **done** (B3) |
| Select All | Ctrl+A | ⌘A | In the focused editor (R27) | M | ✓ |
| Rename | Ctrl+R | ⌘R | Selected clip / track / marker (R8–R11) | M | ✓ · **done** (B3) |
| Quantize | Ctrl+U | ⌘U | Selected notes (R64; decision 3: Q retired) | S | ✓ |
| Settings… (Windows) | Ctrl+, | (App menu ⌘,) | Settings window (R129) | M | ✓ |

### Create
| Item | Windows | macOS | Does | Pri | CEO |
|---|---|---|---|---|---|
| Track | Ctrl+Shift+T | ⌘⇧T | New studio track after the selected one (R84) | M (model) | ✓ |
| MIDI Clip | Ctrl+Shift+M | ⌘⇧M | In the time selection (R93) | M | ✓ |
| Time Signature Change | — | — | At the insert marker (R117) | L | ✓ |

### View
| Item | Windows | macOS | Does | Pri | CEO |
|---|---|---|---|---|---|
| Browser | Ctrl+Alt+B | ⌘⌥B | Show/hide (R80) | M | ✓ |
| Piano Roll / Inspector | Shift+Tab | ⇧Tab | Toggle the bottom panel between clip editor and instrument (R78) | S | ✓ |
| Follow | Ctrl+Shift+F | ⌥⇧F | Auto-scroll while playing, both editors (R135; decision 3) | S | ✓ |
| Zoom In / Out | + / − | + / − | Time zoom of the focused editor (R34) | M | ✓ · **done** (B3) |
| Zoom to Selection | Z | Z | (R30) | M | ✓ · **done** (B3) |
| Zoom Back | X | X | (R31) | M | ✓ · **done** (B3) |
| Zoom to Fit | (double-click the ruler) | same | Whole song (R32) | M | ✓ · **done** (B3) |
| Draw Mode | B | B | Toggle (R5) | M | ✓ · **done** (B3) |
| Clock: Bars / Time | — | — | Replaces the click-on-clock toggle (F-38); Home rewinds | S | ✓ |
| Automation Mode | A | A | (R104) | L | ✓ |

### Help
| Item | Does | Pri | CEO |
|---|---|---|---|
| Keyboard Shortcuts | A generated sheet from the one shortcut table (F-30), per platform | M | ✓ |
| Getting Started | Reopens the empty-project guide (F-14) | S | ✓ · **done** (B3) |
| About ChipDAW (Windows) | Version, commit, licences, font credits (ASSETS.md) | M | ✓ |

### Window (macOS only)
Minimize ⌘M, Zoom, bring all to front: the system defaults.

## 2. Settings window

Built as an **in-app modal panel**, not a second OS window (docs/ux-audit/FEASIBILITY.md §3: the fork's
second window has title and focus bugs, and baseview plugins can't open one).

Stored in `settings.json` in the data dir (`%APPDATA%\ChipDAW`, `~/Library/Application
Support/ChipDAW`). Every field has a default, unknown fields are ignored (today's rule,
`file.rs:677`).

| Tab | Setting | Default | Pri | CEO |
|---|---|---|---|---|
| Audio | Output device | System default | M | ✓ |
| | Buffer size | Device default (show the latency in ms) | M | ✓ |
| | Sample rate | 48 kHz (44.1 / 48 / 96) | M | ✓ |
| Display | Frame-rate cap (replaces F6) | **60 fps** (options 60 / 120; Off in dev mode only) | S | ✓ |
| | Interface scale | 100% (follows the OS) | S | ✓ · **done** (B3): 75–200%, live |
| | Clock shows | Time (m:ss.cc) | S | ✓ |
| Editing | Default arrangement grid | 1/16 | S | ✓ |
| | Default piano-roll grid | 1/16 | S | ✓ |
| | Follow playhead | On | S | ✓ |
| Files | Autosave interval | 2 min | S | ✓ |
| | Offer recovery after an unsaved quit | On | M | ✓ |
| | Open last project at start | Off | S | ✓ |
| Warnings | "Don't ask again" choices, each with a Reset | all asking | M (with F-47) | ✓ |
| | Reset all warnings | — | M | ✓ |

Also stored (not shown): recent files, window size/position, browser open, export options,
split height.

## 3. Project Settings

Stored **in the project file** (v4).

| Setting | Default | Notes | Pri | CEO |
|---|---|---|---|---|
| Hardware profile | None (all studio tracks) after the model change; **2A03 + VRC6** until then | Changing it checks locked tracks and asks first (P5) | M | ✓ |
| Output filter | Profile's default (NES / Famicom / Off) | Moves here from the top bar | M | ✓ |
| Tempo | — | **Not here: it lives in the top bar** (decision 1, R116) | — | ✓ |
| Time signature | — | **Not here: the top bar**, next to tempo (decision 1, R116) | L | ✓ |
| Root and scale | Off | VISION "Scales" | L | ✓ |

## 4. Redundancy decisions (one home each)

| Function | Its one home | Removed from |
|---|---|---|
| Hardware chip / profile | Project Settings | CHIP pill; layouts are templates that ask before changing it (decision 5) |
| Output filter | Project Settings | FILTER pill, F7 |
| Tempo (and later time signature) | Top bar | — (not in Project Settings) |
| New / Open / Save / Import / Export | Menus + shortcuts | Top-bar keys |
| Arrangement grid | Arrangement (context menu + Ctrl+1–5) | Top-bar SNAP pill |
| Clip name | In place (Ctrl+R) | Inspector text field becomes a label |
| Loop on/off | Capsule loop light (+ Ctrl+L sets and enables) | — |
| Rewind | Home key / Stop when stopped | Clock double-click |
| Debug tools | `CHIPDAW_DEV=1` builds only | F3–F7 in release |

## 5. Behaviour tables

### 5.1 Arrangement
| # | Action | Result | Pri | CEO |
|---|---|---|---|---|
| A1 | Click empty space | Insert marker there; clears clip selection (today) | M | ✓ |
| A2 | Drag empty space | Time selection across tracks (today) | M | ✓ |
| A3 | Shift-click | Extends the time selection, across tracks (R44) | S | ✓ · **done** (B3) |
| A4 | Click / Shift-click / Ctrl-click a clip | Select / add range / toggle (R28) | M | ✓ |
| A5 | Drag clip, edges | Move (also to another track) / resize (today) | M | ✓ |
| A6 | Ctrl-drag (⌥-drag) clip | Copy (today; unverified for the arrangement, R48) | S | ✓ |
| A7 | Double-click clip | Open in the piano roll (today; R150) | M | ✓ |
| A8 | Double-click empty lane | New 1-bar clip (today; ours) | S | ✓ |
| A9 | Cmd+C / X / V | Copy / cut / paste clips at the insert marker | M | ✓ · **done** (B3) |
| A10 | Cmd+D | Duplicate clips or time | M | ✓ · **done** (B3) |
| A11 | Del | Delete clips, or the time selection's contents | M | ✓ · **done** (B3) |
| A12 | Cmd+E / Cmd+J | Split / consolidate | M | ✓ · **done** (B3) |
| A13 | Cmd+R | Rename clip (in place) | M | ✓ · **done** (B3) |
| A14 | Z / X / + / − / double-click ruler | Zoom to selection / back / in / out / fit | M | ✓ · **done** (B3) |
| A15 | ← → | Move insert marker / nudge selected clips by the grid (R50, R51) | S | ✓ · **done** (B3) |
| A16 | Home, Shift+Space | Rewind / continue (R137, R138) | S | ✓ · **done** (B3) |
| A17 | Cmd+1 / 2 / 3 / 4 / 5 | Narrow / widen / triplet / snap / adaptive (R56–R59) | S | ✓ · **done** (B3) |
| A18 | Right-click clip | Cut, Copy, Paste, Duplicate, Delete, Split, Consolidate, Rename, Colour ▸, Loop Clip, Make Instrument Unique, Use Track Instrument, Save Instrument as Preset | M | ✓ · **done** (B3) |
| A19 | Right-click empty / selection | Paste, Insert MIDI Clip, Loop Selection, Insert Silence, Delete Time, Grid ▸ | M | ✓ · **done** (B3) |
| A20 | Right-click ruler / scrub area | Set Loop to Selection, Zoom to Fit, Insert Time Signature Change (L) (R17) | S | ✓ · **done** (B3): Set Loop to Selection, Zoom to Fit; the time-signature item comes with F-45 |
| A21 | Drop a `.mid` | Adds tracks at the drop point (F-17; live preview later, F-47) | M | ✓ |
| A22 | Empty project | Guide overlay until the first clip exists (F-14) | M | ✓ · **done** (B3) |

### 5.2 Track headers
| # | Action | Result | Pri | CEO |
|---|---|---|---|---|
| T1 | Click | Select the track (Shift/Ctrl add) | S | ✓ · **done** (B3) |
| T2 | Cmd+R / double-click name | Rename (Tab → next track, R9) | M | ✓ · **done** (B3) |
| T3 | Right-click | Rename, Colour ▸, Insert Track, Duplicate, Delete, Lock/Unlock (L) | M | ✓ · **done** (B3): Rename, Colour ▸. Insert / Duplicate / Delete Track wait for the model change (Batch 4); Lock is L |
| T4 | Drag header | Reorder (R88) | M (model) | ✓ |
| T5 | M / S | Mute / solo (today); Cmd-click solo adds (R143) | S | ✓ · **done** (B3) |
| T6 | Del with a track selected | Delete track (R86) | M (model) | ✓ |
| T7 | Click 🔒 | Unlock: becomes a studio track that sounds the same (VISION) | L | ✓ |

### 5.3 Piano roll
| # | Action | Result | Pri | CEO |
|---|---|---|---|---|
| P1 | Double-click empty | Create a note of the grid length (R2; length unverified, R3) | M | ✓ · **done** (B3) |
| P2 | Double-click note | Delete it (R1) | M | ✓ · **done** (B3) |
| P3 | Click / drag empty (draw mode off) | Deselect / marquee (R76) | M | ✓ · **done** (B3) |
| P4 | B | Toggle draw mode; in draw mode a click draws (today's behaviour) (R5, R6) | M | ✓ · **done** (B3) |
| P5 | ↑ ↓ / Shift | Semitone / octave (today; R60, R61) | M | ✓ |
| P6 | ← → | Nudge by grid (today; R52) | M | ✓ |
| P7 | Shift+← → | Shorten / lengthen (R63) | S | ✓ · **done** (B3) |
| P8 | Cmd+U | Quantize (R64); Q retired | S | ✓ |
| P9 | Cmd+E / Cmd+J | Chop / join notes (R73) | S | ✓ · **done** (B3) |
| P10 | Right-click note / empty | Cut, Copy, Paste, Duplicate, Delete, Quantize, Select All | M | ✓ · **done** (B3) |
| P11 | Z / X / + / − | Zoom (R75) | M | ✓ · **done** (B3) |
| P12 | F / G | Fold / Fold to Scale (R68, R69) | L | ✓ |

### 5.4 Browser
| # | Action | Result | Pri | CEO |
|---|---|---|---|---|
| B1 | Typing in search | Never triggers shortcuts (F-03); Esc clears | M | ✓ · **done** (B3) |
| B2 | Click preset | Select + hear (today); while playing: hear through the preview voice or say why not | S | ✓ · **done** (B3): no preview voice yet, so a click while playing says why |
| B3 | Double-click / drag preset | Load (today) | M | ✓ |
| B4 | Click layout | Select only; double-click or drag loads (F-36). A layout is a template: if it needs another hardware profile, it asks first (decision 5) | S | ✓ · **done** (B3) |
| B5 | Right-click user preset | Rename, Delete, Show in Explorer/Finder | S | ✓ · **done** (B3) |
| B6 | Authentic preset row | Shows 🔒 + profile; loading it locks the track (VISION) | L | ✓ |

### 5.5 Inspector
| # | Action | Result | Pri | CEO |
|---|---|---|---|---|
| I1 | Select a track (no clip) | Shows the track's instrument | S | ✓ · **done** (B3) |
| I2 | Controls the channel lacks | Hidden (triangle Duty/Volume macros) (F-05) | S | ✓ · **done** (B3) |
| I3 | Instrument header | Make Unique, Use Track Instrument, Save as Preset as visible buttons | S | ✓ · **done** (B3) |
| I4 | Hover any control | Tooltip; disabled ones say why (F-34) | S | ✓ · **done** (B3) |

### 5.6 Automation lanes (later)
| # | Action | Result | Pri | CEO |
|---|---|---|---|---|
| L1 | A | Show/hide automation (R104) | L | ✓ |
| L2 | Click on envelope | Add a breakpoint (R106) | L | ✓ |
| L3 | Drag breakpoint / Alt-drag segment | Move / curve (R107, R109) | L | ✓ |
| L4 | Click a breakpoint | **Deletes it** (R108; decision 2) | L | ✓ |
| L5 | Touch an automated control | Override; Re-Enable Automation lights (R111) | L | ✓ |
| L6 | Locked track | Values quantised to register steps, applied on the frame tick (VISION) | L | ✓ |

### 5.7 Export
| # | Action | Result | Pri | CEO |
|---|---|---|---|---|
| E1 | Cmd+Shift+R | Modal dialog with backdrop (F-26) | S | ✓ · **done** (B3) |
| E2 | Esc / Enter | Close / Export | S | ✓ · **done** (B3) |
| E3 | Disabled option | Says why on hover | S | ✓ · **done** (B3) |
| E4 | Reopen | Last options remembered | S | ✓ · **done** (B3) |

## 6. Acceptance tests for every "must"

Written for the scenario harness (`docs/*/tools/*.ps1` synthetic input + a Python checker on the
saved file or screenshot), or as `crates/ui/tests` unit tests where marked [unit].

| ID | Test |
|---|---|
| AT-01 (F-01) | Edit a clip, press Ctrl+N: a "Save changes?" prompt appears; Cancel leaves the clip; Don't Save gives an empty project. Repeat for Ctrl+O, Import MIDI, a dropped `.chipdaw`, and closing the window. With no edits, Ctrl+N shows no prompt |
| AT-02 (F-02) | In a release build without `CHIPDAW_DEV`, press F3–F7: the saved project is byte-identical and no overlay appears (screenshot diff) |
| AT-03 (F-03) | Focus the browser search, type "a b": transport stays stopped (playhead tick unchanged in the saved file) and the field reads "a b". Rename a clip, press Ctrl+Z in the field: the saved clip name is the new one |
| AT-04 (F-04) | Set the profile to 2A03 with clips on VRC6 lanes: a confirmation appears; after it, the VRC6 headers are drawn dimmed (pixel check) |
| AT-06 (F-06) | Menus exist: invoking File > Save via the menu API writes the file; every menu item's accelerator matches the shortcut table [unit] |
| AT-07 (F-07) | Select 2 clips, Ctrl+C, click bar 9, Ctrl+V: the file has 2 more clips starting at bar 9 with identical notes |
| AT-08 (F-08) | Select a clip, Ctrl+R, type "Verse", Enter: the clip is named "Verse"; the same on a track header names the track |
| AT-09 (F-09, F-23) | In the piano roll: double-click empty at C4 bar 1 → one note; double-click it → zero notes; B, click → one note |
| AT-10 (F-10) | Right-click on a track header, a note, empty piano-roll space, the ruler: a menu opens each time (screenshot) with the items in §5 |
| AT-11 (F-11) | Clip bars 1–4, insert marker at bar 3, Ctrl+E → two clips 1–2 and 3–4 with the notes split; select both, Ctrl+J → one clip with all notes |
| AT-12 (F-12) | Time-select bars 5–6, Z: the view spans bars 5–6 ± margin (zoom value in a debug read-out or screenshot); X restores; double-click the ruler fits the song |
| AT-13 (F-13) | Ctrl+Shift+T adds a track; drag it above track 1; Delete removes it; colour from the context menu is saved |
| AT-14 (F-14) | New project shows the guide overlay (screenshot); after double-click in a lane it is gone |
| AT-15 (F-15) | Ctrl+, opens Settings; change the buffer size; restart: the value persists in `settings.json` |
| AT-16 (F-16, F-20, F-21) | Project Settings changes the profile and filter; the top bar has no CHIP/FILTER pills; the values are saved in the project |
| AT-17 (F-17) | With an existing song, File > Import > MIDI adds tracks; the old clips are unchanged in the saved file |
| AT-18 (F-18) | Save two projects; File > Open Recent lists both, newest first; Help > About opens; Help > Keyboard Shortcuts lists every table entry |
| AT-19 (F-19) | The top bar has no NEW/OPEN/SAVE/MIDI IN/EXPORT keys (screenshot); Ctrl+S still saves |
| AT-22 (F-22) | Time-select bars 2–3 across two tracks with clips, press Del: notes in that range are gone, clips split at the edges |
| AT-23 (A15, A17) | Select a clip, → → : it moves 2 × 1/16; Ctrl+2, → : one 1/8 more (saved file) |
| AT-24 (A10, R46, R47) | Time-select bar 2 over three tracks, Ctrl+I: a bar of silence on every track; Ctrl+Shift+Del: every note back in place (saved files) |
| AT-25 (T1, T5, I1) | Click a track header: the inspector shows that channel (UIA text); S on track 1 then 2 = only 2 soloed; Ctrl-click S on 3 adds it (saved file); a disabled button's tooltip (screenshot) |
| AT-26 (T3, A18) | Track header menu > Colour > Sky, clip menu > Colour > Coral: both saved |
| AT-27 (P7, P9, P11) | In the piano roll, Ctrl+A, Shift+→: notes one grid step longer; Ctrl+E chops on the grid; Ctrl+J joins (saved files); Z zooms (screenshot) |
| AT-28 (E1–E4) | Ctrl+Shift+R opens the dialog over a backdrop; Esc closes it; a choice is remembered in `settings.json`; the disabled Selection says why (tooltip); Enter exports |
| AT-29 (B1, B4) | Type in the search, Esc: the field is empty; a click on a layout loads nothing; a double-click asks about the profile, then loads it (saved files) |
| AT-30 (§1 menus) | The Edit, View and Help menus show the Batch 3 items (screenshots); the shortcut sheet lists them |
| AT-31 (B5, I3) | Save as Preset (a visible button) writes a user preset; its browser menu offers Rename, Delete, Show in Explorer |
| [unit] (A9–A12, F-22, P7, P9, R46, R47, format v4) | `state::edit_tests`, `notes::tests::batch3_note_ops`, the undo property test (split, consolidate, time commands, paste, rename, colour), `tests/migration.rs` (v3 → v4 hashes), `tests/robustness.rs` (v4 names and colours) |

Batch 3 results: AT-07…AT-12, AT-14, AT-22 and AT-23…AT-31 pass in
`docs/batch3/tools/at_b3.ps1` (checker `check_b3.py`); AT-01…AT-19 still pass in
`docs/shell/tools/at_shell.ps1`. See [BATCH3.md](BATCH3.md).

## 7. Build order

Batches 1 and 2 were built on `app-shell/batch-1-2` (SHELL.md).

| Batch | Content | Findings | Size |
|---|---|---|---|
| **1. Safety + shell foundation** | Save prompts and dirty flag, debug keys gated, text-field focus rule, one shortcut table, VRC6-off dimming | F-01, F-02, F-03, F-04, F-30 | ~1 week |
| **2. App shell** | Native menus (File/Edit/Create/View/Help, macOS App menu), Open Recent, About, shortcut sheet, Settings window, Project Settings; top bar slimmed (file keys, CHIP, FILTER, SNAP out) | F-06, F-15, F-16, F-18, F-19, F-20, F-21, F-31 | ~2 weeks |
| **3. Ableton-parity quick fixes** | Piano roll double-click/draw mode, zoom keys, rename, context menus, copy/paste, split/consolidate, time-selection editing, keyboard navigation, shortcut conflicts, empty-project guide, export dialog fixes, tooltips | F-07–F-12, F-14, F-22–F-29, F-32–F-39 | ~2–3 weeks |
| **4. The model change** | FEASIBILITY.md §4.6 M0–M4: tracks as instruments, voice pool, profiles and locked tracks, MIDI import as tracks (+ drop preview when FEASIBILITY §2 allows) | F-13, F-17 (full), F-40, F-42, F-43, F-47, F-48 | ~6–8 weeks |
| **5. Time and pitch** | Time signatures, scales | F-45, F-46 | ~2 weeks |
| **6. Automation** | Lanes, clip envelopes, tempo automation (then effects, after the CEO's list) | F-44 | ~3–4 weeks |

Every batch ends with its acceptance tests in the scenario harness (VISION.md, "How we work").
