# usage: run_suites_rn.ps1 [-Exe .\target\release\cartwave.exe] [-Out <dir>] [-Prefix CARTWAVE_|CHIPDAW_] [-Only shell,b4b]
# The rename's gate 5 (RENAME_PROMPT.md §3.5): every scenario suite, with the new environment
# names (CARTWAVE_) or, for the alias run, the old ones (CHIPDAW_). The old-names run spells
# every CARTWAVE_ name CHIPDAW_ in every harness script under docs/ (the suites and the shared
# helpers they dot-source), in place, and restores them from git at the end: so the harness
# files must be committed first. Every inherited variable of either spelling is cleared first,
# so the run sees only the names it sets.
# The earlier suites (shell, Batch 3, design, polish) run as polish 2 runs them: the Overview
# and the status bar hidden (their coordinates predate both; run_old_suites.ps1, RUNLOG P2-D16).
param([string]$Exe = ".\target\release\cartwave.exe", [string]$Out = (Join-Path $env:TEMP "rn-suites"), [string]$Prefix = "CARTWAVE_", [string[]]$Only = @())
$Repo = (Resolve-Path (Join-Path $PSScriptRoot "..\..\..")).Path
Set-Location $Repo
$Exe = (Resolve-Path $Exe).Path
New-Item -ItemType Directory -Force $Out | Out-Null
$sum = Join-Path $Out "summary.txt"
foreach ($v in Get-ChildItem Env: | Where-Object { $_.Name -like "CARTWAVE_*" -or $_.Name -like "CHIPDAW_*" }) { Remove-Item "Env:$($v.Name)" }
$suites = @(
  @("shell", "docs\shell\tools\at_shell.ps1", $true),
  @("batch3", "docs\batch3\tools\at_b3.ps1", $true),
  @("design", "docs\design\tools\at_dp.ps1", $true),
  @("polish", "docs\polish\tools\at_polish.ps1", $true),
  @("p2", "docs\polish2\tools\at_p2.ps1", $false),
  @("b4a", "docs\batch4a\tools\at_b4a.ps1", $false),
  @("b4b", "docs\batch4b\tools\at_b4b.ps1", $false)
)
$touched = @()
if ($Prefix -ne "CARTWAVE_") {
  if (git status --porcelain -- docs) { throw "commit docs/ first: the old-names run rewrites the harness in place and restores it from git" }
  foreach ($f in Get-ChildItem -Recurse -Path docs -Include *.ps1) {
    $t = Get-Content -Raw -Encoding utf8 $f.FullName
    if ($t -and $t.Contains("CARTWAVE_")) { $t.Replace("CARTWAVE_", $Prefix) | Set-Content -NoNewline -Encoding utf8 $f.FullName; $touched += $f.FullName }
  }
}
foreach ($s in $suites) {
  $name, $script, $old = $s
  if ($Only.Count -and $Only -notcontains $name) { continue }
  $run = $script
  if ($old) { Set-Item "Env:${Prefix}OVERVIEW" "0"; Set-Item "Env:${Prefix}HIDE_STATUS" "1" }
  $log = Join-Path $Out "$name.log"
  pwsh -NoProfile -File $run -Exe $Exe -Work (Join-Path $Out $name) *> $log
  $code = $LASTEXITCODE
  Remove-Item "Env:${Prefix}OVERVIEW", "Env:${Prefix}HIDE_STATUS" -ErrorAction SilentlyContinue
  $pass = (Select-String -Path $log -Pattern "^PASS " -ErrorAction SilentlyContinue).Count
  $fail = @(Select-String -Path $log -Pattern "^(FAIL|FAILED)" -ErrorAction SilentlyContinue | ForEach-Object { $_.Line })
  "$name exit $code; PASS lines $pass; " + $(if ($fail.Count) { "FAIL: " + ($fail -join " | ") } else { "no FAIL lines" }) | Tee-Object -Append $sum
}
if ($touched.Count) { git checkout -- docs; "restored $($touched.Count) harness scripts" | Tee-Object -Append $sum }
"logs -> $Out"
