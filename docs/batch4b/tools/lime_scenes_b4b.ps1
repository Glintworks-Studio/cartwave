# usage: lime_scenes_b4b.ps1 -Out <dir> [-Exe .\target\release\cartwave.exe]
# Batch 4b gate 7: the scenes for lime_audit_b4b.py. Projects with no lime in their track
# colours (VISUAL-SPEC §13: lime is only for toggle on-states and the display), so any lime found
# is the interface's own:
#   studio_arr         fx_studio, arrangement, stopped
#   studio_select      a clip selected (the neutral selection)
#   studio_play        playing (the header meters, the Main row's meters and PEAK)
#   studio_mixer_play  Tab: the mixer playing (strip meters, budget bar, Main strip)
#   s64_mixer_play     fx_s64u (64 studio tracks, unison 2) in the mixer, playing, loop on
#   new_arr / new_mixer  File > New (Ctrl+N; nothing edited, so no prompt): its default tracks
param([string]$Out, [string]$Exe = ".\target\release\cartwave.exe")
$Repo = (Resolve-Path (Join-Path $PSScriptRoot "..\..\..")).Path
New-Item -ItemType Directory -Force $Out | Out-Null
$d = Join-Path $env:TEMP ("chipdaw-lime-" + [guid]::NewGuid().ToString("N").Substring(0, 8))
New-Item -ItemType Directory -Force $d | Out-Null
python "$Repo\docs\batch4b\tools\fixtures_b4b.py" $d | Out-Null
'{ "ui_scale": 100, "browser_open": false }' | Set-Content -Encoding ascii (Join-Path $d "settings.json")
. "$Repo\docs\perf\tools\input.ps1" 2>$null
function Shot($name) { pwsh -NoProfile -File "$Repo\docs\perf\tools\shot.ps1" -Out (Join-Path $Out "$name.png") | Out-Null }
function Start-App($proj) {
  Get-Process -Name cartwave -ErrorAction SilentlyContinue | Stop-Process -Force
  $env:CARTWAVE_DATA_DIR = $d; $env:CARTWAVE_DEV = "1"; $env:CARTWAVE_AUDIO = "1"
  Remove-Item Env:CARTWAVE_FROZEN_METERS -ErrorAction SilentlyContinue
  $env:CARTWAVE_PROJECT = (Join-Path $d $proj)
  $script:p = Start-Process (Resolve-Path $Exe).Path -ArgumentList "--size", "1440x920" -PassThru -RedirectStandardError (Join-Path $d "stderr.txt")
  Start-Sleep -Milliseconds 3000
  $global:AppH = (Get-Process -Id $script:p.Id).MainWindowHandle
  [In]::SetForegroundWindow($global:AppH) | Out-Null; Start-Sleep -Milliseconds 300
  Move-To 1300 900
}
Start-App "fx_studio.chipdaw"
Shot "studio_arr"
# The first clip: track 1's lane at bar 1 (rows from 136 + the Overview's 32: the clip header at
# about y 176; the lanes start at x 621 with the browser closed? no: headers 0..380, lanes 380..).
Move-To 420 180; Down; Up; Start-Sleep -Milliseconds 400; Move-To 1300 900; Start-Sleep -Milliseconds 300
Shot "studio_select"
Key 0x20; Start-Sleep -Milliseconds 1500
Shot "studio_play"
Key 0x09; Start-Sleep -Milliseconds 1200
Shot "studio_mixer_play"
Key 0x20; Start-Sleep -Milliseconds 300
# File > New from the mixer, then the arrangement.
KeyDown 0x11; Key 0x4E; KeyUp 0x11; Start-Sleep -Milliseconds 1500
Shot "new_mixer"
Key 0x09; Start-Sleep -Milliseconds 800
Shot "new_arr"
Start-App "fx_s64u.chipdaw"
Key 0x09; Start-Sleep -Milliseconds 800
Key 0x20; Start-Sleep -Milliseconds 2000
Shot "s64_mixer_play"
Get-Process -Name cartwave -ErrorAction SilentlyContinue | Stop-Process -Force
Remove-Item Env:CARTWAVE_PROJECT -ErrorAction SilentlyContinue
Copy-Item (Join-Path $d "stderr.txt") (Join-Path $Out "stderr_last.txt") -ErrorAction SilentlyContinue
