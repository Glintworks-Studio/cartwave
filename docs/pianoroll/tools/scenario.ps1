# usage: scenario.ps1 [-Exe .\target\release\cartwave.exe] [-Repo <repo>] [-Work <dir>]
# Scripted UI scenario (PIANOROLL.md §9): synthetic mouse/keyboard input, as in
# docs/perf/tools/bench.ps1. File > New, set the arrangement grid to a bar, create a
# clip by double-click, open it in the piano roll, draw three notes (the last one by
# click-drag), undo/redo the drag, select all, duplicate, transpose and undo it, set a
# volume, close, and save through the native Save dialog. Then checks the saved file
# and writes screenshots.
# Window 1440x900 at 100% scale (client coordinates below assume it).
# Updated in the app-shell run (SHELL.md): the old coordinates were from before the
# redesign (a SNAP pill and a FILE field in the top bar), and the file check expected
# format v1; it failed on the pre-shell build too.
param([string]$Exe = ".\target\release\cartwave.exe",
  [string]$Repo = (Resolve-Path (Join-Path $PSScriptRoot "..\..\..")).Path,
  [string]$Work = $(if ($env:CARTWAVE_PERF_WORK) { $env:CARTWAVE_PERF_WORK } else { Join-Path $env:TEMP "chipdaw-perf" }))
$env:CARTWAVE_DEV = "1"   # F-02: test hooks (CARTWAVE_PROJECT, _STRESS, …) and F3-F7 need dev mode
$ErrorActionPreference = "Stop"
New-Item -ItemType Directory -Force $Work | Out-Null
Set-Location $Repo
$exePath = (Resolve-Path $Exe).Path
$out = Join-Path $Work "scenario.chipdaw"
Remove-Item $out -ErrorAction SilentlyContinue
Get-Process -Name cartwave -ErrorAction SilentlyContinue | Stop-Process -Force
$env:CARTWAVE_DATA_DIR = Join-Path $Work "scenario-data"
$env:CARTWAVE_AUDIO = "0"
$p = Start-Process $exePath -PassThru -WorkingDirectory $Work
Start-Sleep -Seconds 3
. "$Repo\docs\perf\tools\input.ps1"
. "$Repo\docs\shell\tools\dialog.ps1"
function Shot($name) { pwsh -NoProfile -File $global:Shot -Out (Join-Path $Work "scenario_$name.png") | Out-Null }
function Ctrl($vk) { KeyDown 0x11; Key $vk; KeyUp 0x11; Start-Sleep -Milliseconds 150 }
function DoubleClick($x, $y) { Move-To $x $y; Down; Up; Down; Up; Start-Sleep -Milliseconds 300 }
function Click($x, $y) { Move-To $x $y; Down; Up; Start-Sleep -Milliseconds 150 }
function RightClick($x, $y) { Move-To $x $y; [In]::mouse_event(0x0008, 0, 0, 0, [UIntPtr]::Zero); [In]::mouse_event(0x0010, 0, 0, 0, [UIntPtr]::Zero); Start-Sleep -Milliseconds 300 }

Ctrl 0x4E                       # Ctrl+N: empty project (the start song is unchanged: no prompt)
# Arrangement: ruler bottom at y=100, lanes 60 px; bar 1 at x=599, 92 px per bar.
RightClick 740 130              # empty space on Pulse 1, bar 2: the context menu
Click 781 258                   # Grid: Bar (5th item; the grid left the top bar)
Key 0x1B                        # Esc: drop the one-bar time selection the right-click made
DoubleClick 740 130             # new 1-bar clip on Pulse 1 at bar 2
Start-Sleep -Milliseconds 700   # or the next press counts as a triple click
DoubleClick 740 130             # open it in the piano roll
Shot "1_open"
# Piano roll: grid x0 = 73, 256 px per bar (64 per beat, 16 per 1/16);
# the empty clip is centred on C4: row centre y = 547, 12 px per semitone.
Click 81 547                    # C4 at 1/16 #1
Click 97 499                    # E4 at 1/16 #2
Drag-Path 113 463 177 463 8; Up # G4 at 1/16 #3, dragged to 1/4 + 1/16 long
Start-Sleep -Milliseconds 200
Shot "2_drawn"
Ctrl 0x5A                       # Ctrl+Z: the whole draw-drag is one step (G4 gone)
Shot "3_undo_draw"
Ctrl 0x59                       # Ctrl+Y: back
Ctrl 0x41                       # Ctrl+A
Ctrl 0x44                       # Ctrl+D: duplicate right (the copies are selected)
KeyDown 0x10; KeyExt 0x26; KeyUp 0x10   # Shift+Up: octave up
Shot "4_octave"
Ctrl 0x5A                       # undo the octave
Click 80 847                    # volume lane, first note: about 12
Shot "5_volume"
Key 0x1B                        # Esc: close the piano roll
Ctrl 0x53                       # Ctrl+S: untitled, so the native Save dialog opens
# The File name box has focus. (UI Automation can hang walking the shell's Save
# dialog, so the path goes in through the clipboard.)
$null = Find-Dialog 5000
Set-Clipboard -Value $out
Ctrl 0x41; Ctrl 0x56; Start-Sleep -Milliseconds 200
Key 0x0D
Start-Sleep -Milliseconds 800
Shot "6_saved"
Stop-Process -Id $p.Id -Force
Remove-Item Env:CARTWAVE_AUDIO, Env:CARTWAVE_DATA_DIR

if (-not (Test-Path $out)) { "FAIL: $out was not written"; exit 1 }
python -I -c @"
import json, sys
f = json.load(open(sys.argv[1]))
assert f['format'] == 'chipdaw' and f['version'] == 3, 'header'
clips = f['clips']
assert len(clips) == 1, f'{len(clips)} clips'
c = clips[0]
assert (c['lane'], c['start'], c['len']) == (0, 3840, 3840), (c['lane'], c['start'], c['len'])
n = c['notes']
pitches = [x['pitch'] for x in n]
starts = [x['start'] for x in n]
assert pitches == [60, 64, 67, 60, 64, 67], pitches
span = n[2]['start'] + n[2]['len']
assert starts[3:] == [s + span for s in starts[:3]], (starts, span)
assert all(a['start'] + a['len'] <= b['start'] for a, b in zip(n, n[1:])), 'monophonic'
assert n[0].get('volume') is not None and 10 <= n[0]['volume'] <= 14, n[0]
assert all('volume' not in x for x in n[1:]), 'only the first note has a volume'
print('PASS: 1 clip, notes', [(x['start'], x['len'], x['pitch'], x.get('volume')) for x in n])
"@ $out
