# usage: mixer_match_shot.ps1 -Out <png> [-Scale 100] [-Exe .\target\release\app.exe]
# Batch 4b §9.3: the mixer as the `mixer` board shows it. fx_mixer (tracks 8-16 are the board's
# strips), browser closed, window client 1680 × 740 at 100 % (the board's 1680 × 640 under the
# 76 px top bar, the 24 px status bar under it), Tab, then the strips scrolled fully right (7
# strips: track 8 at the board's first strip). Meters frozen (they are measured by geometry only).
param([string]$Out, [int]$Scale = 100, [string]$Exe = ".\target\release\app.exe")
$Repo = (Resolve-Path (Join-Path $PSScriptRoot "..\..\..")).Path
$d = Join-Path $env:TEMP ("chipdaw-mixmatch-" + [guid]::NewGuid().ToString("N").Substring(0, 8))
New-Item -ItemType Directory -Force $d | Out-Null
python "$Repo\docs\batch4b\tools\fixtures_b4b.py" $d | Out-Null
"{ ""ui_scale"": $Scale, ""browser_open"": false }" | Set-Content -Encoding ascii (Join-Path $d "settings.json")
. "$Repo\docs\perf\tools\input.ps1" 2>$null
Get-Process -Name app -ErrorAction SilentlyContinue | Stop-Process -Force
$env:CHIPDAW_DATA_DIR = $d; $env:CHIPDAW_DEV = "1"; $env:CHIPDAW_AUDIO = "0"; $env:CHIPDAW_FROZEN_METERS = "1"
$env:CHIPDAW_PROJECT = (Join-Path $d "fx_mixer.chipdaw")
$w = 1680; $h = 740  # logical: the app scales it by the interface scale
$p = Start-Process (Resolve-Path $Exe).Path -ArgumentList "--size", "${w}x${h}" -PassThru -RedirectStandardError (Join-Path $d "stderr.txt")
Start-Sleep -Milliseconds 2500
pwsh -NoProfile -File "$Repo\docs\perf\tools\shot.ps1" -Out (Join-Path $d "_place.png") | Out-Null
$global:AppH = (Get-Process -Id $p.Id).MainWindowHandle
[In]::SetForegroundWindow($global:AppH) | Out-Null; Start-Sleep -Milliseconds 300
Key 0x09; Start-Sleep -Milliseconds 600
Move-To ([int](700 * $Scale / 100)) ([int](400 * $Scale / 100)); Start-Sleep -Milliseconds 200
for ($i = 0; $i -lt 20; $i++) { [In]::mouse_event(0x0800, 0, 0, -120, [UIntPtr]::Zero); Start-Sleep -Milliseconds 40 }
Move-To ([int](1660 * $Scale / 100)) ([int](720 * $Scale / 100)); Start-Sleep -Milliseconds 600
pwsh -NoProfile -File "$Repo\docs\perf\tools\shot.ps1" -Out $Out | Out-Null
Stop-Process -Id $p.Id -Force -ErrorAction SilentlyContinue
Remove-Item Env:CHIPDAW_FROZEN_METERS, Env:CHIPDAW_PROJECT -ErrorAction SilentlyContinue
