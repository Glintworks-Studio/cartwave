# usage: ab5_b4a.ps1 [-Work <dir>] [-Rounds 5]
# Batch 4a gate 6, the dedicated A/B for borderline scenarios (polish 2's precedent): the
# baseline and this branch interleaved, -Rounds rounds of bench_b4a.ps1 (all polish 2 scenarios)
# and bench_pr.ps1 (the piano roll, 2,000 notes). Logs: bench_b4a_log.txt, bench_log.txt.
param([string]$Work = (Join-Path $env:TEMP "chipdaw-b4a-ab5"), [int]$Rounds = 5)
$Repo = (Resolve-Path (Join-Path $PSScriptRoot "..\..\..")).Path
$Base = (Resolve-Path (Join-Path $Repo "..\before-batch4a")).Path
New-Item -ItemType Directory -Force $Work | Out-Null
Copy-Item "$env:TEMP\chipdaw-perf-dp\piano_bench.chipdaw" $Work -Force
$env:CHIPDAW_PERF_WORK = $Work
for ($i = 1; $i -le $Rounds; $i++) {
  pwsh -NoProfile -File "$PSScriptRoot\bench_b4a.ps1" -Label "base-$i" -Repo $Base -Work $Work | Out-Null
  pwsh -NoProfile -File "$PSScriptRoot\bench_b4a.ps1" -Label "b4a-$i" -Repo $Repo -Work $Work -Main | Out-Null
  pwsh -NoProfile -File "$Repo\docs\pianoroll\tools\bench_pr.ps1" -Label "pr-base-$i" -Repo $Base -Work $Work -NoteY 623 | Out-Null
  pwsh -NoProfile -File "$Repo\docs\pianoroll\tools\bench_pr.ps1" -Label "pr-b4a-$i" -Repo $Repo -Work $Work -NoteY 623 | Out-Null
}
"done"
