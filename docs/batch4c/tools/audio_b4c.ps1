# usage: audio_b4c.ps1 [-Work <dir>] [-Secs 1800]
# Batch 4c gate 4, the new method (ENGINE.md §7a): the release app plays fx_s64u (64 studio
# tracks, unison 2 / detune 15: 1,024 voices, Main at 0 dB = full volume), 256 frames, -Secs
# seconds, with the reference match on (CARTWAVE_MATCH: the engine's output tap aligned with the
# WASAPI loopback capture sample for sample). Pass: 0 match dropouts, 0 mismatch runs, 0 capture
# gaps / foreign / tap lost, 0 allocation violations, and the match really compared (>= 90 %).
param([string]$Work = (Join-Path $env:TEMP "cartwave-b4c-audio"), [int]$Secs = 1800)
$Repo = (Resolve-Path (Join-Path $PSScriptRoot "..\..\..")).Path
New-Item -ItemType Directory -Force $Work | Out-Null
python "$Repo\docs\batch4b\tools\fixtures_b4b.py" $Work | Out-Null
$app = Join-Path $Repo "target\release\cartwave.exe"
$log = Join-Path $Work "s64u_match.csv"
$all = @{ CARTWAVE_DEV = "1"; CARTWAVE_AUDIOLOG = $log; CARTWAVE_QUIT_AFTER = "$Secs"; CARTWAVE_MATCH = "1"; CARTWAVE_GAIN = "0.3"; CARTWAVE_DATA_DIR = (Join-Path $Work "data"); CARTWAVE_BUFFER = "256"; CARTWAVE_AUTOPLAY = "1"; CARTWAVE_PROJECT = (Join-Path $Work "fx_s64u.chipdaw") }
foreach ($k in $all.Keys) { Set-Item "Env:$k" $all[$k] }
$p = Start-Process -FilePath $app -PassThru -WorkingDirectory $Repo -RedirectStandardError (Join-Path $Work "s64u_match.stderr.txt")
try { if (-not $p.WaitForExit(($Secs + 90) * 1000)) { $p.Kill(); "TIMEOUT" } }
finally { foreach ($k in $all.Keys) { Remove-Item "Env:$k" -ErrorAction SilentlyContinue } }
Get-Content $log | Where-Object { $_ -like "#*" }
