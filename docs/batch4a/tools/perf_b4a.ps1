# usage: perf_b4a.ps1 [-Work <dir>] [-Rounds 3]
# Batch 4a gate 6: the baseline (main 0e669ac in ../before-batch4a) and this branch, interleaved:
# bench_b4a.ps1 (polish 2's scenarios; this branch adds the 64-track ones); then pacing at the
# 60 fps cap (demo, 200 clips, drag) on this branch; then the piano roll (2,000 notes: playing,
# dragging) for both, interleaved (its row in POLISH2.md).
param([string]$Work = (Join-Path $env:TEMP "chipdaw-b4a-perf"), [int]$Rounds = 3)
$Repo = (Resolve-Path (Join-Path $PSScriptRoot "..\..\..")).Path
$Base = (Resolve-Path (Join-Path $Repo "..\before-batch4a")).Path
New-Item -ItemType Directory -Force $Work | Out-Null
Copy-Item "$env:TEMP\chipdaw-perf-dp\piano_bench.chipdaw" $Work -Force
for ($i = 1; $i -le $Rounds; $i++) {
  pwsh -NoProfile -File "$PSScriptRoot\bench_b4a.ps1" -Label "base-$i" -Repo $Base -Work $Work | Out-Null
  pwsh -NoProfile -File "$PSScriptRoot\bench_b4a.ps1" -Label "b4a-$i" -Repo $Repo -Work $Work -Main | Out-Null
}
foreach ($sc in "demo", "stress", "drag") {
  pwsh -NoProfile -File "$Repo\docs\perf\tools\pacing.ps1" -Label "b4a-$sc" -Scenario $sc -Secs 30 -DragX 760 -DragY 172 -Repo $Repo -Work $Work *>> "$Work\pace_out.txt"
}
$env:CARTWAVE_PERF_WORK = $Work
for ($i = 1; $i -le $Rounds; $i++) {
  pwsh -NoProfile -File "$Repo\docs\pianoroll\tools\bench_pr.ps1" -Label "pr-base-$i" -Repo $Base -Work $Work -NoteY 623 | Out-Null
  pwsh -NoProfile -File "$Repo\docs\pianoroll\tools\bench_pr.ps1" -Label "pr-b4a-$i" -Repo $Repo -Work $Work -NoteY 623 | Out-Null
}
"done"
