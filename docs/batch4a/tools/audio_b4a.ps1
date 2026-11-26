# usage: audio_b4a.ps1 [-Work <dir>] [-Secs 1800] [-AppExe <exe>]
# Batch 4a §9.5, the ENGINE.md method (docs/engine/tools/matrix.ps1's Run-App): the release app
# plays with a 23 kHz pilot mixed in, captures its own output through WASAPI loopback and counts
# dropouts (the ground truth), next to the engine's detectors, the allocation checker and the
# callback cost; one CSV line per second plus a summary (crates/ui/src/testing.rs).
#  - S64: the 64-studio-track stress project (docs/batch4a/songs/stress64.chipdaw: 512 voices,
#    dense 8-note chords, unlimited voices), 256-frame buffer, -Secs long (30 min by default).
#  - DEMO: the demo song (every track locked: the console path), 256 frames, 120 s.
param([string]$Work = (Join-Path $env:TEMP "chipdaw-audio-b4a"), [int]$Secs = 1800, [string]$AppExe = "", [string[]]$Runs = @("S64", "DEMO"))
$Repo = (Resolve-Path (Join-Path $PSScriptRoot "..\..\..")).Path
$Runs = $Runs | ForEach-Object { $_ -split "," } | Where-Object { $_ }
New-Item -ItemType Directory -Force $Work | Out-Null
$app = if ($AppExe) { $AppExe } else { Join-Path $Repo "target\release\app.exe" }
$env:CHIPDAW_DEV = "1"

function Run-App([string]$name, [hashtable]$envs, [int]$secs) {
  $log = Join-Path $Work "$name.csv"
  $data = Join-Path $Work "data_$name"
  New-Item -ItemType Directory -Force $data | Out-Null
  $all = @{ CHIPDAW_AUDIOLOG = $log; CHIPDAW_QUIT_AFTER = "$secs"; CHIPDAW_PILOT = "1"; CHIPDAW_GAIN = "0.3"; CHIPDAW_DATA_DIR = $data } + $envs
  foreach ($k in $all.Keys) { Set-Item "Env:$k" $all[$k] }
  $p = Start-Process -FilePath $app -PassThru -WorkingDirectory $Repo -RedirectStandardError (Join-Path $Work "$name.stderr.txt")
  try { if (-not $p.WaitForExit(($secs + 90) * 1000)) { $p.Kill(); "TIMEOUT $name" } }
  finally { foreach ($k in $all.Keys) { Remove-Item "Env:$k" -ErrorAction SilentlyContinue } }
  "$name exit=$($p.ExitCode)"
  Get-Content $log | Where-Object { $_ -like "#*" }
}

foreach ($r in $Runs) {
  switch ($r) {
    "S64" { Run-App "S64_256" @{ CHIPDAW_BUFFER = "256"; CHIPDAW_AUTOPLAY = "1"; CHIPDAW_PROJECT = (Join-Path $Repo "docs\batch4a\songs\stress64.chipdaw") } $Secs }
    # The same at Main -40 dB: the mix stays under full scale, so the pilot (added after Main)
    # survives and loopback can judge dropouts (512 voices at full volume peak ~+34 dBFS and clip).
    "S64M" { Run-App "S64M_256" @{ CHIPDAW_BUFFER = "256"; CHIPDAW_AUTOPLAY = "1"; CHIPDAW_PROJECT = (Join-Path $Repo "docs\batch4a\songs\stress64_main-40.chipdaw") } $Secs }
    "DEMO" { Run-App "DEMO_256" @{ CHIPDAW_BUFFER = "256"; CHIPDAW_AUTOPLAY = "1" } 120 }
  }
}
