# usage: perf_auto_b5.ps1 [-Work <dir>] [-Fixtures <dir>] [-Rounds 10]
# Batch 5's automation scenarios on their own (perf_b5.ps1's last step, run separately: its first
# gate run passed -Plain wrongly, so its plain baselines never ran; RUNLOG B5-D14): bench_b5.ps1,
# the plain songs ("main" in the pairing) against the automated ones, on this build, -Rounds
# rounds with the order alternating; perf_summary_b5.py judges by the §7b rule.
param([string]$Work = (Join-Path $env:TEMP "cartwave-b5-perf-auto"), [string]$Fixtures = "", [int]$Rounds = 10)
$Repo = (Resolve-Path (Join-Path $PSScriptRoot "..\..\..")).Path
New-Item -ItemType Directory -Force $Work | Out-Null
if (-not $Fixtures) { $Fixtures = Join-Path $Work "fixtures" }
if (-not (Test-Path (Join-Path $Fixtures "auto32.cartwave"))) { Push-Location $Repo; & "$env:USERPROFILE\.cargo\bin\cargo.exe" run --release -q -p ui --example auto_fixture -- $Fixtures; Pop-Location }
foreach ($k in "CARTWAVE_NO_AUTO_KEYS", "CARTWAVE_BROWSER_W") { Remove-Item "Env:$k" -ErrorAction SilentlyContinue }
for ($i = 1; $i -le $Rounds; $i++) {
  $order = if ($i % 2) { @("main", "b5") } else { @("b5", "main") }
  foreach ($name in $order) {
    [string[]]$plain = if ($name -eq "main") { @("-Plain") } else { @() }
    pwsh -NoProfile -File "$PSScriptRoot\bench_b5.ps1" -Label "auto-$name-$i" -Work $Work -Fixtures $Fixtures @plain | Out-Null
  }
  "round $i done"
}
Get-Content (Join-Path $Work "bench_b5_log.txt")
python "$PSScriptRoot\perf_summary_b5.py" $Work
