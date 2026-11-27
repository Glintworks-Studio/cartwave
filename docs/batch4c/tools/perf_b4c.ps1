# usage: perf_b4c.ps1 [-Work <dir>] [-Rounds 10]
# Batch 4c gate 5 with batch 4b's method (perf_b4b.ps1, B4B-D26): three builds interleaved,
# -Rounds rounds, the order rotating each round: pre-batch4b (d4f1afd, ../before-batch4b),
# main (5533b5a, ../before-batch4c) and this branch. Each round, per build: bench_b4a.ps1 -Main
# (idle, demo, 200 clips, the gestures, the 64-track scenarios) and the piano roll (bench_pr);
# the mixer bench (bench_mixer) for main and this branch (pre-batch4b has no mixer). Judged on
# the means by perf_summary_b4c.py, which also reports the noise floor.
#
# pre-batch4b predates the rename: it is app.exe and reads only CHIPDAW_* names. As RN-D11
# describes, it runs as a copy named cartwave.exe (in its own folder, outside the repo) and the
# bench scripts run from a mirror of this repo's with every CARTWAVE_ name spelled CHIPDAW_
# (written to -Work each run), so all three builds run the same scenario code. Nothing else may
# run meanwhile: it drives the mouse and keys.
param([string]$Work = (Join-Path $env:TEMP "cartwave-b4c-perf"), [int]$Rounds = 10)
$Repo = (Resolve-Path (Join-Path $PSScriptRoot "..\..\..")).Path
$Main = (Resolve-Path (Join-Path $Repo "..\before-batch4c")).Path
$Pre = (Resolve-Path (Join-Path $Repo "..\before-batch4b")).Path
New-Item -ItemType Directory -Force $Work | Out-Null
Copy-Item "$env:TEMP\chipdaw-perf-dp\piano_bench.chipdaw" $Work -Force
if (-not (Test-Path "$Pre\target\release\cartwave.exe")) { Copy-Item "$Pre\target\release\app.exe" "$Pre\target\release\cartwave.exe" }
# The CHIPDAW_ mirror of the bench scripts, keeping their relative layout.
$Mirror = Join-Path $Work "mirror"
foreach ($f in "docs\perf\tools\input.ps1", "docs\batch4a\tools\bench_b4a.ps1", "docs\pianoroll\tools\bench_pr.ps1") {
  $dst = Join-Path $Mirror $f
  New-Item -ItemType Directory -Force (Split-Path $dst) | Out-Null
  $t = Get-Content (Join-Path $Repo $f) -Raw
  $t = $t.Replace("CARTWAVE_", "CHIPDAW_").Replace('. "$Repo\docs\perf\tools\input.ps1"', ". `"$Mirror\docs\perf\tools\input.ps1`"")
  Set-Content -Path $dst -Value $t -NoNewline
}
$env:CARTWAVE_PERF_WORK = $Work; $env:CHIPDAW_PERF_WORK = $Work
$builds = @(@("pre4b", $Pre, $Mirror), @("main", $Main, $Repo), @("b4c", $Repo, $Repo))
for ($i = 1; $i -le $Rounds; $i++) {
  # Rotate which build goes first, so a drift within a round favours none.
  $k = ($i - 1) % 3
  $order = @($builds[$k], $builds[($k + 1) % 3], $builds[($k + 2) % 3])
  foreach ($o in $order) { pwsh -NoProfile -File "$($o[2])\docs\batch4a\tools\bench_b4a.ps1" -Label "$($o[0])-$i" -Repo $o[1] -Work $Work -Main | Out-Null }
  foreach ($o in $order) { pwsh -NoProfile -File "$($o[2])\docs\pianoroll\tools\bench_pr.ps1" -Label "pr-$($o[0])-$i" -Repo $o[1] -Work $Work -NoteY 623 | Out-Null }
  foreach ($o in $order) { if ($o[0] -ne "pre4b") { pwsh -NoProfile -File "$Repo\docs\batch4b\tools\bench_mixer.ps1" -Label "mixer-$($o[0])-$i" -Repo $o[1] -Work $Work | Out-Null } }
  "round $i done"
}
python "$PSScriptRoot\perf_summary_b4c.py" $Work
