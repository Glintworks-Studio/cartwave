# Ableton Live reference behaviours, polish pass 2 (manual check)

- **Source:** Ableton Live 12 Reference Manual, canonical pages under `https://www.ableton.com/en/live-manual/12/`. No other sources were used.
- **Checked:** 2026-11-26.
- **All text below is paraphrased.** Nothing is quoted from the manual. Command and menu names (for example "Optimize Arrangement Height") are given only so the reader can find them.
- **Verdicts:** *verified* means the manual states the behaviour. *contradicted* means the manual states something different, and the row says what. *unverified* means the pages read here do not state it. That does not prove Live lacks the behaviour.
- **Keys:** shown as Windows / macOS. "same" means the macOS key is identical. Chapter 42 is "Live Keyboard Shortcuts". Rows marked **(added)** are related behaviours found while checking.
- **Pages read:** ch. 2 First Steps, 3 Live Concepts, 4 Working with the Browser, 6 Arrangement View, 18 Mixing, 38 Computer Audio Resources and Strategies, and 42 Live Keyboard Shortcuts.
- **Caveat:** the pages were read through a fetch tool that summarises them; the longest pages were read up to about 100,000 characters, which covers every section cited here (the tail of each page is later chapters' material or the end of the shortcut tables, 42.26 onward).

URL shorthand used below: `…/12/` = `https://www.ableton.com/en/live-manual/12/`.

## 1. Arrangement Overview

| ID | Behaviour | Verdict | Windows / macOS keys | Manual citation (chapter § section, URL) | Paraphrase |
|---|---|---|---|---|---|
| P2-R1 | The Overview is a zoomed-out view of the whole arrangement above the main display | verified | n/a | 6.1 Layout, item 1 "Overview", `…/12/arrangement-view/#layout` | The Overview shows the arrangement's whole clip layout from start to end and is used for navigating and zooming. The layout figure puts it at the top of the Arrangement View. |
| P2-R2 | The visible region is outlined in the Overview | verified | mouse | 6.1 Layout, item 1, `…/12/arrangement-view/#layout` | A black rectangle marks the part of the arrangement that the main display currently shows. |
| P2-R3 | Horizontal drag in the Overview scrolls, vertical drag zooms | verified | mouse | 6.1 Layout, item 1, `…/12/arrangement-view/#layout` | Dragging sideways in the Overview scrolls the arrangement; dragging up or down zooms it. |
| P2-R4 | **(added)** Double-click in the Overview outline shows the whole arrangement | verified | mouse | 6.1 Layout, item 1, `…/12/arrangement-view/#layout` | A double-click inside the outlined region zooms out so the full arrangement fits. |
| P2-R5 | Single click in the Overview jumps the view to that spot | unverified | mouse | 6.1 Layout, item 1, `…/12/arrangement-view/#layout` | The pages read describe drag and double-click on the Overview, but not a single click that recentres the view. |
| P2-R6 | Show/hide the Overview | verified | Ctrl+Alt+O / Cmd+Option+O | 42.1 Showing and Hiding Views, `…/12/live-keyboard-shortcuts/#showing-and-hiding-views` | Hide/Show Overview has its own shortcut. Chapter 6 itself does not mention hiding the Overview, and the View menu item name was not stated on the pages read (it is presumably the same "Overview" command, unverified). |

## 2. Beat-time ruler and scrub area

| ID | Behaviour | Verdict | Windows / macOS keys | Manual citation (chapter § section, URL) | Paraphrase |
|---|---|---|---|---|---|
| P2-R7 | Vertical drag in the beat-time ruler zooms; horizontal drag scrolls (confirms R36) | verified | mouse | 6.1 Layout, item 2 "Beat-time ruler", `…/12/arrangement-view/#layout`; 6.2 Navigation and Zooming, `…/12/arrangement-view/#navigation-and-zooming` | The ruler shows bars-beats-sixteenths; dragging left/right scrolls the view and dragging up/down zooms it. |
| P2-R8 | Double-click the beat-time ruler zooms to the selection, or to the whole arrangement if nothing is selected (confirms R32) | verified | mouse | 6.1 Layout, item 2, `…/12/arrangement-view/#layout` | A double-click on the ruler fits the current selection; with no selection it zooms out to the entire arrangement. |
| P2-R9 | Click in the scrub area starts playback from that point | verified | mouse | 6.1 Layout, item 3 "Scrub area", `…/12/arrangement-view/#layout`; 6.3 Transport and Playback, `…/12/arrangement-view/#transport-and-playback` | Clicking the scrub area starts playback there (with Permanent Scrub Areas on, the default); holding the mouse button down loops a region sized by the global launch quantization. |
| P2-R10 | **(added)** Shift-click scrubs in the scrub area or the beat-time ruler | verified | Shift+click / same | 6.3 Transport and Playback, `…/12/arrangement-view/#transport-and-playback`; 2.3.1 Display & Input, `…/12/first-steps/#display--input` | Shift-clicking either the scrub area or the beat-time ruler scrubs even when Permanent Scrub Areas is turned off in Settings. |
| P2-R11 | A plain click in the beat-time ruler sets a position or starts playback | unverified | mouse | 6.1, 6.2, 6.3, `…/12/arrangement-view/#navigation-and-zooming` | The pages read do not say a plain click in the beat-time ruler sets the play position. The only effect stated is that clicking the ruler pauses Follow (next row). Playback-from-click is described for the scrub area only. |
| P2-R12 | **(added)** Clicking the beat-time ruler pauses Follow | verified | mouse | 6.2 Navigation and Zooming, `…/12/arrangement-view/#navigation-and-zooming` | Follow pauses after an edit, a horizontal scroll, or a click in the beat-time ruler, and resumes when playback stops/restarts or on a click in the Arrangement or clip scrub area. |

## 3. Zoom keys and fit commands

| ID | Behaviour | Verdict | Windows / macOS keys | Manual citation (chapter § section, URL) | Paraphrase |
|---|---|---|---|---|---|
| P2-R13 | W optimizes arrangement width | verified | W / W | 6.1 Layout, item 13 "Optimize Height and Optimize Width", `…/12/arrangement-view/#layout`; 42.16 Arrangement View, `…/12/live-keyboard-shortcuts/#arrangement-view` | The Optimize Width toggle (W) fits the arrangement into the current width of the Arrangement View. |
| P2-R14 | H optimizes arrangement height | verified | H / H | 6.1 Layout, item 13, `…/12/arrangement-view/#layout`; 42.16 Arrangement View | The Optimize Height toggle (H) fits all tracks into the current height of the Arrangement View. Both are toggles, not one-shot zooms. |
| P2-R15 | + / − zoom the time axis | verified | + and − / same | 6.2 Navigation and Zooming, `…/12/arrangement-view/#navigation-and-zooming`; 42.9 Zooming, Display and Selections, `…/12/live-keyboard-shortcuts/#zooming-display-and-selections` | The plus and minus keys zoom in and out step by step around the current selection. Ctrl/Cmd plus mouse wheel or trackpad also zooms. |
| P2-R16 | **(added)** Z zooms to the time selection, X steps back | verified | Z / Z; X / X | 6.2 Navigation and Zooming; 42.16 Arrangement View | Z (also View > Zoom to Arrangement Time Selection) fills the view with the selected time; X returns to the previous zoom and can be repeated. |
| P2-R17 | **(added)** Ctrl+ / Ctrl− zoom the whole window (interface scale), not the timeline | verified | Ctrl+ and Ctrl− / Cmd+ and Cmd− | 42.9 Zooming, Display and Selections | The modified plus/minus keys are listed as window zoom, separate from the time-ruler zoom on plain + and −. |
| P2-R18 | A separate "zoom to fit / show all" command beyond W, Z and the ruler double-click | unverified | n/a | 6.2; 42.9; 42.16 | No other show-all command was found; whole-arrangement fit is reached by W, by double-clicking the ruler with no selection, or by double-clicking inside the Overview outline. |

## 4. Track height in the Arrangement

| ID | Behaviour | Verdict | Windows / macOS keys | Manual citation (chapter § section, URL) | Paraphrase |
|---|---|---|---|---|---|
| P2-R19 | Drag a track's edge (the split line below it) to change its height | verified | mouse | 6.1 Layout (unfolded tracks), `…/12/arrangement-view/#layout`; 18.2 Audio and MIDI Tracks, `…/12/mixing/#audio-and-midi-tracks` | Track edges can be dragged to change height in the Arrangement (width in Session). For an unfolded track, the split line under the unfold button is what you drag. |
| P2-R20 | **(added)** Alt/Option while resizing one track resizes all tracks | verified | Alt+drag / Option+drag | 6.1 Layout, `…/12/arrangement-view/#layout` | Holding Alt/Option during the height drag applies the new height to every track in the arrangement. |
| P2-R21 | Alt/Option + mouse wheel or trackpad over a track changes its height | verified | Alt+wheel / Option+wheel (also Alt/Option+pinch) | 6.2 Navigation and Zooming, `…/12/arrangement-view/#navigation-and-zooming` | Alt/Option plus scrolling in a track's main lane zooms that track vertically; if there is a time selection, every track with selected content zooms. Alt/Option plus pinch on a trackpad or touchscreen also resizes. Chapter 42 does not list the wheel gesture. |
| P2-R22 | **(added)** Keyboard height adjust for selected tracks | verified | Alt+ and Alt− / Option+ and Option− | 42.16 Arrangement View, `…/12/live-keyboard-shortcuts/#arrangement-view` | The shortcut changes the height of the selected tracks or clips, so several selected tracks change together by keyboard. |
| P2-R23 | Optimize Arrangement Height | verified | H / H | 6.1 item 13; 42.16 | See P2-R14. |
| P2-R24 | Dragging the edge of one of several selected tracks resizes all selected tracks | unverified | mouse | 6.1; 18.2 | The pages describe all-tracks (Alt/Option drag) and selected-tracks by keyboard (Alt+/−) or by wheel with a time selection, but not a mouse drag that resizes just the selected set. |

## 5. Track header / mixer section in the Arrangement

| ID | Behaviour | Verdict | Windows / macOS keys | Manual citation (chapter § section, URL) | Paraphrase |
|---|---|---|---|---|---|
| P2-R25 | Arrangement Track Controls sit beside the clips, with a View menu choice of which controls show | verified | View > Arrangement Track Controls submenu | 6.1 Layout, item 10, `…/12/arrangement-view/#layout`; 6.15 The Mixer in Arrangement View, `…/12/arrangement-view/#the-mixer-in-arrangement-view` | Each track has controls for volume, pan, I/O and so on; a View submenu picks which are shown. The 6.15 figure shows them on the right of the clips. |
| P2-R26 | Show/hide the Mixer in the Arrangement | verified | Ctrl+Alt+M / Cmd+Option+M | 6.15 The Mixer in Arrangement View; 6.1 item 16; 42.1 Showing and Hiding Views | View > Mixer, the shortcut, or the Mixer View toggle at the bottom-right corner opens the mixer; its values are shared with the Arrangement Track Controls. |
| P2-R27 | **(added)** Move keyboard focus to the Arrangement mixer | verified | Alt+Shift+M / Option+Shift+M | 42.16 Arrangement View | A separate shortcut moves focus to the mixer. |
| P2-R28 | **(added)** The mixer is made taller by dragging its top edge | verified | mouse | 18.1.1 Additional Mixer Features, `…/12/mixing/#additional-mixer-features` | Dragging the mixer's top edge upward enlarges it, adding meter tick marks, a numeric volume field and resettable peak indicators; a wider track adds a dB scale. |
| P2-R29 | A draggable boundary between the track header area and the clips (resizable header width) | unverified | mouse | 6.1; 6.15; 18.1.1 | No page read describes resizing the width of the track-controls area or dragging its edge. |
| P2-R30 | **(added)** Toggle In/Out, Sends, Return tracks | verified | Ctrl+Alt+I, Ctrl+Alt+S / Cmd+Option+I, Cmd+Option+S; Return Tracks Ctrl+Alt+R / Cmd+Option+R | 42.1 Showing and Hiding Views; 42.19 Commands for Tracks | Each mixer section has its own show/hide shortcut. |

## 6. Typing values into volume and pan

| ID | Behaviour | Verdict | Windows / macOS keys | Manual citation (chapter § section, URL) | Paraphrase |
|---|---|---|---|---|---|
| P2-R31 | Type a value into a focused control, Enter confirms, Esc cancels | verified (generic) | 0…9, Enter, Esc / same | 42.6 Adjusting Values, `…/12/live-keyboard-shortcuts/#adjusting-values` | Typing digits enters a value, Enter confirms and Esc cancels. This is the general value-entry rule; chapter 18 does not describe typing into volume/pan specifically. |
| P2-R32 | Click or double-click a volume/pan control to start typing | unverified | n/a | 18.1 The Live Mixer, `…/12/mixing/#the-live-mixer`; 42.6 | No page read says a click or double-click opens text entry on volume or pan. |
| P2-R33 | Delete returns a control to its default | verified (generic) | Delete / same | 42.6 Adjusting Values; 42.4 Working with Devices and Plug-Ins | Delete resets the focused value to default; for device parameters, Delete or a double-click resets. Backspace is not mentioned. |
| P2-R34 | Pan reset | verified | double-click; click triangle above pan | 18.1 The Live Mixer, item 3, `…/12/mixing/#the-live-mixer` | A double-click on Pan resets it; in the Session or Arrangement mixer the small triangle above the pan control also resets it. Pan has a Stereo mode (default) and a Split Stereo mode chosen from its context menu. |
| P2-R35 | Pan display format (50L … C … 50R) | unverified | n/a | 18.1 | The pages read do not state the pan display format. |
| P2-R36 | Volume range in dB (−inf to +6 dB) | unverified | n/a | 18.1; 18.1.1 | The pages read do not give the fader range; they only mention a numeric volume field when the mixer is enlarged. |
| P2-R37 | **(added)** Changing volume on one of several selected tracks changes them all | verified | mouse | 18.1 The Live Mixer, item 2 | With several tracks selected, adjusting one track's volume adjusts the others too. |

## 7. Status Bar, Info View, overload warnings

| ID | Behaviour | Verdict | Windows / macOS keys | Manual citation (chapter § section, URL) | Paraphrase |
|---|---|---|---|---|---|
| P2-R38 | The Status Bar shows messages | verified | n/a | 3.2 The Status Bar, `…/12/live-concepts/#the-status-bar` | It shows useful information such as error messages and notices about available updates; for a selected MIDI note it shows position, pitch, velocity and probability; hovering an insert marker shows its exact position. The section does not state where on screen the bar sits. |
| P2-R39 | Info View shows the name and function of the control under the mouse | verified | n/a | 2.2.2 Info View (in 2.2 Learning About Live), `…/12/first-steps/#info-view` | The Info View describes whatever interface element the pointer is over. Users can also add their own info text to tracks, clips and devices via a context-menu command, saved with the Set. |
| P2-R40 | Show/hide the Info View | verified | Shift+? / same (42.1); ch. 2 says the "?" key | 2.2.2 Info View; 42.1 Showing and Hiding Views, `…/12/live-keyboard-shortcuts/#showing-and-hiding-views` | A toggle at the bottom-left of the window or the ? key shows and hides it; the shortcut table prints it with Shift (since ? needs Shift on US keyboards). A View menu item name was not stated on the pages read. |
| P2-R41 | Status Bar and Info View are the same thing | unverified | n/a | 2.2.2; 3.2 | The pages describe them separately and do not say how they relate. |
| P2-R42 | CPU/disk overload warnings appear in the Status Bar | contradicted | n/a | 38.1.1 The CPU Load Meter, `…/12/computer-audio-resources-and-strategies/#the-cpu-load-meter`; 38.3 The Overload Indicator, `…/12/computer-audio-resources-and-strategies/#the-overload-indicator`; 3.1 The Control Bar | The CPU meter and the Overload indicator are in the Control Bar, not the Status Bar. The indicator lights for disk problems (showing "Disk") and, only if enabled from the CPU meter's menu, for CPU overload. |

## 8. Left/right placement of panels

| ID | Behaviour | Verdict | Windows / macOS keys | Manual citation (chapter § section, URL) | Paraphrase |
|---|---|---|---|---|---|
| P2-R43 | A setting puts the browser on the left or right | unverified | n/a | 2.3.1 Display & Input, `…/12/first-steps/#display--input`; 2.3.2 Theme & Colors, `…/12/first-steps/#theme--colors`; ch. 4 Working with the Browser, `…/12/working-with-the-browser/`; 3.3 The Browser | No such setting appears in Display & Input, Theme & Colors or the browser chapter. The browser is shown/hidden with Ctrl+Alt+B or Ctrl+Alt+5 / Cmd+Option+B or Cmd+Option+5 (42.1, 3.3). Its right edge and bottom edge can be dragged to resize it, and View > Full-Height Browser extends it to the bottom of the window (ch. 4). |
| P2-R44 | A setting puts the track headers / mixer on the left or right | unverified | n/a | 2.3.1; 6.1; 6.15 | No setting was found. The 6.15 figure shows the Arrangement Track Controls on the right of the clips. |
