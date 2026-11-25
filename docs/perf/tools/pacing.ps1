# usage: pacing.ps1 [-Label run] [-Scenario demo|stress|drag|pr|prdrag] [-Secs 30] [-Env "K=V;..."] [-Exe .\target\release\app.exe] [-Repo <repo>] [-Work <dir>]
# Needs ffmpeg (with ddagrab) and python on PATH. -Exe is relative to -Repo.
# Defaults: -Repo = this script's repo; -Work = $env:CHIPDAW_PERF_WORK or %TEMP%\chipdaw-perf.
# Set $env:CHIPDAW_REFRESH_HZ to the display refresh rate (default 59.951) for pacing_analyze.py.
param(
  [string]$Label = "run",
  [ValidateSet("demo", "stress", "drag", "pr", "prdrag")][string]$Scenario = "demo",
  [int]$Secs = 30,
  [string]$Env = "",
  [string]$Exe = ".\target\release\app.exe",
  [string]$Repo = (Resolve-Path (Join-Path $PSScriptRoot "..\..\..")).Path,
  [string]$Work = $(if ($env:CHIPDAW_PERF_WORK) { $env:CHIPDAW_PERF_WORK } else { Join-Path $env:TEMP "chipdaw-perf" })
)
$env:CHIPDAW_DEV = "1"   # F-02: test hooks (CHIPDAW_PROJECT, _STRESS, …) and F3-F7 need dev mode
# 30 s frame-pacing run: app frame log (submit intervals, per-phase timings) +
# DXGI desktop-duplication capture (ffmpeg ddagrab, dup_frames=0, no encode) to
# count display refreshes that showed no new frame.
New-Item -ItemType Directory -Force $Work | Out-Null
Set-Location $Repo
$S = $Work
Get-Process -Name app -ErrorAction SilentlyContinue | Stop-Process -Force
if ($Env) { foreach ($kv in $Env.Split(';')) { $k, $v = $kv.Split('='); Set-Item "Env:$k" $v } }
$log = "$S\pace_$Label`_$Scenario.csv"
$env:CHIPDAW_FRAMELOG = $log
"exe: $Exe"
$p = Start-Process $Exe -PassThru -RedirectStandardError "$S\pace_err.txt"
Start-Sleep -Seconds 3
. "$PSScriptRoot\input.ps1"
Move-To 1430 890
Key 0x20
# pr / prdrag: the piano-roll benchmark (PIANOROLL.md §8); pass its project via -Env.
if ($Scenario -in "stress", "drag") { Key 0x73 }
Start-Sleep -Seconds 2
$ff = Start-Process ffmpeg -ArgumentList "-hide_banner -loglevel info -f lavfi -i ddagrab=framerate=240:dup_frames=0:draw_mouse=0 -t $Secs -vf showinfo -f null -" -PassThru -NoNewWindow -RedirectStandardError "$S\pace_showinfo.txt"
$p.Refresh(); $c0 = $p.TotalProcessorTime; $t0 = [Diagnostics.Stopwatch]::StartNew()
if ($Scenario -eq "drag") {
  Move-To 720 130; Down   # a clip in the 200-clip view (300, 160 is a track header since the redesign)
  $i = 0
  while (-not $ff.HasExited) { Move-To (720 + [int](120 * [Math]::Sin($i / 12.0))) (130 + [int](64 * [Math]::Sin($i / 30.0))); $i++; Start-Sleep -Milliseconds 8 }
  Up
}
if ($Scenario -eq "prdrag") {
  # The handle note (C4, bar 1) of piano_bench.chipdaw at 32 px per bar.
  Move-To 88 535; Down   # the C4 handle note (layout since the redesign; was 98, 591)
  $i = 0
  while (-not $ff.HasExited) { Move-To (88 + [int](12 * [Math]::Sin($i / 12.0))) (535 + [int](48 * [Math]::Sin($i / 30.0))); $i++; Start-Sleep -Milliseconds 8 }
  Up
}
$ff.WaitForExit()
$p.Refresh(); $cpu = ($p.TotalProcessorTime - $c0).TotalMilliseconds / $t0.Elapsed.TotalMilliseconds * 100
Stop-Process -Id $p.Id -Force
$env:CHIPDAW_FRAMELOG = ""
if ($Env) { foreach ($kv in $Env.Split(';')) { $k = $kv.Split('=')[0]; Remove-Item "Env:$k" -ErrorAction SilentlyContinue } }
Select-String -Path "$S\pace_showinfo.txt" -Pattern "pts_time:\s*([0-9.]+)" | ForEach-Object { $_.Matches[0].Groups[1].Value } | Set-Content "$S\pace_pts.txt"
python -I "$PSScriptRoot\pacing_analyze.py" $log "$S\pace_pts.txt" $Secs ("{0:N1}" -f $cpu) "$Label/$Scenario"
# Proof the scenario ran (F-02): dev mode on (the frame log needs it), F4 for stress/drag.
$err = Get-Content "$S\pace_err.txt" -Raw -ErrorAction SilentlyContinue
$ok = ($err -match "dev: mode on") -and (Test-Path $log) -and ((Get-Content $log | Measure-Object -Line).Lines -gt 100)
if ($Scenario -in "stress", "drag") { $ok = $ok -and ($err -match "dev: F4 stress set on") }
if ($Scenario -in "pr", "prdrag") { $ok = $ok -and ($err -match "dev: CHIPDAW_OPEN_CLIP \d+ \(piano roll open: true\)") }
"scenario $Scenario " + $(if ($ok) { "reached" } else { "NOT REACHED (dev-mode lines missing on stderr, or no frame log)" })
