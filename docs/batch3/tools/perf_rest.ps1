# usage: perf_rest.ps1 [-Before ..\prerun-b3] [-Work <dir>]
# The rest of gate 3 after ab_bench.ps1: pacing (demo, 200 clips, drag; 30 s), the piano-roll
# bench (2,000 notes) A/B against the pre-run build, and bench.ps1 at 150% interface scale
# (a data dir whose settings.json sets ui_scale 150; the drag coordinates scaled by 1.5).
param([string]$Before = "..\prerun-b3", [string]$Work = (Join-Path $env:TEMP "chipdaw-b3-perf"))
$Repo = (Resolve-Path (Join-Path $PSScriptRoot "..\..\..")).Path
$BeforeRepo = (Resolve-Path (Join-Path $Repo $Before)).Path
New-Item -ItemType Directory -Force $Work | Out-Null
foreach ($sc in "demo", "stress", "drag") {
  pwsh -NoProfile -File "$Repo\docs\perf\tools\pacing.ps1" -Label "batch3-$sc" -Scenario $sc -Secs 30 -Repo $Repo -Work $Work
}
for ($i = 1; $i -le 2; $i++) {
  pwsh -NoProfile -File "$Repo\docs\pianoroll\tools\bench_pr.ps1" -Label "pr-before-$i" -Repo $BeforeRepo -Work "$env:TEMP\b3pr_before"
  pwsh -NoProfile -File "$Repo\docs\pianoroll\tools\bench_pr.ps1" -Label "pr-batch3-$i" -Repo $Repo -Work "$env:TEMP\b3pr"
}
$data = Join-Path $env:TEMP "b3s150"
for ($i = 1; $i -le 2; $i++) {
  pwsh -NoProfile -File "$Repo\docs\perf\tools\bench.ps1" -Label "scale150-$i" -Repo $Repo -Work $Work -Env "CARTWAVE_DATA_DIR=$data" -DragX 1080 -DragY 195
}
"--- pacing / bench (150%) ---"; Get-Content (Join-Path $Work "*.txt") -ErrorAction SilentlyContinue | Select-Object -Last 30
"--- piano roll ---"; Get-Content "$env:TEMP\b3pr_before\*.txt", "$env:TEMP\b3pr\*.txt" -ErrorAction SilentlyContinue | Select-Object -Last 10
