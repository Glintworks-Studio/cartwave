# usage: audio_b5.ps1 [-Work <dir>] [-Fixtures <dir>] [-Secs 600] [-Every 19]
# Batch 5's audio gate (BATCH5_PROMPT.md §8.4), Batch 4c's method (ENGINE.md §7a: the engine's
# output tap matched sample for sample with the WASAPI loopback capture): the release app plays
# auto32 (the demo band, 32 automated parameters on 8 tracks, studio and locked, plus the Main
# gain; the loop is the whole song) for -Secs at 256 frames with the reference match on, while the
# dev hook CARTWAVE_AUTO_OVERRIDE overrides the first automated studio fader every 2 x -Every s
# and re-enables it -Every s later. Pass: 0 match dropouts, 0 mismatch runs, 0 capture gaps /
# foreign / tap lost, 0 allocation violations, the match really compared (>= 90 %), and the
# override toggled (>= 2 x Secs / Every / 2 events).
param([string]$Work = (Join-Path $env:TEMP "cartwave-b5-audio"), [string]$Fixtures = "", [int]$Secs = 600, [double]$Every = 19)
$Repo = (Resolve-Path (Join-Path $PSScriptRoot "..\..\..")).Path
New-Item -ItemType Directory -Force $Work | Out-Null
if (-not $Fixtures) { $Fixtures = Join-Path $Work "fixtures" }
if (-not (Test-Path (Join-Path $Fixtures "auto32.cartwave"))) { Push-Location $Repo; & "$env:USERPROFILE\.cargo\bin\cargo.exe" run --release -q -p ui --example auto_fixture -- $Fixtures; Pop-Location }
$app = Join-Path $Repo "target\release\cartwave.exe"
$log = Join-Path $Work "auto32_match.csv"
$err = Join-Path $Work "auto32_match.stderr.txt"
Remove-Item -Recurse -Force (Join-Path $Work "data") -ErrorAction SilentlyContinue
$all = @{ CARTWAVE_DEV = "1"; CARTWAVE_AUDIOLOG = $log; CARTWAVE_QUIT_AFTER = "$Secs"; CARTWAVE_MATCH = "1"; CARTWAVE_GAIN = "0.3"; CARTWAVE_DATA_DIR = (Join-Path $Work "data"); CARTWAVE_BUFFER = "256"; CARTWAVE_AUTOPLAY = "1"; CARTWAVE_AUTO_OVERRIDE = "$Every"; CARTWAVE_PROJECT = (Join-Path $Fixtures "auto32.cartwave") }
foreach ($k in "CARTWAVE_BROWSER_W", "CARTWAVE_AUTO_STEP") { Remove-Item "Env:$k" -ErrorAction SilentlyContinue }
foreach ($k in $all.Keys) { Set-Item "Env:$k" $all[$k] }
$p = Start-Process -FilePath $app -PassThru -WorkingDirectory $Repo -RedirectStandardError $err
try { if (-not $p.WaitForExit(($Secs + 90) * 1000)) { $p.Kill(); "TIMEOUT" } }
finally { foreach ($k in $all.Keys) { Remove-Item "Env:$k" -ErrorAction SilentlyContinue } }
$sum = @(Get-Content $log | Where-Object { $_ -like "#*" })
$over = @(Select-String -Path $err -Pattern "^dev: auto override \d+ \(override\)" -ErrorAction SilentlyContinue).Count
$re = @(Select-String -Path $err -Pattern "^dev: auto override \d+ \(re-enable\)" -ErrorAction SilentlyContinue).Count
$seen = @(Select-String -Path $err -Pattern "^dev: automation overridden track.gain" -ErrorAction SilentlyContinue).Count
$sum
"override events $over (automation overridden lines $seen), re-enable events $re"
