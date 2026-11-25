# UX audit, part 2: findings against VISION.md

*2026-11-25 · build `stabilize/v1.1` @ `c0037d3` · the inventory is [INVENTORY.md](INVENTORY.md),
screenshots are in [screens/](screens/), and the reference behaviours are in
[manual-refs.md](manual-refs.md).*

**How to read this.**
- **Ref** cites a row of [manual-refs.md](manual-refs.md) (R1…R151). Each row there gives the
  Live 12 Reference Manual chapter, section and URL, as a paraphrase. Where the manual didn't
  confirm a behaviour, the cell says **unverified**.
- **P** is the VISION.md principle: P1 copy behaviour · P2 one home · P3 nothing pretends ·
  P4 authenticity opt-in · P5 warn, don't surprise · P6 ask only when it matters · P7 game-ready ·
  P8 fast. Pillars: E (as easy as Ableton), W (powerful), A (authentic). **Shell** means VISION.md,
  "The app shell".
- **Effort:** S ≤ 1 day · M ≤ 1 week · L > 1 week. **Dep:** **model** = needs the
  tracks-as-instruments model change; **menus** = needs native menus; "—" = none.
- **Priority** is proposed (must / should / later). **The CEO decides.**
- Sorted by priority, then effort. IDs are labels, not ranks: the low ones were assigned first
  and are referenced from INVENTORY.md.

**Overall picture.** The editing core (clips, notes, instruments, export) is solid and fast. The
gaps are an **app shell** that isn't there yet (menus, settings, Open Recent, About, shortcut
reference, save prompts), **data-safety holes** (four ways to lose work with one keystroke or
click), **Ableton conventions** missing in the arrangement (copy/paste, split, rename, zoom keys,
context menus beyond clips), and a **model** (8 fixed hardware lanes) that VISION.md replaces.

---

## MUST

| ID | Surface | What happens now | Expected (VISION / reference) | P | Proposed resolution | Effort | Dep |
|---|---|---|---|---|---|---|---|
| **F-01** | App, files | **Unsaved work is lost without a prompt.** Close the window, NEW (Ctrl+N), OPEN (Ctrl+O), MIDI IN (Ctrl+I), or drop a `.mid`/`.chipdaw`: the song is replaced and the undo history cleared. On a clean quit the session marker is removed, so the autosave isn't offered next time either (`lib.rs:518-528, 635-641, 2052-2096`; `file.rs:768`) | Every DAW asks to save changes before closing or replacing a document (R120–R122 cover the commands; the prompt itself is **unverified** in the manual) | P5, P6 | Track a "dirty" flag (history position ≠ saved position). Ask "Save changes to *x*?" (Save · Don't Save · Cancel) only when dirty. Title bar shows `*` when dirty | S | — |
| **F-02** | Keyboard | **F4 replaces the song with the 200-clip stress set** and starts playing; F4 again loads the *demo* song, not the user's. Not undoable. F3 (debug overlay), F5 (reload styles), F6 (frame cap) and F7 (filter) are also live in release builds (`lib.rs:466-484, 679-683`) | Debug tools aren't user shortcuts. The manual's F-keys mean other things: F1–F8 toggle track activators (R142), F9 record (R140), F10 back to arrangement (R139), F12 clip/device (R78) | P3, P5 | Gate F3–F7 behind `cfg(debug_assertions)` or an env var (`CHIPDAW_DEV=1`) | S | — |
| **F-03** | Text fields | **Global shortcuts fire while typing.** Space in the browser search starts playback ([18](screens/18-space-in-search-field.png)). Ctrl+Z in the clip-name field runs the *document* undo ([19](screens/19-ctrl-z-in-clip-name-field.png)), because vizia's `Textbox` has no undo of its own and lets the key through (`vizia_core/src/views/textbox.rs:1163-1360` handles A, C, V, X only). Ctrl+D, Ctrl+L, Ctrl+N… also fire | Typing in a field never triggers app commands | P3 | In the root key handler, skip global shortcuts (except Ctrl+S/Ctrl+Shift+R and Esc) while a text field has focus. Give text fields their own Ctrl+Z (a field-local undo, or none) | S | — |
| **F-04** | Top bar CHIP / lanes | **CHIP = 2A03 leaves the three VRC6 lanes fully active-looking**: clips, badges, M/S keys, instruments and the inspector all work, but those lanes are silent ([14](screens/14-chip-2a03-only.png); `lib.rs:1009`) | VISION: unavailable things are hidden or clearly disabled with the reason | P3, P5 | Now: dim the VRC6 lanes and headers, show "VRC6 is off: enable it in Project Settings" on hover, and warn before switching off a chip that has clips. Later superseded by profiles (F-40) | S | — |
| **F-06** | App shell | **No menu bar** on any platform. Commands are spread over top-bar keys, shortcuts and two context menus. Nothing is discoverable | Shell: native menus File, Edit, Create, View, Help (+ macOS App, Window). Live has a full menu bar (R120–R133) | Shell, E | Add native menus (FEASIBILITY.md §1) with the structure in UX-DRAFT.md §1 | M | menus |
| **F-07** | Arrangement | **No clip copy / cut / paste.** Ctrl+C/X/V do nothing in the arrangement, silently (they work only on notes in the piano roll) | Cut/Copy/Paste on clips and on time (R24, R46) | E, P3 | Clip clipboard (shared with notes by type); paste at the insert marker on the selected track; time variants with Shift | M | — |
| **F-08** | Arrangement, headers | **No rename shortcut.** Clips can be renamed only in the inspector's CLIP field; tracks can't be renamed at all | Ctrl/Cmd+R renames the selected clip, track or locator (R8–R11); Tab moves to the next track (R9) | E, Shell | Ctrl/Cmd+R: in-place edit on the clip header or track name; Edit > Rename. Tracks renamable now (names are per project; today `default_lanes` names are fixed) | M | — |
| **F-09** | Piano roll | **Double-click doesn't delete or create** a note: it acts as a press (`piano_roll.rs:749-752`). A single click on empty space always draws | Double-click a note deletes it; double-click empty space creates one (R1, R2); single-click draws only in Draw Mode (R5, R6) | E, Shell | Default (draw mode off): click selects, drag on empty space = marquee, double-click creates/deletes. **B** toggles draw mode, where a click draws as today | S | — |
| **F-10** | Everywhere | **Context menus only on clips and empty lane space.** Nothing on track headers ([06](screens/06-right-click-track-header.png)), notes or the piano roll ([11](screens/11-right-click-piano-roll.png)), the ruler, the loop brace, the browser, the inspector | Right-click menus on clips, track headers, notes, the arrangement background, the scrub area, locators and the loop brace, envelopes (R12–R19) | E, Shell | Menus per UX-DRAFT.md §4. Disable (not hide) items that can't apply, with the reason (F-24) | M | — |
| **F-11** | Arrangement | **No split or consolidate** | Split at the insert marker / selection: Ctrl/Cmd+E (R38). Consolidate: Ctrl/Cmd+J (R39). Crop: Ctrl/Cmd+Shift+J (R40) | E | Split = existing `resolve_overlaps` split logic applied at a tick; consolidate = merge clips on one track into one clip (notes re-based) | M | — |
| **F-12** | Arrangement, piano roll | **No zoom keys, no zoom to selection, no zoom to fit.** Only Ctrl/Cmd+wheel | Z zooms to the selection, X zooms back (R30, R31); double-click the ruler fits the whole arrangement (R32); + / − zoom (R34); W/H optimise (R33) | E | Z / X / + / −, ruler double-click = fit. The piano roll gets Z/X as well (R75) | S | — |
| **F-13** | Tracks | **Tracks can't be created, deleted, reordered, recoloured or renamed.** The 8 hardware lanes are fixed | Insert MIDI track Ctrl/Cmd+Shift+T (R84); delete (R86); drag to reorder (R88); clip colour via context menu (R90; per-track colour chooser **unverified**, R89) | E, model | The model change (FEASIBILITY.md §4). Interim: rename + colour per lane are possible today | L | model |
| **F-14** | App | **The empty project gives no guidance** ([16](screens/16-new-empty-project.png)): 8 empty lanes, a full-width loop brace, nothing that says what to do | Shell: "An empty-project start that tells a new user what to do first." Live has a Learn View with lessons (R130); what a new Set contains is **unverified** (R134) | Shell, E | An overlay on the empty arrangement: "Double-click a lane to make a clip · Drag a preset from the browser · Drop a MIDI file here", with links to the shortcut reference. Hidden as soon as a clip exists | S | — |
| **F-15** | App shell | **No Settings window.** The only preference (clock format) is changed by clicking the clock; audio device, buffer, sample rate, frame cap, default grid, autosave can't be set | Shell: Settings (Preferences on macOS) with Audio, Display, Editing, Files, Warnings; Ctrl/Cmd+, (R129) | Shell | Settings as specified in UX-DRAFT.md §2 (window vs panel: FEASIBILITY.md §3) | M | menus |
| **F-16** | App shell | **No Project Settings.** Tempo, chip and filter sit in the top bar; there is no time signature | Shell: Project Settings with tempo, time signature, hardware profile, filter; chip and filter leave the top bar. Live keeps tempo and time signature in the control bar (R116) | Shell, P2 | Project Settings dialog (File > Project Settings…) holds the hardware profile and filter. **Decision for the CEO:** tempo and time signature stay in the top bar (Ableton parity, R116) and are *not* repeated in Project Settings (P2). See UX-DRAFT.md §3 | M | menus |
| **F-17** | Files | **MIDI import replaces the whole song** (Ctrl+I, MIDI IN, drop), keeping only the lane instruments | VISION, "Importing MIDI": drag a `.mid` over the arrangement and it adds tracks at the drop position, with a live preview; a dialog only on conflicts. Live imports MIDI via Create > Import MIDI File or a drag (R126, R127); one track per part is **unverified** (R128) | P5, P6, E | Import *into* the current song at the insert marker (or drop position), as new tracks; the old "replace" becomes File > New from MIDI File. Full preview after the drop probe (FEASIBILITY.md §2) | M (interim) / L (preview) | model (for one track per part) |
| **F-18** | App shell | **No Open Recent, no About box, no shortcut reference, no Help** | Shell. Open Recent is in Live's File menu (R121); Help menu contents and About are **unverified** (R132) | Shell | Open Recent (10 entries in `settings.json`), Help > Keyboard Shortcuts (a generated sheet from one table, F-30), Help > About (version, licences, font credits from ASSETS.md) | S | menus |
| **F-19** | Top bar | **File keys duplicate future menu items** (NEW, OPEN, SAVE, MIDI IN, EXPORT) and take the space the prototype left empty | P2: one home. Menus are that home | P2 | Remove the five keys once menus land; keep the shortcuts | S | menus |
| **F-20** | Top bar | **CHIP pill vs channel layouts** (the CEO's example): both set the chip; a layout silently changes it, and nothing undoes a chip change | P2; P5 | P2, P5 | Hardware profile lives in Project Settings only (F-16). Layouts become track templates; if one needs another profile, it asks first (P5) | S (after F-16) | menus |
| **F-21** | Top bar | **FILTER pill + F7** do the same thing, and the filter isn't a performance control | Shell: filter moves to Project Settings | P2 | Project Settings only; drop the pill and F7 | S (after F-16) | menus |
| **F-22** | Arrangement | **Arrangement selection semantics are partial.** Delete with only a time selection does nothing; Ctrl+D with only a time selection does nothing; Shift-click doesn't extend a time selection; no "time" commands | Delete/Duplicate/Cut/Copy/Paste Time (R46); Shift-click extends, also across tracks (R44); Insert Silence Ctrl/Cmd+I (R47) | E, P3 | Edit commands act on the time selection when there is one (clips are split at its edges), else on selected clips. Time variants on Ctrl+Shift | M | — |
| **F-23** | Piano roll | **Draw is always on** and there is no draw-mode toggle; marquee needs Shift | B toggles Draw Mode (R5, R6); plain drag on empty space selects (R76) | E | See F-09 | S | — |

## SHOULD

| ID | Surface | What happens now | Expected | P | Proposed resolution | Effort | Dep |
|---|---|---|---|---|---|---|---|
| **F-05** | Inspector (triangle) | The triangle's inspector says "No controls: the triangle has no volume or duty", yet the INSTRUMENT section offers **Volume** and **Duty** macro graphs to draw on ([07](screens/07-inspector-triangle.png)). Duty can't apply to a triangle; whether its Volume macro does anything (e.g. only 0 = silence) must be confirmed in `driver.rs` (the note type says the triangle has no volume, `driver.rs:31-33`) | P3 | P3, A | Hide macros the channel can't use (or show them disabled with the reason) | S | — |
| **F-24** | Context menus | Items are **never disabled** ("Use Lane Instrument" on a clip that already uses it, "Make Instrument Unique" on an instrument only one clip uses). Shortcut labels are Windows-only on macOS ("Ctrl+D", `state.rs:278-282`) | P3: disabled with the reason; platform-correct labels | P3 | Enabled/disabled per item + reason line; labels from one shortcut table (F-30) | S | — |
| **F-25** | Labels | Hints are **hard-coded Windows text**: "Ctrl+Alt+B" in the browser head, "Shift-drag select · Ctrl-drag copy · Alt no snap · Q quantize · Esc close" in the piano-roll header. On macOS the copy and no-snap keys are Option and Cmd (`input.rs`) | Platform-correct | P3 | Generate from the shortcut table; move the hints into tooltips / the shortcut reference | S | — |
| **F-26** | Export dialog | Esc doesn't close it ([13](screens/13-export-dialog-after-esc.png)); no backdrop, so the arrangement stays live under it; "Selection" is greyed with no reason; options are forgotten between sessions; styling is stock buttons, unlike every other surface ([12](screens/12-export-dialog.png)) | P3; consistent styling (DESIGN.md) | P3, P7 | Modal with backdrop, Esc = Close, Enter = Export; reason on hover; remember options in `settings.json`; restyle as split pills | S | — |
| **F-27** | Keyboard | **Shortcut conflicts with the reference:** Ctrl+I = Import MIDI here, Insert Silence in Live (R47). Q quantizes here, Ctrl/Cmd+U in Live (R64). Follow has no key; Live uses Ctrl+Shift+F / Option+Shift+F (R135, contradicted in the manual itself) | P1: an Ableton user's keys do the expected thing | P1, E | Ctrl+I → Insert Silence (import goes to the File menu, no key or Ctrl+Shift+I); quantize → Ctrl/Cmd+U (keep Q as an alias? No: P2); Follow → Ctrl+Shift+F | S | — |
| **F-28** | Piano roll | No **Shift+←/→** to change note length (R63); no Ctrl/Cmd+E chop / Ctrl/Cmd+J join (R73); no Alt+arrows to step between notes (R74) | E | E | Add them (notes.rs already has the operations' building blocks) | S | — |
| **F-29** | Arrangement | **No keyboard navigation:** arrows don't move the insert marker or nudge clips (R50, R51); no Home (R138); no Shift+Space continue (R137); Enter-then-arrows edge resize (R55) absent | E | E | Arrows move the insert marker / nudge selected clips by the grid; Home; Shift+Space | S | — |
| **F-30** | Keyboard | Shortcuts live in four places (`lib.rs`, `arrangement.rs`, `piano_roll.rs`, `state.rs` labels) plus hard-coded hint text | Shell: "a full shortcut map" | P2 | One table (action → Windows key, macOS key, scope) that drives handlers, menu accelerators, context-menu labels, hints and the Help sheet | M | — |
| **F-31** | Grid | SNAP is in the top bar and only cycles forward; the piano roll has a separate GRID button that also only cycles. Neither has keys | Narrow/widen grid Ctrl/Cmd+1/2, triplet Ctrl/Cmd+3, snap on/off Ctrl/Cmd+4, fixed/adaptive Ctrl/Cmd+5 (R56–R59); each editor has its own grid | E, P2 | Keep a grid per editor (as Live) but move the arrangement's out of the top bar into the arrangement (context menu + keys); a click opens a list, not a cycle | S | — |
| **F-32** | Arrangement | **No follow / auto-scroll** in the arrangement while playing (the piano roll has one) | Follow Ctrl+Shift+F (R135) | E | One global Follow (View menu + key) for both editors | S | — |
| **F-33** | Undo | Mute/solo, chip, filter, snap, loop on/off and the loop brace are **not undoable**; New/Open/Import clear the history | Live has an Undo History (R29); whether mixer/mute/solo changes are undoable is **unverified** (R141) | E | Make the loop brace, loop on/off and profile changes undoable (they change what plays/exports). Leave mute/solo out until verified | S | — |
| **F-34** | Tooltips | **No tooltips anywhere** (no `tooltip` in the UI code). Disabled controls can't explain themselves | P3 requires "the reason on hover"; Live's Info View explains whatever is hovered (R131) | P3 | A tooltip helper (vizia has `Tooltip`), used first on disabled items and icon-only controls (play light, loop light, M/S) | M | — |
| **F-35** | Track headers | Clicking a header **selects nothing**; there is no track selection, so the inspector can't show a lane without a clip, and "selected lane" is inferred from the selected clip | Live selects a track by clicking its header; Up/Down moves between tracks (R82); several selected tracks take a button toggle together (R144) | E | Track selection (click, Shift/Ctrl-click); the inspector shows the selected track's instrument even with no clip | M | — |
| **F-36** | Browser | A **single click on a channel layout replaces all 8 instruments** (undoable, but surprising); presets audition on click but do nothing while playing, silently | P5 | P5, P3 | Layouts load on double-click or drag, like presets; audition during playback either works (preview voice) or the row says why not | S | — |
| **F-37** | Notices | The notice capsule floats over the ruler and the first bars ([17](screens/17-notice-nothing-to-undo.png)); import reports vanish after a few seconds | Readable, non-blocking | P3 | Dock notices bottom-right; keep import reports available (e.g. "Show report" in the notice) | S | — |
| **F-38** | Capsule | The clock's double-click **stops and rewinds**, and its single click toggles the format: two hidden behaviours on a read-out. CPU shows "0%" and kHz "--" with no audio device, which reads as "working" | P3 | P3 | Keep scrub; move the format toggle to View/Settings (or a right-click menu on the clock); show "No audio" in the CPU pill when the engine is off | S | — |
| **F-39** | Visual | **Readability:** chip badges are 9 px text, the instrument name in the track row 11 px; long clip names are cut with no ellipsis on short clips ("Crash"); the piano-roll header runs the title into "GRID" ("PULSE 1 GRID", [10](screens/10-piano-roll.png)); the device title says "DEVICE · PULSE 1" though "device" means something else in a DAW; the export dialog and piano-roll buttons use stock styles unlike the rest | Consistent, legible surfaces (DESIGN.md §8 already lists the stock buttons) | — | Min 11 px for informative text; ellipsis on clip names; spacing in the piano-roll header; rename DEVICE → CHANNEL (or the track's name); restyle stock buttons | S | — |
| **F-41** | Scaling | No UI-scale setting (Live zooms the whole UI with Ctrl/Cmd +/−, R34's note); text relies on the OS scale | Display settings | E | Settings > Display > Interface scale (vizia's scale factor) | S | — |
| **F-50** | File drop | Found by the drag probe: an Esc that cancels an OS file drag is also delivered to the window as a key press afterwards, so it can close the piano roll or clear the selection (FEASIBILITY.md §2) | Cancelling a drag has no other effect | P3 | Ignore the Esc that ends a drag (fork: track the hover state, which the drop-preview fork change adds anyway) | S | — |

## LATER (plans: the VISION.md model)

These are the gaps between today's model and VISION.md's. Each is a plan, not code; the
engine/model side is sized in [FEASIBILITY.md §4](FEASIBILITY.md).

| ID | Gap | What in the code it affects | What can be reused | Effort | Dep |
|---|---|---|---|---|---|
| **F-40** | **Hardware profiles, locked tracks (🔒 chip name), channel budget, profile warnings** (VISION "Hardware profiles and locked tracks") | `SoundChip` + `set_chip` (`lib.rs:1009`) becomes a `Profile` in Project Settings; track header gets the lock chip; a budget strip (e.g. in the top bar's left, where the file keys were); warnings at: add effect to locked track, chord on locked track, note out of range, too many channels of a kind, preset from another expansion chip. Options: Unlock · Fit to hardware · Cancel | `presets.rs` `kinds` (which channel a preset fits) = the profile check; the browser's "can't play here" dimming = the drag-time warning; the piano roll's unplayable-row shading = out-of-range; `notes::resolve_mono` = "Fit" for chords; `midi::arrange` = Fit to hardware | M | model |
| **F-42** | **Tracks as instruments** (unlimited, polyphonic, effects, Voices cap, CPU meter) | FEASIBILITY.md §4 (state, driver, engine snapshot, file v4, export, undo) | Emulator, macros, snapshot + garbage queue, undo, file migration framework | L | model |
| **F-43** | **Voice pool** (pre-allocated, grown from the UI thread, graceful stealing) | `engine` core and `Shared` | The garbage-queue pattern (`core.rs:35, 116-146`), no-alloc guard, stress harness | M | model |
| **F-44** | **Automation**: lanes per track for any parameter, breakpoints (click adds, drag moves, Alt-drag curves, draw mode, delete), clip envelopes, override + Re-Enable, authentic stepping on locked tracks, tempo automation | New `Automation` in `Doc` (undo), file v4, snapshot (per-tick evaluation on the frame tick for locked tracks: the driver already runs per frame), a lane view under each track (the piano roll's volume lane is the drawing prototype) | Frame-tick driver (authentic stepping = evaluate once per frame and quantise to the register's steps, e.g. `scale_volume`); the macro graph editor's drawing code; `ParamId` as the target list | L | model |
| | Reference notes | Breakpoints are **removed with a single click**, not a double-click (R108, contradicts VISION.md's "delete with a double-click"); a double-click away from the line adds one. Draw mode B (R7), Alt-drag curves (R109), Re-Enable Automation (R111), lock envelopes (R112), tempo on the main track (R113), A toggles Automation Mode (R104), delete automation Ctrl+Backspace / Cmd+Delete (R114, printed two ways in the manual) | | | |
| **F-45** | **Time signatures and change markers** | `TICKS_PER_BAR` is a constant used by the ruler, grid, snap, clock, loop brace, MIDI import, export, piano roll. Becomes a tempo/meter map (tick ↔ bar.beat) | One conversion module already exists (`view.x_to_tick` etc.); MIDI parser already reads meta events (add 0x58 time signature) | M | — |
| | Reference notes | Project signature in the control bar (R116); insert markers from the scrub-area context menu or the Create menu (R117); edit with Ctrl/Cmd+R, drag to move, Delete (R118); grid follows (R119, partial) | | | |
| **F-46** | **Scales** (root + scale per project, clip override; out-of-scale rows disabled; Fold; in-scale transpose; no effect on drums/DPCM) | Piano roll rows (`RowKind`), `notes::transpose`, project + clip model, file v4 | Unplayable-row shading (= out-of-scale dimming), `RowKind::Noise/Dpcm` (= exempt rows) | M | — |
| | Reference notes | Live 12 Scale Mode per clip with root and scale (R70), Highlight Scale K is global (R71), Fold F (R68), Fold to Scale G (R69). **Arrow-key transposition within the scale is unverified** (R62): only the Transpose control is documented as scale-aware. VISION.md asks for ↑/↓ in-scale; that goes beyond the reference and should be labelled our behaviour | | | |
| **F-47** | **MIDI drag-and-drop preview + conflict dialog** (ghost tracks with names and note counts at the drop position; warnings while dragging; dialog only on conflict: Import as studio tracks · Fit to hardware · Cancel · Don't ask again) | `open_dropped` (`lib.rs:2052`), the browser's drag ghost (`browser.rs:521`) as the drawing model, `midi::parse` (parse on hover to get parts and counts) | The preset drag ghost and lane dimming; `midi::arrange` report text | M | model; cursor position during an OS drag (FEASIBILITY.md §2) |
| **F-48** | **Unlimited studio tracks** UI: add/delete/reorder/colour/group, track height, CPU meter per track | `track_headers` (rows built once), lane height signal, browser drop targets | `TrackRow` drawing, scroll and split logic | M | model |
| **F-49** | Undo History window (R29), Learn/Help view (R130), Info View hover help (R131), groups (R91), track height (R145), W/H optimise (R33), deactivate with 0 (R97) | various | — | M | some: model |

---

## Redundancy list (category 1)

| # | Duplicate | Keep in | Remove / change | Finding |
|---|---|---|---|---|
| RD-1 | CHIP pill · channel layouts · (future) profile | Project Settings: hardware profile | CHIP pill goes; layouts become templates that ask before changing the profile | F-20, F-40 |
| RD-2 | FILTER pill · F7 | Project Settings: output filter | Both go | F-21, F-02 |
| RD-3 | BPM pill · VISION's Project Settings "tempo" | **Top bar** (tempo is a performance control; Live keeps it in the control bar, R116) | Not repeated in Project Settings. **CEO to confirm** (VISION.md lists tempo under Project Settings) | F-16 |
| RD-4 | NEW / OPEN / SAVE / MIDI IN / EXPORT keys · shortcuts · (future) menus | Menus + shortcuts | The five keys | F-19 |
| RD-5 | Import MIDI: MIDI IN key · Ctrl+I · drop · (menu) | File > Import > MIDI… + drop | MIDI IN key; Ctrl+I goes to Insert Silence | F-17, F-27 |
| RD-6 | Import audio: browser row · drop · (menu) | File > Import > Audio as DPCM… + drop + the browser row (in-context entry next to the slots it fills: kept as the one exception, because it shows free space) | — | — |
| RD-7 | SNAP (top bar) · GRID (piano roll) | Each editor keeps its own grid (as in Live, R56–R59), styled alike | SNAP leaves the top bar | F-31 |
| RD-8 | Clip name: inspector CLIP field · (future) Ctrl+R in place | In place (Ctrl+R / double-click the clip name); the inspector shows the name read-only | The editable field | F-08 |
| RD-9 | Meters: top spectrum · track strip flashes · (removed) inspector meters | Already resolved in v1.1: spectrum + strip flashes. With studio tracks, add per-track level in the header and keep the spectrum as the master view | — | STABILIZE §A4.5 |
| RD-10 | Loop on/off: capsule loop light · double-click brace · Ctrl+L (sets and enables) | Loop light (+ Ctrl+L to set) | Double-click on the brace (unverified in Live, R100) → becomes "zoom to loop"? Proposed: keep as is until verified | — |
| RD-11 | Filter mode also set implicitly by the chip switch | Profile sets a default filter once; the user's choice wins | — | F-21 |
| RD-12 | Transport: play light · Space · clock double-click (stop + rewind) | Play light + Space; Home rewinds | Clock double-click | F-38 |

Checked and **not** redundant: preset loading (click = hear, double-click = load, drag = place
are three gestures for three results), playhead placement (ruler, arrangement click, piano-roll
ruler, clock scrub: one result in each editor's own time area), instrument commands (only in the
clip context menu: they need a second home in the inspector header as the *visible* home, with the
menu as a shortcut; that's a gesture, not a duplicate control).

## "Pretends to work" list (category 2)

| # | Control | State where it does nothing | Finding |
|---|---|---|---|
| PW-1 | VRC6 lanes, their clips, M/S, inspector | CHIP = 2A03 | F-04 |
| PW-2 | Triangle Volume/Duty macro graphs | always | F-05 |
| PW-3 | Ctrl+C / X / V in the arrangement | always (notes only) | F-07 |
| PW-4 | Delete / Ctrl+D with only a time selection | always | F-22 |
| PW-5 | Ctrl+L / Loop Selection with nothing selected | no selection | F-24 (disable the item) |
| PW-6 | Context-menu items that don't apply | e.g. Use Lane Instrument on a lane-instrument clip | F-24 |
| PW-7 | Export "Selection" | greyed, no reason | F-26 |
| PW-8 | Browser click-to-hear | while playing | F-36 |
| PW-9 | CPU "0%" / kHz "--" / empty spectrum | no audio device | F-38 |
| PW-10 | M/S on a lane whose chip is off | CHIP = 2A03 | F-04 |
| PW-11 | Recovery after a clean quit with unsaved work | never offered | F-01 |
| PW-12 | Keys and pills that look pressable but show nothing on hover (no tooltips) | always | F-34 |

## Assumptions made in this audit

- The newest code is `stabilize/v1.1` (main has not been merged).
- Screenshots were taken with audio off (`CHIPDAW_AUDIO=0`), so the CPU/kHz pills and the spectrum
  are idle in them. The audio-dependent behaviours are read from source.
- "Effort" assumes one engineer who knows the codebase, using the existing patterns.
- Where VISION.md and the reference disagree (breakpoint deletion R108, in-scale arrow transpose
  R62, tempo's home RD-3), this audit flags it for the CEO instead of choosing silently.
