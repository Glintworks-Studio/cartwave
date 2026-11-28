# usage: lime_scenes_b5.ps1 -Out <dir> [-Exe .\target\release\cartwave.exe]
# Batch 5's lime scenes (gate 6, VISUAL-SPEC §13 and §14: "no lime anywhere in automation"): 4b's
# projects with no lime in their track colours (fx_studio; fx_locked, the same song locked), so
# any lime found is the interface's own. Automation in every state: Automation Mode with lanes and
# breakpoints (one selected), the chooser open, Draw Mode, a lane's time selection, Automation
# Arm on, a control overridden (Re-Enable lit), a locked track's stepped lanes, the device with
# an automated knob's dot. Client px at 100 % as at_b5.ps1 (rows from 136; track 0's lane
# 209..282, its 0 dB line at y 220; the header fader at (770, 147); Arm (358, 38), Re-Enable (407, 38)).
param([string]$Out, [string]$Exe = ".\target\release\cartwave.exe")
$Repo = (Resolve-Path (Join-Path $PSScriptRoot "..\..\..")).Path
New-Item -ItemType Directory -Force $Out | Out-Null
$d = Join-Path $env:TEMP ("cartwave-lime-b5-" + [guid]::NewGuid().ToString("N").Substring(0, 8))
New-Item -ItemType Directory -Force $d | Out-Null
python "$Repo\docs\batch4b\tools\fixtures_b4b.py" $d | Out-Null
python "$Repo\docs\presets\tools\fixture_locked.py" (Join-Path $d "fx_studio.chipdaw") (Join-Path $d "fx_locked.chipdaw")
'{ "ui_scale": 100 }' | Set-Content -Encoding ascii (Join-Path $d "settings.json")
foreach ($k in "CARTWAVE_BROWSER_W", "CARTWAVE_OVERVIEW", "CARTWAVE_HIDE_STATUS", "CARTWAVE_NO_AUTO_KEYS") { Remove-Item "Env:$k" -ErrorAction SilentlyContinue }
. "$Repo\docs\perf\tools\input.ps1" 2>$null
function Shot($name) { Move-To 1300 120; Start-Sleep -Milliseconds 400; pwsh -NoProfile -File "$Repo\docs\perf\tools\shot.ps1" -Out (Join-Path $Out "$name.png") | Out-Null }
function C($x, $y) { Move-To $x $y; Start-Sleep -Milliseconds 120; Down; Up; Start-Sleep -Milliseconds 350 }
function Drag($x0, $y0, $x1, $y1) { Move-To $x0 $y0; Start-Sleep -Milliseconds 100; Down; for ($i = 1; $i -le 16; $i++) { Move-To ([int]($x0 + ($x1 - $x0) * $i / 16)) ([int]($y0 + ($y1 - $y0) * $i / 16)); Start-Sleep -Milliseconds 20 }; Up; Start-Sleep -Milliseconds 400 }
function Start-App($proj) {
  Get-Process -Name cartwave -ErrorAction SilentlyContinue | Stop-Process -Force
  $env:CARTWAVE_DATA_DIR = $d; $env:CARTWAVE_DEV = "1"; $env:CARTWAVE_AUDIO = "1"
  $env:CARTWAVE_PROJECT = (Join-Path $d $proj)
  $script:p = Start-Process (Resolve-Path $Exe).Path -ArgumentList "--size", "1440x920" -PassThru -RedirectStandardError (Join-Path $d "stderr.txt")
  Start-Sleep -Milliseconds 3000
  $global:AppH = (Get-Process -Id $script:p.Id).MainWindowHandle
  [In]::SetForegroundWindow($global:AppH) | Out-Null; Start-Sleep -Milliseconds 300
  pwsh -NoProfile -File "$Repo\docs\perf\tools\shot.ps1" -Out (Join-Path $d "_place.png") | Out-Null
}
Start-App "fx_studio.chipdaw"
Key 0x41; Start-Sleep -Milliseconds 500                     # A
C 897 220; C 1081 220; C 1265 220                           # three breakpoints
Start-Sleep -Milliseconds 700
Drag 1081 220 1081 262                                      # one moved (it stays selected)
Shot "auto_mode"
C 470 222; Start-Sleep -Milliseconds 300                    # the chooser
Shot "auto_chooser"; Key 0x1B; Start-Sleep -Milliseconds 300
Drag 900 214 1150 214                                       # a lane time selection
Shot "auto_timesel"
Key 0x42; Drag 1300 400 1420 380; Shot "auto_draw"; Key 0x42   # Draw Mode on track 1's lane
C 358 38                                                    # Automation Arm on
Shot "auto_armed"
C 358 38                                                    # off
Key 0x20; Start-Sleep -Milliseconds 800
Drag 770 150 770 170                                        # the fader moved by hand: overridden
Start-Sleep -Milliseconds 300
Shot "auto_overridden"
Key 0x20
C 520 160                                                   # the track's device (its markers)
Shot "auto_device"
Get-Process -Name cartwave -ErrorAction SilentlyContinue | Stop-Process -Force
Start-App "fx_locked.chipdaw"
Key 0x41; Start-Sleep -Milliseconds 500
C 897 237; C 1265 237; Start-Sleep -Milliseconds 700; Drag 1265 237 1265 214
Shot "auto_locked"
Get-Process -Name cartwave -ErrorAction SilentlyContinue | Stop-Process -Force
Get-Content (Join-Path $d "stderr.txt") | Select-String "dev: (automation|menu AutoParam)" | Select-Object -Last 6
"scenes -> $Out"
