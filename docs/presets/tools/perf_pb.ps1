# usage: perf_pb.ps1 [-Work <dir>] [-Rounds 10]
# The Presets + Browser batch's performance gate (PRESETS_PROMPT.md §2, §8.6; ENGINE.md §7b):
# main (1b8a288, ../before-presets) and this branch interleaved, -Rounds rounds, the order
# alternating each round. Per round and build:
#   - bench_pb.ps1 -Main: Batch 4c's scenarios (idle, demo, 200 clips, the gestures, the
#     64-track ones), this branch with the dev hook CARTWAVE_BROWSER_W=240 so the coordinates
#     and the arrangement's width are main's (RUNLOG PB-D17);
#   - bench_pr.ps1: the piano roll playing and dragging;
#   - bench_mixer.ps1: the 64-track mixer (with the same hook);
#   - bench_pb.ps1 -NewOnly: the device panel (main: the inspector) open while playing, the
#     browser searched and scrolled (100+ presets here), and on this branch ◀ ▶ stepping.
# Every run logs per-thread cycle counters (CARTWAVE_AUDIOLOG / FRAMELOG) and each scenario's
# window; perf_summary_pb.py averages UI kcycles per frame and audio kcycles per callback per
# scenario (the primary measure), process CPU beside them, and judges by the §2 rule.
# Nothing else may run meanwhile: it drives the mouse and keys.
param([string]$Work = (Join-Path $env:TEMP "cartwave-pb-perf"), [int]$Rounds = 10)
$Repo = (Resolve-Path (Join-Path $PSScriptRoot "..\..\..")).Path
$Main = (Resolve-Path (Join-Path $Repo "..\before-presets")).Path
New-Item -ItemType Directory -Force $Work | Out-Null
if (-not (Test-Path (Join-Path $Work "piano_bench.chipdaw"))) {
  if (Test-Path "$env:TEMP\chipdaw-perf-dp\piano_bench.chipdaw") { Copy-Item "$env:TEMP\chipdaw-perf-dp\piano_bench.chipdaw" $Work -Force }
  else { Push-Location $Repo; & "$env:USERPROFILE\.cargo\bin\cargo.exe" run --release -q -p ui --example make_songs -- $Work | Out-Null; Pop-Location }
}
$env:CARTWAVE_PERF_WORK = $Work
$builds = @(@("main", $Main, 0), @("pb", $Repo, 240))
function Logs($tag) { $env:CARTWAVE_AUDIOLOG = Join-Path $Work "$tag.csv"; $env:CARTWAVE_FRAMELOG = Join-Path $Work "$tag.frames" }
function Clear-Logs { Remove-Item Env:CARTWAVE_AUDIOLOG, Env:CARTWAVE_FRAMELOG, Env:CARTWAVE_BROWSER_W -ErrorAction SilentlyContinue }
for ($i = 1; $i -le $Rounds; $i++) {
  $order = if ($i % 2) { @($builds[0], $builds[1]) } else { @($builds[1], $builds[0]) }
  foreach ($o in $order) {
    $name, $root, $bw = $o
    pwsh -NoProfile -File "$PSScriptRoot\bench_pb.ps1" -Label "$name-$i" -Repo $root -Work $Work -Main -BrowserW $bw | Out-Null
  }
  foreach ($o in $order) {
    $name, $root, $bw = $o
    if ($bw) { $env:CARTWAVE_BROWSER_W = "$bw" }
    Logs "pr-$name-$i"
    pwsh -NoProfile -File "$Repo\docs\pianoroll\tools\bench_pr.ps1" -Label "pr-$name-$i" -Repo $root -Work $Work -Exe (Join-Path $root "target\release\cartwave.exe") -NoteY 623 | Out-Null
    Clear-Logs
  }
  foreach ($o in $order) {
    $name, $root, $bw = $o
    if ($bw) { $env:CARTWAVE_BROWSER_W = "$bw" }
    Logs "mixer-$name-$i"
    pwsh -NoProfile -File "$Repo\docs\batch4b\tools\bench_mixer.ps1" -Label "mixer-$name-$i" -Repo $root -Work $Work | Out-Null
    Clear-Logs
  }
  foreach ($o in $order) {
    $name, $root, $bw = $o
    $pbArgs = @("-Label", "new-$name-$i", "-Repo", $root, "-Work", $Work, "-NewOnly")
    if ($name -eq "pb") { $pbArgs += "-Pb" }
    pwsh -NoProfile -File "$PSScriptRoot\bench_pb.ps1" @pbArgs | Out-Null
  }
  "round $i done"
}
python "$PSScriptRoot\perf_summary_pb.py" $Work
