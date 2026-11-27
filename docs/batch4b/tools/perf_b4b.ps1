# usage: perf_b4b.ps1 [-Work <dir>] [-Rounds 10]
# Batch 4b gate 6 with the §3.13 method: the baseline (main d4f1afd in ../before-batch4b) and this
# branch interleaved, -Rounds rounds, each round: bench_b4a.ps1 -Main for both (the baseline has
# the Main row too, so both use the same geometry), the piano roll (2,000 notes) for both, and
# the new mixer scenarios (this branch only). Judged on the means by perf_summary_b4b.py, which
# also reports the noise floor. Nothing else may run meanwhile: it drives the mouse and keys.
param([string]$Work = (Join-Path $env:TEMP "chipdaw-b4b-perf"), [int]$Rounds = 10)
$Repo = (Resolve-Path (Join-Path $PSScriptRoot "..\..\..")).Path
$Base = (Resolve-Path (Join-Path $Repo "..\before-batch4b")).Path
New-Item -ItemType Directory -Force $Work | Out-Null
Copy-Item "$env:TEMP\chipdaw-perf-dp\piano_bench.chipdaw" $Work -Force
$env:CARTWAVE_PERF_WORK = $Work
for ($i = 1; $i -le $Rounds; $i++) {
  # Alternate which build goes first, so a drift within a round favours neither.
  $order = if ($i % 2) { @(@("base", $Base), @("b4b", $Repo)) } else { @(@("b4b", $Repo), @("base", $Base)) }
  foreach ($o in $order) { pwsh -NoProfile -File "$Repo\docs\batch4a\tools\bench_b4a.ps1" -Label "$($o[0])-$i" -Repo $o[1] -Work $Work -Main | Out-Null }
  foreach ($o in $order) { pwsh -NoProfile -File "$Repo\docs\pianoroll\tools\bench_pr.ps1" -Label "pr-$($o[0])-$i" -Repo $o[1] -Work $Work -NoteY 623 | Out-Null }
  pwsh -NoProfile -File "$PSScriptRoot\bench_mixer.ps1" -Label "mixer-$i" -Repo $Repo -Work $Work | Out-Null
}
python "$PSScriptRoot\perf_summary_b4b.py" $Work
