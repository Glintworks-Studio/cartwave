| Screenshot | Lime px in allowed places | Outside (violations) |
|---|---|---|
| budget.png |  | 0 |
| insert-track.png | Overview clips (track colour): 199, Overview loop line (loop on): 1638, browser list: kind markers (lane colour): 490, colour strips (+ spill): 720, display indicators: 69, lanes: clips and notes (track colour): 14043, loop brace + glow: 3276, selected view icon: 108 | 0 |
| lock-menu.png | Overview clips (track colour): 203, Overview loop line (loop on): 1638, browser list: kind markers (lane colour): 490, colour strips (+ spill): 720, display indicators: 69, lanes: clips and notes (track colour): 3483, loop brace + glow: 2944, selected view icon: 108 | 0 |
| lock-reset-dialog.png | Overview clips (track colour): 203, Overview loop line (loop on): 1624, browser list: kind markers (lane colour): 490, colour strips (+ spill): 720, display indicators: 69, lanes: clips and notes (track colour): 3483, loop brace + glow: 2944, selected view icon: 108 | 14 in (1440, 76)–(1446, 77) |
| locked-header.png | Overview clips (track colour): 203, Overview loop line (loop on): 1638, browser list: kind markers (lane colour): 490, colour strips (+ spill): 720, display indicators: 69, lanes: clips and notes (track colour): 3483, loop brace + glow: 2944, selected view icon: 108 | 0 |
| main-track.png | Overview clips (track colour): 199, Overview loop line (loop on): 1638, browser list: kind markers (lane colour): 840, colour strips (+ spill): 720, display indicators: 69, lanes: clips and notes (track colour): 14045, loop brace + glow: 3276, selected view icon: 108 | 0 |

Violations: 14

**Reading.** The 14 px in `lock-reset-dialog.png` are the Overview's loop line (allowed while the
loop is on), dimmed by the dialog's backdrop: that shot was cut from a desktop capture (the Windows
dialog sits partly outside the app), so its client area starts 6 px further right than the
checker assumes and the line's last pixels fall past the zone's end. No lime outside the allowed
"on" states. (The shot was first committed as a full-desktop capture in 7dee8aa; it is now cut to
the app and the dialog.)
