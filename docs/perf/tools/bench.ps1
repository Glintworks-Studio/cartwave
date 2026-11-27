# usage: bench.ps1 [-Label run] [-Secs 12] [-Exe .\target\release\cartwave.exe] [-Env "K=V;K2=V2"] [-Repo <repo>] [-Work <dir>]
# Measures process CPU (% of one core) over $Secs per scenario: idle, demo playing,
# 200 clips playing, 200-clip drag. Window at 1440x900, 100% scale.
# Appends one line per run to <Work>\bench_log.txt. -Exe is relative to -Repo.
# Defaults: -Repo = this script's repo; -Work = $env:CARTWAVE_PERF_WORK or %TEMP%\chipdaw-perf.
param([string]$Label = "run", [int]$Secs = 12, [string]$Exe = ".\target\release\cartwave.exe", [string]$Env = "",
  # Where the drag starts (client px): on a clip in the 200-clip view. (300, 160) was right before
  # the redesign; since then it is a track header, so that "drag" moved nothing (STABILIZE.md A2).
  # Since the design pass's layout (bars from x 621) the same grab is 760,140: 720,130 there
  # is a clip's left resize zone, which resized the clip instead (POLISH.md §perf).
  [int]$DragX = 720, [int]$DragY = 130,
  [string]$Repo = (Resolve-Path (Join-Path $PSScriptRoot "..\..\..")).Path,
  [string]$Work = $(if ($env:CARTWAVE_PERF_WORK) { $env:CARTWAVE_PERF_WORK } else { Join-Path $env:TEMP "chipdaw-perf" }))
$env:CARTWAVE_DEV = "1"   # F-02: test hooks (CARTWAVE_PROJECT, _STRESS, …) and F3-F7 need dev mode
New-Item -ItemType Directory -Force $Work | Out-Null
Set-Location $Repo
Get-Process -Name cartwave -ErrorAction SilentlyContinue | Stop-Process -Force
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
  Move-To $DragX $DragY; Down
  $p.Refresh(); $c0 = $p.TotalProcessorTime; $t0 = [Diagnostics.Stopwatch]::StartNew(); $i = 0
  while ($t0.Elapsed.TotalSeconds -lt $Secs) { Move-To ($DragX + [int](120 * [Math]::Sin($i / 12.0))) ($DragY + [int](64 * [Math]::Sin($i / 30.0))); $i++; Start-Sleep -Milliseconds 8 }
  $p.Refresh(); $pct = ($p.TotalProcessorTime - $c0).TotalMilliseconds / $t0.Elapsed.TotalMilliseconds * 100
  Up
  $global:Res[$name] = [math]::Round($pct, 1)
}
# Park the mouse on empty canvas under the lanes (both layouts). Until the design pass it was
# (700, 40): empty top bar before, but over the display now, where an open tooltip costs CPU.
Move-To 1300 800
Cpu "idle"
Key 0x20; Start-Sleep -Seconds 2
Cpu "demo_playing"
Key 0x73; Start-Sleep -Seconds 3
Cpu "stress_playing"
CpuDrag "stress_drag"
Stop-Process -Id $p.Id -Force
if ($Env) { foreach ($kv in $Env.Split(';')) { $k = $kv.Split('=')[0]; Remove-Item "Env:$k" -ErrorAction SilentlyContinue } }
# Proof the scenario ran (F-02): dev mode on, and F4 really swapped in the 200-clip set.
$err = Get-Content "$Work\bench_err.txt" -Raw -ErrorAction SilentlyContinue
$proof = if ($err -match "dev: mode on" -and $err -match "dev: F4 stress set on") { "reached" } else { "NOT REACHED (no dev-mode F4 line on stderr)" }
$line = "{0,-28} idle {1,5}%  demo {2,5}%  200 {3,5}%  200drag {4,5}%  stress set {5}" -f $Label, $Res.idle, $Res.demo_playing, $Res.stress_playing, $Res.stress_drag, $proof
Add-Content "$Work\bench_log.txt" $line
$line
