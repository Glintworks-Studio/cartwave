# usage: final_gates.ps1 [-Out <dir>]
# Polish 2: every mouse-driven gate on the current release build, in order (performance,
# regression at 100% and 150%, P2-01…11, the earlier suites). One summary line per gate.
param([string]$Out = (Join-Path $env:TEMP "chipdaw-p2-final"))
$Repo = (Resolve-Path (Join-Path $PSScriptRoot "..\..\..")).Path
Set-Location $Repo
Remove-Item -Recurse -Force $Out -ErrorAction SilentlyContinue
New-Item -ItemType Directory -Force $Out | Out-Null
$sum = Join-Path $Out "summary.txt"
pwsh -NoProfile -File "$PSScriptRoot\perf_p2.ps1" -Work (Join-Path $Out "perf") *> (Join-Path $Out "perf.log")
"perf done" | Add-Content $sum
$base = (Resolve-Path "..\before-polish2\target\release\app.exe").Path  # built before the rename
foreach ($sc in 100, 150) {
  pwsh -NoProfile -File "$PSScriptRoot\regress.ps1" -Exe $base -Out (Join-Path $Out "before$sc") -Scale $sc | Out-Null
  pwsh -NoProfile -File "$PSScriptRoot\regress.ps1" -Exe ".\target\release\cartwave.exe" -Out (Join-Path $Out "after$sc") -Scale $sc -New | Out-Null
  $r = python "$PSScriptRoot\diff.py" (Join-Path $Out "before$sc") (Join-Path $Out "after$sc") $sc (Join-Path $Out "diff$sc")
  ($r | Select-Object -Last 1) + " @ $sc%" | Add-Content $sum
}
$r = pwsh -NoProfile -File "$PSScriptRoot\at_p2.ps1" -Work (Join-Path $Out "p2")
($r | Select-Object -Last 1) | Add-Content $sum
pwsh -NoProfile -File "$PSScriptRoot\run_old_suites.ps1" -Out (Join-Path $Out "old") | Out-Null
Get-Content (Join-Path $Out "old\summary.txt") | Add-Content $sum
"all done" | Add-Content $sum
