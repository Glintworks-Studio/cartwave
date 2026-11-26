# usage: lime_scenes_p2.ps1 [-Exe .\target\release\app.exe] [-Out <dir>]
# Screenshots for the polish 2 lime audit (lime_audit_p2.py): stopped, playing (strips lit,
# meters moving), loop off, loop on, solo, mute, a clip selected (inspector), Project Settings,
# the headers on the right with the strip by the clips, and the browser on the right.
param([string]$Exe = ".\target\release\app.exe", [string]$Out = (Join-Path $env:TEMP "chipdaw-lime-p2"))
$Repo = (Resolve-Path (Join-Path $PSScriptRoot "..\..\..")).Path
New-Item -ItemType Directory -Force $Out | Out-Null
$global:Work = $Out
. "$Repo\docs\batch3\tools\lib.ps1"
$global:Exe = (Resolve-Path $Exe).Path
foreach ($k in "CHIPDAW_OVERVIEW", "CHIPDAW_HIDE_STATUS", "CHIPDAW_FAKE_PROBLEM", "CHIPDAW_FROZEN_METERS") { Remove-Item "Env:$k" -ErrorAction SilentlyContinue }
$d = Join-Path $Out "data"
Remove-Item -Recurse -Force $d -ErrorAction SilentlyContinue
Launch -Data $d
ClickL 1300 820; Shot "stopped"
K 0x20; Start-Sleep -Milliseconds 1500; Shot "playing"
ClickL 293 38; Start-Sleep -Milliseconds 400; Shot "loop_off"
ClickL 293 38; Start-Sleep -Milliseconds 400; Shot "loop_on"
ClickL (241 + 300) (136 + 2 * 73 + 22); Start-Sleep -Milliseconds 700; Shot "solo"     # S on the triangle
ClickL (241 + 268) (136 + 3 * 73 + 22); Start-Sleep -Milliseconds 700; Shot "mute"     # M on the noise
K 0x20; Start-Sleep -Milliseconds 300
ClickL 700 172; Start-Sleep -Milliseconds 500; Shot "inspector"
Invoke-Menu "File" 8; Start-Sleep -Milliseconds 500; Shot "project_settings"; K 0x1B
Stop-App
$d2 = Join-Path $Out "data2"
Remove-Item -Recurse -Force $d2 -ErrorAction SilentlyContinue
New-Item -ItemType Directory -Force $d2 | Out-Null
'{ "headers_right": true, "strip_by_clips": true }' | Set-Content -Encoding ascii (Join-Path $d2 "settings.json")
Launch -Data $d2
ClickL 700 820; K 0x20; Start-Sleep -Milliseconds 1500; Shot "right_playing"; K 0x20
Stop-App
$d3 = Join-Path $Out "data3"
Remove-Item -Recurse -Force $d3 -ErrorAction SilentlyContinue
New-Item -ItemType Directory -Force $d3 | Out-Null
'{ "browser_right": true }' | Set-Content -Encoding ascii (Join-Path $d3 "settings.json")
Launch -Data $d3
ClickL 900 820; K 0x20; Start-Sleep -Milliseconds 1500; Shot "browser_right_playing"; K 0x20
Stop-App
"shots -> $Out"
