# usage: bench.ps1 [-Label run] [-Secs 12] [-Exe .\target\release\app.exe] [-Env "K=V;K2=V2"] [-Repo <repo>] [-Work <dir>]
# Measures process CPU (% of one core) over $Secs per scenario: idle, demo playing,
# 200 clips playing, 200-clip drag. Window at 1440x900, 100% scale.
# Appends one line per run to <Work>\bench_log.txt. -Exe is relative to -Repo.
# Defaults: -Repo = this script's repo; -Work = $env:CHIPDAW_PERF_WORK or %TEMP%\chipdaw-perf.
param([string]$Label = "run", [int]$Secs = 12, [string]$Exe = ".\target\release\app.exe", [string]$Env = "",
  [string]$Repo = (Resolve-Path (Join-Path $PSScriptRoot "..\..\..")).Path,
  [string]$Work = $(if ($env:CHIPDAW_PERF_WORK) { $env:CHIPDAW_PERF_WORK } else { Join-Path $env:TEMP "chipdaw-perf" }))
New-Item -ItemType Directory -Force $Work | Out-Null
Set-Location $Repo
Get-Process -Name app -ErrorAction SilentlyContinue | Stop-Process -Force
if ($Env) { foreach ($kv in $Env.Split(';')) { $k, $v = $kv.Split('='); Set-Item "Env:$k" $v } }
$p = Start-Process $Exe -PassThru -RedirectStandardError "$Work\bench_err.txt"
Start-Sleep -Seconds 3
. "$PSScriptRoot\input.ps1"
$global:Res = [ordered]@{}
function Cpu($name) {
  $p.Refresh(); $c0 = $p.TotalProcessorTime; $t0 = [Diagnostics.Stopwatch]::StartNew()
  Start-Sleep -Seconds $Secs
  $p.Refresh(); $pct = ($p.TotalProcessorTime - $c0).TotalMilliseconds / $t0.Elapsed.TotalMilliseconds * 100
  $global:Res[$name] = [math]::Round($pct, 1)
}
function CpuDrag($name) {
  Move-To 300 160; Down
  $p.Refresh(); $c0 = $p.TotalProcessorTime; $t0 = [Diagnostics.Stopwatch]::StartNew(); $i = 0
  while ($t0.Elapsed.TotalSeconds -lt $Secs) { Move-To (300 + [int](120 * [Math]::Sin($i / 12.0))) (160 + [int](64 * [Math]::Sin($i / 30.0))); $i++; Start-Sleep -Milliseconds 8 }
  $p.Refresh(); $pct = ($p.TotalProcessorTime - $c0).TotalMilliseconds / $t0.Elapsed.TotalMilliseconds * 100
  Up
  $global:Res[$name] = [math]::Round($pct, 1)
}
Move-To 700 40
Cpu "idle"
Key 0x20; Start-Sleep -Seconds 2
Cpu "demo_playing"
Key 0x73; Start-Sleep -Seconds 3
Cpu "stress_playing"
CpuDrag "stress_drag"
Stop-Process -Id $p.Id -Force
if ($Env) { foreach ($kv in $Env.Split(';')) { $k = $kv.Split('=')[0]; Remove-Item "Env:$k" -ErrorAction SilentlyContinue } }
$line = "{0,-28} idle {1,5}%  demo {2,5}%  200 {3,5}%  200drag {4,5}%" -f $Label, $Res.idle, $Res.demo_playing, $Res.stress_playing, $Res.stress_drag
Add-Content "$Work\bench_log.txt" $line
$line
