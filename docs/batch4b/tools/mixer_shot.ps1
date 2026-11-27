# usage: mixer_shot.ps1 -Out <png> [-Project fx_mixer] [-Size 1680x760] [-Scale 100] [-Play] [-Exe .\target\release\app.exe]
# Batch 4b §7: a window shot of the mixer view (Tab from the arrangement) on a fixture from
# fixtures_b4b.py, browser hidden. -Play: with audio, plays for 3 s first (lit meters, VOICES);
# otherwise meters frozen at rest.
param([string]$Out, [string]$Project = "fx_mixer", [string]$Size = "1680x760", [int]$Scale = 100, [switch]$Play, [string]$Exe = ".\target\release\app.exe", [int]$Wait = 3000)
$Repo = (Resolve-Path (Join-Path $PSScriptRoot "..\..\..")).Path
$d = Join-Path $env:TEMP ("chipdaw-mix-" + [guid]::NewGuid().ToString("N").Substring(0, 8))
New-Item -ItemType Directory -Force $d | Out-Null
python "$Repo\docs\batch4b\tools\fixtures_b4b.py" $d | Out-Null
"{ ""ui_scale"": $Scale, ""browser_open"": false }" | Set-Content -Encoding ascii (Join-Path $d "settings.json")
. "$Repo\docs\perf\tools\input.ps1" 2>$null
Get-Process -Name app -ErrorAction SilentlyContinue | Stop-Process -Force
$env:CHIPDAW_DATA_DIR = $d; $env:CHIPDAW_DEV = "1"; $env:CHIPDAW_PROJECT = (Join-Path $d "$Project.chipdaw")
if ($Play) { $env:CHIPDAW_AUDIO = "1"; Remove-Item Env:CHIPDAW_FROZEN_METERS -ErrorAction SilentlyContinue } else { $env:CHIPDAW_AUDIO = "0"; $env:CHIPDAW_FROZEN_METERS = "1" }
$p = Start-Process (Resolve-Path $Exe).Path -ArgumentList "--size", $Size -PassThru -RedirectStandardError (Join-Path $d "stderr.txt")
Start-Sleep -Milliseconds 2500
$global:AppH = (Get-Process -Id $p.Id).MainWindowHandle
[In]::SetForegroundWindow($global:AppH) | Out-Null; Start-Sleep -Milliseconds 300
Key 0x09; Start-Sleep -Milliseconds 600   # Tab: the mixer
if ($Play) { Key 0x20; Start-Sleep -Milliseconds $Wait }
pwsh -NoProfile -File "$Repo\docs\perf\tools\shot.ps1" -Out $Out | Out-Null
Stop-Process -Id $p.Id -Force -ErrorAction SilentlyContinue
Remove-Item Env:CHIPDAW_FROZEN_METERS, Env:CHIPDAW_PROJECT -ErrorAction SilentlyContinue
