# usage: bench_b5.ps1 -Label <auto-(main|b5)-N> -Work <dir> -Fixtures <dir> [-Plain] [-Secs 12]
# Batch 5's automation scenarios (BATCH5_PROMPT.md §8.5) on this build, with the per-thread cycle
# counters and each scenario's window in <Work>\windows.tsv (perf_summary_b5.py). -Plain plays the
# same songs and lanes without envelopes: the like-for-like baseline ("main" in the summary's
# pairing; main itself can't open format 10). Scenarios:
#   auto_playing    auto32 (32 automated parameters on 8 tracks, studio and locked, plus Main) playing
#   auto_scroll     Automation Mode on while it plays: the lanes scrolled with the wheel
#   auto_timesel    Automation Mode: a time selection dragged in track 0's first lane
#   auto_draw_2000  auto2000 (one lane of 2,000 breakpoints): Draw Mode drawing on it
# Proof per run: the dev lines (automation mode, the point count) in the kept stderr.
param([string]$Label = "auto-b5-1", [string]$Work, [string]$Fixtures, [switch]$Plain, [int]$Secs = 12)
$Repo = (Resolve-Path (Join-Path $PSScriptRoot "..\..\..")).Path
foreach ($k in "CARTWAVE_OVERVIEW", "CARTWAVE_HIDE_STATUS", "CARTWAVE_PROJECT", "CARTWAVE_FROZEN_METERS", "CARTWAVE_BROWSER_W", "CARTWAVE_AUDIOLOG", "CARTWAVE_FRAMELOG", "CARTWAVE_AUTOPLAY") { Remove-Item "Env:$k" -ErrorAction SilentlyContinue }
$env:CARTWAVE_DEV = "1"
$Windows = Join-Path $Work "windows.tsv"
if (-not (Test-Path $Windows)) { "label`tlog`tscenario`tt0`tt1" | Set-Content $Windows }
. "$Repo\docs\perf\tools\input.ps1"
Add-Type -Namespace B5 -Name Fg -MemberDefinition '[DllImport("user32.dll")] public static extern IntPtr GetForegroundWindow(); [DllImport("user32.dll")] public static extern bool SetForegroundWindow(IntPtr h); [DllImport("user32.dll")] public static extern bool ShowWindow(IntPtr h, int c);' -ErrorAction SilentlyContinue
function App-Front { [B5.Fg]::GetForegroundWindow() -eq $global:AppH }
function Focus-App { for ($t = 0; $t -lt 5 -and -not (App-Front); $t++) { [In]::keybd_event(0x12, 0, 0, [UIntPtr]::Zero); [In]::keybd_event(0x12, 0, 2, [UIntPtr]::Zero); [B5.Fg]::ShowWindow($global:AppH, 5) | Out-Null; [B5.Fg]::SetForegroundWindow($global:AppH) | Out-Null; Start-Sleep -Milliseconds 200 }; App-Front }
$global:FocusLost = 0
$global:Res = [ordered]@{}
function Measure-Cpu([scriptblock]$during) {
  $p.Refresh(); $c0 = $p.TotalProcessorTime; $t0 = [Diagnostics.Stopwatch]::StartNew(); $i = 0
  $w0 = ((Get-Date) - $p.StartTime).TotalSeconds
  while ($t0.Elapsed.TotalSeconds -lt $Secs) { & $during $i; $i++; Start-Sleep -Milliseconds 8 }
  $w1 = ((Get-Date) - $p.StartTime).TotalSeconds
  $global:LastWindow = "{0:F2}`t{1:F2}" -f $w0, $w1
  $p.Refresh(); [math]::Round(($p.TotalProcessorTime - $c0).TotalMilliseconds / $t0.Elapsed.TotalMilliseconds * 100, 1)
}
function Note($name) { Add-Content $Windows "$Label`t$global:LogTag`t$name`t$global:LastWindow" }
function Cpu($name) { $global:Res[$name] = Measure-Cpu { param($i) }; Note $name }
function CpuDrag($name, $x, $y, $ax, $ay) { Move-To $x $y; Down; $global:Res[$name] = Measure-Cpu { param($i) Move-To ($x + [int]($ax * [Math]::Sin($i / 12.0))) ($y + [int]($ay * [Math]::Sin($i / 30.0))) }; Note $name; Up; Start-Sleep -Milliseconds 300 }
function Start-App($song, $tag) {
  Get-Process -Name cartwave -ErrorAction SilentlyContinue | Stop-Process -Force
  $global:LogTag = $tag
  $env:CARTWAVE_AUDIOLOG = Join-Path $Work "$Label-$tag.csv"; $env:CARTWAVE_FRAMELOG = Join-Path $Work "$Label-$tag.frames"
  $d = Join-Path $Work "data-$Label-$tag"; Remove-Item -Recurse -Force $d -ErrorAction SilentlyContinue; New-Item -ItemType Directory -Force $d | Out-Null
  $env:CARTWAVE_DATA_DIR = $d
  $env:CARTWAVE_PROJECT = $song; $env:CARTWAVE_AUTOPLAY = "1"
  $script:p = Start-Process (Join-Path $Repo "target\release\cartwave.exe") -ArgumentList "--size", "1440x920" -PassThru -RedirectStandardError (Join-Path $Work "err-$Label-$tag.txt")
  Start-Sleep -Seconds 4
  $script:p.Refresh(); $global:AppH = $script:p.MainWindowHandle
  Remove-Item Env:CARTWAVE_PROJECT, Env:CARTWAVE_AUTOPLAY
  if (-not (Focus-App)) { $global:FocusLost++ }
}
$suffix = if ($Plain) { "_plain" } else { "" }
# 32 automated parameters, playing; then Automation Mode (scroll, a lane's time selection).
Start-App (Join-Path $Fixtures "auto32$suffix.cartwave") "a32"
Move-To 1300 500
Cpu "auto_playing"
if (App-Front) { Key 0x41 } else { $global:FocusLost++ }; Start-Sleep -Milliseconds 600   # A
Move-To 400 500
$global:Res["auto_scroll"] = Measure-Cpu { param($i) $d = if (($i % 40) -lt 20) { -120 } else { 120 }; [In]::mouse_event(0x0800, 0, 0, $d, [UIntPtr]::Zero) }; Note "auto_scroll"
for ($k = 0; $k -lt 80; $k++) { [In]::mouse_event(0x0800, 0, 0, 120, [UIntPtr]::Zero); Start-Sleep -Milliseconds 5 }
Start-Sleep -Milliseconds 400
# Track 0's first lane (Duty, stepped 0..3): 209..282. Pressed in its step-0 band, below the line
# in both songs (their values stay at 1 or more), so the drag selects time. y 214 hit the automated
# song's line and dragged a new breakpoint instead (its proof line caught it; B5-D14).
CpuDrag "auto_timesel" 1000 276 160 0
Stop-Process -Id $script:p.Id -Force
# One lane of 2,000 breakpoints: Draw Mode drawing on it.
Start-App (Join-Path $Fixtures "auto2000$suffix.cartwave") "a2000"
if (App-Front) { Key 0x41; Start-Sleep -Milliseconds 400; Key 0x42 } else { $global:FocusLost++ }; Start-Sleep -Milliseconds 600   # A, then B
CpuDrag "auto_draw_2000" 1000 245 200 25
Stop-Process -Id $script:p.Id -Force
$e1 = Get-Content (Join-Path $Work "err-$Label-a32.txt") -Raw -ErrorAction SilentlyContinue
$e2 = Get-Content (Join-Path $Work "err-$Label-a2000.txt") -Raw -ErrorAction SilentlyContinue
$proof = if ($e1 -match "dev: automation mode true lanes (\d+)") { "a32 lanes $($Matches[1])" } else { "a32 Automation Mode NOT REACHED" }
$proof += if ($e1 -match "dev: automation time selection (\d+) (\d+)") { "; time selection $($Matches[1])..$($Matches[2])" } else { "; time selection NOT REACHED" }
$proof += if ($e2 -match "dev: automation points (\d+)") { "; a2000 points after drawing $($Matches[1])" } else { "; a2000 drawing NOT REACHED" }
$line = "{0,-16} " -f $Label + (($Res.Keys | ForEach-Object { "{0} {1,5}%" -f $_, $Res[$_] }) -join "  ") + "  $proof" + $(if ($global:FocusLost) { "  FOCUS LOST x$($global:FocusLost): INVALID" } else { "  focus held" })
Add-Content (Join-Path $Work "bench_b5_log.txt") $line
$line
