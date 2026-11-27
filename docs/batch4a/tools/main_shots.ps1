# usage: main_shots.ps1 -Out <dir> [-Exe .\target\release\cartwave.exe] [-Scale 100]
# Batch 4a §9.3: window shots of the Main row in the four header layouts (Settings > Look & Feel:
# headers left/right x strip outside/by the clips) and at 520 px wide, on a 4-track project with
# the Overview and status bar shown; mockup_main.py measures them.
param([string]$Out, [string]$Exe = ".\target\release\cartwave.exe", [int]$Scale = 100)
$Repo = (Resolve-Path (Join-Path $PSScriptRoot "..\..\..")).Path
New-Item -ItemType Directory -Force $Out | Out-Null
python "$Repo\docs\batch4a\tools\fixtures_b4a.py" $Out | Out-Null
$layouts = @(
  @("left_outside", "false", "false", 380), @("left_byclips", "false", "true", 380),
  @("right_byclips", "true", "true", 380), @("right_outside", "true", "false", 380),
  @("left_outside_520", "false", "false", 520))
foreach ($l in $layouts) {
  $name, $right, $by, $w = $l
  $d = Join-Path $Out "data_$name"
  New-Item -ItemType Directory -Force $d | Out-Null
  "{ ""ui_scale"": $Scale, ""headers_right"": $right, ""strip_by_clips"": $by, ""header_width"": $w }" | Set-Content -Encoding ascii (Join-Path $d "settings.json")
  Get-Process -Name cartwave -ErrorAction SilentlyContinue | Stop-Process -Force
  $env:CARTWAVE_DATA_DIR = $d; $env:CARTWAVE_AUDIO = "0"; $env:CARTWAVE_DEV = "1"; $env:CARTWAVE_FROZEN_METERS = "1"
  $env:CARTWAVE_PROJECT = (Join-Path $Out "fx4_studio.chipdaw")
  $p = Start-Process (Resolve-Path $Exe).Path -ArgumentList "--size", "1440x920" -PassThru -RedirectStandardError (Join-Path $d "stderr.txt")
  Start-Sleep -Milliseconds 2500
  pwsh -NoProfile -File "$Repo\docs\perf\tools\shot.ps1" -Out (Join-Path $Out "$name.png") | Out-Null
  Stop-Process -Id $p.Id -Force -ErrorAction SilentlyContinue
  Start-Sleep -Milliseconds 300
}
Remove-Item Env:CARTWAVE_PROJECT, Env:CARTWAVE_FROZEN_METERS -ErrorAction SilentlyContinue
