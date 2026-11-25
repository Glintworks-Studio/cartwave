# Ableton Live reference behaviours (manual check)

- **Source:** Ableton Live 12 Reference Manual (the version 12 manual served at https://www.ableton.com/en/manual/, canonical pages under `https://www.ableton.com/en/live-manual/12/`). No other sources were used.
- **Checked:** 2026-11-25.
- **All text below is paraphrased.** Nothing is quoted from the manual. Command and menu names (for example "Insert Silence") are given only so the reader can find them.
- **Verdicts:** *verified* means the manual states the behaviour. *contradicted* means the manual states something different, and the row says what. *unverified* means the pages read here do not state it. That does not prove Live lacks the behaviour.
- **Keys:** shown as Windows / macOS. "same" means the macOS key is identical. Chapter 42 is "Live Keyboard Shortcuts". Rows marked **(added)** are related behaviours found while checking.
- **Pages read:** ch. 2 First Steps, 3 Live Concepts, 4 Working with the Browser, 5 Managing Files and Sets, 6 Arrangement View, 7 Session View, 8 Clip View, 10 Editing MIDI, 18 Mixing, 19 Recording New Clips, 26 Automation, 27 Clip Envelopes, 38 Computer Audio Resources, and 42 Live Keyboard Shortcuts.

URL shorthand used below: `…/12/` = `https://www.ableton.com/en/live-manual/12/`.

## 1. MIDI note editor: creating and deleting notes

| ID | Behaviour | Verdict | Windows / macOS keys | Manual citation (chapter § section, URL) | Paraphrase |
|---|---|---|---|---|---|
| R1 | Double-click an existing note deletes it | verified | mouse | 10.4.1 Editing MIDI > Adding MIDI Notes > Draw Mode, `…/12/editing-midi/#draw-mode` | When Draw Mode is off, a double-click on a note removes it. |
| R2 | Double-click empty grid space creates a note | verified | mouse | 10.4 Adding MIDI Notes, `…/12/editing-midi/#adding-midi-notes` | A double-click on an empty spot in the note editor adds a note there, snapped to the grid if snapping is on. |
| R3 | Length of a note created by double-click | unverified | n/a | 10.4 Adding MIDI Notes, `…/12/editing-midi/#adding-midi-notes` | The manual does not say what length the new note gets. A grid-step length is the common assumption, but it is not documented. |
| R4 | Delete key removes selected notes | unverified | Delete / same (generic, 42.5) | 42.5 Editing, `…/12/live-keyboard-shortcuts/#editing`; ch. 10 does not mention it | Chapter 42 lists Delete as the general delete command. The MIDI chapter only describes deleting notes by double-click or a Draw Mode click. |

## 2. Draw Mode

| ID | Behaviour | Verdict | Windows / macOS keys | Manual citation (chapter § section, URL) | Paraphrase |
|---|---|---|---|---|---|
| R5 | B toggles Draw Mode | verified | B / B | 42.13 Grid Snapping and Drawing, `…/12/live-keyboard-shortcuts/#grid-snapping-and-drawing`; 10.4.1 Draw Mode, `…/12/editing-midi/#draw-mode` | B, the Control Bar switch, or the Options menu turns Draw Mode on and off. |
| R6 | Draw Mode in the MIDI editor | verified | B; Alt / Option toggles pitch lock while drawing | 10.4.1 Draw Mode, `…/12/editing-midi/#draw-mode`; 2.3.1 Display & Input, `…/12/first-steps/#display--input` | In Draw Mode a click-drag paints notes, and a click on an existing note erases it. A setting can lock drawing to one pitch row, and Alt/Option temporarily switches to the other behaviour. |
| R7 | Draw Mode on automation envelopes | verified | B (hold B for momentary) | 26.6.2 Drawing Automation Envelopes with Draw Mode, `…/12/automation/#drawing-automation-envelopes-with-draw-mode`; 42.25 Momentary Latching Shortcuts | Draw Mode lets you paint envelope steps freehand at the grid resolution. Turning off snap, or holding Alt/Cmd, gives smoother freehand curves. |

## 3. Rename

| ID | Behaviour | Verdict | Windows / macOS keys | Manual citation (chapter § section, URL) | Paraphrase |
|---|---|---|---|---|---|
| R8 | Rename the selected item | verified | Ctrl+R / Cmd+R | 42.5 Editing, `…/12/live-keyboard-shortcuts/#editing` | Rename is an Edit command with its own shortcut. It applies to whatever is selected. |
| R9 | Rename a track; Tab jumps to the next track | verified | Ctrl+R / Cmd+R, then Tab | 18.2 Audio and MIDI Tracks, `…/12/mixing/#audio-and-midi-tracks`; 42.19 Commands for Tracks | Tracks are renamed with Edit > Rename or the shortcut. Tab moves to the next track's title so you can rename several in turn. |
| R10 | Rename a clip | verified | Ctrl+R / Cmd+R or context menu | 8.1.1 Clip Title Bar, `…/12/clip-view/#clip-title-bar`; 7.1 Session View, `…/12/session-view/` | A clip can be renamed from its context menu, from the Clip View title bar menu, or with Edit > Rename. Several selected clips can be renamed at once. |
| R11 | Rename a locator | verified | Ctrl+R / Cmd+R | 6.4 Launching the Arrangement with Locators, `…/12/arrangement-view/#launching-the-arrangement-with-locators` | Select a locator, then use Edit > Rename or the shortcut to change its label. |

## 4. Context menus

| ID | Behaviour | Verdict | Windows / macOS keys | Manual citation (chapter § section, URL) | Paraphrase |
|---|---|---|---|---|---|
| R12 | How a context menu opens | verified | Right-click (or Menu key / Shift+F10) / Ctrl-click (or VO+Shift+M) | 42.29 Using Live's Context Menu, `…/12/live-keyboard-shortcuts/` | Context menus open with a right-click on Windows or a Ctrl-click on macOS, with keyboard alternatives for accessibility. |
| R13 | Clip context menu items | verified (partial list) | right-click | 8.1.1 Clip Title Bar, `…/12/clip-view/#clip-title-bar`; 6.9 Selecting Clips and Time; 8 (deactivate) | Items named in the manual are Rename, Edit Info Text, a colour palette, Reverse Clip(s), and activate/deactivate. The manual does not give a complete list. |
| R14 | Track header context menu items | verified (partial list) | right-click | 6.14.1 Linking and Unlinking Tracks, `…/12/arrangement-view/#linking-and-unlinking-tracks`; 18.2 and 18.3 Mixing | Items named are Link Tracks / Unlink Track(s), Edit Info Text, and on Group Tracks the command that pushes the group colour to its tracks and clips. No full list is given. |
| R15 | Note context menu items | verified (partial list) | right-click | 10.5.2, 10.5.5, 10.5.7, 10.5.8, 10.5.13.1, `…/12/editing-midi/` | Items named are Invert Selection, Fit to Time Range, Deactivate Note(s), Split, Chop on Grid, Join Notes, Group/Ungroup Notes, Draw Mode, and Highlight Scales. |
| R16 | Arrangement background (main lane) context menu | verified (partial list) | right-click | 6.10 Using the Editing Grid, `…/12/arrangement-view/#using-the-editing-grid`; 6.12 Splitting Clips; 6.13 Consolidating Clips | The main lane menu holds the grid settings (line spacing, triplets, snap) plus Split and Consolidate for the current selection. |
| R17 | Scrub area (above the tracks) context menu | verified | right-click | 6.4 Locators, `…/12/arrangement-view/#launching-the-arrangement-with-locators`; 6.5 Time Signature Changes, `…/12/arrangement-view/#time-signature-changes` | The scrub area menu offers Add Locator, Insert Time Signature Change, and Edit Time Signature. |
| R18 | Locator / loop brace / time-signature marker context menus | verified | right-click | 6.4, 6.5, 6.6 The Arrangement Loop, `…/12/arrangement-view/#the-arrangement-loop` | Locators offer looping to the next locator, Set Song Start Time Here, and Edit Info Text. The loop brace offers Set Song Start Time Here. Time-signature markers offer editing, and fragmentary bars can be deleted or completed. |
| R19 | Envelope / breakpoint context menus | verified (partial list) | right-click | 26.6.1, 26.6.3, 26.7.1, 26.7.3, 26.8, `…/12/automation/` | Envelope menus offer Edit Value and Add Value (exact numbers), Delete, insertable shapes (sine, triangle, saw, square, ramp, ADSR), Simplify Envelope, and the Clear Envelope variants. |
| R20 | Beat-time ruler context menu | unverified | n/a | 6.1 Layout, `…/12/arrangement-view/#layout` | The manual describes dragging and double-clicking the ruler but does not describe a context menu for it. |

## 5. Edit menu commands

| ID | Behaviour | Verdict | Windows / macOS keys | Manual citation (chapter § section, URL) | Paraphrase |
|---|---|---|---|---|---|
| R21 | Undo | verified | Ctrl+Z / Cmd+Z | 42.5 Editing, `…/12/live-keyboard-shortcuts/#editing` | Undo reverses the last action. |
| R22 | Redo on Windows is Ctrl+Y | verified | Ctrl+Y / Cmd+Shift+Z | 42.5 Editing | Redo is Ctrl+Y on Windows and Cmd+Shift+Z on macOS. |
| R23 | Redo on Windows is Ctrl+Shift+Z | contradicted | manual: Ctrl+Y | 42.5 Editing | The manual lists only Ctrl+Y for Windows redo. Ctrl+Shift+Z is the macOS-style key and is not documented for Windows. |
| R24 | Cut / Copy / Paste | verified | Ctrl+X/C/V / Cmd+X/C/V | 42.5 Editing | These are the standard clipboard commands. |
| R25 | Duplicate | verified | Ctrl+D / Cmd+D | 42.5 Editing; 10.7.3 Looping, `…/12/editing-midi/#looping` | Duplicate copies the selection. In a MIDI clip with the loop brace selected, it doubles the loop and its notes. |
| R26 | Delete | verified | Delete / Delete | 42.5 Editing; 18.2 (tracks), 42.19 | Delete removes the selection. A selected track is removed with Edit > Delete or Delete on its title bar. |
| R27 | Select All | verified | Ctrl+A / Cmd+A | 42.5 Editing; 42.12 MIDI Note Editor | Select All selects everything in the focused area, including all notes in the note editor. |
| R28 | Multi-select with Ctrl/Cmd-click and range-select with Shift-click | verified | Ctrl+click / Cmd+click; Shift+click | 42.5 Editing; 42.9 Zooming, Display and Selections | Ctrl/Cmd-click adds items one at a time. Shift-click selects a contiguous range. |
| R29 | Undo History window **(added)** | verified | Ctrl+Alt+Z / Cmd+Option+Z | 5.4.2 Accessing a Set's Undo History, `…/12/managing-files-and-sets/#accessing-a-sets-undo-history` | The View menu has an undo history list, newest first, where you can jump back or forward. It is cleared when the Set is reopened, and opening the Set is the first, non-undoable entry. |

## 6. Zoom

| ID | Behaviour | Verdict | Windows / macOS keys | Manual citation (chapter § section, URL) | Paraphrase |
|---|---|---|---|---|---|
| R30 | Z zooms to the time selection | verified | Z / Z | 6.2 Navigation and Zooming, `…/12/arrangement-view/#navigation-and-zooming`; 42.16 Arrangement View | Z fills the view with the current time selection. Holding Z works as a momentary latch (42.25). |
| R31 | X zooms back | verified | X / X | 6.2; 42.16 | X steps back through earlier zoom states, one step per press. It does not jump straight to the whole arrangement. |
| R32 | Zoom to fit the whole arrangement | verified (via ruler double-click) | double-click the beat-time ruler | 6.2 Navigation and Zooming | Double-clicking the beat-time ruler zooms to the selection. With nothing selected, it zooms out to show the whole arrangement. No single key does this. |
| R33 | W = optimize width, H = optimize height | verified | W / W; H / H | 6.2; 42.16 | W fits the arrangement's length into view and H fits all tracks vertically. Both toggle. In the Clip View editors, W and H fit the content (42.11, 42.12). |
| R34 | + / - zoom in and out | verified | + / - | 6.2; 42.9 Zooming, Display and Selections | Plus and minus zoom the timeline in and out around the selection. Ctrl/Cmd with + or - zooms the whole window (UI scale) instead. |
| R35 | Ctrl/Cmd + mouse wheel zooms | verified | Ctrl+wheel / Cmd+wheel or trackpad | 6.2 Navigation and Zooming; 10.2, `…/12/editing-midi/#zooming-and-navigating-in-the-midi-note-editor` | Wheel or trackpad with Ctrl/Cmd zooms the time axis. With Alt/Option it zooms a track's height (arrangement) or the key rows (MIDI editor). |
| R36 | Drag vertically in the beat-time ruler to zoom | verified | mouse | 6.2; 10.2 | A vertical drag in the time ruler zooms and a horizontal drag scrolls. This works in both the arrangement and the note editor. |
| R37 | Shift+wheel scrolls horizontally **(added)** | verified | Shift+scroll / same | 42.9 Zooming, Display and Selections | Holding Shift turns wheel scrolling into left/right scrolling. |

## 7. Split and consolidate

| ID | Behaviour | Verdict | Windows / macOS keys | Manual citation (chapter § section, URL) | Paraphrase |
|---|---|---|---|---|---|
| R38 | Split clip at the insert marker or selection | verified | Ctrl+E / Cmd+E | 6.12 Splitting Clips, `…/12/arrangement-view/#splitting-clips`; 42.16 | Split cuts a clip at the insert marker. With a time range selected, it cuts out that range as its own clip. |
| R39 | Consolidate | verified | Ctrl+J / Cmd+J | 6.13 Consolidating Clips, `…/12/arrangement-view/#consolidating-clips`; 42.16 | Consolidate merges the selected material on each track into one new clip. |
| R40 | Crop selected clips **(added)** | verified | Ctrl+Shift+J / Cmd+Shift+J | 42.16 Arrangement View | A separate command trims the selected clips to the selection. |
| R41 | Split Arrangement clip from the note editor's time selection **(added)** | verified | Ctrl+Shift+E / Cmd+Shift+E | 42.12 Clip View MIDI Note Editor | From the MIDI editor, this key splits the Arrangement clip at the editor's time selection. In the note editor itself, Ctrl/Cmd+E chops or splits notes. |

## 8. Arrangement selection, time commands, moving and copying

| ID | Behaviour | Verdict | Windows / macOS keys | Manual citation (chapter § section, URL) | Paraphrase |
|---|---|---|---|---|---|
| R42 | Clicking in a track sets the insert marker | verified | mouse | 6.9 Selecting Clips and Time, `…/12/arrangement-view/#selecting-clips-and-time` | A click in a track places the blinking insert marker, which is also where playback will start. |
| R43 | Dragging makes a time selection | verified | mouse | 6.9 | A click-drag selects a span of time. |
| R44 | Shift-click extends the selection, including across tracks | verified | Shift+click; Shift+arrows | 6.9; 42.9 | Shift-click grows the selection within a track or onto other tracks. Shift with arrow keys extends or shrinks it. |
| R45 | Select All in the arrangement | verified | Ctrl+A / Cmd+A | 42.5 Editing | Select All selects all the material. |
| R46 | Cut / Copy / Paste / Duplicate / Delete Time | verified | Ctrl+Shift+X / C / V / D / Delete; macOS Cmd+Shift+same | 6.11 Using the …Time Commands, `…/12/arrangement-view/#using-the-time-commands`; 42.16 | The "…Time" commands act on every track and on time-signature markers. Cut and Delete Time close the gap. Paste and Duplicate Time push later material to the right. |
| R47 | Insert Silence | verified | Ctrl+I / Cmd+I | 6.11; 42.16 | Insert Silence inserts the chosen amount of empty time at the insert marker across all tracks. |
| R48 | Alt/Option-drag or Ctrl-drag copies a clip in the arrangement | unverified (Arrangement); verified for Session and notes | Session clips and notes: Ctrl+drag / Option+drag | 42.15 Session View; 10.5.4 Moving Notes, `…/12/editing-midi/#moving-notes`; 6.7 Moving and Resizing Clips (no copy modifier given) | The manual gives Ctrl-drag (Win) / Option-drag (Mac) for copying Session clips and MIDI notes. The Arrangement chapter gives no copy-drag modifier for clips. Note that the Windows modifier is Ctrl, not Alt. |
| R49 | Holding a modifier bypasses grid snap | verified | Alt / Cmd (hold while dragging) | 6.10 Using the Editing Grid, `…/12/arrangement-view/#using-the-editing-grid`; 42.5; 42.13 | Holding Alt on Windows or Cmd on macOS suspends snapping during a drag. If the grid is off, the same key snaps temporarily. |
| R50 | Arrow keys move the insert marker | verified | Left/Right; Up/Down; Ctrl+Left/Right (Win) / Option+Left/Right (Mac) | 6.9 Selecting Clips and Time | Left/Right move the insert marker in time and Up/Down move it to other tracks. Ctrl (Win) / Option (Mac) with Left/Right jumps to locators and clip edges. |
| R51 | Arrow keys nudge selected clips | verified | Left/Right | 42.16 Arrangement View | With clips selected, Left/Right nudges them by the grid. |
| R52 | Arrow keys nudge selected notes | verified | Left/Right; Alt+arrows (Win) / Cmd+arrows (Mac) to bypass grid | 10.5.4 Moving Notes, `…/12/editing-midi/#moving-notes` | Left/Right moves selected notes by grid steps. The snap-bypass modifier makes the move unsnapped. |
| R53 | Moving clips by dragging, including to another track | verified | mouse | 6.7 Moving and Resizing Clips, `…/12/arrangement-view/#moving-and-resizing-clips` | You drag a clip by its title bar to move it in time or onto another track. It snaps to the grid, clip edges, locators and meter changes. |
| R54 | Bypass snap while moving clips with a three-key modifier **(added)** | verified | Ctrl+Alt+Shift+drag / Cmd+Option+Shift+drag | 6.7 Moving and Resizing Clips | The Arrangement chapter also gives a three-key drag modifier that ignores snapping for clips. |
| R55 | Resize a clip edge with the keyboard **(added)** | verified | Enter, then Left/Right | 42.16 Arrangement View | When the insert marker sits on a clip edge, Enter then the arrow keys move that edge. |

## 9. Grid

| ID | Behaviour | Verdict | Windows / macOS keys | Manual citation (chapter § section, URL) | Paraphrase |
|---|---|---|---|---|---|
| R56 | Narrow / widen the grid | verified | Ctrl+1 / Ctrl+2; Cmd+1 / Cmd+2 | 6.10 Using the Editing Grid; 42.13 | Ctrl/Cmd+1 makes grid lines denser and Ctrl/Cmd+2 makes them sparser. |
| R57 | Triplet grid | verified | Ctrl+3 / Cmd+3 | 6.10; 42.13 | This toggles triplet spacing. |
| R58 | Snap on/off | verified | Ctrl+4 / Cmd+4 | 6.10; 42.13 | This toggles snapping to the grid. |
| R59 | Fixed vs zoom-adaptive grid | verified | Ctrl+5 / Cmd+5 | 6.10; 42.13 | This switches between a fixed grid size and one that follows the zoom level. |

## 10. MIDI note editor

| ID | Behaviour | Verdict | Windows / macOS keys | Manual citation (chapter § section, URL) | Paraphrase |
|---|---|---|---|---|---|
| R60 | Up/Down transposes selected notes by a semitone | verified | Up / Down | 10.5.2 Selecting Notes and Timespan, `…/12/editing-midi/#selecting-notes-and-timespan` | With notes selected, the vertical arrows shift them one semitone. |
| R61 | Shift+Up/Down transposes by an octave | verified | Shift+Up / Shift+Down | 10.5.9 Pitch and Time Utilities, `…/12/editing-midi/#pitch-and-time-utilities`; 42.6 | Adding Shift moves the notes by an octave. |
| R62 | Arrow-key transposition follows the clip scale | unverified | n/a | 10.5.9.1 Transpose; 10.6 Folding and Scales | Only the Transpose slider is documented as moving by scale degrees when a clip scale is active. The manual does not say the arrow keys do this. |
| R63 | Shift+Left/Right changes note length | verified | Shift+Left / Shift+Right (add Alt / Cmd to skip snapping) | 10.5.5 Changing Note Length, `…/12/editing-midi/#changing-note-length` | Shift with the horizontal arrows lengthens or shortens selected notes by a grid step. |
| R64 | Quantize | verified | Ctrl+U / Cmd+U | 10.5.11 Quantizing Notes, `…/12/editing-midi/#quantizing-notes`; 42.12 | Quantize applies the current quantize settings to the selection. |
| R65 | Quantize settings | verified | Ctrl+Shift+U / Cmd+Shift+U | 10.5.11; 42.12 | This opens the quantize options. |
| R66 | Velocity lane editing | verified | Ctrl+Up/Down (Win) / Cmd+Up/Down (Mac) by 10; type 0–127 + Enter | 10.5.12 Editing Velocities, `…/12/editing-midi/#editing-velocities`; 10.1 layout; 42.12 | Velocities are dragged as stems in a lane below the notes, which is shown by default and chosen with a lane selector. Selected velocities can also be typed or stepped with keys. |
| R67 | Modifier-drag on a note changes its velocity | verified | Alt+drag / Cmd+drag | 10.5.12; 42.12 | Dragging a selected note vertically while holding Alt (Win) or Cmd (Mac) changes its velocity without the lane. |
| R68 | Fold (button / F key) | verified | F (editor focused) / F | 10.6 Folding and Scales, `…/12/editing-midi/#folding-and-scales` | Fold hides pitch rows that contain no notes. It is available from a Clip View button, the F key, or the View menu. |
| R69 | Fold to Scale **(added)** | verified | G / G | 10.6 Folding and Scales | With a clip scale active, G hides rows outside the scale. Off-scale rows that already hold notes stay visible. |
| R70 | Live 12 clip scale (Scale Mode, root, scale name) | verified | toggle in clip panel or Control Bar | 8.2.4 Clip Scale, `…/12/clip-view/#clip-scale`; 10.6 | Each clip can have Scale Mode on, with a chosen root and scale name. The choosers sit in the main clip properties panel, and the Control Bar also has a toggle. |
| R71 | Highlight scale rows | verified | K / K | 10.6; 42.12 | K shades in-scale rows and piano keys, with the root emphasized. The setting is global, not per clip. |
| R72 | Copy notes by dragging | verified | Ctrl+drag / Option+drag | 10.5.4 Moving Notes; 42.12 | Holding Ctrl (Win) / Option (Mac) while dragging copies the notes, and the key can be pressed after the drag starts. |
| R73 | Chop / split / join notes **(added)** | verified | Ctrl+E (chop/split) / Cmd+E; Ctrl+J (join) / Cmd+J; hold E and click to split | 10.5.8, `…/12/editing-midi/`; 42.12 | In the note editor, Ctrl/Cmd+E chops notes on the grid or at the time selection, Ctrl/Cmd+J joins them, and holding E lets you split a note where you click. |
| R74 | Select next/previous note by keyboard **(added)** | verified | Alt+Up/Down, Alt+Left/Right / Option+same | 42.12 Clip View MIDI Note Editor | Alt/Option with arrows steps the selection to neighbouring notes. |
| R75 | Note editor zoom keys | verified | Z, X, +/- | 10.2 Zooming and Navigating; 42.12 | In the note editor, Z zooms to the selection, X goes back to the whole clip, and +/- zoom horizontally. |
| R76 | Marquee and Shift-click note selection | verified | drag; Shift+click; Esc deselects | 10.5.2 Selecting Notes and Timespan | A drag selects notes and a time range. Shift-click toggles individual notes. Shift-click on a piano key selects that whole row. |

## 11. Keyboard navigation and views

| ID | Behaviour | Verdict | Windows / macOS keys | Manual citation (chapter § section, URL) | Paraphrase |
|---|---|---|---|---|---|
| R77 | Tab switches Session/Arrangement | verified | Tab / Tab | 42.1 Showing and Hiding Views; 3 Live Concepts (View Selector) | Tab flips between the two main views unless the "use Tab to navigate" option is on. |
| R78 | Shift+Tab toggles Clip/Device view | verified | Shift+Tab or F12 / same | 42.1 Showing and Hiding Views, `…/12/live-keyboard-shortcuts/#showing-and-hiding-views` | Shift+Tab (or F12) switches the lower panel between the clip and the device chain. |
| R79 | Ctrl/Cmd+Alt/Option+L toggles the Detail View | unverified | not listed | 42.1 (not present) | The Live 12 shortcut list has no Detail View toggle on this key. It lists separate show/hide keys for Clip View (Ctrl+Alt+3 / Cmd+Option+3) and Device View (Ctrl+Alt+4 / Cmd+Option+4). |
| R80 | Show/hide browser | verified | Ctrl+Alt+B or Ctrl+Alt+5 / Cmd+Option+B or Cmd+Option+5 | 42.1; 42.22 Browser | Either key pair shows or hides the browser. |
| R81 | Focus a specific view by keyboard **(added)** | verified | Alt+0…8 / Option+0…8 | 42.2 Keyboard Focus and Navigation | Alt/Option with a digit moves keyboard focus to the Control Bar, Session, Arrangement, Clip, Device, Browser and other panels. |
| R82 | Up/Down selects tracks | verified (as insert-marker movement) | Up / Down | 6.9 Selecting Clips and Time | In the arrangement the vertical arrows move the insert marker, and so the focus, to the track above or below. |
| R83 | Open Clip View for a clip | verified | double-click clip; Ctrl+Alt+3 / Cmd+Option+3 | 8 Clip View intro, `…/12/clip-view/` | Double-clicking a clip in either view opens it in Clip View. The view selector and a show/hide key also work. |

## 12. Tracks

| ID | Behaviour | Verdict | Windows / macOS keys | Manual citation (chapter § section, URL) | Paraphrase |
|---|---|---|---|---|---|
| R84 | Insert MIDI track | verified | Ctrl+Shift+T / Cmd+Shift+T | 18.2 Audio and MIDI Tracks, `…/12/mixing/#audio-and-midi-tracks`; 42.19 | The Create menu or this shortcut adds a MIDI track. |
| R85 | Insert audio track | verified | Ctrl+T / Cmd+T | 18.2; 42.19 | The Create menu or this shortcut adds an audio track. |
| R86 | Delete track | verified | Delete (track title bar selected) / same | 18.2; 42.19 | Select the track and use Edit > Delete or the Delete key. |
| R87 | Duplicate track | unverified | (generic Ctrl+D / Cmd+D) | 42.5 lists only a generic Duplicate | No page read here describes duplicating a whole track. Only the general Duplicate command is documented. |
| R88 | Reorder tracks by dragging the header | verified | mouse; Ctrl+arrows / Cmd+arrows for non-adjacent tracks | 6.1 Layout, `…/12/arrangement-view/#layout`; 18.2; 42.19 | Drag a track by its title bar to move it. Non-adjacent selections collapse together on drop unless moved with Ctrl/Cmd plus the arrow keys. |
| R89 | Track colour chooser (context menu) | unverified | n/a | 2.3.2 Theme & Colors, `…/12/first-steps/#theme--colors`; 18.3 | The pages read here describe automatic or default track colouring in Settings and a Group Track command to push its colour down. They do not describe a per-track palette. |
| R90 | Clip colour chooser | verified | right-click | 8.1.1 Clip Title Bar; 7.1 Session View | A clip's colour is picked from a palette in its context menu or the Clip View title bar menu. A command can also apply the track colour to its clips. |
| R91 | Group tracks | verified | Ctrl+G / Cmd+G; ungroup Ctrl+Shift+G / Cmd+Shift+G | 18.3 Group Tracks, `…/12/mixing/`; 42.19 | Selected tracks can be wrapped in a Group Track, and groups can nest. |
| R92 | Fold/unfold a group or track | verified | U (or Left/Right); Alt+U / Option+U unfolds all | 6.1; 18.3; 42.16; 42.19 | U or the fold button collapses or expands the selected track or group. Alt/Option unfolds everything. |

## 13. Clips

| ID | Behaviour | Verdict | Windows / macOS keys | Manual citation (chapter § section, URL) | Paraphrase |
|---|---|---|---|---|---|
| R93 | Insert empty MIDI clip in the Arrangement | verified | Ctrl+Shift+M / Cmd+Shift+M | 10.3 Creating a MIDI Clip, `…/12/editing-midi/` | Select a span on a MIDI track and use Create > Insert Empty MIDI Clip(s) or the shortcut. Double-clicking a MIDI track's lane also makes a clip. |
| R94 | Ctrl/Cmd+L toggles the loop / Loop Selection | verified | Ctrl+L / Cmd+L | 6.6 The Arrangement Loop, `…/12/arrangement-view/#the-arrangement-loop`; 42.16 | In the Arrangement, Ctrl/Cmd+L turns the loop on and fits the loop brace to the current time selection. |
| R95 | Clip loop switch in Clip View | verified | Loop toggle in clip panel | 8.2.1 Clip and Loop Region Settings, `…/12/clip-view/#clip-and-loop-region-settings` | Each clip has its own Loop toggle and loop position/length fields in its properties panel. |
| R96 | Resize clips by dragging edges | verified | mouse | 6.7 Moving and Resizing Clips | Dragging a clip's left or right edge shortens or lengthens it. |
| R97 | Deactivate with 0 | verified | 0 / 0 | 6.9; 8.1.1.1; 42.16 | 0 toggles the selected clips, or the selected track if its header is selected, on or off. |

## 14. Loop brace, ruler, punch

| ID | Behaviour | Verdict | Windows / macOS keys | Manual citation (chapter § section, URL) | Paraphrase |
|---|---|---|---|---|---|
| R98 | Clicking the loop brace | verified | mouse | 6.6 The Arrangement Loop | A click on the brace selects everything inside it. |
| R99 | Dragging the loop brace | verified | mouse | 6.6 | Dragging an end changes the loop start or end. Dragging the bar moves the whole loop and keeps its length. |
| R100 | Double-clicking the loop brace | unverified | n/a | 6.6 | The manual does not describe a double-click action on the brace. |
| R101 | Keyboard loop editing **(added)** | verified | arrows nudge; Up/Down move by length; Ctrl+arrows / Cmd+arrows resize; Ctrl+Shift+L / Cmd+Shift+L selects contents | 6.2; 42.8 Loop Brace and Start/End Markers | With the brace selected, arrows nudge it by the grid or by its own length. Ctrl/Cmd plus arrows halves, doubles, shortens or lengthens it. |
| R102 | Punch in / punch out | verified | Control Bar switches | 19.3.1 Recording Into the Arrangement, `…/12/recording-new-clips/` | Punch-In and Punch-Out switches stop recording outside the loop range, so the loop brace defines the punch region. |
| R103 | Click in the scrub area plays from there **(added)** | verified | mouse | 6.3 Transport and Playback, `…/12/arrangement-view/#transport-and-playback` | A click in the scrub strip starts playback at that point. Holding the button loops a short span. |

## 15. Automation

| ID | Behaviour | Verdict | Windows / macOS keys | Manual citation (chapter § section, URL) | Paraphrase |
|---|---|---|---|---|---|
| R104 | Automation Mode toggle | verified | A / A (hold for momentary) | 26.5.2 Viewing Automation in Arrangement View, `…/12/automation/#viewing-automation-in-arrangement-view`; 42.7 | A or the button above the track headers shows every track's automation lane. |
| R105 | Show/hide automation lanes per track | verified | mouse; Alt / Cmd click adds all automated lanes | 26.5.2; 26.5.2.1 Context Menu Options for Automation Lanes | Lanes are added or removed with per-lane buttons. A parameter's context menu can show its envelope, optionally in a new lane. Choosing None hides the envelope. |
| R106 | Click on an envelope adds a breakpoint | verified | mouse (Alt / Cmd skips snap) | 26.6.1 Adding Automation Envelope Breakpoints, `…/12/automation/#adding-automation-envelope-breakpoints` | A click on the envelope line adds a point. A double-click away from the line also adds one. |
| R107 | Drag breakpoints | verified | mouse; Shift = horizontal only; Shift = fine | 26.7.1 Editing Breakpoints and Segments, `…/12/automation/#editing-breakpoints-and-segments`; 42.7 | Breakpoints are dragged to move them. Modifiers restrict the movement or bypass the grid. |
| R108 | Delete a breakpoint by double-click | contradicted | single click | 26.7.1 Editing Breakpoints and Segments | The manual says a single click on an existing breakpoint removes it and joins its neighbours. Double-click is for adding a point off the line. Selections can also be removed with Delete. |
| R109 | Curved segments | verified | Alt+drag / Option+drag | 26.7.1; 42.7 | Alt/Option-dragging a sloped segment bends it. An Alt/Option double-click straightens it again. |
| R110 | Clip envelopes live with the clip | verified (partial) | Envelopes tab; Alt+Shift+2 / Option+Shift+2 | 27 Clip Envelopes, `…/12/clip-envelopes/`; 42.10 | Every clip can have its own envelopes, edited like automation in the Clip View Envelopes tab. Arrangement clip envelopes modulate relative to the track value. The manual does not state outright that they move when the clip is dragged. |
| R111 | Touching an automated control overrides it; Re-Enable Automation | verified | Control Bar button | 26.4 Overriding Automation, `…/12/automation/` | Changing an automated control by hand turns its automation LED grey and lights the Re-Enable Automation button, which restores all overridden automation. A single parameter can also be restored from its context menu. |
| R112 | Lock envelopes | verified | toggle; Options menu | 26.9.2 Locking Envelopes in Automation Lanes | With Lock Envelopes on, automation stays at its song position when clips are moved. With it off, automation follows the clip. |
| R113 | Tempo automation on the main track | verified | n/a | 26.9.1 Automating the Tempo | Tempo is automated in the Main track's lane under Mixer > Song Tempo, with a range setting for the shown BPM span. |
| R114 | Delete automation | verified | Ctrl+Backspace / Cmd+Delete (manual text); 42.7 lists Ctrl+Delete / Cmd+Delete | 26.8 Deleting Automation; 42.7 Commands for Breakpoint Envelopes | A parameter's automation is removed from its context menu or with this key. Envelope menus also offer clearing one or all envelopes. |
| R115 | Simplify Envelope **(added)** | verified | context menu | 26.7.3 Simplifying Envelopes | With a time range selected on an envelope, Simplify removes redundant breakpoints. |

## 16. Time signatures and tempo

| ID | Behaviour | Verdict | Windows / macOS keys | Manual citation (chapter § section, URL) | Paraphrase |
|---|---|---|---|---|---|
| R116 | Project time signature in the Control Bar | verified | n/a | 3.1 Control Bar; 6.5 Time Signature Changes, `…/12/arrangement-view/#time-signature-changes` | Numerator and denominator fields sit next to the tempo. Editing them changes the meter at the current play position. |
| R117 | Insert a time-signature change marker | verified | context menu on scrub area or Create menu | 6.5 Time Signature Changes | A meter marker is placed at the insert marker through the scrub area menu or the Create menu. Denominators are limited to 1, 2, 4, 8 or 16. |
| R118 | Edit / move / delete meter markers | verified | Ctrl+R / Cmd+R edit; drag or arrows to move; Delete | 6.5 | Markers are edited from the menu or the rename key, moved by dragging or arrows, and removed with Delete. They are not quantized, so an off-bar marker leaves a fragmentary bar that can be fixed from its menu. |
| R119 | Meter changes affect the grid / ruler | verified (partial) | n/a | 6.7; 6.11; 6.5 | Clips snap to meter markers, the time commands move meter markers too, and fragmentary bars show in the ruler. The manual does not say in more detail how the grid redraws. |

## 17. Files and app shell

| ID | Behaviour | Verdict | Windows / macOS keys | Manual citation (chapter § section, URL) | Paraphrase |
|---|---|---|---|---|---|
| R120 | New Live Set | verified | Ctrl+N / Cmd+N | 5.4.1 Creating, Opening and Saving Sets, `…/12/managing-files-and-sets/#creating-opening-and-saving-sets`; 42.3 | The File menu creates a new Set. |
| R121 | Open / Open Recent | verified | Ctrl+O / Cmd+O | 5.4.1; 42.3 | The File menu opens a Set from disk or from a recent-sets list. Sets also open from the browser. |
| R122 | Save / Save As / Save a Copy | verified | Ctrl+S, Ctrl+Shift+S / Cmd+S, Cmd+Shift+S | 5.4.1; 42.3 | Save keeps the current file, Save As picks a new name or location, and Save a Copy writes a copy. |
| R123 | Collect All and Save | verified | File menu | 5.7 Collecting External Files, `…/12/managing-files-and-sets/#collecting-external-files` | This File menu command copies outside files into the project folder, with a dialog to pick which locations to collect from. |
| R124 | Export Audio/Video | verified | Ctrl+Shift+R / Cmd+Shift+R | 5.1.3 Exporting Audio and Video, `…/12/managing-files-and-sets/#exporting-audio-and-video`; 42.3 | The dialog sets which output to render (main, all tracks, selected, or one track) and the start and length, which default to the whole Set or the time selection. It also covers return/main effects, render-as-loop, mono, normalize, sample rate, WAV/AIFF/FLAC at 16/24/32 bit, dither choices, and optional MP3. |
| R125 | Export MIDI clip | verified | Ctrl+Shift+E / Cmd+Shift+E | 5.2.1 Exporting MIDI Files, `…/12/managing-files-and-sets/#exporting-midi-files`; 42.3 | A File menu command saves the selected MIDI clip as a Standard MIDI File. |
| R126 | Import MIDI files (Create menu or browser) | verified | Create > Import MIDI File… | 5.2 MIDI Files, `…/12/managing-files-and-sets/#midi-files` | In the Arrangement, an imported MIDI file lands at the insert marker. The data is copied into the Set and loses its link to the file. |
| R127 | Drag files from Explorer/Finder or the browser | verified | mouse | 4.10 Adding Content from the Browser to a Live Set, `…/12/working-with-the-browser/#adding-content-from-the-browser-to-a-live-set` | Files can be dropped from the OS or the browser. Dropping below the last arrangement track creates a new track. |
| R128 | Multi-track MIDI file creates one track per part | unverified | n/a | 5.2; 4.10 | The manual does not say how a MIDI file with several tracks is split on import. |
| R129 | Settings window | verified | Ctrl+, / Cmd+, | 2.3 Live's Settings, `…/12/first-steps/#lives-settings`; 42.1 | Settings opens from the Options menu (Win) or the Live menu (Mac). Its tabs are Display & Input, Theme & Colors, Audio, Link, Tempo & MIDI, File & Folder, Library, Plug-Ins, Record/Warp/Launch, and Licenses & Updates. Ctrl+Tab cycles the tabs (42.26). |
| R130 | Help View / lessons for new users | verified (renamed Learn View) | Ctrl+Alt+7 / Cmd+Option+7 | 2.2.1 Learn View, `…/12/first-steps/#learn-view`; 42.1 | Live 12 calls its built-in tutorials Learn View. These are online video-plus-text modules that are reopened from the Help menu or the shortcut. |
| R131 | Info View hover help **(added)** | verified | ? / ? (Shift+?) | 2.2.2 Info View, `…/12/first-steps/#info-view`; 42.1 | A corner panel describes whatever control is under the mouse. Users can attach their own notes with Edit Info Text. |
| R132 | Help menu contents, keyboard-shortcut reference, About box | unverified | n/a | (no section found) | The pages read do not list the Help menu items, an in-app shortcut sheet, or the About dialog. |
| R133 | Default Set / templates | verified | File menu | 5.4.5 Template Sets, `…/12/managing-files-and-sets/#template-sets` | Any Set can be saved as the default that new Sets start from, carrying I/O setup, preset devices and mappings. Separate templates appear in the browser and open untitled. |
| R134 | What a fresh new Set contains | unverified | n/a | 5.4.5 | The manual does not list the tracks of the factory default Set. |

## 18–24. Transport, mixer, misc

| ID | Behaviour | Verdict | Windows / macOS keys | Manual citation (chapter § section, URL) | Paraphrase |
|---|---|---|---|---|---|
| R135 | Follow toggled with Ctrl/Cmd+F | contradicted | manual: Ctrl+Shift+F / Option+Shift+F (42.16; 42.9 prints Alt+Shift+F for Windows) | 42.9; 42.16; 6.2 Navigation and Zooming; 42.22 | Follow is a Control Bar switch and Options menu item with Ctrl+Shift+F (Win) / Option+Shift+F (Mac). Ctrl/Cmd+F is the browser search key. Follow pauses after edits or manual scrolling and resumes on play or stop. |
| R136 | Space play/stop | verified | Space / Space | 42.20 Transport | Space starts playback from the start marker, or plays the arrangement selection, and stops. |
| R137 | Shift+Space continues from the stop point | verified | Shift+Space / same | 42.20; 6.3 | Shift+Space resumes where playback last stopped. |
| R138 | Home returns to the start | verified | Home / Home or Fn+Left | 42.20; 6.9 | Home puts the insert marker at the beginning. Double-clicking Stop does the same. |
| R139 | Back to Arrangement | verified | F10 / F10 | 3.7 Tracks (Live Concepts); 42.20 | The button lights when any track plays Session material instead of the arrangement, and clicking it (or F10) hands all tracks back. |
| R140 | Record / metronome **(added)** | verified | F9 record; O metronome | 42.20 Transport | F9 records and O toggles the metronome. |
| R141 | Undo includes mixer / mute / solo changes | unverified | n/a | 5.4.2 Accessing a Set's Undo History | The manual does not say which kinds of action (such as mixer, mute or solo) go into undo history. |
| R142 | Track Activator (mute) and Solo buttons | verified | S solos selected tracks; F1–F8 toggle activators for tracks 1–8 | 18.1 The Live Mixer, `…/12/mixing/#the-live-mixer`; 42.19; 42.20 | Each track has an activator that silences it and a Solo switch that silences the others. |
| R143 | Exclusive solo; Ctrl/Cmd-click to solo several | verified | Ctrl+click / Cmd+click | 18.1 The Live Mixer | By default only one track is soloed at a time. Holding Ctrl/Cmd, or turning off Exclusive Solo in Settings, adds tracks instead. Exclusive Arm works the same way. |
| R144 | Toggling a button on a multi-selection affects all selected tracks | verified | Shift / Ctrl / Cmd-click to select | 18.1; 18.2 | With several tracks selected, flipping one track's activator, solo or arm flips them all. |
| R145 | Track height resizing | verified | drag split line; Alt+plus/minus / Option+plus/minus; Alt+drag = all tracks | 6.1; 6.9; 42.16 | Track height is changed by dragging below the fold button or with Alt/Option plus +/-. Holding Alt/Option while dragging resizes every track. |
| R146 | Track header width resizing in the Arrangement | unverified | n/a | 6.1 | The Arrangement chapter covers height only. Width dragging is documented for Session View tracks (18.2). |
| R147 | Show/hide clip content preview | unverified | n/a | ch. 6 | The manual does not describe hiding clip previews in the arrangement. |
| R148 | Freeze track (note only) | verified | Ctrl+Alt+Shift+F / Cmd+Option+Shift+F | 42.19 Commands for Tracks | Freeze is a track command with this shortcut. It is out of scope here. |
| R149 | CPU meter in the Control Bar | verified | right-click for options | 38.1.1 The CPU Load Meter, `…/12/computer-audio-resources-and-strategies/#the-cpu-load-meter` | The meter shows audio-processing time as a share of each buffer's playback time, so it can exceed 100%. It ignores UI drawing, shows the average by default, and can be switched to current (peak) load. A separate indicator can warn of overloads. |
| R150 | Double-click a clip opens it in Clip View; name and colour edited there | verified | double-click | 8 Clip View; 8.1.1 Clip Title Bar | Double-click opens the clip's editor below. The name and colour are changed from the clip title bar menu or the clip's own context menu. |
| R151 | Per-track CPU indicator **(added)** | verified | n/a | 18.9 Performance Impact Track Indicators, `…/12/mixing/#performance-impact-track-indicators` | Each track can show a small segmented meter of its relative CPU cost. |
