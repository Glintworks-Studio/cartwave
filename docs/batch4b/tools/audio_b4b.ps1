# usage: audio_b4b.ps1 [-Work <dir>] [-Secs 1800] [-Runs S64U,DEMO]
# Batch 4b gate 5, batch 4a's method (audio_b4a.ps1, ENGINE.md): the release app plays with a
# 23 kHz pilot, captures its own output through WASAPI loopback and counts dropouts (the ground
# truth), next to the engine's detectors, the allocation checker (NoAllocGuard) and the callback
# cost; one CSV line per second plus a summary (crates/ui/src/testing.rs).
#  - S64U: fx_s64u (64 studio tracks, unison 2 at detune 15: 1,024 voices at the densest chord,
#    two pans per voice), **the Main fader at 0 dB** (full volume: the studio bus limiter keeps
#    the mix under -1 dBFS, so the pilot survives without 4a's -40 dB copy), 256 frames, -Secs.
#  - DEMO: the demo song (locked tracks: the console path), 256 frames, 120 s.
param([string]$Work = (Join-Path $env:TEMP "chipdaw-audio-b4b"), [int]$Secs = 1800, [string[]]$Runs = @("S64U", "DEMO"))
$Repo = (Resolve-Path (Join-Path $PSScriptRoot "..\..\..")).Path
$Runs = $Runs | ForEach-Object { $_ -split "," } | Where-Object { $_ }
New-Item -ItemType Directory -Force $Work | Out-Null
python "$Repo\docs\batch4b\tools\fixtures_b4b.py" $Work | Out-Null
$app = Join-Path $Repo "target\release\cartwave.exe"
$env:CARTWAVE_DEV = "1"
function Run-App([string]$name, [hashtable]$envs, [int]$secs) {
  $log = Join-Path $Work "$name.csv"
  $data = Join-Path $Work "data_$name"
  New-Item -ItemType Directory -Force $data | Out-Null
  $all = @{ CARTWAVE_AUDIOLOG = $log; CARTWAVE_QUIT_AFTER = "$secs"; CARTWAVE_PILOT = "1"; CARTWAVE_GAIN = "0.3"; CARTWAVE_DATA_DIR = $data } + $envs
  foreach ($k in $all.Keys) { Set-Item "Env:$k" $all[$k] }
  $p = Start-Process -FilePath $app -PassThru -WorkingDirectory $Repo -RedirectStandardError (Join-Path $Work "$name.stderr.txt")
  try { if (-not $p.WaitForExit(($secs + 90) * 1000)) { $p.Kill(); "TIMEOUT $name" } }
  finally { foreach ($k in $all.Keys) { Remove-Item "Env:$k" -ErrorAction SilentlyContinue } }
  "$name exit=$($p.ExitCode)"
  Get-Content $log | Where-Object { $_ -like "#*" }
}
foreach ($r in $Runs) {
  switch ($r) {
    "S64U" { Run-App "S64U_256" @{ CARTWAVE_BUFFER = "256"; CARTWAVE_AUTOPLAY = "1"; CARTWAVE_PROJECT = (Join-Path $Work "fx_s64u.chipdaw") } $Secs }
    # The third source for a pilot count that disagrees with the engine (B4B-D33): the same
    # project at Main -40 dB (4a's S64M method), so the mix can't mask or clip the pilot.
    "S64UM" {
      python -c "import json,sys; p=json.load(open(sys.argv[1],encoding='utf-8')); p['master_db']=-40.0; json.dump(p,open(sys.argv[2],'w',encoding='utf-8'))" (Join-Path $Work "fx_s64u.chipdaw") (Join-Path $Work "fx_s64u_main-40.chipdaw")
      Run-App "S64UM_256" @{ CARTWAVE_BUFFER = "256"; CARTWAVE_AUTOPLAY = "1"; CARTWAVE_PROJECT = (Join-Path $Work "fx_s64u_main-40.chipdaw") } $Secs
    }
    "DEMO" { Run-App "DEMO_256" @{ CARTWAVE_BUFFER = "256"; CARTWAVE_AUTOPLAY = "1" } 120 }
  }
}
