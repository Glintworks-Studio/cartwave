# usage: trace_app.ps1 -Name <name> [-Scenario demo|stress|drag] [-Secs 12] [-Exe .\target\profiling\app.exe] [-Repo <repo>] [-Work <dir>]
# Runs the profiling build (cargo build --profile profiling -p app) through a scenario while the
# elevated capture_server.ps1 (same -Work) records an xperf trace, then runs prof.sh on it.
# Defaults: -Repo = this script's repo; -Work = $env:CHIPDAW_PERF_WORK or %TEMP%\chipdaw-perf.
# Needs Git Bash (bash on PATH or in Program Files\Git), samply, inferno and python.
param([string]$Name, [ValidateSet("demo", "stress", "drag")][string]$Scenario = "demo", [int]$Secs = 12,
  [string]$Exe = ".\target\profiling\app.exe",
  [string]$Repo = (Resolve-Path (Join-Path $PSScriptRoot "..\..\..")).Path,
  [string]$Work = $(if ($env:CHIPDAW_PERF_WORK) { $env:CHIPDAW_PERF_WORK } else { Join-Path $env:TEMP "chipdaw-perf" }))
New-Item -ItemType Directory -Force $Work | Out-Null
Set-Location $Repo
Get-Process -Name app -ErrorAction SilentlyContinue | Stop-Process -Force
$p = Start-Process $Exe -PassThru
Start-Sleep -Seconds 3
. "$PSScriptRoot\input.ps1"
Move-To 700 40; Key 0x20
if ($Scenario -ne "demo") { Key 0x73 }
Start-Sleep -Seconds 2
TraceCmd start $Name | Out-Null
[In]::SetForegroundWindow($global:AppH) | Out-Null
if ($Scenario -eq "drag") {
  Move-To 300 160; Down
  $t0 = [Diagnostics.Stopwatch]::StartNew(); $i = 0
  while ($t0.Elapsed.TotalSeconds -lt $Secs) { Move-To (300 + [int](120 * [Math]::Sin($i / 12.0))) (160 + [int](64 * [Math]::Sin($i / 30.0))); $i++; Start-Sleep -Milliseconds 8 }
  Up
} else { Start-Sleep -Seconds $Secs }
TraceCmd stop $Name | Out-Null
Stop-Process -Id $p.Id -Force
$bash = (Get-Command bash -ErrorAction SilentlyContinue).Source
if (-not $bash) { $bash = Join-Path $env:ProgramFiles "Git\bin\bash.exe" }
$env:CHIPDAW_PERF_WORK = $Work; $env:CHIPDAW_REPO = $Repo
& $bash "$PSScriptRoot/prof.sh" $Name
