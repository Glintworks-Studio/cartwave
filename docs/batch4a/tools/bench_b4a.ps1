# usage: bench_b4a.ps1 -Label <run> -Repo <repo> [-Main] [-Secs 12] [-Work <dir>]
# Batch 4a §9.6: polish 2's bench (bench_p2.ps1 -New) for both builds; -Main: this branch (its
# Main row makes the lanes 73 px shorter, so the time scrollbar sits 73 px higher), plus the new
# scenarios on the 64-studio-track project: playing, scrolling the lanes while it plays,
# inserting and deleting tracks, dragging a header to reorder.
# (Below: bench_p2.ps1's own notes.)
# Polish 2 gate 6: bench.ps1's scenarios (idle, demo playing, 200 clips playing, 200-clip drag;
# process CPU, % of one core) and, with -New, the new gestures on the 200-clip set while it
# plays: a row-resize drag, a header-width drag, a ruler zoom drag, an Overview drag and an
# Overview zoom drag. Both builds also run the baseline's own continuous-redraw gestures
# (Ctrl+wheel zoom, a time-scrollbar drag) for a like-for-like comparison. Default settings (the polish 2 build shows the Overview and the status
# bar, so its rows start 32 px lower: the clip grab moves from (760, 140) to (760, 172)).
# Appends one line per run to <Work>\bench_p2_log.txt with proof the 200-clip set was on.
param([string]$Label = "run", [int]$Secs = 12, [string]$Repo, [switch]$Main, [string]$Only = "",
  [string]$Work = (Join-Path $env:TEMP "chipdaw-b4a-perf"))
$New = $true
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
function Want($n) { -not $Only -or ($Only.Split(",") -contains $n) }
function Cpu($name) { $global:Res[$name] = Measure-Cpu { param($i) } }
# A drag held for $Secs: press at (x, y), then sway by (ax, ay).
function CpuDrag($name, $x, $y, $ax, $ay) {
  Move-To $x $y; Down
  $global:Res[$name] = Measure-Cpu { param($i) Move-To ($x + [int]($ax * [Math]::Sin($i / 12.0))) ($y + [int]($ay * [Math]::Sin($i / 30.0))) }
  Up; Start-Sleep -Milliseconds 300
}
$dy = if ($New) { 32 } else { 0 }
Move-To 1300 800
if (Want "idle") { Cpu "idle" }
Key 0x20; Start-Sleep -Seconds 2
if (Want "demo") { Cpu "demo" }
Key 0x73; Start-Sleep -Seconds 3
if (Want "200") { Cpu "200" }
if (Want "200drag") { CpuDrag "200drag" 760 (140 + $dy) 120 64 }
# The baseline's own continuous-redraw gestures (both builds): Ctrl+wheel zoom over the lanes,
# a notch per frame in and out, and a drag of the time scrollbar (its thumb at the lanes' bottom:
# the window's client bottom, less the status bar on this branch).
if (Want "zoom_wheel") {
Move-To 900 (400 + $dy); KeyDown 0x11
$global:Res["zoom_wheel"] = Measure-Cpu { param($i) $d = if (($i % 8) -lt 4) { 120 } else { -120 }; [In]::mouse_event(0x0800, 0, 0, $d, [UIntPtr]::Zero) }
KeyUp 0x11; Start-Sleep -Milliseconds 300
}
$sb = 920 - 6 - 24 - $(if ($Main) { 73 } else { 0 })
if (Want "scrollbar_drag") { CpuDrag "scrollbar_drag" 650 $sb 160 0 }
# A window-width resize (the lanes' width changes every frame, as in a header-width drag).
Add-Type -Namespace P2 -Name Win -MemberDefinition '[DllImport("user32.dll")] public static extern bool GetWindowRect(IntPtr h, out RECT r); [DllImport("user32.dll")] public static extern bool SetWindowPos(IntPtr h, IntPtr a, int x, int y, int w, int hh, uint f); public struct RECT { public int L, T, R, B; }' -ErrorAction SilentlyContinue
$wr = New-Object P2.Win+RECT; [P2.Win]::GetWindowRect($global:AppH, [ref]$wr) | Out-Null
$W0 = $wr.R - $wr.L; $H0 = $wr.B - $wr.T
if (Want "window_resize") { $global:Res["window_resize"] = Measure-Cpu { param($i) [P2.Win]::SetWindowPos($global:AppH, [IntPtr]::Zero, $wr.L, $wr.T, $W0 - 60 + [int](60 * [Math]::Sin($i / 12.0)), $H0, 0x0014) | Out-Null }
[P2.Win]::SetWindowPos($global:AppH, [IntPtr]::Zero, $wr.L, $wr.T, $W0, $H0, 0x0014) | Out-Null; Start-Sleep -Milliseconds 500 }
if ($New -and -not $Only) {
  CpuDrag "row_resize" 441 (136 + 71) 0 60
  CpuDrag "header_width" 619 600 100 0
  CpuDrag "ruler_zoom" 900 122 6 70
  CpuDrag "overview_drag" 680 92 160 0
  CpuDrag "overview_zoom" 680 92 4 50
}
Stop-Process -Id $p.Id -Force
$err = Get-Content "$Work\bench_p2_err.txt" -Raw -ErrorAction SilentlyContinue
# The 64-studio-track project (this branch only: the baseline can't open format 7).
if ($Main -and -not $Only) {
  Get-Process -Name app -ErrorAction SilentlyContinue | Stop-Process -Force
  $env:CHIPDAW_PROJECT = (Join-Path $Repo "docs\batch4a\songs\stress64.chipdaw"); $env:CHIPDAW_AUTOPLAY = "1"
  $p = Start-Process ".\target\release\app.exe" -ArgumentList "--size", "1440x920" -PassThru -RedirectStandardError "$Work\bench_s64_err.txt"
  Start-Sleep -Seconds 4
  $global:AppH = $p.MainWindowHandle
  Remove-Item Env:CHIPDAW_PROJECT, Env:CHIPDAW_AUTOPLAY
  Move-To 1300 500
  Cpu "s64_playing"
  # Scrolling the lanes (wheel over the headers) while it plays: down, then up.
  Move-To 400 500
  $global:Res["s64_scroll"] = Measure-Cpu { param($i) $d = if (($i % 40) -lt 20) { -120 } else { 120 }; [In]::mouse_event(0x0800, 0, 0, $d, [UIntPtr]::Zero) }
  # Insert and delete tracks: Ctrl+Shift+T, then Delete (the new track is selected), repeatedly.
  Move-To 420 210; Down; Up; Start-Sleep -Milliseconds 300
  $global:Res["s64_insert_delete"] = Measure-Cpu { param($i) if (($i % 24) -eq 0) { KeyDown 0x11; KeyDown 0x10; Key 0x54; KeyUp 0x10; KeyUp 0x11 } elseif (($i % 24) -eq 12) { Key 0x2E } }
  # A header dragged up and down (reorder) while it plays.
  CpuDrag "s64_header_drag" 420 283 0 140
  Stop-Process -Id $p.Id -Force
  $e2 = Get-Content "$Work\bench_s64_err.txt" -Raw -ErrorAction SilentlyContinue
  $proof64 = if ($e2 -match "stress64.chipdaw \(64 clips\)" -and $e2 -match "insert track" -and $e2 -match "move track") { "64 tracks reached, inserts and moves seen" } else { "64-track scenario NOT REACHED" }
  $err += "`n" + $proof64
}
$proof = if ($err -match "dev: mode on" -and $err -match "dev: F4 stress set on") { "reached" } else { "NOT REACHED" }
$line = "{0,-16} " -f $Label + (($Res.Keys | ForEach-Object { "{0} {1,5}%" -f $_, $Res[$_] }) -join "  ") + "  stress set $proof" + $(if ($Main) { "  " + ($err -split "`n")[-1] } else { "" })
Add-Content "$Work\bench_b4a_log.txt" $line
$line
