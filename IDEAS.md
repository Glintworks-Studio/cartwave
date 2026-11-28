# IDEAS: the parking lot

Ideas go here first, not straight into a build. Each one moves into VISION.md or UX.md
only when the CEO picks it for a build. Keep entries to a line or two.

| Idea | Source | Notes |
|---|---|---|
| Export a file that plays on real hardware / hardware-format emulators | Marcus | Only meaningful when all tracks are locked and within budget |
| Frame-locked tempo (tempo in whole video frames) | Sam, Hannah | Makes live loops match exports exactly (STABILIZE.md §C) |
| Per-note effects (arpeggio, vibrato, slides, duty changes) | Hannah | The `NoteFx` extension point exists |
| "Linear stems" export option (sums exactly to the mix; less authentic) | Hannah | The non-linear mixer gives about a −11 dB difference today |
| TPDF dither on 16-bit export | Hannah | Small |
| Verified loop setup docs for Unity, FMOD, Wwise | Marcus | Only Godot is verified so far |
| Metal (macOS) / Direct3D (Windows) presentation path | Priya | The last big CPU and pacing gain. The Metal backend is in the roadmap (step 11, 2026-11-26); Direct3D stays here |
| More console profiles | Hannah | Each new console = a new hardware profile |
| MIDI keyboard / computer-keyboard note input | — | macOS latency is already low enough (1.9–3.3 ms). In the roadmap as "recording and MIDI input" (step 8, 2026-11-26) |
| Suggest a scale on MIDI import | Sam | Listed as proposed in VISION.md |
| Calibrate the VRC6 level from a real hardware recording | Hannah, Rachel | Buy a Famicom, a VRC6 cartridge and a loopback cable. Part of the hardware A/B recordings below |
| Submit the upstream Vizia PRs and cpal issues | Leo | Ready in `docs/upstream/`; needs a human to submit |
| Custom-drawn menu bar (Windows/Linux; macOS keeps the system bar); custom window frame later | CEO (parked, polish pass) | Today: native bar, dark owner-drawn dropdowns (DARK-MENUS.md) |
| Roadmap order | CEO | **Replaced 2026-11-26** by the meeting's 13-step order in VISION.md "Build roadmap"; the items here wait for their slot |
| Per-preset authenticity marks (🔒 + profile on a preset row, UX.md B6) | Batch 4a | Today the channel layouts are the authentic presets (RUNLOG B4A-D14). Decided 2026-11-26: the browser's Hardware folder (VISION.md "The browser") |
| Typing a dB value on the Main fader; Main gain as an undo step | Batch 4a | Drag and wheel work; see RUNLOG B4A-D15 |
| "Don't ask again" box on macOS (NSAlert suppression button) | Batch 4a | Windows has it (task dialog); macOS shows the three buttons only |
| Optional classic green / yellow / red meter theme in Settings | Marcus | Not now. The mixer's meters are the warm palette (VISUAL-SPEC.md §13); `meter-colours` option 3 is superseded |
| Hardware A/B recordings of a real Famicom and a VRC6 cart | Mika | Once the hardware is funded; feeds the authenticity method (VISION.md "Sound systems") |
| Mika's own showcase song, for demos and A/B tests | Mika | Ours, listed in ASSETS.md; never a cover of game music |
| A draggable browser width (as the headers' edge), remembered per user | Presets batch | The browser is the board's 420 px; the lanes are 180 px narrower (PRESETS.md §4) |
| Renamable collections (and more than seven) | Presets batch | Today: Favourites and six fixed colours (RUNLOG PB-D15) |
| The mixer's strips fit the room the bottom pane leaves (or the pane hides in the mixer until Shift+Tab) | Presets batch | The 356 px pane covers the strips' readouts while a track is selected (PRESETS.md §4) |
| Studio noise / DPCM voices at the console's level | Presets batch | 2.3–3.6 LU louder than through the console's mixer (RUNLOG PB-D7); after 4c's tnd calibration |
