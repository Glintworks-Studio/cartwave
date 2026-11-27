# usage: run_old_suites.ps1 [-Exe .\target\release\cartwave.exe] [-Out <dir>]
# Polish 2 §9.4: the earlier scenario suites (AT-01…AT-19 in at_shell.ps1; AT-07…AT-31 in
# at_b3.ps1; DP-01…05 in at_dp.ps1; PL-01…05 in at_polish.ps1) on this build. Their
# coordinates assume rows from y 104 and the lanes down to the window's bottom, so they run
# with the Overview hidden (CARTWAVE_OVERVIEW=0) and the status bar hidden
# (CARTWAVE_HIDE_STATUS=1): dev-only hooks that move nothing else (RUNLOG P2-D).
param([string]$Exe = ".\target\release\cartwave.exe", [string]$Out = (Join-Path $env:TEMP ("chipdaw-old-suites-" + (Get-Date -Format "MMdd-HHmmss"))))
$Repo = (Resolve-Path (Join-Path $PSScriptRoot "..\..\..")).Path
Set-Location $Repo
New-Item -ItemType Directory -Force $Out | Out-Null
$env:CARTWAVE_OVERVIEW = "0"; $env:CARTWAVE_HIDE_STATUS = "1"
$suites = @(
  @("shell", "docs\shell\tools\at_shell.ps1"),
  @("batch3", "docs\batch3\tools\at_b3.ps1"),
  @("design", "docs\design\tools\at_dp.ps1"),
  @("polish", "docs\polish\tools\at_polish.ps1")
)
foreach ($s in $suites) {
  $name, $script = $s
  $log = Join-Path $Out "$name.log"
  pwsh -NoProfile -File $script -Exe $Exe -Work (Join-Path $Out $name) *> $log
  "$name exit $LASTEXITCODE" | Tee-Object -Append (Join-Path $Out "summary.txt")
}
Remove-Item Env:CARTWAVE_OVERVIEW, Env:CARTWAVE_HIDE_STATUS -ErrorAction SilentlyContinue
"logs -> $Out"
