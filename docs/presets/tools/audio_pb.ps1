# usage: audio_pb.ps1 [-Work <dir>] [-Secs 600] [-Every 19]
# The Presets + Browser batch's audio gate (PRESETS_PROMPT.md §8.5), Batch 4c's method (ENGINE.md
# §7a: the engine's output tap matched sample for sample with the WASAPI loopback capture): the
# release app plays the demo for -Secs (256 frames, the reference match on) while ▶ steps the
# first track (Pulse 1) through its preset folder every -Every seconds (the dev hook
# CARTWAVE_AUTO_STEP: 30 steps, each loaded at once, a run of steps one undo step). Pass: 0 match
# dropouts, 0 mismatch runs, 0 capture gaps / foreign / tap lost, 0 allocation violations, the
# match really compared (>= 90 %), and 30 steps made.
param([string]$Work = (Join-Path $env:TEMP "cartwave-pb-audio"), [int]$Secs = 600, [double]$Every = 19)
$Repo = (Resolve-Path (Join-Path $PSScriptRoot "..\..\..")).Path
New-Item -ItemType Directory -Force $Work | Out-Null
$app = Join-Path $Repo "target\release\cartwave.exe"
$log = Join-Path $Work "demo_steps_match.csv"
$err = Join-Path $Work "demo_steps_match.stderr.txt"
Remove-Item -Recurse -Force (Join-Path $Work "data") -ErrorAction SilentlyContinue
$all = @{ CARTWAVE_DEV = "1"; CARTWAVE_AUDIOLOG = $log; CARTWAVE_QUIT_AFTER = "$Secs"; CARTWAVE_MATCH = "1"; CARTWAVE_GAIN = "0.3"; CARTWAVE_DATA_DIR = (Join-Path $Work "data"); CARTWAVE_BUFFER = "256"; CARTWAVE_AUTOPLAY = "1"; CARTWAVE_AUTO_STEP = "$Every"; CARTWAVE_AUTO_STEP_MAX = "30" }
foreach ($k in "CARTWAVE_PROJECT", "CARTWAVE_BROWSER_W") { Remove-Item "Env:$k" -ErrorAction SilentlyContinue }
foreach ($k in $all.Keys) { Set-Item "Env:$k" $all[$k] }
$p = Start-Process -FilePath $app -PassThru -WorkingDirectory $Repo -RedirectStandardError $err
try { if (-not $p.WaitForExit(($Secs + 90) * 1000)) { $p.Kill(); "TIMEOUT" } }
finally { foreach ($k in $all.Keys) { Remove-Item "Env:$k" -ErrorAction SilentlyContinue } }
$sum = @(Get-Content $log | Where-Object { $_ -like "#*" })
$steps = @(Select-String -Path $err -Pattern "^dev: auto step" -ErrorAction SilentlyContinue).Count
$undo = @(Select-String -Path $err -Pattern "^dev: step preset" -ErrorAction SilentlyContinue | ForEach-Object { if ($_.Line -match "undo steps \((\d+), 0\)") { $Matches[1] } }) | Select-Object -Unique
$sum
"auto steps $steps; undo positions during the run: $($undo -join ',')"
