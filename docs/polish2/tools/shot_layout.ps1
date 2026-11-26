# usage: shot_layout.ps1 -Out <png> [-Settings '<json fields>'] [-Project <path>] [-Size 1440x920] [-Vars "K=V;K2=V2"] [-Keep]
# Launches the release build with a fresh data dir, the given settings.json fields and
# project, waits, screenshots the window (shot.ps1) and closes it (unless -Keep).
# Meters frozen, audio off, dev mode on.
param([string]$Out, [string]$Settings = "", [string]$Project = "", [string]$Size = "1440x920", [string]$Vars = "", [switch]$Keep, [string]$Exe = "")
$Repo = (Resolve-Path (Join-Path $PSScriptRoot "..\..\..")).Path
if (-not $Exe) { $Exe = Join-Path $Repo "target\release\app.exe" }
Get-Process -Name app -ErrorAction SilentlyContinue | Stop-Process -Force
Start-Sleep -Milliseconds 300
$d = Join-Path $env:TEMP "chipdaw-p2-shot"
Remove-Item -Recurse -Force $d -ErrorAction SilentlyContinue
New-Item -ItemType Directory -Force $d | Out-Null
$fields = if ($Settings) { ", $Settings" } else { "" }
"{ ""ui_scale"": 100 $fields }" | Set-Content -Encoding ascii (Join-Path $d "settings.json")
foreach ($k in "CHIPDAW_PROJECT", "CHIPDAW_HIDE_STATUS", "CHIPDAW_PLAYHEAD", "CHIPDAW_FAKE_PROBLEM", "CHIPDAW_OVERVIEW") { Remove-Item "Env:$k" -ErrorAction SilentlyContinue }
$env:CHIPDAW_DATA_DIR = $d; $env:CHIPDAW_DEV = "1"; $env:CHIPDAW_AUDIO = "0"; $env:CHIPDAW_FROZEN_METERS = "1"
if ($Project) { $env:CHIPDAW_PROJECT = (Resolve-Path $Project).Path }
if ($Vars) { foreach ($kv in $Vars.Split(";")) { $k, $v = $kv.Split("=", 2); Set-Item "Env:$k" $v } }
$p = Start-Process $Exe -ArgumentList "--size", $Size -PassThru -RedirectStandardError (Join-Path $d "stderr.txt")
for ($i = 0; $i -lt 50; $i++) { Start-Sleep -Milliseconds 100; if ((Get-Process -Id $p.Id).MainWindowHandle -ne 0) { break } }
Start-Sleep -Milliseconds 1800
pwsh -NoProfile -File "$Repo\docs\perf\tools\shot.ps1" -Out $Out | Out-Null
if (-not $Keep) { Stop-Process -Id $p.Id -Force -ErrorAction SilentlyContinue }
"shot -> $Out"
