# UX.md: the build spec

*Adopted 2026-11-25 from the UX audit's [UX-DRAFT.md](docs/ux-audit/UX-DRAFT.md). **Every priority
was accepted by the CEO as proposed** (✓ in the CEO column). [VISION.md](VISION.md) still wins on
any conflict; this page is the build spec under it. Each line keeps its acceptance test (§6).
Background: [findings](docs/ux-audit/FINDINGS.md), [feasibility](docs/ux-audit/FEASIBILITY.md),
[manual references](docs/ux-audit/manual-refs.md).*

Conventions: **Cmd** = Ctrl on Windows, ⌘ on macOS; **Alt** = ⌥ Option on macOS. macOS keys are
written in Apple's modifier order (⌃⌥⇧⌘), exactly as the menu bar and the shortcut sheet show them
(verified with real key events, MAC-CATCHUP.md §4). Keys follow the Live 12 manual where the audit
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

Native menus (docs/ux-audit/FEASIBILITY.md §1). On Windows, About goes under Help and Settings… under
Edit (Windows convention), Exit under File.

**macOS (as built, MAC-CATCHUP.md):** the system menu bar: App, File, Edit, Create, View, Window, Help.
- **App menu** (the name becomes Cartwave with the planned code rename, VISION.md): About ChipDAW (our About: version, commit, licences, font credits), Settings… ⌘,,
  Services ▸, Hide ChipDAW ⌘H, Hide Others ⌥⌘H, Show All, Quit ChipDAW ⌘Q. (Unbundled, the menu bar's
  bold title reads the process name, "app", until the app ships as an `.app` bundle.)
- **File** has no Quit; **Edit** has no Settings; **Help** has no About. macOS adds its own Edit items
  (Writing Tools, AutoFill, Start Dictation…, Emoji & Symbols) and View ▸ Enter Full Screen.
- **Window:** Minimize ⌘M, Zoom, Bring All to Front. No window tabs (View has no Show Tab Bar).
- While a text field is being edited, Edit ▸ Cut / Copy / Paste / Select All (⌘X / ⌘C / ⌘V / ⌘A)
  act on the field; editor commands do nothing; File and app commands still work. While a dialog
  is open, only File and app commands work from the menu bar.
- Closing the window quits (one window), with the same save sheet as ⌘Q.

### File
| Item | Windows | macOS | Does | Pri | CEO |
|---|---|---|---|---|---|
| New | Ctrl+N | ⌘N | New empty project (asks to save if dirty) | M | ✓ |
| New from MIDI File… | — | — | Today's "replace with MIDI" behaviour | S | ✓ |
| Open… | Ctrl+O | ⌘O | Open a `.chipdaw` (asks to save if dirty) | M | ✓ |
| Open Recent ▸ | — | — | Last 10 projects; "Clear Menu" | M | ✓ |
| Save | Ctrl+S | ⌘S | Save | M | ✓ |
| Save As… | Ctrl+Shift+S | ⇧⌘S | Save under a new name | M | ✓ |
| Import ▸ MIDI File… | — | — | Adds the file's parts as new tracks at the insert marker (F-17) | M | ✓ |
| Import ▸ Audio as DPCM Sample… | — | — | WAV → DPCM slot | M | ✓ |
| Export Audio… | Ctrl+Shift+R | ⇧⌘R | Export dialog (R124) | M | ✓ |
| Export MIDI Clip… | Ctrl+Shift+E | ⇧⌘E | Selected clip as `.mid` (R125) | L | ✓ |
| Project Settings… | — | — | Profile, filter (§3) | M | ✓ |
| Exit (Windows) | Alt+F4 | (App menu: Quit ⌘Q) | Quit (asks to save if dirty) | M | ✓ |

### Edit
| Item | Windows | macOS | Does | Pri | CEO |
|---|---|---|---|---|---|
| Undo | Ctrl+Z | ⌘Z | Undo (R21) | M | ✓ |
| Redo | Ctrl+Y (also Ctrl+Shift+Z, ours) | ⇧⌘Z | Redo (R22; Ctrl+Shift+Z on Windows is not in the manual, R23) | M | ✓ |
| Cut / Copy / Paste | Ctrl+X / C / V | ⌘X / C / V | Clips, notes, or the time selection (R24) | M | ✓ · **done** (B3) |
| Duplicate | Ctrl+D | ⌘D | Clips/notes/time, placed right after (R25) | M | ✓ · **done** (B3) |
| Delete | Del | ⌫ / Del | Clips/notes/time (R26) | M | ✓ · **done** (B3) |
| Cut / Copy / Paste / Duplicate / Delete Time | Ctrl+Shift+X/C/V/D/Del | ⇧⌘ + same | Time commands (R46) | S | ✓ · **done** (B3) |
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
| Insert Track | Ctrl+Shift+T | ⇧⌘T | A studio track of the current track's type below it (R84) | M (model) | ✓ · **done** (4a) |
| Insert Track ▸ Pulse / Triangle / Noise / DPCM / VRC6 Pulse / Saw | — | — | A studio track of that type below the current one, named "Pulse 2"…, coloured by `next_track_color` (§4a C5) | M (model) | ✓ · **done** (4a) |
| Duplicate Track | (Ctrl+D with a track selected) | (⌘D) | Copies below, with copies of its clips; a copy of a locked track is a studio track (R87 unverified) | M (model) | ✓ · **done** (4a) |
| Delete Track | (Del with a track selected) | (⌫) | The track and its clips (R86); the last track stays | M (model) | ✓ · **done** (4a) |
| MIDI Clip | Ctrl+Shift+M | ⇧⌘M | In the time selection (R93) | M | ✓ |
| Time Signature Change | — | — | At the insert marker (R117) | L | ✓ |

### View
| Item | Windows | macOS | Does | Pri | CEO |
|---|---|---|---|---|---|
| Browser | Ctrl+Alt+B | ⌥⌘B | Show/hide (R80) | M | ✓ |
| Piano Roll / Inspector | Shift+Tab | ⇧Tab | Toggle the bottom panel between clip editor and instrument (R78) | S | ✓ |
| Follow | Ctrl+Shift+F | ⌥⇧F | Auto-scroll while playing, both editors (R135; decision 3) | S | ✓ |
| Zoom In / Out | + / − | + / − | Time zoom of the focused editor (R34) | M | ✓ · **done** (B3) |
| Zoom to Selection | Z | Z | (R30) | M | ✓ · **done** (B3) |
| Zoom Back | X | X | (R31) | M | ✓ · **done** (B3) |
| Zoom to Fit | (double-click the ruler) | same | Whole song (R32) | M | ✓ · **done** (B3) |
| Fit the Whole Song | W | W | The whole song in view; pressed again, back (the manual's Optimize Arrangement Width, P2-R13) | M | polish 2 · **done** |
| Show Overview | Ctrl+Alt+O | ⌥⌘O | The Overview strip above the ruler, with a checkmark; on by default, saved per user (P2-R6) | S | polish 2 · **done** |
| Draw Mode | B | B | Toggle (R5) | M | ✓ · **done** (B3) |
| Clock: Bars / Time | — | — | Replaces the click-on-clock toggle (F-38); Home rewinds | S | ✓ · **retired** (design pass): the display shows TIME and POSITION together |
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
| | Clock shows | Time (m:ss.cc) | S | ✓ · **retired** (design pass): both are shown |
| | Dark menus (Windows) | **On** (owner-drawn dropdowns; docs/design/DARK-MENUS.md) | S | polish pass: **on by default** (CEO-approved) |
| Look & Feel | Track headers | **Left** (or Right) | S | polish 2 · **done** |
| | Colour strip | **Outside** (at the window's edge; or By the clips) | S | polish 2 · **done** |
| | Browser | **Left** (or Right) | S | polish 2 · **done** |
| Editing | Default arrangement grid | 1/16 | S | ✓ |
| | Default piano-roll grid | 1/16 | S | ✓ |
| | Follow playhead | On | S | ✓ |
| | **Track and Clip Colors:** Auto-assign track colours | On (off: the **Default track colour**, #7F86E0) | S | polish pass · **done** (used by Batch 4's Create Track) |
| | Reduced automatic colours | Off (on: the palette's middle row, 14 colours) | S | polish pass · **done** |
| | New clip colour | **Track colour** (or Random: distinct from the clips beside it) | S | polish pass · **done** |
| Files | Autosave interval | 2 min | S | ✓ |
| | Offer recovery after an unsaved quit | On | M | ✓ |
| | Open last project at start | Off | S | ✓ |
| Warnings | "Don't ask again" choices, each with a Reset | all asking | M (with F-47) | ✓ |
| | Reset all warnings | — | M | ✓ |

Also stored (not shown): recent files, window size/position, browser open, export options,
split height, the track headers' width (380–600, the boundary drag) and View ▸ Show Overview.
The manual has no left/right setting for headers or the browser (P2-R43, P2-R44): those three are ours.

## 3. Project Settings

Stored **in the project file** (v5 since the design pass).

| Setting | Default | Notes | Pri | CEO |
|---|---|---|---|---|
| Hardware profile | **None (studio only)** for a new project (4a); NES (2A03) / Famicom + VRC6 | The channel budget is shown under it (§4e). Switching to a profile without a channel in use lists the tracks and asks: Unlock tracks / Switch anyway / Cancel (P5) | M | ✓ · **done** (4a) |
| Output filter | Profile's default (NES / Famicom / Off for None) | Moves here from the top bar | M | ✓ · **done** |
| Tempo | — | **Not here: it lives in the top bar** (decision 1, R116) | — | ✓ |
| Time signature | — | **Not here: the top bar**, next to tempo (decision 1, R116) | L | ✓ |
| Root and scale | Off | VISION "Scales" | L | ✓ |
| Sample rate, Bit depth | 48 kHz, 24-bit (files before v5: 48 kHz, 16-bit) | The project's format: the display's indicators show it and Export Audio starts from it. The output device's rate stays in Settings > Audio | S | design pass · **done** |

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

## 4a. Colours (polish pass)

Reference behaviour, Live 12 Reference Manual, chapter 2 "First Steps", §2.3.2 "Theme & Colors"
(paraphrased): track colours can be auto-assigned or not, with a default colour for all tracks;
"Reduced Automatic Colors" lowers the number of colours used for auto-assigning; the Clip Color
setting makes new clips get random colours or their track's colour. The manual gives no defaults
and doesn't describe the colour chooser itself, so the defaults and the grid below are ours.
**Copied: the behaviour. Not copied: the palette or the look.**

| # | Behaviour | Pri | Status |
|---|---|---|---|
| C1 | **Palette**: our own 70 base colours, 5 × 14 (hue, then lightness), `theme/palette.rs`; today's colours kept; none passes for the toggle lime (one logged exception, the legacy pulse colour) | S | done |
| C2 | **Colour grid**: Colour ▸ in the track and clip menus opens the 70 colours in the menu itself; hovering previews on the selection; a click applies to every selected track or clip (one undo step, unsaved mark, saved); the current colour is ringed. **A clip's colour is its own**: it stays when its track's colour changes; the clip menu's "Track Colour" clears it so the clip follows its track again (format v4) | S | done |
| C3 | **Clip text** is `#111111` or `#F2F2F2`, whichever reaches 4.5:1 on the header | M | done |
| C4 | **Settings ▸ Editing ▸ Track and Clip Colors** (§2) | S | done |
| C5 | New clips follow **New clip colour**; `next_track_color` is ready for Create Track (Batch 4) | S | done (clips) / ready (tracks) |

## 4b. The top bar (design pass, Option D)

See [docs/design/VISUAL-SPEC.md](docs/design/VISUAL-SPEC.md) §2.

| # | Control | Does | Pri | Status |
|---|---|---|---|---|
| TB1 | Arrangement / Mixer keys | Arrangement is the only view; Mixer is disabled with its reason (Batch 4) | S | done |
| TB2 | Play, Stop, Record, Loop | Play starts (does nothing while playing, ours); Stop stops, again goes to the start; Record disabled ("Recording arrives with MIDI input"); Loop switches the song loop. The keys never light: the display shows the state | M | done |
| TB3 | The display | Read-only: indicators (playing, looping, the project's rate and bit depth, lime while active), TIME, POSITION, START and LENGTH (the time selection or the selected clips' span), BPM. **BPM**: drag (Shift: finer) or wheel; double-click to type | M | done |
| TB4 | TAP (key **T**, ours) | Each tap sets BPM from the average of the last 4 intervals; taps more than 2 s apart start over; one sequence is one undo step | S | done |
| TB5 | The spectrum | The mix; right-click: Bars, LED Segments, Line, Peak Dots (remembered in settings.json) | S | done |
| TB6 | CPU / MEM | The audio callback's load (OFF without output); the process's private memory | S | done |

**Text-size exception (design pass, CEO-approved mockup):** the display's labels (TIME,
POSITION, START, LENGTH, BPM) are Inter 600 **10 px**, below the 11 px minimum for informative text
(F-39). They are uppercase, tracked .1em, and `#808080` on `#151515` is 4.6:1, above 4.5:1.

## 4c. The status bar (polish 2)

A 24 px bar along the window's bottom (VISUAL-SPEC.md §10). The reference's Status Bar shows
messages (Live 12 manual, ch. 3 Live Concepts, §3.2 The Status Bar, P2-R38) and its Info View
shows the name and function of the control under the mouse (ch. 2 First Steps, §2.2.2 Info View,
P2-R39). We copy the behaviour, not the look.

| # | Behaviour | Pri | Status |
|---|---|---|---|
| SB1 | Left, `role=status`: the latest non-urgent message with a small icon, 8 s, then the idle text ("Saved · name" / "Unsaved changes · name"). A click shows a long message whole. Every former toast moved here (docs/polish2/notices.md) | S | done |
| SB2 | Persistent problems (no audio device, a device that won't open, dropouts until 10 s pass without one) put a warning icon on the bar until resolved; a click lists them. (In the reference, overloads show in the Control Bar, not the Status Bar: P2-R42, contradicted; our CEO's spec wins.) A hardware profile over its budget joins this list with Batch 4 | S | done |
| SB3 | Right: one line of hover help for the control under the mouse: header controls, transport, display, TAP, ruler, Overview, spectrum, CPU/MEM (their tooltips converted). Inspector, browser and dialog tooltips stay tooltips | S | done |
| SB4 | Dialogs stay for decisions only: save prompts, the MIDI-drop conflict, profile changes, anything destructive; the recovery offer stays in its capsule | M | done |

## 4d. Navigation: ruler and Overview (polish 2)

| # | Action | Result | Pri | Status |
|---|---|---|---|---|
| N1 | Drag the beat-time ruler **vertically** | Zooms smoothly (down: in), anchored under the mouse so the grabbed bar stays put (P2-R7, ch. 6 Arrangement View, §6.1 Layout item 2, §6.2 Navigation and Zooming) | M | done |
| N2 | Drag the ruler **horizontally** | Scrolls (the grabbed bar follows the mouse) (P2-R7) | M | done |
| N3 | Click the ruler | Places the playhead, as before (a press that moves under 3 px is a click; the manual doesn't say what a plain click does, P2-R11). **Shift+drag** scrubs (P2-R10, §6.3). The loop brace keeps its gestures | M | done |
| N4 | Double-click the ruler | Zooms to the time selection, or the whole song (P2-R8, R32) | M | done (B3) |
| N5 | W | Fits the whole song; again, back (P2-R13) | M | done |
| N6 | + / − | Zoom in / out (P2-R15) | M | done (B3) |
| N7 | The Overview (32 px above the ruler) | The whole arrangement, one row per track (several per row past 14), the visible part outlined (P2-R1, P2-R2); drag sideways scrolls, up/down zooms where grabbed (P2-R3); a click outside the outline jumps there (ours: P2-R5 unverified); double-click as the ruler (P2-R4) | S | done |
| N8 | View ▸ Show Overview, Ctrl+Alt+O (⌥⌘O) | Shows / hides it (P2-R6); on by default, saved per user; hidden, its 32 px go back to the lanes | S | done |

## 4e. Tracks, profiles, locks and Main (Batch 4a)

VISION.md "The core model". Code: `tracks.rs` (model), `track_ui.rs` (commands, warnings),
`engine::studio` / `engine::mixdown` (voices), `views/main_track.rs`. Decisions: RUNLOG B4A-D6…D16.

| # | Behaviour | Pri | Status |
|---|---|---|---|
| M1 | **A track is an instrument** of one voice type (Pulse, Triangle, Noise, DPCM, VRC6 Pulse, Saw; *2026-11-26: becomes Chip Synth with a Voice selector plus the DPCM Sampler in the Presets + Browser + device panel batch, VISION.md "Instruments"*). **Studio** (default): unlimited tracks, unlimited polyphony (a pool of voices grown from the UI thread; the quietest voice is stolen if a pool is ever full), fader and pan work. **Locked**: bound to one real channel, the full chip emulator, monophonic (the last note wins), fader at 0 dB and pan centred | M | done |
| M2 | **Voices ▸** (studio tracks): Unlimited, 1 … 16: a cap on the track's voices | S | done |
| M3 | **The badge**: locked: the lock and the channel ("2A03 P1", "VRC6 SAW"); studio: the type ("PULSE"); an amber warning mark when the track is locked to a channel the profile lacks. A click opens the lock menu | M | done |
| M4 | **Locking** a track whose fader isn't at 0 dB or pan isn't centred asks ("This track's fader is at −6.0 dB. Locking resets it to 0 dB and centre": Reset and lock / Cancel); one undo step restores lock, fader and pan. **Unlocking** restores the fader and pan from before locking | M | done |
| M5 | **Locked header**: the fader thumb at 0 dB, disabled (40% opacity); a lock glyph instead of the dB value; pan centred and disabled; hover help "Locked to the 2A03 Pulse 1: level comes from the chip's Volume register (Inspector > Volume)" | M | done |
| M6 | **The channel budget** (`Pulse 2/2 · Triangle 1/1 · …`), red when over: in Project Settings, and compact in the TRACKS cell (`P 2/2 · T 1/1 · …`, hover help in full). One expansion chip per project | M | done |
| M7 | **Profile warnings** ask before an action breaks the profile: a chord or overlap on a locked track (Unlock / Fit: the last note wins / Cancel), a note outside the channel's range (Fit: by octaves / Cancel), DPCM notes without a sample (Fit: removed / Cancel), a layout for another chip (Switch profile / Load on studio tracks / Cancel), a profile without a channel in use (Unlock tracks / Switch anyway / Cancel). "Don't ask again" (Settings ▸ Warnings resets it). No dialog when nothing conflicts; an old file over budget is a status-bar problem | M | done |
| M8 | **Authentic presets** are the channel layouts: loading one puts the project under its profile and locks the tracks it fills (B4A-D14). *2026-11-26: they move to the browser's Hardware folder (Presets + Browser batch)* | L | done (layouts) |
| M9 | **The Main track**: pinned at the bottom of the track list (Live 12 manual, ch. 18 Mixing, §18.4: "the bottom end of the Arrangement View"; never scrolling is ours), a black strip, "Main", OUT and the output device, PEAK (held; a click resets), the master fader (−inf … +6 dB) applied after the whole mix, saved in the project and the export JSON. It follows the header layouts. Its lane: the grid and the playhead (Batch 5: master automation) | M | done |
| M10 | **Migration**: older projects open with every track locked to its channel; rendered audio is bit-identical (golden render and v2…v6 hashes) | M | done |

## 5. Behaviour tables

### 5.1 Arrangement
| # | Action | Result | Pri | CEO |
|---|---|---|---|---|
| A1 | Click empty space | Insert marker there; clears clip selection (today) | M | ✓ |
| A2 | Drag empty space | Time selection across tracks (today) | M | ✓ |
| A3 | Shift-click | Extends the time selection, across tracks (R44) | S | ✓ · **done** (B3) |
| A4 | Click / Shift-click / Ctrl-click a clip | Select / add range / toggle (R28: Live 12 manual §42.5 Editing, §42.9 Zooming, Display and Selections). **Batch 4a:** Ctrl-click (⌘-click on macOS) adds or removes one clip; Shift-click selects the range from the last selected clip to this one (every clip on the tracks between that overlaps the time between); a Ctrl-drag (⌥-drag) still copies, made once the drag moves | M | ✓ · **done** (4a) |
| A5 | Drag clip, edges | Move (also to another track) / resize (today) | M | ✓ |
| A6 | Ctrl-drag (⌥-drag on macOS; ⌘-drag there bypasses the snap) clip | Copy (today; unverified for the arrangement, R48) | S | ✓ |
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
| T3 | Right-click | Rename, Colour ▸, Insert Track ▸, Duplicate Track, Delete Track, Lock ▸, Voices ▸ | M | ✓ · **done** (4a) |
| T4 | Drag header | Reorder (R88: drag a track by its title bar, Live 12 §6.1 item 11, §18.2); a 2 px line shows where it lands; clips move with their track; one undo step | M (model) | ✓ · **done** (4a) |
| T5 | M / S | Mute / solo (today); Cmd-click solo adds (R143) | S | ✓ · **done** (B3) |
| T6 | Del with a track selected | Delete track (R86) | M (model) | ✓ · **done** (4a) |
| T7 | Click the badge | The lock menu: Studio (unlocked) and the profile's channels of the track's type; a taken channel is disabled and names its owner (§4e) | L | ✓ · **done** (4a) |
| T8 | Volume meter-fader | **Polish 2:** a vertical fader in **dB** (−inf … +6 dB, position `(dB + 48) / 54`): a gain after the chip on every track (RUNLOG P2-D4). Drag (press sets the value) or wheel (1 dB); **double-click the dB value to type it** (`-3`, `-3 dB`, `−3`, `+2.5`, `-inf`; Enter commits, Esc cancels, a click away commits; clamped; one undo step; unsaved mark; P2-R31). The post-fader meter fills from the bottom on the same scale, hot at 0 dB, never past the thumb. dB scale marks (0 6 12 24) on rows ≥ 96 px | S | polish 2 · **done** |
| T9 | Pan | **Polish 2:** editable and saved (−50 … +50, shown 50L … C … 50R): drag, wheel, or double-click the value to type `C`, `0`, `25L`, `30R`, `-25`, `+25`. Heard once stereo mixing arrives (Batch 4); its hover help says so | L | polish 2 · **done** (not heard yet) |
| T10 | Strip | Lights on each note-on and fades over ~250 ms; toned down ~20% (polish 2) | S | design pass · **done** |
| T11 | Row height | 72 (minimum) … 300 px in 4 px steps: **drag a track's bottom edge** (P2-R19; Alt / ⌥ while dragging: every track, P2-R20) or **Alt+wheel** (⌥+wheel; on a trackpad the steps accumulate, MC-D14) over a track (P2-R21). Saved per track in the project (format v6), undoable. Taller rows keep the number, name and column A in a 72 px band; only the fader stretches | S | polish 2 · **done** |
| T12 | Header width | Drag the boundary between headers and lanes: 380 … 600 px, extra width to the name only; saved per user. (The manual doesn't describe one: P2-R29, ours) | S | polish 2 · **done** |
| T13 | Layout | Settings ▸ Look & Feel: headers Left / Right, strip Outside / By the clips (the order table, VISUAL-SPEC.md §4a); the TRACKS and OVERVIEW cells move with the headers | S | polish 2 · **done** |

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
*Decided 2026-11-26:* this flat browser is **replaced, not kept alongside**, by Live's structure
(Collections / Library / Places, folders first) in the Presets + Browser + device panel batch
(VISION.md "The browser"). The rows below describe today's browser.

| # | Action | Result | Pri | CEO |
|---|---|---|---|---|
| B1 | Typing in search | Never triggers shortcuts (F-03); Esc clears | M | ✓ · **done** (B3) |
| B2 | Click preset | Select + hear (today); while playing: hear through the preview voice or say why not | S | ✓ · **done** (B3): no preview voice yet, so a click while playing says why |
| B3 | Double-click / drag preset | Load (today) | M | ✓ |
| B4 | Click layout | Select only; double-click or drag loads (F-36). A layout is a template: if it needs another hardware profile, it asks first (decision 5) | S | ✓ · **done** (B3) |
| B5 | Right-click user preset | Rename, Delete, Show in Explorer/Finder | S | ✓ · **done** (B3) |
| B6 | Authentic preset row | Shows 🔒 + profile; loading it locks the track (VISION) | L | ✓ |
| B7 | Preview on select | Selecting a preset (click or ↑/↓) auditions it while stopped; the header's Preview key (headphones) turns it off and on (on by default, remembered); while playing it says why it's silent | S | design pass · **done** |

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

| P2-01 (T8) | Double-click track 1's dB value, type `-3`, Enter: saved at −3 dB, the title marks it unsaved; one Ctrl+Z restores 0 dB; Esc cancels; `-inf` is silence; `+2.5` and a click elsewhere commits |
| P2-02 (T9) | Type pan values `25L`, `30R` (one Ctrl+Z back to 25L), `-12` (12L): saved |
| P2-03 (T8, T9) | Drag the fader down: lower volume, one Ctrl+Z undoes the drag; drag the pan thumb right: pans right |
| P2-04 (T11) | Drag track 1's bottom edge down 72 px: 144; Alt+wheel up over track 2: 84; Ctrl+Z: 72 (saved files) |
| P2-05 (T12) | Drag the boundary: 520 (and it stops at 600 and 380); settings.json keeps it after a restart |
| P2-06 (T13) | Settings ▸ Look & Feel: Right, By the clips, Browser Right: each saved and applied at once |
| P2-07 (N1–N4) | Ruler: a click places the playhead; a vertical drag zooms in; a horizontal drag scrolls; a double-click fits; its hover help |
| P2-08 (N5) | W fits the song; W again goes back |
| P2-09 (N7, N8) | Overview: a click outside the outline jumps; drags scroll and zoom; a double-click fits; Ctrl+Alt+O hides and shows it (settings.json) |
| P2-10 (SB1–SB3) | Hover help for the fader, M, pan, view keys, transport, display, TAP, spectrum, CPU/MEM, ruler, Overview; a click on the problem icon lists the problems |
| P2-11 (T13, §5.4) | In all four layouts: M, S, the dB value, the pan value, the fader, a row's bottom edge, the name (rename), the width boundary, the ruler and the Overview hit what they should (saved file, settings.json) |
| [unit] (polish 2) | `state::fader_tests` (dB scale round trip, meter rule, value parsing, scale marks, row heights), `header_layout::tests` (the order table, width), `overview::tests` (row plan, the shared nav drag), `palette` (strips lit/unlit apart ≥ 0.15 ΔE, the overview recipe), `chip::mixer` (gained mix = plain mix at unity, a lone channel ×2), `tests/migration.rs` (v5 → v6 hashes, heights/pan/volume saved) |

| B4A-01 (MC-D9) | A clip selected, Export open: Backspace, Delete; Settings open: Return, Backspace, Delete: the clips are unchanged and the piano roll never opens |
| B4A-02 (C2, MC-D15) | Hover a colour, Esc: the old colour, nothing unsaved; hover A, pick B, Ctrl+Z: the original colour |
| B4A-03 (A4, R28) | Ctrl-click toggles a clip (no copies); Shift-click selects the range; Ctrl-drag still copies |
| B4A-04 (AT-13) | Ctrl+Shift+T adds a studio track below track 1; drag it above track 1 (clips move with tracks); Delete removes it; a colour from the header menu is saved |
| B4A-05 (M3, M4) | The badge's menu (a taken channel disabled); lock with "Reset and lock" (0 dB / C, old values kept); one Ctrl+Z restores; unlocking restores the fader and pan |
| B4A-06 (M7) | Switching Famicom + VRC6 → NES with a locked VRC6 track asks: Unlock / Switch anyway / Cancel each do what they say |
| B4A-07 (M7) | A chord on a locked track asks: Fit (no overlaps), Unlock (the chord on a studio track), Cancel (the edit undone) |
| B4A-08 (M9) | Three wheel notches on the Main fader: −3 dB, saved; PEAK's hover help |
| B4A-09 (T3, M2) | Insert Track ▸ Noise from the header menu; Voices ▸ 4; Ctrl+D on a selected track duplicates it with its clip |
| [unit] (Batch 4a) | `engine::mixdown::tests` (bit-identity without studio tracks, 8-voice chords, quietest-voice stealing, the Voices cap, pool growth, Main gain), `core::studio_rack_plays_and_grows_without_allocating`, `chip::voice::tests` (one implementation; a lone voice = the console within 0.1 dB), `tracks::tests` (names, insert/delete/move/duplicate, lock/unlock, budget, mute/solo), `params::tests`, `palette::black_is_reserved_for_main`, `warn::tests`, `edit::colour_preview_tests`, `selection_tests`, `tests/glyphs.rs`, `tests/migration.rs` (v6 → v7 hashes, the profile rules, the −3 dB report) |

Batch 4a results: B4A-01…B4A-09 in `docs/batch4a/tools/at_b4a.ps1` (checker `check_b4a.py`); see
[BATCH4A.md](BATCH4A.md).

Polish 2 results: P2-01…P2-11 pass in `docs/polish2/tools/at_p2.ps1` (checker `check_p2.py`); the
earlier suites run through `docs/polish2/tools/run_old_suites.ps1` (see POLISH2.md §5).

| PL-01 (C2) | Colour ▸ on a track, a click on a grid cell: the track takes it, one Ctrl+Z restores it, the file saves it |
| PL-02 (C2) | Two clips selected, Colour ▸ cell: both take it; the track's colour then changes and the clips keep theirs |
| PL-03 (C5) | New clip colour Random: a clip inserted with Ctrl+Shift+M gets a palette colour of its own |
| PL-04 (polish §2.4) | At 1536×816 with the inspector open, the wheel over the headers scrolls the lanes |
| PL-05 (polish §2.4) | The window can't be made smaller than 1296×720 (logical); the top bar's groups don't touch there |
| DP-01 (TB4) | Four taps 500 ms apart → 120 BPM; one Ctrl+Z restores the tempo; the TAP key taps too |
| DP-02 (TB5) | The spectrum style chosen from its menu is in settings.json and back after a restart |
| DP-03 (B7) | A click and ↓ each audition a preset; with the Preview key off a click is silent; while playing it says why |
| DP-04 (TB1, TB2, T8, T9) | The Mixer and Record keys, the pan slider and the triangle's fader show their reasons on hover |
| DP-05 (§3) | Project Settings' Bit depth is saved (format v5) and shown in the display |
| [unit] (design pass) | `tap::tests` (120 BPM, last four intervals, restart), `state::fader_tests` (the meter never passes the thumb; fader rules), `topbar::tests`, `tests/migration.rs` (v4 → v5 hashes) |

Design pass results: DP-01…DP-05 pass in `docs/design/tools/at_dp.ps1` (checker
`check_dp.py`); AT-01…AT-19 and AT-07…AT-31 still pass with the harness coordinates moved to the
new layout. See [DESIGNPASS.md](DESIGNPASS.md).

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
| **4a. The model change** | Tracks as instruments, voice pools, profiles and locks, the Main track (BATCH4A.md) | F-13, F-40, F-42, F-43, F-48 | **done** |
| **Authenticity A/B** | Reference oracles outside the repo, register-write diffs, Mika's ear review (VISION.md "Sound systems") | | next |
| **4b.** | Visual fixups (§8, VISUAL-SPEC.md §13, the `fixups` and `main-track` boards), the mixer view, stereo pan audible, MIDI import as tracks with the drop preview, unison and detune | F-17 (full), F-47 | |
| **Presets + Browser + device panel** | Chip Synth (Voice selector), DPCM Sampler, the in-instrument preset picker, the new browser, Mika's 100+ bank | | |
| **5. Automation** | Lanes, clip envelopes, tempo automation (the registry is ready: docs/batch4a/parameters.md) | F-44 | ~3–4 weeks |
| **Then** | The device chain (MIDI effects, then audio effects) → tracker import → recording and MIDI input → time signatures and scales (F-45, F-46) → the export polish → the Metal backend → the arcade board (ymfm) → the SNES (VISION.md roadmap, decided 2026-11-26) | | |

Every batch ends with its acceptance tests in the scenario harness (VISION.md, "How we work").

## 8. Open issues

Known bugs, listed 2026-11-26 (the meeting); fixed in 4b's visual fixups unless marked closed.
Each fix gets an automated test.

| # | Issue | Platform | Target | Status |
|---|---|---|---|---|
| K1 | The menu bar (File, Edit, Create…) **flickers while the window is resized**. Suspected cause: the dark-menu owner-drawn bar (docs/design/DARK-MENUS.md). Test with dark menus off | Windows | **0 flashing frames** in a scripted resize capture | open (4b) |
| K2 | The browser search field's **focus ring is clipped** | macOS | Draw the ring inside the field, or pad the container | open (4b) |
| K3 | **Track numbers ≥ 10** crowd the name | both | The number column's width follows the largest track number (VISUAL-SPEC.md §13) | open (4b) |
| K4 | **Return and Backspace reach the song behind dialogs** (MAC-CATCHUP.md §8, MC-D9) | shared | Keys never reach the song while a dialog is open | **closed** by Batch 4a (RUNLOG B4A-D2; scenario B4A-01, which fails on the baseline). Tested on Windows; re-run B4A-01's steps on the next Mac run |
| K5 | **Colour-preview leaks**: Esc after hovering a colour kept it; hover A, pick B, undo gave A (MC-D15) | shared | Previews never reach undo | **closed** by Batch 4a (B4A-D3; `edit::colour_preview_tests`, scenario B4A-02). Tested on Windows; re-check on the next Mac run |
| K6 | The project loader **panics on a fuzzed file in debug builds**: `load_clips`' error message adds 1 to a huge lane index (`file.rs:790`); `robustness::project_loader_never_panics_or_hangs` fails under `cargo test`, passes under `--release` (RUNLOG DOC-D17) | shared | The fuzz test passes in debug and release | open (4b) |
