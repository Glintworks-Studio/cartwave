# usage: lime_scenes_pb.ps1 -Out <dir> [-Exe .\target\release\cartwave.exe]
# The Presets + Browser batch's lime scenes (gate 7, VISUAL-SPEC §13): 4b's projects with no lime
# in their track colours (fx_studio, fx_board), so any lime found is the interface's own. The new
# UI in every state: the device on a studio and a locked track (stopped and playing: the meter),
# the browser's sections, a search, a folder opened with an item selected, the preset picker, the
# Voice menu, and the DPCM Sampler with Loop on (a toggle on-state, allowed).
param([string]$Out, [string]$Exe = ".\target\release\cartwave.exe")
$Repo = (Resolve-Path (Join-Path $PSScriptRoot "..\..\..")).Path
New-Item -ItemType Directory -Force $Out | Out-Null
$d = Join-Path $env:TEMP ("cartwave-lime-pb-" + [guid]::NewGuid().ToString("N").Substring(0, 8))
New-Item -ItemType Directory -Force $d | Out-Null
python "$Repo\docs\batch4b\tools\fixtures_b4b.py" $d | Out-Null
'{ "ui_scale": 100 }' | Set-Content -Encoding ascii (Join-Path $d "settings.json")
foreach ($k in "CARTWAVE_BROWSER_W", "CARTWAVE_OVERVIEW", "CARTWAVE_HIDE_STATUS") { Remove-Item "Env:$k" -ErrorAction SilentlyContinue }
. "$Repo\docs\perf\tools\input.ps1" 2>$null
function Shot($name) { Move-To 1300 120; Start-Sleep -Milliseconds 400; pwsh -NoProfile -File "$Repo\docs\perf\tools\shot.ps1" -Out (Join-Path $Out "$name.png") | Out-Null }
function C($x, $y) { Move-To $x $y; Start-Sleep -Milliseconds 120; Down; Up; Start-Sleep -Milliseconds 350 }
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
C 520 160                                   # track 1 (studio): the device
Shot "device_studio"
C 452 625; Start-Sleep -Milliseconds 300    # the Voice menu
Shot "voice_menu"; Key 0x1B
Key 0x20; Start-Sleep -Milliseconds 1500
Shot "device_studio_playing"
Key 0x20
C 664 572; Start-Sleep -Milliseconds 400    # the picker
Shot "picker"; Key 0x1B; Start-Sleep -Milliseconds 300
C 75 244; C 260 192; C 260 216; C 300 240   # Sounds > Leads > Square > the first preset
Shot "browser_sounds"
foreach ($sec in @(@("drums", 268), @("instruments", 292), @("hardware", 316), @("clips", 340), @("samples", 364), @("user", 416), @("project", 440), @("favourites", 192))) {
  C 75 $sec[1]; C 260 192
  Shot ("browser_" + $sec[0])
}
C 220 130; foreach ($ch in "kick".ToCharArray()) { Key ([int][char]::ToUpper($ch)) }; Start-Sleep -Milliseconds 400
Shot "browser_search"; Key 0x1B
Start-App "fx_board.chipdaw"
C 520 160                                   # a locked track
Shot "device_locked"
Key 0x20; Start-Sleep -Milliseconds 1500
Shot "device_locked_playing"
Key 0x20
C 664 572; Start-Sleep -Milliseconds 400
Shot "picker_locked"; Key 0x1B
# The DPCM track (the board's fifth): the Sampler, Loop on.
C 520 (136 + 4 * 73 + 24); Start-Sleep -Milliseconds 300
C 776 770; Start-Sleep -Milliseconds 300            # LOOP (the key at device (520, 206))
Shot "dpcm_loop_on"
Get-Process -Name cartwave -ErrorAction SilentlyContinue | Stop-Process -Force
Remove-Item Env:CARTWAVE_PROJECT -ErrorAction SilentlyContinue
Copy-Item (Join-Path $d "stderr.txt") (Join-Path $Out "stderr_last.txt") -ErrorAction SilentlyContinue
