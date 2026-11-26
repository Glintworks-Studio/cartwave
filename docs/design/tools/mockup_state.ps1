# usage: mockup_state.ps1 [-Exe .\target\release\app.exe] [-Out app.png] [-NoInput] [-Audio]
# Puts the app into the mockup's state (docs/design/mockup-D.dc.html): the
# demo song playing with the loop on, the browser closed, 100 px per bar,
# Triangle soloed, Noise muted, a time selection over bars 9-11, then saves
# a window screenshot. The window's client area starts under the menu bar;
# compare.py finds it from the top bar's top line.
param([string]$Exe = ".\target\release\app.exe", [string]$Out = "app.png", [switch]$NoInput,
  [switch]$Audio, [int]$Wait = 2500, [string]$Data = (Join-Path $env:TEMP "chipdaw-mockup-state"),
  [string]$Extra = "")
$Repo = (Resolve-Path (Join-Path $PSScriptRoot "..\..\..")).Path
$ErrorActionPreference = "Continue"
. "$Repo\docs\perf\tools\input.ps1" 2>$null
$ErrorActionPreference = "Stop"
Get-Process -Name app -ErrorAction SilentlyContinue | Stop-Process -Force
Start-Sleep -Milliseconds 300
Remove-Item -Recurse -Force $Data -ErrorAction SilentlyContinue
New-Item -ItemType Directory -Force $Data | Out-Null
"{ ""browser_open"": false $Extra }" | Set-Content -Encoding ascii (Join-Path $Data "settings.json")
$env:CHIPDAW_DATA_DIR = $Data; $env:CHIPDAW_DEV = "1"; $env:CHIPDAW_PX_PER_BAR = "100"
$env:CHIPDAW_AUDIO = $(if ($Audio) { "1" } else { "0" })
$env:CHIPDAW_AUTOPLAY = "1"
Remove-Item Env:CHIPDAW_PROJECT -ErrorAction SilentlyContinue
$p = Start-Process (Resolve-Path $Exe).Path -PassThru -RedirectStandardError (Join-Path $Data "stderr.txt")
for ($i = 0; $i -lt 50; $i++) { Start-Sleep -Milliseconds 100; if ((Get-Process -Id $p.Id).MainWindowHandle -ne 0) { break } }
Start-Sleep -Milliseconds 1200
pwsh -NoProfile -File "$Repo\docs\perf\tools\shot.ps1" -Out (Join-Path $Data "probe.png") | Out-Null
$global:AppH = (Get-Process -Id $p.Id).MainWindowHandle
[In]::SetForegroundWindow($global:AppH) | Out-Null
if (-not $NoInput) {
  # Client px at 100%: rows start at 104 (76 top bar + 28 ruler), 73 px apart;
  # the M / S keys sit at x 312 / 344, 16 px into a row.
  Move-To 356 (104 + 2 * 73 + 28); Down; Up; Start-Sleep -Milliseconds 200   # solo Triangle
  Move-To 324 (104 + 3 * 73 + 28); Down; Up; Start-Sleep -Milliseconds 200   # mute Noise
  # Time selection: from empty space on lane 7 at bar 9 up to lane 1 at bar 11.
  Move-To 1182 (104 + 6 * 73 + 36); Down
  for ($k = 1; $k -le 12; $k++) { Move-To ([int](1182 + 196 * $k / 12)) ([int](104 + 6 * 73 + 36 - (6 * 73) * $k / 12)); Start-Sleep -Milliseconds 15 }
  Up; Start-Sleep -Milliseconds 200
  Move-To 1300 820   # off the controls (no tooltip in the shot)
}
Start-Sleep -Milliseconds $Wait
pwsh -NoProfile -File "$Repo\docs\perf\tools\shot.ps1" -Out $Out
Get-Process -Name app -ErrorAction SilentlyContinue | Stop-Process -Force
