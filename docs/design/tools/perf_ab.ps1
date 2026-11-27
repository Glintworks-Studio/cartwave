# usage: perf_ab.ps1 [-Before ..\prerun-dp] [-Work <dir>]
# Design pass gate 7: PERF.md's UI scenarios A/B against the pre-run build (3 rounds),
# pacing at the 60 fps cap (demo, 200 clips, drag; 30 s), and the piano-roll bench A/B.
param([string]$Before = "..\prerun-dp", [string]$Work = (Join-Path $env:TEMP "chipdaw-dp-perf"))
$Repo = (Resolve-Path (Join-Path $PSScriptRoot "..\..\..")).Path
$BeforeRepo = (Resolve-Path (Join-Path $Repo $Before)).Path
New-Item -ItemType Directory -Force $Work | Out-Null
for ($i = 1; $i -le 3; $i++) {
  pwsh -NoProfile -File "$Repo\docs\perf\tools\bench.ps1" -Label "before-$i" -Repo $BeforeRepo -Work $Work
  pwsh -NoProfile -File "$Repo\docs\perf\tools\bench.ps1" -Label "design-$i" -Repo $Repo -Work $Work
}
foreach ($sc in "demo", "stress", "drag") {
  pwsh -NoProfile -File "$Repo\docs\perf\tools\pacing.ps1" -Label "before-$sc" -Scenario $sc -Secs 30 -Repo $BeforeRepo -Work $Work
  pwsh -NoProfile -File "$Repo\docs\perf\tools\pacing.ps1" -Label "design-$sc" -Scenario $sc -Secs 30 -Repo $Repo -Work $Work
}
$env:CARTWAVE_PERF_WORK = $Work
Copy-Item "$env:TEMP\chipdaw-perf-dp\piano_bench.chipdaw" $Work -Force
for ($i = 1; $i -le 2; $i++) {
  pwsh -NoProfile -File "$Repo\docs\pianoroll\tools\bench_pr.ps1" -Label "pr-before-$i" -Repo $BeforeRepo -Work $Work -NoteY 535
  pwsh -NoProfile -File "$Repo\docs\pianoroll\tools\bench_pr.ps1" -Label "pr-design-$i" -Repo $Repo -Work $Work -NoteY 591
}
Get-ChildItem $Work -Filter *.txt | ForEach-Object { "== $($_.Name)"; Get-Content $_.FullName | Select-Object -Last 12 }
