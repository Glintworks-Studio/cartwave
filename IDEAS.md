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
| Metal (macOS) / Direct3D (Windows) presentation path | Priya | The last big CPU and pacing gain |
| More console profiles | Hannah | Each new console = a new hardware profile |
| MIDI keyboard / computer-keyboard note input | — | macOS latency is already low enough (1.9–3.3 ms) |
| Suggest a scale on MIDI import | Sam | Listed as proposed in VISION.md |
| Calibrate the VRC6 level from a real hardware recording | Hannah, Rachel | Buy a Famicom, a VRC6 cartridge and a loopback cable |
| Submit the upstream Vizia PRs and cpal issues | Leo | Ready in `docs/upstream/`; needs a human to submit |
| Custom-drawn menu bar (Windows/Linux; macOS keeps the system bar); custom window frame later | CEO (parked, polish pass) | Today: native bar, dark owner-drawn dropdowns (DARK-MENUS.md) |
| Roadmap order (CEO, Batch 4a prompt): 4a → 4b → Batch 5 automation → input and recording → time signatures and scales → effects → the arcade FM board → the SNES | CEO | Recorded in VISION.md "Build roadmap"; the items below wait for their slot |
| Per-preset authenticity marks (🔒 + profile on a preset row, UX.md B6) | Batch 4a | Today the channel layouts are the authentic presets (RUNLOG B4A-D14) |
| Typing a dB value on the Main fader; Main gain as an undo step | Batch 4a | Drag and wheel work; see RUNLOG B4A-D15 |
| "Don't ask again" box on macOS (NSAlert suppression button) | Batch 4a | Windows has it (task dialog); macOS shows the three buttons only |
