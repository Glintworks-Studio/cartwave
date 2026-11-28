# usage: perf_b5.ps1 [-Work <dir>] [-Rounds 10]
# Batch 5's performance gate (BATCH5_PROMPT.md §8.5; ENGINE.md §7b): main (4529f1e,
# ../before-batch5) and this branch interleaved, -Rounds rounds, the order alternating each
# round. Per round and build, the Presets batch's scenarios (perf_pb.ps1: bench_pb.ps1 -Main, the
# piano roll, the 64-track mixer, bench_pb.ps1 -NewOnly with ◀ ▶), this branch with the dev hook
# CARTWAVE_NO_AUTO_KEYS=1 (the transport's two automation keys left out, so the top bar's
# coordinates are main's; RUNLOG B5-D12); then the automation scenarios (bench_b5.ps1) on this
# build, the plain songs ("main" in the pairing) against the automated ones. perf_summary_b5.py
# judges by the §7b rule. Nothing else may run meanwhile: it drives the mouse and keys.
param([string]$Work = (Join-Path $env:TEMP "cartwave-b5-perf"), [int]$Rounds = 10)
$Repo = (Resolve-Path (Join-Path $PSScriptRoot "..\..\..")).Path
$Main = (Resolve-Path (Join-Path $Repo "..\before-batch5")).Path
$Pb = Join-Path $Repo "docs\presets\tools"
New-Item -ItemType Directory -Force $Work | Out-Null
$Fix = Join-Path $Work "fixtures"
if (-not (Test-Path (Join-Path $Fix "auto32.cartwave"))) { Push-Location $Repo; & "$env:USERPROFILE\.cargo\bin\cargo.exe" run --release -q -p ui --example auto_fixture -- $Fix; Pop-Location }
# The piano roll's song from main's make_songs (format 9: both builds open it). This branch's
# writes format 10, which main refused, so the first runs' pr_* pairs compared other songs (B5-D14).
$PrSong = Join-Path $Work "piano_bench.chipdaw"
if ((Test-Path $PrSong) -and ((Get-Content $PrSong -TotalCount 5 | Out-String) -notmatch '"version":\s*9\b')) { Remove-Item $PrSong }
if (-not (Test-Path $PrSong)) { Push-Location $Main; & "$env:USERPROFILE\.cargo\bin\cargo.exe" run --release -q -p ui --example make_songs -- $Work | Out-Null; Pop-Location }
$env:CARTWAVE_PERF_WORK = $Work
$builds = @(@("main", $Main), @("b5", $Repo))
function Hook($name) { if ($name -eq "b5") { $env:CARTWAVE_NO_AUTO_KEYS = "1" } else { Remove-Item Env:CARTWAVE_NO_AUTO_KEYS -ErrorAction SilentlyContinue } }
function Logs($tag) { $env:CARTWAVE_AUDIOLOG = Join-Path $Work "$tag.csv"; $env:CARTWAVE_FRAMELOG = Join-Path $Work "$tag.frames" }
function Clear-Logs { Remove-Item Env:CARTWAVE_AUDIOLOG, Env:CARTWAVE_FRAMELOG, Env:CARTWAVE_BROWSER_W -ErrorAction SilentlyContinue }
# The earlier scenarios' coordinates are the Presets batch's: its branch ran them with the browser at
# the old 240 px and its pane 124 px taller (perf_pb.ps1). main has both now, so both builds run them
# that way (B5-D14: without it the header drag hit the Display and no track moved).
for ($i = 1; $i -le $Rounds; $i++) {
  $order = if ($i % 2) { @($builds[0], $builds[1]) } else { @($builds[1], $builds[0]) }
  foreach ($o in $order) {
    $name, $root = $o; Hook $name
    $shots = if ($i -eq 1) { @("-ShotBeforeScroll", (Join-Path $Work "proof-$name-scrollbar.png"), "-ShotHeader", (Join-Path $Work "proof-$name-header.png")) } else { @() }
    pwsh -NoProfile -File "$Pb\bench_pb.ps1" -Label "$name-$i" -Repo $root -Work $Work -Main -BrowserW 240 -PaneGrow 124 @shots | Out-Null
  }
  foreach ($o in $order) {
    $name, $root = $o; Hook $name
    Logs "pr-$name-$i"; $env:CARTWAVE_BROWSER_W = "240"
    pwsh -NoProfile -File "$Repo\docs\pianoroll\tools\bench_pr.ps1" -Label "pr-$name-$i" -Repo $root -Work $Work -Exe (Join-Path $root "target\release\cartwave.exe") -NoteY 623 | Out-Null
    Clear-Logs
  }
  foreach ($o in $order) {
    $name, $root = $o; Hook $name
    Logs "mixer-$name-$i"; $env:CARTWAVE_BROWSER_W = "240"
    pwsh -NoProfile -File "$Repo\docs\batch4b\tools\bench_mixer.ps1" -Label "mixer-$name-$i" -Repo $root -Work $Work | Out-Null
    Clear-Logs
  }
  foreach ($o in $order) {
    $name, $root = $o; Hook $name
    pwsh -NoProfile -File "$Pb\bench_pb.ps1" -Label "new-$name-$i" -Repo $root -Work $Work -NewOnly -Pb | Out-Null
  }
  Remove-Item Env:CARTWAVE_NO_AUTO_KEYS -ErrorAction SilentlyContinue
  # The automation scenarios: plain ("main") and automated ("b5"), this build, alternating.
  foreach ($o in $order) {
    $name = $o[0]
    [string[]]$plain = if ($name -eq "main") { @("-Plain") } else { @() }   # typed: an if unrolls a 1-item array (B5-D14)
    pwsh -NoProfile -File "$PSScriptRoot\bench_b5.ps1" -Label "auto-$name-$i" -Work $Work -Fixtures $Fix @plain | Out-Null
  }
  "round $i done"
}
python "$PSScriptRoot\perf_summary_b5.py" $Work
