# usage: look.ps1 -Project <file.chipdaw> -Out <shot.png> [-Exe .\target\release\app.exe] [-Wait 2500] [-Env "K=V;K2=V2"]
# Launches the app on a project (dev mode, no audio, a throw-away data dir), waits, saves a
# screenshot of the window and stops the app: a quick look at a state.
param([string]$Project, [string]$Out, [string]$Exe = ".\target\release\app.exe", [int]$Wait = 2500, [string]$Env = "")
$Repo = (Resolve-Path (Join-Path $PSScriptRoot "..\..\..")).Path
Get-Process -Name app -ErrorAction SilentlyContinue | Stop-Process -Force
$data = Join-Path $env:TEMP ("chipdaw-look-" + [guid]::NewGuid().ToString("N").Substring(0, 8))
New-Item -ItemType Directory -Force $data | Out-Null
$env:CHIPDAW_DATA_DIR = $data; $env:CHIPDAW_AUDIO = "0"; $env:CHIPDAW_DEV = "1"; $env:CHIPDAW_PROJECT = (Resolve-Path $Project).Path
foreach ($kv in ($Env -split ";" | Where-Object { $_ })) { $k, $v = $kv -split "=", 2; Set-Item "Env:$k" $v }
$p = Start-Process (Resolve-Path $Exe).Path -PassThru -RedirectStandardError (Join-Path $data "stderr.txt")
Start-Sleep -Milliseconds $Wait
pwsh -NoProfile -File "$Repo\docs\perf\tools\shot.ps1" -Out $Out | Out-Null
Stop-Process -Id $p.Id -Force -ErrorAction SilentlyContinue
Get-Content (Join-Path $data "stderr.txt") -ErrorAction SilentlyContinue | Select-Object -Last 5
