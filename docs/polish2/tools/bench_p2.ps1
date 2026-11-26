# usage: bench_p2.ps1 -Label <run> -Repo <repo> [-New] [-Secs 12] [-Work <dir>]
# Polish 2 gate 6: bench.ps1's scenarios (idle, demo playing, 200 clips playing, 200-clip drag;
# process CPU, % of one core) and, with -New, the new gestures on the 200-clip set while it
# plays: a row-resize drag, a header-width drag, a ruler zoom drag, an Overview drag and an
# Overview zoom drag. Default settings (the polish 2 build shows the Overview and the status
# bar, so its rows start 32 px lower: the clip grab moves from (760, 140) to (760, 172)).
# Appends one line per run to <Work>\bench_p2_log.txt with proof the 200-clip set was on.
param([string]$Label = "run", [int]$Secs = 12, [string]$Repo, [switch]$New,
  [string]$Work = (Join-Path $env:TEMP "chipdaw-p2-perf"))
$env:CHIPDAW_DEV = "1"
foreach ($k in "CHIPDAW_OVERVIEW", "CHIPDAW_HIDE_STATUS", "CHIPDAW_FAKE_PROBLEM", "CHIPDAW_PROJECT", "CHIPDAW_FROZEN_METERS") { Remove-Item "Env:$k" -ErrorAction SilentlyContinue }
New-Item -ItemType Directory -Force $Work | Out-Null
$d = Join-Path $Work "data-$Label"
Remove-Item -Recurse -Force $d -ErrorAction SilentlyContinue
New-Item -ItemType Directory -Force $d | Out-Null
$env:CHIPDAW_DATA_DIR = $d
Set-Location $Repo
Get-Process -Name app -ErrorAction SilentlyContinue | Stop-Process -Force
$p = Start-Process ".\target\release\app.exe" -ArgumentList "--size", "1440x920" -PassThru -RedirectStandardError "$Work\bench_p2_err.txt"
Start-Sleep -Seconds 3
. "$PSScriptRoot\..\..\perf\tools\input.ps1"
$global:AppH = $p.MainWindowHandle
$global:Res = [ordered]@{}
function Measure-Cpu([scriptblock]$during) {
  $p.Refresh(); $c0 = $p.TotalProcessorTime; $t0 = [Diagnostics.Stopwatch]::StartNew(); $i = 0
  while ($t0.Elapsed.TotalSeconds -lt $Secs) { & $during $i; $i++; Start-Sleep -Milliseconds 8 }
  $p.Refresh(); [math]::Round(($p.TotalProcessorTime - $c0).TotalMilliseconds / $t0.Elapsed.TotalMilliseconds * 100, 1)
}
function Cpu($name) { $global:Res[$name] = Measure-Cpu { param($i) } }
# A drag held for $Secs: press at (x, y), then sway by (ax, ay).
function CpuDrag($name, $x, $y, $ax, $ay) {
  Move-To $x $y; Down
  $global:Res[$name] = Measure-Cpu { param($i) Move-To ($x + [int]($ax * [Math]::Sin($i / 12.0))) ($y + [int]($ay * [Math]::Sin($i / 30.0))) }
  Up; Start-Sleep -Milliseconds 300
}
$dy = if ($New) { 32 } else { 0 }
Move-To 1300 800
Cpu "idle"
Key 0x20; Start-Sleep -Seconds 2
Cpu "demo"
Key 0x73; Start-Sleep -Seconds 3
Cpu "200"
CpuDrag "200drag" 760 (140 + $dy) 120 64
if ($New) {
  CpuDrag "row_resize" 441 (136 + 71) 0 60
  CpuDrag "header_width" 619 600 100 0
  CpuDrag "ruler_zoom" 900 122 6 70
  CpuDrag "overview_drag" 680 92 160 0
  CpuDrag "overview_zoom" 680 92 4 50
}
Stop-Process -Id $p.Id -Force
$err = Get-Content "$Work\bench_p2_err.txt" -Raw -ErrorAction SilentlyContinue
$proof = if ($err -match "dev: mode on" -and $err -match "dev: F4 stress set on") { "reached" } else { "NOT REACHED" }
$line = "{0,-16} " -f $Label + (($Res.Keys | ForEach-Object { "{0} {1,5}%" -f $_, $Res[$_] }) -join "  ") + "  stress set $proof"
Add-Content "$Work\bench_p2_log.txt" $line
$line
