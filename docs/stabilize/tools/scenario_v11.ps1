# usage: scenario_v11.ps1 [-Exe .\target\release\app.exe] [-Work <dir>]
# Scripted UI scenario for the v1.1 features (STABILIZE.md §A3), synthetic input as in
# docs/pianoroll/tools/scenario.ps1. Window 1440x900 at 100%, browser open (the default).
# It opens a copy of docs/songs/v2/demo_band.chipdaw (a version-2 file, so it also exercises
# the migration), saves with Ctrl+S after each stage, and check_v11.py checks every saved file.
#   1. Instrument editor: draw on all four macro graphs (two start new macros), undo all four
#      strokes, save; redo all four, save.
#   2. Drag the volume release marker, wheel the vibrato depth, save.
#   3. Browser: search "cello"; drag it onto a clip (clip instrument), onto the noise lane
#      (refused: Cello can't play noise), onto the VRC6 Pulse 2 header (lane default); save.
#   4. Double-click Cello: it loads on the selected lane (Pulse 1); save.
#   5. Click the "Chamber Strings" layout; save.
param([string]$Exe = ".\target\release\app.exe",
  [string]$Repo = (Resolve-Path (Join-Path $PSScriptRoot "..\..\..")).Path,
  [string]$Work = (Join-Path $env:TEMP "chipdaw-scenario-v11"))
$ErrorActionPreference = "Stop"
Set-Location $Repo
New-Item -ItemType Directory -Force $Work | Out-Null
Get-ChildItem $Work -Filter "s*.chipdaw" -ErrorAction SilentlyContinue | Remove-Item
Copy-Item docs\songs\v2\demo_band.chipdaw "$Work\scen.chipdaw" -Force
Get-Process -Name app -ErrorAction SilentlyContinue | Stop-Process -Force
$env:CHIPDAW_PROJECT = "$Work\scen.chipdaw"; $env:CHIPDAW_DATA_DIR = "$Work\data"; $env:CHIPDAW_AUDIO = "0"
$p = Start-Process (Resolve-Path $Exe).Path -PassThru
Start-Sleep -Seconds 3
. "$Repo\docs\perf\tools\input.ps1"
function Ctrl($vk) { KeyDown 0x11; Key $vk; KeyUp 0x11; Start-Sleep -Milliseconds 200 }
function Click($x, $y) { Move-To $x $y; Down; Up; Start-Sleep -Milliseconds 200 }
function Stroke($x0, $y0, $x1, $y1) { Drag-Path $x0 $y0 $x1 $y1 12; Up; Start-Sleep -Milliseconds 250 }
function Save($n) { Ctrl 0x53; Start-Sleep -Milliseconds 500; Copy-Item "$Work\scen.chipdaw" "$Work\s$n.chipdaw" -Force }
function Shot($n) { pwsh -NoProfile -File "$Repo\docs\perf\tools\shot.ps1" -Out "$Work\shot_$n.png" | Out-Null }
# Drag from the browser to a target, slowly enough for the drag threshold and drop targets.
function DragTo($x0, $y0, $x1, $y1) { Drag-Path $x0 $y0 $x1 $y1 25; Start-Sleep -Milliseconds 300; Up; Start-Sleep -Milliseconds 300 }

Save 0                                   # as loaded (migrated to v3)
Click 700 130                            # select "Intro Lead" (Pulse 1, Square Lead)
# 1. The four graphs (client px): volume 857-1129 x 737-779, arpeggio 1139-1411 x 737-779,
#    pitch 857-1129 x 803-845, duty 1139-1411 x 803-845.
Stroke 870 775 1000 745                  # volume
Stroke 1150 770 1300 740                 # arpeggio (unused: starts a 16-step macro)
Stroke 870 840 1000 812                  # pitch (unused: starts one)
Click 1300 806                           # duty (1 step)
Shot "1_drawn"
Ctrl 0x5A; Ctrl 0x5A; Ctrl 0x5A; Ctrl 0x5A   # undo the four strokes
Save 1
Ctrl 0x59; Ctrl 0x59; Ctrl 0x59; Ctrl 0x59   # redo them
Save 2
# 2. The release marker (R, at step 5 of 11) dragged right; vibrato depth +3.
Stroke 981 745 1060 745
Move-To 1000 862; Wheel 1; Wheel 1; Wheel 1; Start-Sleep -Milliseconds 200
Save 3
Shot "2_points"
# 3. Browser.
Click 120 110
foreach ($c in "CELLO".ToCharArray()) { Key ([int][char]$c) }
Start-Sleep -Milliseconds 300
DragTo 42 237 1100 130                   # onto "Hook A" (Pulse 1): that clip's instrument
DragTo 42 237 700 309                    # onto the noise lane: refused
DragTo 42 237 300 489                    # onto the VRC6 Pulse 2 header: lane default
Save 4
Shot "3_browser"
# 4. Double-click: loads on the selected lane (Pulse 1).
Move-To 42 237; Down; Up; Down; Up; Start-Sleep -Milliseconds 400
Save 5
# 5. A channel layout.
Click 76 177
Start-Sleep -Milliseconds 300
Save 6
Shot "4_layout"
Stop-Process -Id $p.Id -Force
Remove-Item Env:CHIPDAW_PROJECT, Env:CHIPDAW_DATA_DIR, Env:CHIPDAW_AUDIO
python -I "$PSScriptRoot\check_v11.py" $Work
