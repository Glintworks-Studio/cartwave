# usage: probe_textfocus.ps1 — do global shortcuts fire while typing in a text field?
# 1) Type "A B" into the browser search: does Space start playback?  2) Rename a clip in the
# inspector, then Ctrl+Z inside the field: does the global undo run?
param([string]$Exe = ".\target\release\app.exe",
  [string]$Repo = (Resolve-Path (Join-Path $PSScriptRoot "..\..\..")).Path,
  [string]$Work = (Join-Path $env:TEMP "chipdaw-ux"))
$ErrorActionPreference = "Stop"
Set-Location $Repo
$Out = Join-Path $Repo "docs\ux-audit\screens"
Get-Process -Name app -ErrorAction SilentlyContinue | Stop-Process -Force
Remove-Item "$Work\data\*" -ErrorAction SilentlyContinue
$env:CHIPDAW_DATA_DIR = "$Work\data"; $env:CHIPDAW_AUDIO = "0"
$p = Start-Process (Resolve-Path $Exe).Path -PassThru
Start-Sleep -Seconds 3
. "$Repo\docs\perf\tools\input.ps1"
function Click($x, $y) { Move-To $x $y; Down; Up; Start-Sleep -Milliseconds 250 }
function Shot($n) { pwsh -NoProfile -File "$Repo\docs\perf\tools\shot.ps1" -Out "$Out\$n.png" | Out-Null; [In]::SetForegroundWindow($global:AppH) | Out-Null; Start-Sleep -Milliseconds 200 }
Click 120 110
Key 0x41; Key 0x20; Key 0x42; Start-Sleep -Milliseconds 1200
Shot "18-space-in-search-field"
Key 0x1B
Click 1099 133
Click 660 734
KeyDown 0x11; Key 0x41; KeyUp 0x11
Key 0x58; Key 0x59; Key 0x0D; Start-Sleep -Milliseconds 300
Click 660 734
KeyDown 0x11; Key 0x5A; KeyUp 0x11; Start-Sleep -Milliseconds 400
Shot "19-ctrl-z-in-clip-name-field"
"done"
