# usage: perf_p2.ps1 [-Work <dir>] [-Rounds 3]
# Polish 2 gate 6: the baseline (`main` 8644ff3 in ../before-polish2) and this branch,
# interleaved: bench_p2.ps1 (idle, demo, 200 clips, 200-clip drag; and on this branch the new
# gestures), then pacing at the 60 fps cap (demo, 200 clips, drag) on this branch, then the
# piano roll (2,000 notes: playing, dragging) for both, interleaved: its own row (§9.6).
# Default settings: this branch shows the Overview and the status bar (its rows and piano
# roll sit 32 px lower: the clip grab is (760, 172), the piano-roll handle note y 623).
param([string]$Work = (Join-Path $env:TEMP "chipdaw-p2-perf"), [int]$Rounds = 3)
$Repo = (Resolve-Path (Join-Path $PSScriptRoot "..\..\..")).Path
$Base = (Resolve-Path (Join-Path $Repo "..\before-polish2")).Path
New-Item -ItemType Directory -Force $Work | Out-Null
Copy-Item "$env:TEMP\chipdaw-perf-dp\piano_bench.chipdaw" $Work -Force
for ($i = 1; $i -le $Rounds; $i++) {
  pwsh -NoProfile -File "$PSScriptRoot\bench_p2.ps1" -Label "base-$i" -Repo $Base -Work $Work | Out-Null
  pwsh -NoProfile -File "$PSScriptRoot\bench_p2.ps1" -Label "p2-$i" -Repo $Repo -Work $Work -New | Out-Null
}
foreach ($sc in "demo", "stress", "drag") {
  pwsh -NoProfile -File "$Repo\docs\perf\tools\pacing.ps1" -Label "p2-$sc" -Scenario $sc -Secs 30 -DragX 760 -DragY 172 -Repo $Repo -Work $Work *>> "$Work\pace_out.txt"
}
$env:CHIPDAW_PERF_WORK = $Work
for ($i = 1; $i -le $Rounds; $i++) {
  pwsh -NoProfile -File "$Repo\docs\pianoroll\tools\bench_pr.ps1" -Label "pr-base-$i" -Repo $Base -Work $Work -NoteY 591 | Out-Null
  pwsh -NoProfile -File "$Repo\docs\pianoroll\tools\bench_pr.ps1" -Label "pr-p2-$i" -Repo $Repo -Work $Work -NoteY 623 | Out-Null
}
"done"
