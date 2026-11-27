# usage: thread_cpu.ps1 -Repo <repo> [-DragY <client y>] [-Secs 12]
# Batch 4a gate 6 attribution: CPU per thread (ms over -Secs) while the 200-clip set plays (F4),
# optionally during a drag held at (650, DragY) swaying 160 px (the time scrollbar's thumb).
# Prints the busiest threads by start order (thread 0 = the UI thread).
param([string]$Repo, [int]$DragY = 0, [int]$Secs = 12, [switch]$Demo, [int]$PreIdle = 3)
# -Demo: the bench's demo step instead (no autoplay; idle, then Space, 2 s, measure; no F4).
Get-Process -Name app -ErrorAction SilentlyContinue | Stop-Process -Force
$env:CHIPDAW_DEV = "1"; if (-not $Demo) { $env:CHIPDAW_AUTOPLAY = "1" } else { Remove-Item Env:CHIPDAW_AUTOPLAY -ErrorAction SilentlyContinue }; $env:CHIPDAW_DATA_DIR = Join-Path $env:TEMP "chipdaw-thread-cpu"
$p = Start-Process (Join-Path $Repo "target\release\app.exe") -ArgumentList "--size", "1440x920" -PassThru -WorkingDirectory $Repo
Start-Sleep -Seconds 4
$p.Refresh(); $global:AppH = $p.MainWindowHandle
. (Join-Path $PSScriptRoot "..\..\perf\tools\input.ps1")
Add-Type -Namespace TcFg -Name W -MemberDefinition '[DllImport("user32.dll")] public static extern IntPtr GetForegroundWindow(); [DllImport("user32.dll")] public static extern bool SetForegroundWindow(IntPtr h);' -ErrorAction SilentlyContinue
for ($t = 0; $t -lt 5 -and [TcFg.W]::GetForegroundWindow() -ne $global:AppH; $t++) { [In]::keybd_event(0x12, 0, 0, [UIntPtr]::Zero); [In]::keybd_event(0x12, 0, 2, [UIntPtr]::Zero); [TcFg.W]::SetForegroundWindow($global:AppH) | Out-Null; Start-Sleep -Milliseconds 200 }
$front = [TcFg.W]::GetForegroundWindow() -eq $global:AppH
if ($Demo) { Move-To 1300 800; Start-Sleep -Seconds $PreIdle; Key 0x20; Start-Sleep -Seconds 2 } else { Move-To 1300 700; Key 0x73; Start-Sleep -Seconds 2 }
$p.Refresh(); $t0 = @{}; foreach ($th in $p.Threads) { try { $t0[$th.Id] = $th.TotalProcessorTime.TotalMilliseconds } catch {} }
$order = @{}; $k = 0; foreach ($th in ($p.Threads | Sort-Object StartTime)) { $order[$th.Id] = $k; $k++ }
$sw = [Diagnostics.Stopwatch]::StartNew()
if ($DragY) { Move-To 650 $DragY; Down }
$i = 0
while ($sw.Elapsed.TotalSeconds -lt $Secs) { if ($DragY) { Move-To (650 + [int](160 * [Math]::Sin($i / 12.0))) $DragY }; $i++; Start-Sleep -Milliseconds 8 }
if ($DragY) { Up }
$p.Refresh()
$rows = foreach ($th in $p.Threads) { try { $d = $th.TotalProcessorTime.TotalMilliseconds - $t0[$th.Id]; [pscustomobject]@{ Order = $order[$th.Id]; Id = $th.Id; Pct = [math]::Round($d / $sw.Elapsed.TotalMilliseconds * 100, 2) } } catch {} }
Stop-Process -Id $p.Id -Force
"front $front; total $([math]::Round(($rows | Measure-Object Pct -Sum).Sum, 1))%"
$rows | Sort-Object Pct -Descending | Where-Object { $_.Pct -gt 0 } | ForEach-Object { "  thread #$($_.Order) ($($_.Id)): $($_.Pct)%" }
