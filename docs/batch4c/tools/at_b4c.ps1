# usage: at_b4c.ps1 [-Exe .\target\release\cartwave.exe] [-Out <dir>]
# Batch 4c's scenarios (gate 3), PASS/FAIL each:
#  B4C-1  the reference match catches injected audio stalls that drop out and nothing else: the
#         1,024-voice project at full Main volume, stalls of 10/19/30/40 ms every 3 s for 40 s.
#         Pass: one match dropout per 30/40 ms stall and none for 10/19 ms; the starved detector
#         agrees; no capture gaps, foreign sound or tap losses; 0 allocation violations.
#  B4C-2  K1: the menu bar while the song plays and the window is resized (corner and right edge,
#         dark menus on): 0 flash, 0 misdrawn frames and 0 new light blocks; and the capture's
#         positive control (a white patch twice) is caught (>= 8 frames).
#  B4C-3  the reference match on a clean run: the demo playing 20 s, 0 dropouts, 0 mismatches,
#         and it really compared (matched blocks cover >= 90 % of the run).
# The engine-side units (OutputTap, Matcher, the tap under the allocation checker) and the
# perf fixes (Doc::matches, clip marks, sync_ctls) are unit tests in `cargo test`.
param([string]$Exe = ".\target\release\cartwave.exe", [string]$Out = (Join-Path $env:TEMP "cartwave-b4c-at"))
$Repo = (Resolve-Path (Join-Path $PSScriptRoot "..\..\..")).Path
Set-Location $Repo
$Exe = (Resolve-Path $Exe).Path
New-Item -ItemType Directory -Force $Out | Out-Null
$results = @()
function Check($id, $ok, $what) { $script:results += "{0}  {1}  {2}" -f $id, $(if ($ok) { "PASS" } else { "FAIL" }), $what }
function Run-Audio($name, [hashtable]$envs, [int]$secs) {
  $log = Join-Path $Out "$name.csv"
  $all = @{ CARTWAVE_DEV = "1"; CARTWAVE_AUDIOLOG = $log; CARTWAVE_QUIT_AFTER = "$secs"; CARTWAVE_MATCH = "1"; CARTWAVE_GAIN = "0.3"; CARTWAVE_DATA_DIR = (Join-Path $Out "data_$name"); CARTWAVE_BUFFER = "256"; CARTWAVE_AUTOPLAY = "1" } + $envs
  foreach ($k in $all.Keys) { Set-Item "Env:$k" $all[$k] }
  $p = Start-Process -FilePath $Exe -PassThru -WorkingDirectory $Repo -RedirectStandardError (Join-Path $Out "$name.stderr.txt")
  try { if (-not $p.WaitForExit(($secs + 60) * 1000)) { $p.Kill() } } finally { foreach ($k in $all.Keys) { Remove-Item "Env:$k" -ErrorAction SilentlyContinue } }
  Get-Content $log | Where-Object { $_ -like "#*" }
}
function Num($lines, $pat) { $m = [regex]::Match(($lines -join "`n"), $pat); if ($m.Success) { [double]$m.Groups[1].Value } else { -1 } }

# B4C-1
python "$Repo\docs\batch4b\tools\fixtures_b4b.py" $Out | Out-Null
$s = Run-Audio "b4c1" @{ CARTWAVE_PROJECT = (Join-Path $Out "fx_s64u.chipdaw"); CARTWAVE_INJECT_STALLS = "10,19,30,40"; CARTWAVE_INJECT_EVERY = "3" } 40
$inj = $s | Where-Object { $_ -match "^#\s+[\d.]+\s+[\d.]+ ->" }
$good = 0; $bad = 0; $long = 0
foreach ($l in $inj) {
  $m = [regex]::Match($l, "^#\s+[\d.]+\s+([\d.]+) -> \d+ \[(.*)\]")
  $ms = [double]$m.Groups[1].Value; $gaps = ($m.Groups[2].Value.Trim() -split " " | Where-Object { $_ }).Count
  if ($ms -ge 30) { $long++; if ($gaps -eq 1) { $good++ } else { $bad++ } } elseif ($gaps -ne 0) { $bad++ }
}
$starved = Num $s "starved (\d+)"; $alloc = Num $s "alloc violations (\d+)"
$cg = Num $s "(\d+) capture gaps"; $fo = Num $s "(\d+) foreign"; $tl = Num $s "tap lost (\d+)"
Check "B4C-1" ($inj.Count -ge 10 -and $bad -eq 0 -and $good -eq $long -and $starved -eq $long -and $alloc -eq 0 -and $cg -eq 0 -and $fo -eq 0 -and $tl -eq 0) "stalls $($inj.Count) (>=30 ms: $long, caught $good, wrong $bad); starved $starved; capture gaps $cg, foreign $fo, tap lost $tl; alloc $alloc"
# B4C-3
$s = Run-Audio "b4c3" @{} 20
$md = Num $s "(\d+) dropouts \(max"; $mr = Num $s "(\d+) mismatch runs"; $mb = Num $s "# match: (\d+) blocks matched"
$cover = $mb * 128 / (20 * 48000)
Check "B4C-3" ($md -eq 0 -and $mr -eq 0 -and $cover -ge 0.9) "demo 20 s: match dropouts $md, mismatch runs $mr, matched $([math]::Round($cover * 100, 1)) % of the run"
# B4C-2
$k1 = Join-Path $Out "k1"
$okk = $true; $what = @()
foreach ($e in "corner", "right") {
  $d = Join-Path $k1 "play-$e"
  pwsh -NoProfile -File "$PSScriptRoot\menu_flicker_b4c.ps1" -Out $d -Exe $Exe -Edge $e -Play -DarkMenus -Below 60 | Out-Null
  $f = (python "$PSScriptRoot\flicker_b4c.py" $d 60 | ConvertFrom-Json)
  $b = (python "$PSScriptRoot\blocks_b4c.py" $d | ConvertFrom-Json)
  $okk = $okk -and $f.frames -ge 500 -and $f.flash_frames -eq 0 -and $f.misdrawn_frames -eq 0 -and $b.light_block_frames -eq 0
  $what += "$e $($f.frames) frames: flash $($f.flash_frames), misdrawn $($f.misdrawn_frames), blocks $($b.light_block_frames)"
}
$d = Join-Path $k1 "selftest"
pwsh -NoProfile -File "$PSScriptRoot\menu_flicker_b4c.ps1" -Out $d -Exe $Exe -SelfTest -DarkMenus | Out-Null
$st = (python "$PSScriptRoot\flicker_b4c.py" $d | ConvertFrom-Json)
$okk = $okk -and $st.flash_frames -ge 8
$what += "control $($st.flash_frames) flash frames"
Check "B4C-2" $okk ($what -join "; ")
$results | Set-Content (Join-Path $Out "summary.txt")
$results
