# UX audit, part 1: inventory (what exists today)

*2026-11-25 · audited build: `stabilize/v1.1` @ `c0037d3` (main has not been merged yet, so this is
the newest code) · Windows 11, 1440×900 window at 100% · built-in demo song, audio off, a
throw-away data dir · screenshots made by [`tools/screens_ux.ps1`](tools/screens_ux.ps1) and
[`tools/probe_textfocus.ps1`](tools/probe_textfocus.ps1) (synthetic input, as in the earlier
scenario scripts).*

Every row comes from the source (file:line given) and, where a screenshot is listed, from the
running app. "→" means "does". Findings (what is wrong with it) are in [FINDINGS.md](FINDINGS.md);
IDs like **F-12** point there.

| Screen | |
|---|---|
| [01-main-idle](screens/01-main-idle.png) | Main window, nothing selected: browser, track headers, arrangement |
| [02-clip-selected-inspector](screens/02-clip-selected-inspector.png) | A pulse clip selected: device inspector |
| [03-context-menu-clip](screens/03-context-menu-clip.png) | Right-click on a clip |
| [04-context-menu-empty](screens/04-context-menu-empty.png) | Right-click on empty lane space |
| [05-context-menu-time-selection](screens/05-context-menu-time-selection.png) | Right-click inside a time selection |
| [06-right-click-track-header](screens/06-right-click-track-header.png) | Right-click on a track header: nothing happens |
| [07-inspector-triangle](screens/07-inspector-triangle.png) | Triangle clip: "No controls", but four macro graphs |
| [08-inspector-noise-kit](screens/08-inspector-noise-kit.png) | Noise clip: kit drum map |
| [09-inspector-dpcm](screens/09-inspector-dpcm.png) | DPCM clip |
| [10-piano-roll](screens/10-piano-roll.png) | Piano roll (double-click a clip) |
| [11-right-click-piano-roll](screens/11-right-click-piano-roll.png) | Right-click in the piano roll: nothing happens |
| [12-export-dialog](screens/12-export-dialog.png), [13-export-dialog-after-esc](screens/13-export-dialog-after-esc.png) | Export dialog; Esc doesn't close it |
| [14-chip-2a03-only](screens/14-chip-2a03-only.png) | CHIP = 2A03: the VRC6 lanes still look active |
| [15-browser-closed](screens/15-browser-closed.png) | Ctrl+Alt+B hides the browser |
| [16-new-empty-project](screens/16-new-empty-project.png) | Ctrl+N: the empty project |
| [17-notice-nothing-to-undo](screens/17-notice-nothing-to-undo.png) | The notice capsule ("Nothing to undo") |
| [18-space-in-search-field](screens/18-space-in-search-field.png) | Typing "a b" in the browser search started playback |
| [19-ctrl-z-in-clip-name-field](screens/19-ctrl-z-in-clip-name-field.png) | Ctrl+Z inside the clip-name field ran the global undo |

Older screenshots still current for this build: `docs/screens/v1.1/` (knobs, clock formats,
clip's own instrument, export dialog), `docs/screens/product/02-drag-preset.png` (preset drag),
`docs/screens/redesign/main-playing-150.png` / `-200.png` (scaling).

---

## 1. Window and app shell

| Item | Today | Source |
|---|---|---|
| Window | One window, "ChipDAW", 1440×900 default, minimum 1140×812 logical (`ui::MIN_WINDOW_SIZE`), vsync on | `crates/app/src/main.rs` |
| Native menu bar | **None** (no File/Edit/View menus on any platform; macOS gets only winit's default app menu) | — |
| Settings window | **None.** One preference exists (clock format), toggled by clicking the clock | `file.rs:677` |
| Project settings | **None as a place.** Tempo, chip and filter are pills in the top bar; there is no time signature (4/4 is fixed: `TICKS_PER_BAR`) | `lib.rs:1667` |
| About box, shortcut reference, Help | **None.** Hints are baked into labels ("Ctrl+Alt+B", the piano-roll header line) | `browser.rs:498`, `lib.rs:1855` |
| Open Recent | **None** | — |
| Quit | The window's close button. No "save changes?" prompt (**F-01**) | `lib.rs:635` |
| Command line | `app [song.chipdaw|song.mid|sample.wav …] [--size WxH] [--render-demo DIR]`; env `CHIPDAW_PROJECT`, `CHIPDAW_DATA_DIR`, `CHIPDAW_AUDIO=0`, `CHIPDAW_VSYNC=0`, `CHIPDAW_SONG=heavy` | `main.rs`, PRODUCT.md §4 |
| Files dropped on the window | `.mid` → import (replaces the song), `.wav` → DPCM sample, `.chipdaw` → open (replaces the song); anything else → notice "Drop a .mid, .wav or .chipdaw file." No hover feedback | `lib.rs:2052` |

## 2. Top bar ([01](screens/01-main-idle.png))

Left to right. All are custom-drawn "keys" and "split pills" (`views/chrome.rs`).

| Control | Shows | Does | States | Source |
|---|---|---|---|---|
| **NEW** key | "NEW" | New empty project (same as Ctrl+N). No save prompt | normal / hover / pressed | `lib.rs:518` |
| **OPEN** key | "OPEN" | Native open dialog, `.chipdaw` (Ctrl+O). No save prompt | same | `lib.rs` OpenProject |
| **SAVE** key | "SAVE" | Save to the current path, else a save dialog (Ctrl+S). Save As has no key (Ctrl+Shift+S only) | same | `lib.rs:523` |
| **MIDI IN** key | "MIDI IN" | Native open dialog for `.mid`; the file **replaces** the arrangement, keeping the lane instruments (Ctrl+I) | same | `lib.rs:524, 2069` |
| **EXPORT** key | "EXPORT" | Opens the export dialog (Ctrl+Shift+R) | same | `export_dialog.rs` |
| **BPM** pill | tempo, 1 decimal | Drag vertically (Shift = fine), wheel ±1, double-click → 150 (default). Range `Transport::MIN_BPM..MAX_BPM`. Undoable | hover | `chrome.rs:341-392` |
| **SNAP** pill | arrangement grid (1/16, 1/8, 1/4, 1/2, Bar…) | Click cycles to the next value. No way back except cycling. Not undoable and not saved in the project | hover | `lib.rs:434` |
| **CHIP** pill | "2A03 + VRC6" / "2A03" | Click toggles VRC6 on/off. The VRC6 lanes stay visible and editable when off ([14](screens/14-chip-2a03-only.png), **F-04**). Also changes the filter unless the filter was set by hand. Not undoable | hover | `lib.rs:343, 1009` |
| **FILTER** pill | "NES" / "FAMICOM" / "OFF"… | Click cycles the output filter (also F7). Not undoable | hover | `lib.rs:425` |
| **Transport capsule**: play light | ▶ / ❚❚ (lime while playing) | Toggles play from the playhead (= Space) | stopped / playing | `chrome.rs:580` |
| capsule: clock | `m:ss.cc` or bars.beats.sixteenths | Click toggles the format (saved in `settings.json`). Drag horizontally scrubs the playhead in 1/16 steps. Double-click → stop and return to the start | hover (↔ cursor) | `chrome.rs:537-600` |
| capsule: loop light | ↻ (lime when on) | Toggles song looping (also double-click on the loop brace) | on / off | `chrome.rs:581` |
| **Spectrum box** | 24 bands from per-channel engine levels, peak-hold | Display only | idle / animating | `chrome.rs:836` |
| **CPU** pill | audio callback mean load % | Display only, refreshed ≤ 1/s | "0%" with audio off | `chrome.rs` PillKind::Cpu |
| **kHz** pill | real output rate | Display only | "--" with audio off | same |
| **Notice capsule** (under the capsule) | status text, audio problems, the crash-recovery prompt with **RECOVER** / **DISCARD** keys | Appears on events; status text expires | — | `lib.rs:1619` ([17](screens/17-notice-nothing-to-undo.png)) |

There is no stop button (Space toggles; the clock's double-click returns to the start), no record
button (removed, DESIGN.md §5; `AppEvent::ToggleRecord` remains), no metronome, no time-signature
field, no follow toggle in the arrangement.

## 3. Browser sidebar ([01](screens/01-main-idle.png), [15](screens/15-browser-closed.png))

| Control | Does | Source |
|---|---|---|
| Header "BROWSER" + hint "Ctrl+Alt+B" | Hint text is hard-coded (wrong on macOS) | `browser.rs:498` |
| Search field | Filters presets as you type. Space typed here **also starts playback** ([18](screens/18-space-in-search-field.png), **F-03**). Esc doesn't clear it | `browser.rs:502` |
| **CHANNEL LAYOUTS** (5 rows) | **Single click** applies the layout: sets the chip and replaces all 8 lane instruments (undoable, one step) | `browser.rs:236`, `lib.rs:994` |
| **DPCM SAMPLES** | "+ Import audio as DPCM · n KB free" row → WAV open dialog; each filled slot: press-and-hold to hear it, click the right-hand zone to remove it | `browser.rs:238-246` |
| Preset categories (Leads, Bass, Strings, Winds, Brass, Keys & Mallets, Arpeggios, Drums, Drum Kits, Sound Effects, User Presets) | Click: select + audition while held. Double-click: load onto the selected clip's lane (or the first lane that fits). Drag: onto a track header/lane → lane default; onto a clip → that clip's instrument. Lanes that can't play it dim during the drag. Dots on the right show which channel kinds it fits. "ON" marks the selected lane's preset | `browser.rs:223-300` |
| Info text under the list | The selected preset's description; hidden while the piano roll is open | `browser.rs:507` |
| Wheel | Scrolls the list | `browser.rs:216` |

No context menu, no rename/delete for user presets, no folders, no "show in Explorer".

## 4. Track headers ([01](screens/01-main-idle.png), [06](screens/06-right-click-track-header.png))

Eight fixed rows (Pulse 1, Pulse 2, Triangle, Noise, DPCM, VRC6 Pulse 1, VRC6 Pulse 2, VRC6 Saw;
`state::default_lanes`).

| Element | Does | Source |
|---|---|---|
| Colour strip | Brightens and spills light on each note-on. Not clickable | `chrome.rs:1253` |
| Name, chip badge (2A03 / VRC6), instrument name, ghosted lane number | Display only. The row lifts when the selected clip is on this lane | same |
| **M** / **S** keys | Toggle mute / solo for the lane (lime glyph when lit). Not undoable, saved in the project | `lib.rs:415-424` |
| Click on the row | **Nothing** (no track selection) | — |
| Double-click / right-click / Ctrl+R | **Nothing** ([06](screens/06-right-click-track-header.png)) | — |
| Wheel | Scrolls the lanes when they overflow (piano roll open) | `chrome.rs:1261` |
| Drop target | A preset dropped on the row becomes the lane default | `browser.rs:46` |

No add/delete/reorder/colour/rename, no arm, no volume/pan, no track height control.

## 5. Ruler and loop brace

| Gesture | Does | Source |
|---|---|---|
| Click / drag in the ruler | Moves (scrubs) the playhead | `ruler.rs:134-147` |
| Drag the brace body | Moves the loop (snapped) | `ruler.rs:136`, DESIGN.md §5b |
| Drag a brace end | Resizes the loop (snapped) | same |
| Double-click the brace | Toggles looping | `ruler.rs:150` |
| Right-click | **Nothing** | — |
| Cursor | ↔ on brace ends, grab on body, hand elsewhere | `ruler.rs:157` |

The ruler shows bars only (no time ruler, no locators/markers, no time-signature lane). Outside
the loop the canvas is dimmed while looping is on.

## 6. Arrangement ([01](screens/01-main-idle.png)–[05](screens/05-context-menu-time-selection.png))

### Mouse

| Gesture | Where | Does | Source |
|---|---|---|---|
| Click | clip | Select it (Shift: toggle in the selection) | `arrangement.rs:267-280` |
| Click | empty space | Clear the selection (Shift keeps it); while stopped, move the playhead (insert marker) there | `arrangement.rs:247-262, 476-484` |
| Drag | empty space | Time selection across the lanes covered, snapped (Alt / macOS Cmd: no snap) | `arrangement.rs:364-383` |
| Drag | clip body | Move (horizontal, snapped; vertical changes lane). Dropped clips win overlaps (they trim or split what they cover) | `arrangement.rs:414-438, 489-496` |
| Drag | clip left/right edge (↔ cursor) | Resize; the left edge keeps notes at their song positions; minimum length `MIN_CLIP_TICKS` | `arrangement.rs:439-471` |
| Ctrl-drag (macOS Option-drag) | clip body | Duplicate the selection and drag the copies | `arrangement.rs:295` |
| Alt-drag (macOS Cmd-drag) | any drag | No snap | `input.rs:40` |
| Double-click | clip | Open it in the piano roll | `arrangement.rs:796` |
| Double-click | empty lane space | New 1-bar clip at the snapped position, selected | `arrangement.rs:797-811` |
| Right-click | clip | Selects it, opens the clip menu (§9) | `arrangement.rs:530-544` |
| Right-click | empty space | Selects that bar on that lane as a time selection, opens the selection menu | `arrangement.rs:545-556` |
| Wheel | — | Scrolls time (or the lanes, when they overflow) | `arrangement.rs:855-879` |
| Shift+wheel | — | Scrolls time | same |
| Ctrl/Cmd+wheel | — | Zooms around the pointer (×1.15 per notch) | same |
| Drag / click | scrollbars (bottom: time; right: lanes when overflowing) | Scroll / jump | `arrangement.rs:577-697` |
| Drag (from the browser) | lane or clip | Assign a preset (§3) | `browser.rs` |

### Keyboard (arrangement focused, i.e. after a click in it)

| Key | Does | Source |
|---|---|---|
| Delete / Backspace | Delete the selected clips (a time selection alone: nothing) | `arrangement.rs:883-892` |
| Enter | Open the one selected clip in the piano roll | `arrangement.rs:893` |
| Esc | Clear clip and time selection | `arrangement.rs:902` |
| Ctrl/Cmd+A | Select all clips | `arrangement.rs:906` |
| Ctrl/Cmd+D, Ctrl/Cmd+L, Ctrl/Cmd+Shift+M | Global (§10) | `lib.rs:657-659` |

There is no clip copy/paste in the arrangement (Ctrl+C/V/X do nothing there), no split, no
consolidate, no arrow-key navigation, no zoom keys, no rename shortcut.

## 7. Bottom panel: device inspector (a clip selected; [02](screens/02-clip-selected-inspector.png), [07](screens/07-inspector-triangle.png)–[09](screens/09-inspector-dpcm.png))

| Section | Controls | States | Source |
|---|---|---|---|
| **DEVICE · <lane>** | Dials for the lane's hardware controls: pulse Duty, Volume, Envelope, Sweep, Sweep rate; noise Mode, Period, Volume…; DPCM Sample, Rate, Loop; VRC6 pulse Duty, Volume; saw Rate. Triangle: the text "No controls: the triangle has no volume or duty." Dials: vertical drag (Shift fine), wheel, double-click → default; value label while dragging | normal; **"SET BY INSTRUMENT / KIT"** (faded, inert) when a macro overrides it; Volume reads "Decay" in hardware-decay modes | `lib.rs:1739`, `state.rs:708-720`, `dial.rs` |
| **CLIP** | Clip-name text field (submit with Enter → rename, undoable). Placeholder "No clip selected" | — | `lib.rs:1774-1786` |
| **INSTRUMENT · <name> · LANE DEFAULT / CLIP'S OWN · n CLIPS** | Four macro graphs (Volume, Arpeggio, Pitch, Duty): drag to draw; an unused graph says "not used · draw to add" and drawing starts a 16-step macro. Loop (↺) and release (R) markers: drag to move, Shift-click / Alt-click a step to set, double-click to remove. Vibrato row: Depth, Rate, Delay (drag or wheel). A kit shows its drum map | The triangle still offers Volume and Duty graphs ([07](screens/07-inspector-triangle.png), **F-05**) | `views/instrument.rs` |

The inspector opens when a clip is selected and closes when the selection is cleared. Selecting a
track (not possible) can't open it. **Make Instrument Unique**, **Use Lane Instrument** and **Save
Instrument as Preset** exist only in the clip context menu.

## 8. Bottom panel: piano roll ([10](screens/10-piano-roll.png), [11](screens/11-right-click-piano-roll.png))

Header: title "PIANO ROLL · <clip> · <LANE>", **GRID** button (click cycles the note grid),
**Follow** toggle, **Loop clip** toggle, the hovered row's read-out (note, period, tuning error in
cents), a hint line "Shift-drag select · Ctrl-drag copy · Alt no snap · Q quantize · Esc close"
(hard-coded, wrong on macOS), **Close**. A split handle above it sets the arrangement height
(drag; double-click restores four lanes). The arrangement goes compact (36 px lanes).

| Gesture / key | Does | Source |
|---|---|---|
| Click empty cell | Draw a note of the grid length (draw is always on); drag right while drawing sets its length | `piano_roll.rs:296-321` |
| Click / drag note body | Select / move (time and pitch); auditions while stopped | `piano_roll.rs:323-380, 449-479` |
| Drag note edge | Resize | `piano_roll.rs:480-509` |
| Shift-click note / Shift-drag empty | Toggle in selection / marquee | `piano_roll.rs:289-334` |
| Ctrl-drag (macOS Option-drag) | Copy notes | `piano_roll.rs:342` |
| Alt (macOS Cmd) | No snap | `input.rs:40` |
| Double-click note or empty cell | **Acts as a press** (selects / draws); it never deletes | `piano_roll.rs:749-752` |
| Volume lane: click/drag | Per-note volume (paints across notes) | `piano_roll.rs:262-283` |
| Volume lane: double-click a bar | Back to the channel default | `piano_roll.rs:733-748` |
| Keys column: press | Audition the row (stopped only) | `piano_roll.rs:253-260` |
| Piano-roll ruler: click/drag | Move the playhead | `piano_roll.rs:245-251` |
| Wheel / Shift+wheel / Ctrl+wheel | Scroll time / scroll rows / zoom time | `piano_roll.rs:780-803` |
| Right-click | **Nothing** | — |
| Delete/Backspace, Ctrl+A, Ctrl+D (repeat right), Ctrl+C/X/V, Q (quantize), ↑/↓ (semitone; Shift = octave), ←/→ (nudge by grid), Esc (close) | as named | `piano_roll.rs:612-690` |

The one-note-at-a-time rule applies on every lane (a dropped note trims what it overlaps).
Noise and DPCM rows are named (drum / sample names). No velocity colours, no fold, no scale, no
note-length keys, no zoom-to-fit, no draw/select mode toggle.

## 9. Context menus (custom popups, not native)

| Where | Items (shortcut label as shown) | Source |
|---|---|---|
| Clip ([03](screens/03-context-menu-clip.png)) | Duplicate (Ctrl+D) · Delete (Del) · Loop Clip (Ctrl+L) · Make Instrument Unique · Use Lane Instrument · Save Instrument as Preset | `arrangement.rs:537-544`, `state.rs:262-282` |
| Empty space / time selection ([04](screens/04-context-menu-empty.png), [05](screens/05-context-menu-time-selection.png)) | Loop Selection (Ctrl+L) · Insert MIDI Clip (Ctrl+Shift+M) | `arrangement.rs:555` |
| Track header, ruler, loop brace, piano roll, notes, browser, inspector, top bar | **none** | — |

Menu items show Windows shortcut labels on every platform (`state.rs:278-282`). Items are never
disabled (e.g. "Use Lane Instrument" on a clip that already uses it). Esc or a click outside closes
the menu; Esc **also** clears the arrangement selection, which closes the inspector.

## 10. Keyboard map (complete)

Global handler `lib.rs:642-685`, arrangement `arrangement.rs:880-913`, piano roll
`piano_roll.rs:612-690`. "Cmd" = Ctrl on Windows/Linux, ⌘ on macOS (`input::command`).

| Keys (Windows / macOS) | Does | Scope |
|---|---|---|
| Space | Play / pause from the playhead | global (**fires even while typing**, F-03) |
| Ctrl+Z / ⌘Z | Undo | global (also inside text fields, F-03) |
| Ctrl+Shift+Z, Ctrl+Y / ⌘⇧Z, ⌘Y | Redo | global |
| Ctrl+N / ⌘N | New project | global |
| Ctrl+O / ⌘O | Open | global |
| Ctrl+S / ⌘S | Save | global |
| Ctrl+Shift+S / ⌘⇧S | Save As | global |
| Ctrl+I / ⌘I | Import MIDI (replaces the song) | global |
| Ctrl+Shift+R / ⌘⇧R | Export dialog | global |
| Ctrl+L / ⌘L | Loop the time selection or the selected clips' span | global |
| Ctrl+D / ⌘D | Duplicate selected clips (in the piano roll: duplicate notes) | global / piano roll |
| Ctrl+Shift+M / ⌘⇧M | Insert MIDI clip in the time selection | global |
| Ctrl+Alt+B / ⌘⌥B | Show / hide the browser | global |
| Esc | Close the menu; else close the piano roll; in the arrangement also clear the selection | global + views |
| Delete, Backspace | Delete selected clips / notes | arrangement / piano roll |
| Enter | Open the selected clip | arrangement |
| Ctrl+A / ⌘A | Select all clips / notes | arrangement / piano roll |
| Ctrl+C, X, V / ⌘C, X, V | Copy, cut, paste **notes** | piano roll only |
| Q | Quantize selected notes | piano roll |
| ↑ ↓ (Shift: octave) | Transpose notes | piano roll |
| ← → | Nudge notes by the grid | piano roll |
| **F3** | Debug overlay (frame timings) | global, in release builds |
| **F4** | **Replaces the song with the 200-clip stress set and starts playing; F4 again loads the demo song** (not the user's song; not undoable: **F-02**) | global, in release builds |
| **F5** | Reload stylesheets | global, in release builds |
| **F6** | Cycle the frame-rate cap | global, in release builds |
| **F7** | Cycle the output filter (same as the FILTER pill) | global, in release builds |

## 11. Export dialog ([12](screens/12-export-dialog.png), [13](screens/13-export-dialog-after-esc.png))

An in-window panel over the arrangement (no backdrop; the rest of the window stays live).
Rows of toggle buttons: **Range** Song / Loop region (default) / Selection (disabled without a
time selection, no reason given) · **Format** 44.1 kHz / 48 kHz, 16-bit / 24-bit · **Loop files**
One WAV + loop points / Intro + loop / JSON · **Stems** Mix only / 8 channels / Groups (lane chips
cycle melody/bass/drums/none) · **Loudness** Authentic chip level / Peak −1 dBFS / −16 LUFS
(≤ −1 dBTP) · **Export…** (asks for a base file name, renders on a worker thread with progress and
Cancel) · **Close**. Esc does not close it. Options are not remembered between sessions.
Source: `views/export_dialog.rs`, docs/EXPORT.md.

## 12. Notices and dialogs

| Kind | When | Source |
|---|---|---|
| Native file dialogs (rfd) | Open, Save As, Import MIDI, Import audio, Export base name | `lib.rs` |
| Notice capsule | Save/open status, import reports ("Imported x: …"), "Nothing to undo/redo", audio device problems, "Layout: …" | `lib.rs:1619` |
| Crash recovery | At start, if the last session didn't end cleanly and an autosave exists: "Last session didn't close cleanly" + RECOVER / DISCARD | `lib.rs:605-617`, `file.rs:752` |
| Confirmations | **None anywhere** (no "save changes?", no overwrite-the-song confirmation for MIDI import or layouts) | — |

## 13. Settings and persistence

| What | Where | Notes |
|---|---|---|
| Clock format (`clock_bars`) | `settings.json` in the data dir | The only app preference (`file.rs:677`) |
| User presets | `user_presets.json` in the data dir | "Save Instrument as Preset"; listed under User Presets |
| Autosave / recovery | `recovery.chipdaw` + `session.lock` in the data dir | Every 2 min and on a clean quit; the marker is removed on a clean quit, so the autosave is **not** offered after a normal quit with unsaved work (F-01) |
| Data dir | `CHIPDAW_DATA_DIR`, else `%APPDATA%\ChipDAW` (Windows), `~/Library/Application Support/ChipDAW` (macOS), `$XDG_DATA_HOME/chipdaw` | `file.rs:711` |
| Project file (`.chipdaw`, JSON v3) | Wherever the user saves | Holds clips, notes, instruments, lane defaults, samples, BPM, chip, filter (+ auto flag), loop start/end/on, mute, solo, channel params. Not saved: piano-roll grid, zoom, browser state, split height, export options |
| Not remembered at all | window size/position, browser open/closed, recent files, last folder, export options, frame cap | — |

## 14. Undo coverage (from `history.rs` and PIANOROLL.md §10)

Undoable: clip and note edits, renames, instrument/macro/vibrato edits, presets and layouts, BPM,
channel parameters (dials), user DPCM samples (the undo `Doc` = clips, BPM, params, instruments,
samples: `history.rs:35`). **Not** undoable: mute/solo, chip, filter, snap, loop on/off, the loop
brace, F4. New / Open / Import MIDI / Recover **clear** the history. 1000 steps / 64 MB.
