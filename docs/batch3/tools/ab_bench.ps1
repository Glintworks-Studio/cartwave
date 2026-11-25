# usage: ab_bench.ps1 [-Rounds 3] [-Before ..\prerun-b3] [-Work <dir>]
# PERF.md UI scenarios, A/B: alternates bench.ps1 between the pre-run build (a worktree of
# the commit this batch started from) and this branch's build, -Rounds times each. Every run
# appends a line to <Work>\bench_log.txt (bench.ps1's format, with its "reached" proof).
param([int]$Rounds = 3, [string]$Before = "..\prerun-b3", [string]$Work = (Join-Path $env:TEMP "chipdaw-b3-ab"), [string]$Env = "")
$Repo = (Resolve-Path (Join-Path $PSScriptRoot "..\..\..")).Path
$BeforeRepo = (Resolve-Path (Join-Path $Repo $Before)).Path
New-Item -ItemType Directory -Force $Work | Out-Null
for ($i = 1; $i -le $Rounds; $i++) {
  pwsh -NoProfile -File "$Repo\docs\perf\tools\bench.ps1" -Label "before-$i" -Repo $BeforeRepo -Work $Work -Env $Env
  pwsh -NoProfile -File "$Repo\docs\perf\tools\bench.ps1" -Label "batch3-$i" -Repo $Repo -Work $Work -Env $Env
}
Get-Content (Join-Path $Work "bench_log.txt")
