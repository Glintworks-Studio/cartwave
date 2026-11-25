# usage: scenario.ps1 [-Exe .\target\release\app.exe] [-Repo <repo>] [-Work <dir>]
# Scripted UI scenario (PIANOROLL.md §9): synthetic mouse/keyboard input, as in
# docs/perf/tools/bench.ps1. File > New, create a clip by double-click, open it
# in the piano roll, draw three notes (the last one by click-drag), undo/redo the
# drag, select all, duplicate, transpose and undo it, set a volume, close, type
# a file name and save. Then checks the saved file and writes screenshots.
# Window 1440x900 at 100% scale (client coordinates below assume it).
param([string]$Exe = ".\target\release\app.exe",
  [string]$Repo = (Resolve-Path (Join-Path $PSScriptRoot "..\..\..")).Path,
  [string]$Work = $(if ($env:CHIPDAW_PERF_WORK) { $env:CHIPDAW_PERF_WORK } else { Join-Path $env:TEMP "chipdaw-perf" }))
$ErrorActionPreference = "Stop"
New-Item -ItemType Directory -Force $Work | Out-Null
Set-Location $Repo
$exePath = (Resolve-Path $Exe).Path
$out = Join-Path $Work "scenario.chipdaw"
Remove-Item $out -ErrorAction SilentlyContinue
Get-Process -Name app -ErrorAction SilentlyContinue | Stop-Process -Force
$env:CHIPDAW_DATA_DIR = Join-Path $Work "scenario-data"
$env:CHIPDAW_AUDIO = "0"
$p = Start-Process $exePath -PassThru -WorkingDirectory $Work
Start-Sleep -Seconds 3
. "$Repo\docs\perf\tools\input.ps1"
function Shot($name) { pwsh -NoProfile -File $global:Shot -Out (Join-Path $Work "scenario_$name.png") | Out-Null }
function Ctrl($vk) { KeyDown 0x11; Key $vk; KeyUp 0x11; Start-Sleep -Milliseconds 150 }
function DoubleClick($x, $y) { Move-To $x $y; Down; Up; Down; Up; Start-Sleep -Milliseconds 300 }
function Click($x, $y) { Move-To $x $y; Down; Up; Start-Sleep -Milliseconds 150 }

Ctrl 0x4E                       # Ctrl+N: empty project
Click 570 39; Click 570 39      # SNAP: 1/16 -> 1/4 -> Bar
# Lanes are 64 px (ruler bottom at y=106); one bar = 92 px from x=218.
DoubleClick 356 138             # new 1-bar clip on Pulse 1 at bar 2
Start-Sleep -Milliseconds 700   # or the next press counts as a triple click
DoubleClick 356 138             # open it in the piano roll
Shot "1_open"
# Piano roll: grid x0 = 82, 256 px per bar (64 per beat, 16 per 1/16);
# the empty clip is centred on C4: row centre y = 604, 12 px per semitone.
Click 90 604                    # C4 at 1/16 #1
Click 106 556                   # E4 at 1/16 #2
Drag-Path 122 520 186 520 8; Up # G4 at 1/16 #3, dragged to 1/4 + 1/16 long
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
Click 84 840                    # volume lane, first note: about 12
Shot "5_volume"
Key 0x1B                        # Esc: close the piano roll
Click 744 39                    # FILE field
foreach ($c in "SCENARIO".ToCharArray()) { Key ([int][char]$c) }
Key 0x0D                        # Enter: save
Start-Sleep -Milliseconds 500
Shot "6_saved"
Stop-Process -Id $p.Id -Force
Remove-Item Env:CHIPDAW_AUDIO, Env:CHIPDAW_DATA_DIR

if (-not (Test-Path $out)) { "FAIL: $out was not written"; exit 1 }
python -I -c @"
import json, sys
f = json.load(open(sys.argv[1]))
assert f['format'] == 'chipdaw' and f['version'] == 1, 'header'
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
