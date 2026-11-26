# usage: perf_polish.ps1 [-Work <dir>]
# Polish gate 4: PERF.md's UI scenarios, three builds interleaved, 3 rounds: the pre-design
# build (../prerun-dp, main 23b5da9), design/option-d (../before-polish: the design pass plus
# the dev hooks only) and this branch. Then pacing at the 60 fps cap and the piano-roll bench
# for this branch against design/option-d.
param([string]$Work = (Join-Path $env:TEMP "chipdaw-polish-perf"))
$Repo = (Resolve-Path (Join-Path $PSScriptRoot "..\..\..")).Path
$Pre = (Resolve-Path (Join-Path $Repo "..\prerun-dp")).Path
$Design = (Resolve-Path (Join-Path $Repo "..\before-polish")).Path
New-Item -ItemType Directory -Force $Work | Out-Null
Copy-Item "$env:TEMP\chipdaw-perf-dp\piano_bench.chipdaw" $Work -Force
for ($i = 1; $i -le 3; $i++) {
  pwsh -NoProfile -File "$Repo\docs\perf\tools\bench.ps1" -Label "pre-$i" -Repo $Pre -Work $Work | Out-Null
  # The same grab in each layout (pacing.ps1's -DragX note): a clip moved, not resized.
  pwsh -NoProfile -File "$Repo\docs\perf\tools\bench.ps1" -Label "design-$i" -Repo $Design -Work $Work -DragX 760 -DragY 140 | Out-Null
  pwsh -NoProfile -File "$Repo\docs\perf\tools\bench.ps1" -Label "polish-$i" -Repo $Repo -Work $Work -DragX 760 -DragY 140 | Out-Null
}
foreach ($sc in "demo", "stress", "drag") {
  pwsh -NoProfile -File "$Repo\docs\perf\tools\pacing.ps1" -Label "polish-$sc" -Scenario $sc -Secs 30 -DragX 760 -DragY 140 -Repo $Repo -Work $Work *>> "$Work\pace_out.txt"
}
$env:CHIPDAW_PERF_WORK = $Work
for ($i = 1; $i -le 2; $i++) {
  pwsh -NoProfile -File "$Repo\docs\pianoroll\tools\bench_pr.ps1" -Label "pr-design-$i" -Repo $Design -Work $Work -NoteY 591 | Out-Null
  pwsh -NoProfile -File "$Repo\docs\pianoroll\tools\bench_pr.ps1" -Label "pr-polish-$i" -Repo $Repo -Work $Work -NoteY 591 | Out-Null
}
"done"
