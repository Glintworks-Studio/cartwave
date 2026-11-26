# usage: bench_pr.ps1 [-Label run] [-Secs 12] [-Exe .\target\release\app.exe] [-Repo <repo>] [-Work <dir>]
# Process CPU (% of one core) with the piano roll open on piano_bench.chipdaw
# (2,000 visible notes, 32 px per bar): playing, then dragging the handle note
# while playing. Same method as docs/perf/tools/bench.ps1. Generate the project
# first: cargo run --release -p ui --example make_songs -- <Work>
# -NoteY: the handle note's client y (535 up to Batch 3; 591 in the design pass's layout).
param([string]$Label = "run", [int]$Secs = 12, [int]$NoteY = 535, [string]$Exe = ".\target\release\app.exe",
  [string]$Repo = (Resolve-Path (Join-Path $PSScriptRoot "..\..\..")).Path,
  [string]$Work = $(if ($env:CHIPDAW_PERF_WORK) { $env:CHIPDAW_PERF_WORK } else { Join-Path $env:TEMP "chipdaw-perf" }))
$env:CHIPDAW_DEV = "1"   # F-02: test hooks (CHIPDAW_PROJECT, _STRESS, …) and F3-F7 need dev mode
Set-Location $Repo
Get-Process -Name app -ErrorAction SilentlyContinue | Stop-Process -Force
$env:CHIPDAW_PROJECT = Join-Path $Work "piano_bench.chipdaw"
$env:CHIPDAW_OPEN_CLIP = "1"; $env:CHIPDAW_PIANO_ZOOM = "32"; $env:CHIPDAW_DATA_DIR = Join-Path $Work "bench-data"
$p = Start-Process $Exe -PassThru -RedirectStandardError "$Work\bench_pr_err.txt"
Start-Sleep -Seconds 3
. "$Repo\docs\perf\tools\input.ps1"
function Cpu([scriptblock]$during) {
  $p.Refresh(); $c0 = $p.TotalProcessorTime; $t0 = [Diagnostics.Stopwatch]::StartNew()
  & $during $t0
  $p.Refresh(); [math]::Round(($p.TotalProcessorTime - $c0).TotalMilliseconds / $t0.Elapsed.TotalMilliseconds * 100, 1)
}
Move-To 700 300; Key 0x20; Start-Sleep -Seconds 2
$play = Cpu { param($t) Start-Sleep -Seconds $Secs }
Move-To 88 $NoteY; Down   # the C4 handle note (layout since the redesign; was 98, 591)
$drag = Cpu { param($t) $i = 0; while ($t.Elapsed.TotalSeconds -lt $Secs) { Move-To (88 + [int](12 * [Math]::Sin($i / 12.0))) ($NoteY + [int](48 * [Math]::Sin($i / 30.0))); $i++; Start-Sleep -Milliseconds 8 } }
Up
Stop-Process -Id $p.Id -Force
Remove-Item Env:CHIPDAW_PROJECT, Env:CHIPDAW_OPEN_CLIP, Env:CHIPDAW_PIANO_ZOOM, Env:CHIPDAW_DATA_DIR
# Proof the scenario ran (F-02): the project hook loaded the 2,000-note song and the
# piano roll opened on it.
$err = Get-Content "$Work\bench_pr_err.txt" -Raw -ErrorAction SilentlyContinue
$proof = if ($err -match "dev: CHIPDAW_PROJECT .*piano_bench" -and $err -match "dev: CHIPDAW_OPEN_CLIP 1 \(piano roll open: true\)") { "reached" } else { "NOT REACHED (dev-mode lines missing on stderr)" }
$line = "{0,-24} piano roll 2000 notes: playing {1,5}%  dragging {2,5}%  scenario {3}" -f $Label, $play, $drag, $proof
Add-Content "$Work\bench_log.txt" $line
$line
