# usage: threads_drag.ps1 -Exe <cartwave.exe> [-DragX 760 -DragY 140] [-Secs 12]
# Per-thread CPU (% of one core) during the 200-clip drag, the busiest threads first, with
# each thread's start module (UI, audio, GL driver...). Attribution for POLISH.md §perf.
param([string]$Exe, [int]$DragX = 760, [int]$DragY = 140, [int]$Secs = 12)
$Repo = (Resolve-Path (Join-Path $PSScriptRoot "..\..\..")).Path
. "$Repo\docs\perf\tools\input.ps1"
Get-Process cartwave -ErrorAction SilentlyContinue | Stop-Process -Force
$d = Join-Path $env:TEMP "chipdaw-threads"; New-Item -ItemType Directory -Force $d | Out-Null
$env:CARTWAVE_DATA_DIR = $d; $env:CARTWAVE_DEV = "1"; $env:CARTWAVE_AUDIO = "1"; $env:CARTWAVE_STRESS = "1"
$p = Start-Process (Resolve-Path $Exe).Path -PassThru
for ($i = 0; $i -lt 50; $i++) { Start-Sleep -Milliseconds 100; $p.Refresh(); if ($p.MainWindowHandle -ne 0) { break } }
Start-Sleep 3
pwsh -NoProfile -File "$Repo\docs\perf\tools\shot.ps1" -Out (Join-Path $d "probe.png") | Out-Null
$p.Refresh(); $global:AppH = $p.MainWindowHandle
[In]::SetForegroundWindow($global:AppH) | Out-Null
Move-To $DragX $DragY; Down; Start-Sleep 1
$p.Refresh()
$t0 = @{}; foreach ($t in $p.Threads) { try { $t0[$t.Id] = $t.TotalProcessorTime.TotalMilliseconds } catch {} }
$sw = [Diagnostics.Stopwatch]::StartNew(); $i = 0
while ($sw.Elapsed.TotalSeconds -lt $Secs) { Move-To ($DragX + [int](120 * [Math]::Sin($i / 12.0))) ($DragY + [int](64 * [Math]::Sin($i / 30.0))); $i++; Start-Sleep -Milliseconds 8 }
$p.Refresh()
$el = $sw.Elapsed.TotalMilliseconds
$rows = foreach ($t in $p.Threads) { try { [pscustomobject]@{ Id = $t.Id; Pct = [math]::Round(($t.TotalProcessorTime.TotalMilliseconds - $t0[$t.Id]) / $el * 100, 2); Prio = $t.PriorityLevel } } catch {} }
Up
$rows | Sort-Object Pct -Descending | Select-Object -First 6 | Format-Table -AutoSize | Out-String
"total {0:N2}%" -f (($rows | Measure-Object Pct -Sum).Sum)
Stop-Process -Id $p.Id -Force
