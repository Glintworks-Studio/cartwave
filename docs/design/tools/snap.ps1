# usage: snap.ps1 -Exe <cartwave.exe> -Out <png> [-Play] [-Select] [-Wait 1500] [-Audio]
# Launches the app (dev mode, throw-away data dir, the demo song), optionally
# starts playback (CARTWAVE_AUTOPLAY) or selects the first clip (a click on
# it, which opens the inspector), waits, saves a window screenshot, and quits.
# Used for the design pass's before/after captures (DESIGNPASS.md).
param([string]$Exe = ".\target\release\cartwave.exe", [string]$Out = "snap.png",
  [switch]$Play, [switch]$Select, [int]$Wait = 1500, [switch]$Audio,
  [string]$Data = (Join-Path $env:TEMP "chipdaw-snap"), [string]$Settings = "",
  [int]$SelX = 700, [int]$SelY = 170)
$Repo = (Resolve-Path (Join-Path $PSScriptRoot "..\..\..")).Path
$ErrorActionPreference = "Continue"
. "$Repo\docs\perf\tools\input.ps1" 2>$null
$ErrorActionPreference = "Stop"
Get-Process -Name cartwave -ErrorAction SilentlyContinue | Stop-Process -Force
Start-Sleep -Milliseconds 300
Remove-Item -Recurse -Force $Data -ErrorAction SilentlyContinue
New-Item -ItemType Directory -Force $Data | Out-Null
if ($Settings) { $Settings | Set-Content -Encoding ascii (Join-Path $Data "settings.json") }
$env:CARTWAVE_DATA_DIR = $Data; $env:CARTWAVE_DEV = "1"
$env:CARTWAVE_AUDIO = $(if ($Audio) { "1" } else { "0" })
$env:CARTWAVE_AUTOPLAY = $(if ($Play) { "1" } else { "0" })
Remove-Item Env:CARTWAVE_PROJECT -ErrorAction SilentlyContinue
$p = Start-Process (Resolve-Path $Exe).Path -PassThru -RedirectStandardError (Join-Path $Data "stderr.txt")
for ($i = 0; $i -lt 50; $i++) { Start-Sleep -Milliseconds 100; if ((Get-Process -Id $p.Id).MainWindowHandle -ne 0) { break } }
Start-Sleep -Milliseconds 1200
pwsh -NoProfile -File "$Repo\docs\perf\tools\shot.ps1" -Out (Join-Path $Data "probe.png") | Out-Null
if ($Select) {
  # A clip on lane 1 (client px; -SelX/-SelY override for other layouts).
  $global:AppH = (Get-Process -Id $p.Id).MainWindowHandle
  [In]::SetForegroundWindow($global:AppH) | Out-Null
  Move-To $SelX $SelY; Down; Up
}
Start-Sleep -Milliseconds $Wait
pwsh -NoProfile -File "$Repo\docs\perf\tools\shot.ps1" -Out $Out
Get-Process -Name cartwave -ErrorAction SilentlyContinue | Stop-Process -Force
