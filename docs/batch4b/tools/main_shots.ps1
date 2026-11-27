# usage: main_shots.ps1 -Out <dir> [-Exe .\target\release\cartwave.exe] [-Scale 100] [-Project fx_board]
# Batch 4b §3.2 / §9.3: window shots of the board's song in the four header layouts (Settings >
# Look & Feel: headers left/right x strip outside/by the clips) and at 520 px wide, browser hidden
# (the board has none), Overview and status bar shown, meters frozen; mockup_match_main.py measures
# them against the `main-track` board.
param([string]$Out, [string]$Exe = ".\target\release\cartwave.exe", [int]$Scale = 100, [string]$Project = "fx_board")
$Repo = (Resolve-Path (Join-Path $PSScriptRoot "..\..\..")).Path
New-Item -ItemType Directory -Force $Out | Out-Null
python "$Repo\docs\batch4b\tools\fixtures_b4b.py" $Out | Out-Null
$layouts = @(
  @("left_outside", "false", "false", 380), @("left_byclips", "false", "true", 380),
  @("right_byclips", "true", "true", 380), @("right_outside", "true", "false", 380),
  @("left_outside_520", "false", "false", 520))
$h = [int](536 * $Scale / 100)
$w = [int](1440 * $Scale / 100)
foreach ($l in $layouts) {
  $name, $right, $by, $hw = $l
  $d = Join-Path $Out "data_$name"
  New-Item -ItemType Directory -Force $d | Out-Null
  "{ ""ui_scale"": $Scale, ""headers_right"": $right, ""strip_by_clips"": $by, ""header_width"": $hw, ""browser_open"": false }" | Set-Content -Encoding ascii (Join-Path $d "settings.json")
  Get-Process -Name cartwave -ErrorAction SilentlyContinue | Stop-Process -Force
  $env:CARTWAVE_DATA_DIR = $d; $env:CARTWAVE_AUDIO = "0"; $env:CARTWAVE_DEV = "1"; $env:CARTWAVE_FROZEN_METERS = "1"
  $env:CARTWAVE_PROJECT = (Join-Path $Out "$Project.chipdaw")
  $p = Start-Process (Resolve-Path $Exe).Path -ArgumentList "--size", "1440x720" -PassThru -RedirectStandardError (Join-Path $d "stderr.txt")
  Start-Sleep -Milliseconds 2500
  pwsh -NoProfile -File "$Repo\docs\perf\tools\shot.ps1" -Out (Join-Path $Out "$name.png") | Out-Null
  Stop-Process -Id $p.Id -Force -ErrorAction SilentlyContinue
  Start-Sleep -Milliseconds 300
}
Remove-Item Env:CARTWAVE_PROJECT, Env:CARTWAVE_FROZEN_METERS -ErrorAction SilentlyContinue
