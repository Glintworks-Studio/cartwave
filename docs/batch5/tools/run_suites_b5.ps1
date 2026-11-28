# usage: run_suites_b5.ps1 [-Exe .\target\release\cartwave.exe] [-Out <dir>] [-Only shell,b4b,pb]
# Batch 5 gate 3 (BATCH5_PROMPT.md §8.3): every earlier suite plus AU. Every earlier suite runs with
# the dev hook CARTWAVE_NO_AUTO_KEYS=1 (the transport's two automation keys left out: their top-bar
# coordinates predate them, RUNLOG B5-D12). From the Presets batch's runner:
# plus PB. The earlier suites' coordinates predate the 420 px browser: they run with the dev hook
# CARTWAVE_BROWSER_W=240 (the old width; RUNLOG PB-D12), and the oldest (shell, Batch 3, design,
# polish) with the Overview and the status bar hidden as well, as polish 2 runs them (P2-D16).
param([string]$Exe = ".\target\release\cartwave.exe", [string]$Out = (Join-Path $env:TEMP "b5-suites"), [string[]]$Only = @())
$Repo = (Resolve-Path (Join-Path $PSScriptRoot "..\..\..")).Path
Set-Location $Repo
$Exe = (Resolve-Path $Exe).Path
$Only = @($Only | ForEach-Object { $_ -split "," } | Where-Object { $_ })
New-Item -ItemType Directory -Force $Out | Out-Null
$sum = Join-Path $Out "summary.txt"
foreach ($v in Get-ChildItem Env: | Where-Object { $_.Name -like "CARTWAVE_*" -or $_.Name -like "CHIPDAW_*" }) { Remove-Item "Env:$($v.Name)" }
# name, script, oldest (Overview + status bar hidden), earlier (the old browser width)
$suites = @(
  @("shell", "docs\shell\tools\at_shell.ps1", $true, $true),
  @("batch3", "docs\batch3\tools\at_b3.ps1", $true, $true),
  @("design", "docs\design\tools\at_dp.ps1", $true, $true),
  @("polish", "docs\polish\tools\at_polish.ps1", $true, $true),
  @("p2", "docs\polish2\tools\at_p2.ps1", $false, $true),
  @("b4a", "docs\batch4a\tools\at_b4a.ps1", $false, $true),
  @("b4b", "docs\batch4b\tools\at_b4b.ps1", $false, $true),
  @("pb", "docs\presets\tools\at_pb.ps1", $false, $false),
  @("b5", "docs\batch5\tools\at_b5.ps1", $false, $false)
)
foreach ($s in $suites) {
  $name, $script, $old, $earlier = $s
  if ($Only.Count -and $Only -notcontains $name) { continue }
  if (-not (Test-Path $script)) { "${name}: no script yet" | Tee-Object -Append $sum; continue }
  if ($old) { $env:CARTWAVE_OVERVIEW = "0"; $env:CARTWAVE_HIDE_STATUS = "1" }
  if ($earlier) { $env:CARTWAVE_BROWSER_W = "240" }
  if ($name -ne "b5") { $env:CARTWAVE_NO_AUTO_KEYS = "1" }
  $log = Join-Path $Out "$name.log"
  pwsh -NoProfile -File $script -Exe $Exe -Work (Join-Path $Out $name) *> $log
  $code = $LASTEXITCODE
  Remove-Item Env:CARTWAVE_OVERVIEW, Env:CARTWAVE_HIDE_STATUS, Env:CARTWAVE_BROWSER_W, Env:CARTWAVE_NO_AUTO_KEYS -ErrorAction SilentlyContinue
  $pass = (Select-String -Path $log -Pattern "^PASS " -ErrorAction SilentlyContinue).Count
  $fail = @(Select-String -Path $log -Pattern "^(FAIL|FAILED)" -ErrorAction SilentlyContinue | ForEach-Object { $_.Line })
  "$name exit $code; PASS lines $pass; " + $(if ($fail.Count) { "FAIL: " + ($fail -join " | ") } else { "no FAIL lines" }) | Tee-Object -Append $sum
}
"logs -> $Out"
