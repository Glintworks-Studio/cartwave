# usage: lime_scenes.ps1 [-Exe .\target\release\cartwave.exe] [-Out <dir>]
# Screenshots for the lime audit (DESIGN_PASS_PROMPT.md §4.3): stopped, playing,
# loop off, loop on, solo, mute, a clip selected (inspector), the browser open,
# Project Settings open. lime_audit.py scans them.
param([string]$Exe = ".\target\release\cartwave.exe", [string]$Out = (Join-Path $env:TEMP "chipdaw-lime"))
$Repo = (Resolve-Path (Join-Path $PSScriptRoot "..\..\..")).Path
New-Item -ItemType Directory -Force $Out | Out-Null
$global:Work = $Out
. "$Repo\docs\batch3\tools\lib.ps1"
$global:Exe = (Resolve-Path $Exe).Path
$d = Join-Path $Out "data"
Remove-Item -Recurse -Force $d -ErrorAction SilentlyContinue
Launch -Data $d
ClickL 1300 820; Shot "stopped"                          # loop on (the default), stopped
K 0x20; Start-Sleep -Milliseconds 1500; Shot "playing"   # Space: play
ClickL 293 38; Start-Sleep -Milliseconds 400; Shot "loop_off"   # the Loop key
ClickL 293 38; Start-Sleep -Milliseconds 400; Shot "loop_on"
ClickL (241 + 356) (104 + 2 * 73 + 28); Start-Sleep -Milliseconds 600; Shot "solo"    # S on the triangle
ClickL (241 + 324) (104 + 3 * 73 + 28); Start-Sleep -Milliseconds 600; Shot "mute"    # M on the noise
K 0x20; Start-Sleep -Milliseconds 300
ClickL 700 140; Start-Sleep -Milliseconds 500; Shot "inspector"                       # a clip: the inspector
Invoke-Menu "File" 8; Start-Sleep -Milliseconds 500; Shot "project_settings"; K 0x1B
Stop-App
"shots -> $Out"
