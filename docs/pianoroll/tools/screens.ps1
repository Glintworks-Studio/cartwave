# usage: screens.ps1 [-Exe .\target\release\app.exe] [-Repo <repo>]
# Writes the piano-roll screenshots in docs/screens/pianoroll/ (1440x900, 100%).
param([string]$Exe = ".\target\release\app.exe",
  [string]$Repo = (Resolve-Path (Join-Path $PSScriptRoot "..\..\..")).Path)
Set-Location $Repo
$out = Join-Path $Repo "docs\screens\pianoroll"
New-Item -ItemType Directory -Force $out | Out-Null
# Relative, so the FILE field shows no user folder in the screenshots.
$song = "docs\songs\scale_test.chipdaw"
$env:CHIPDAW_DATA_DIR = Join-Path $env:TEMP "chipdaw-screens"
$env:CHIPDAW_AUDIO = "0"
function Open-App($project, $clip) {
  Get-Process -Name app -ErrorAction SilentlyContinue | Stop-Process -Force
  if ($project) { $env:CHIPDAW_PROJECT = $project } else { Remove-Item Env:CHIPDAW_PROJECT -ErrorAction SilentlyContinue }
  $env:CHIPDAW_OPEN_CLIP = "$clip"
  $script:p = Start-Process $Exe -PassThru
  Start-Sleep -Seconds 3
}
# input.ps1 must be dot-sourced at script scope after each launch (it binds the
# new window); inside a function its helpers would vanish with the scope.
$inputPs1 = "$Repo\docs\perf\tools\input.ps1"
function Shot($name) { pwsh -NoProfile -File "$Repo\docs\perf\tools\shot.ps1" -Out (Join-Path $out "$name.png") | Out-Null }

# Demo set, "Hook A" open; pointer on the keys shows the tuning read-out.
Open-App $null 2
. $inputPs1
Move-To 50 640; Start-Sleep -Milliseconds 400
Shot "open-clip"
# Marquee: Shift-drag over part of the clip (captured mid-drag).
KeyDown 0x10; Drag-Path 330 500 760 720 12; Start-Sleep -Milliseconds 300
Shot "marquee"
Up; KeyUp 0x10
Stop-Process -Id $p.Id -Force

# Test song: Pulse 1 scale with per-note volumes (a swell), one selected.
Open-App $song 1
. $inputPs1
Move-To 300 560; Down; Up; Start-Sleep -Milliseconds 300
Shot "volume-lane"
Stop-Process -Id $p.Id -Force
Open-App $song 7
. $inputPs1
Move-To 50 600; Start-Sleep -Milliseconds 400
Shot "noise-rows"
Stop-Process -Id $p.Id -Force
Open-App $song 9
. $inputPs1
Move-To 50 500; Start-Sleep -Milliseconds 400
Shot "dpcm-rows"
Stop-Process -Id $p.Id -Force
Remove-Item Env:CHIPDAW_PROJECT, Env:CHIPDAW_OPEN_CLIP, Env:CHIPDAW_AUDIO, Env:CHIPDAW_DATA_DIR -ErrorAction SilentlyContinue
Get-ChildItem $out | Select-Object Name, Length
