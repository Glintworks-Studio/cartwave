# Batch 5: a quick look at Automation Mode in the real app (development aid; the gated
# scenarios are at_b5.ps1). usage: try_b5.ps1 [-Exe ...] [-Out dir] [-Project file] [-Steps edit]
# Client px at 100 % (1440 x 920): rows from 136 (73 px each); in Automation Mode each track's
# first lane is the 73 px under its row; the lanes start at x 801.
param([string]$Exe = ".\target\release\cartwave.exe", [string]$Out = (Join-Path $env:TEMP "cartwave-try-b5"), [string]$Project = "docs\songs\v9\v9_vrc6_presets.chipdaw", [string]$Steps = "edit")
$global:Work = $Out
New-Item -ItemType Directory -Force $Out | Out-Null
. (Join-Path $PSScriptRoot "..\..\batch3\tools\lib.ps1")
Set-Location $Repo
$global:Exe = (Resolve-Path $Exe).Path
$d = Join-Path $Out "data"; Remove-Item -Recurse -Force $d -ErrorAction SilentlyContinue
Launch -Project (Resolve-Path $Project).Path -Data $d
pwsh -NoProfile -File "$Repo\docs\perf\tools\shot.ps1" -Out "$Out\_place.png" | Out-Null
Start-Sleep -Milliseconds 300
K 0x41                       # A: Automation Mode
Start-Sleep -Milliseconds 500
Shot "auto_mode"
if ($Steps -eq "edit") {
  # Track 1's Duty lane: 209..281; the dashed line (duty 2 of 0..3) at about y 237.
  ClickL 900 237; ClickL 1100 237; ClickL 1250 237
  DragL 1100 237 1100 214
  Shot "auto_edit"
  ClickL 1250 237              # a click on a breakpoint deletes it
  Shot "auto_deleted"
}
Stop-App
Get-Content (Join-Path $d "stderr.txt") | Select-String "dev: (automation|view|help)" | Select-Object -Last 12
