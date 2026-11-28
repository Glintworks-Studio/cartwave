# usage: at_b5.ps1 [-Exe .\target\release\cartwave.exe] [-Work <dir>] [-Only AU-11,AU-15]
# Batch 5's app scenarios (BATCH5_PROMPT.md §8.3; the Batch 3 harness method: synthetic input
# into the real app, dev-mode stderr lines, saved project files checked with check_b5.py).
# Client px at 100 % (1440 x 920) with the Overview on: top bar 0..76 (Automation Arm at 358, 38;
# Re-Enable Automation at 407, 38), Overview 76..108, ruler 108..136, rows from 136 (73 px). In
# Automation Mode each track's first lane is the 73 px under its row: track 0's 209..282, track 1's
# 355..428. The studio fixture (v7_studio_demo, 150 BPM): bar n at x = 805 + 92 (n - 1); a
# Volume lane's 0 dB line at y 220 (track 0) and 366 (track 1). Track 0's header fader thumb
# at (770, 147).
# The unit-level AU tests (AU-01 … AU-10: the model, the engine, the export, format 10) are
# cargo tests (src/automation.rs, tests/automation.rs, tests/migration.rs, chip and engine).
param([string]$Exe = ".\target\release\cartwave.exe",
  [string]$Work = (Join-Path $env:TEMP ("cartwave-at-b5-" + (Get-Date -Format "MMdd-HHmmss"))),
  [string[]]$Only = @())
$global:Work = $Work
New-Item -ItemType Directory -Force $Work | Out-Null
. (Join-Path $PSScriptRoot "..\..\batch3\tools\lib.ps1")
Set-Location $Repo
$global:Exe = (Resolve-Path $Exe).Path
foreach ($k in "CARTWAVE_OVERVIEW", "CARTWAVE_HIDE_STATUS", "CARTWAVE_BROWSER_W", "CARTWAVE_PROJECT") { Remove-Item "Env:$k" -ErrorAction SilentlyContinue }
$Only = @($Only | ForEach-Object { $_ -split "," } | Where-Object { $_ })
function Want($id) { $Only.Count -eq 0 -or $Only -contains $id }
function Place { pwsh -NoProfile -File "$Repo\docs\perf\tools\shot.ps1" -Out "$global:Work\_place.png" | Out-Null; Start-Sleep -Milliseconds 300 }
function Lines($d, $pat) { @(StdErr $d | Where-Object { $_ -match $pat }) }
function Last($d, $pat) { (Lines $d $pat | Select-Object -Last 1) }
$results = @()
function Check($id, $ok, $what) { $line = "{0} {1}  {2}" -f $(if ($ok) { "PASS" } else { "FAIL" }), $id, $what; $script:results += $line; $line }
function BarX($n) { 805 + 92 * ($n - 1) }
function AltDragL($x0, $y0, $x1, $y1) { KeyDown 0x12; DragL $x0 $y0 $x1 $y1; KeyUp 0x12; Start-Sleep -Milliseconds 300 }
function SlowDragL($x0, $y0, $x1, $y1, $steps, $ms) { $a = P $x0 $y0; $b = P $x1 $y1; Move-To $a[0] $a[1]; Start-Sleep -Milliseconds 100; Down; for ($i = 1; $i -le $steps; $i++) { Move-To ([int]($a[0] + ($b[0] - $a[0]) * $i / $steps)) ([int]($a[1] + ($b[1] - $a[1]) * $i / $steps)); Start-Sleep -Milliseconds $ms }; Up; Start-Sleep -Milliseconds 300 }
function Pts($f, $t, $p) { (python "$Repo\docs\batch5\tools\check_b5.py" points $f $t $p) }
function At($f, $t, $p, [int[]]$ticks) { $a = @("$Repo\docs\batch5\tools\check_b5.py", "at", $f, "$t", $p) + @($ticks | ForEach-Object { "$_" }); (& python @a) }
function ShiftTab { KeyDown 0x10; Key 0x09; KeyUp 0x10; Start-Sleep -Milliseconds 300 }
function N($s) { if ($s -match "^n=(\d+)") { [int]$Matches[1] } else { -1 } }
# A copy of a fixture in the work dir, opened with the app (Ctrl+S saves into it).
function Fixture($name, $n) { $f = Join-Path $Work "$n-$name"; Copy-Item "$Repo\docs\songs\v9\$name" $f -Force; $f }
function Launch-With($f, $n, [string]$Settings = "", [switch]$Audio) {
  $d = Join-Path $Work "d$n"; Remove-Item -Recurse -Force $d -ErrorAction SilentlyContinue; New-Item -ItemType Directory -Force $d | Out-Null
  if ($Settings) { $Settings | Set-Content -Encoding ascii (Join-Path $d "settings.json") }
  Launch -Project $f -Data $d
  if ($Audio) {
    # Launch turns the engine off; this scenario needs the playhead to move.
    Stop-App
    $env:CARTWAVE_AUDIO = "1"
    $script:proc = Start-Process $global:Exe -PassThru -RedirectStandardError (Join-Path $d "stderr.txt")
    for ($i = 0; $i -lt 50; $i++) { Start-Sleep -Milliseconds 100; if ((Get-Process -Id $script:proc.Id).MainWindowHandle -ne 0) { break } }
    Start-Sleep -Milliseconds 1500
    $global:AppH = (Get-Process -Id $script:proc.Id).MainWindowHandle
    [In]::SetForegroundWindow($global:AppH) | Out-Null
    $env:CARTWAVE_AUDIO = "0"
  }
  Place; $d
}

# ─────────── AU-11: in the app, add, move, curve and delete breakpoints ───────────
if (Want "AU-11") {
  $f = Fixture "v7_studio_demo.chipdaw" "11"; $d = Launch-With $f "11"
  K 0x41                                              # A: Automation Mode
  ClickL (BarX 2) 220; ClickL (BarX 4) 220; ClickL (BarX 6) 220   # three points at 0 dB
  DragL (BarX 4) 220 (BarX 4) 262                     # the middle one down
  AltDragL (BarX 3) 241 (BarX 3) 211                  # Alt-drag bends the segment bar 2 -> 4
  ClickL (BarX 6) 220                                 # a click on a point deletes it
  Shot "au11_edit"
  Save
  $p = Pts $f 0 "track.gain"
  $ok = ($p -match "^n=2 \d+:0:(-?[0-9.]+) \d+:(-[0-9.]+):0") -and ([double]$Matches[1] -ne 0) -and ([double]$Matches[2] -lt -5)
  Check "AU-11" $ok "points [$p]"
  Stop-App
}

# ─────────── AU-12 Draw Mode; AU-13 a lane's time selection: copy, paste, delete ───────────
if ((Want "AU-12") -or (Want "AU-13")) {
  $f = Fixture "v7_studio_demo.chipdaw" "12"; $d = Launch-With $f "12"
  K 0x41
  K 0x42                                              # B: Draw Mode
  DragL ((BarX 2) + 4) 400 ((BarX 4) + 40) 380        # steps at the grid across bars 2..4
  Shot "au12_draw"
  K 0x42
  Save
  $p12 = Pts $f 1 "track.gain"
  $v = At $f 1 "track.gain" @(4100, 5000, 9000, 11000)
  $vals = @($v -split " " | ForEach-Object { [double]$_ })
  Check "AU-12" ((N $p12) -ge 8 -and $vals[0] -lt -20 -and $vals[3] -lt -15) "draw: [$($p12.Substring(0, [Math]::Min(120, $p12.Length)))] values [$v]"
  # AU-13: select bars 2..3 in the lane (above the line), copy, select bars 7..8, paste;
  # then select bars 3..4 and delete.
  DragL ((BarX 2) + 1) 362 ((BarX 3) + 1) 362
  Ctrl 0x43
  DragL ((BarX 7) + 1) 362 ((BarX 8) + 1) 362
  Ctrl 0x56
  Shot "au13_paste"
  Save
  $before = At $f 1 "track.gain" @(4100, 4700, 6000, 7000)
  # Bar 7 is 5 bars (19200 ticks) after bar 2: the selections start at the same offset.
  $after = At $f 1 "track.gain" @(23300, 23900, 25200, 26200)
  DragL ((BarX 3) + 1) 362 ((BarX 4) + 1) 362
  K 0x2E                                              # Delete
  Save
  $del = Pts $f 1 "track.gain"
  Check "AU-13" ($before -eq $after -and (N $del) -lt (N $p12) + 6) "copied bars 2-3 [$before] pasted at 7-8 [$after]; after delete [$($del.Substring(0, [Math]::Min(100, $del.Length)))]"
  Stop-App
}

# ─────────── AU-14: a moved clip carries its automation; with Lock Envelopes it doesn't ───────────
if (Want "AU-14") {
  foreach ($lock in @($false, $true)) {
    $n = if ($lock) { "14b" } else { "14" }
    $f = Fixture "v7_studio_demo.chipdaw" $n
    $set = if ($lock) { '{ "lock_envelopes": true }' } else { "" }
    $d = Launch-With $f $n -Settings $set
    K 0x41
    ClickL (BarX 2) 220; ClickL (BarX 4) 220                     # two points at 0 dB
    Start-Sleep -Milliseconds 700                                # (not a double-click)
    DragL (BarX 2) 220 (BarX 2) 262                              # the first one down
    Save
    $p0 = Pts $f 0 "track.gain"
    DragL 900 170 (900 + 92 * 8) 170                             # the first clip (bars 1-5) 8 bars right
    Save
    $p1 = Pts $f 0 "track.gain"
    if ($lock) {
      Check "AU-14b" ($p0 -eq $p1 -and (N $p0) -ge 2) "Lock Envelopes on: [$p0] -> [$p1]"
    } else {
      # The clip moved 8 bars (30720 ticks): its points moved with it.
      $want = @([regex]::Matches($p0, "(?:^| )(\d+):") | ForEach-Object { [int]$_.Groups[1].Value + 30720 })
      $moved = @($want | Where-Object { $p1 -match "(^| )$($_):" }).Count -eq $want.Count -and $want.Count -ge 2
      Check "AU-14" ($moved -and $p0 -ne $p1) "Lock Envelopes off: [$p0] -> [$p1]"
    }
    Stop-App
  }
}

# ─────────── AU-15 arm and record (touch), one undo step; AU-16 override and Re-Enable ───────────
if ((Want "AU-15") -or (Want "AU-16")) {
  $f = Fixture "v7_studio_demo.chipdaw" "15"; $d = Launch-With $f "15" -Audio
  K 0x41
  ClickL 358 38                                       # Automation Arm
  $arm = Last $d "^dev: automation arm true"
  K 0x20                                              # play
  Start-Sleep -Milliseconds 600
  SlowDragL 770 147 770 190 30 50                     # the fader, held about 1.5 s
  K 0x20                                              # stop
  Start-Sleep -Milliseconds 400
  Save
  $rec = Pts $f 0 "track.gain"
  $recLine = Last $d "^dev: automation recorded"
  Ctrl 0x5A                                           # Undo: the whole recording, one step
  Start-Sleep -Milliseconds 300
  Save
  $undo = Pts $f 0 "track.gain"
  Check "AU-15" ($arm -and (N $rec) -ge 8 -and (N $undo) -eq 0 -and $recLine) "arm [$arm] recorded n=$(N $rec) [$recLine]; one undo -> [$undo]"
  # AU-16: an envelope, disarmed, played, the fader moved: overridden; Re-Enable puts it back.
  Ctrl 0x59                                           # Redo: the recording again
  Start-Sleep -Milliseconds 300
  ClickL 358 38                                       # disarm
  K 0x20
  Start-Sleep -Milliseconds 500
  SlowDragL 770 160 770 150 6 40
  Start-Sleep -Milliseconds 300
  Shot "au16_overridden"
  K 0x20
  $over = Last $d "^dev: automation overridden track.gain"
  ClickL 407 38                                       # Re-Enable Automation
  $re = Last $d "^dev: re-enable automation \(true\)"
  Shot "au16_reenabled"
  Save
  $kept = Pts $f 0 "track.gain"
  Check "AU-16" ($over -and $re -and (N $kept) -ge 8) "override [$over] re-enable [$re] envelope kept n=$(N $kept)"
  Stop-App
}

# ─────────── AU-17: locked tracks: no fader or pan lane; lanes only for the channel's controls ───────────
if (Want "AU-17") {
  $f = Fixture "v9_vrc6_presets.chipdaw" "17"; $d = Launch-With $f "17"
  K 0x41
  ClickL 470 222                                      # Pulse 1's lane chooser
  $m0 = Last $d "^dev: menu AutoParam\(0, 0\)"
  Shot "au17_chooser"
  K 0x1B
  ClickL 470 514                                      # the Triangle's lane chooser (track 2)
  $m2 = Last $d "^dev: menu AutoParam\(2, 0\)"
  K 0x1B
  $ok = $m0 -and $m2 -and ($m0 -notmatch "Mixer . Volume|Pan") -and ($m0 -match "Duty") -and ($m2 -notmatch "Duty|Volume macro|Mixer") -and ($m2 -match "Gate")
  Check "AU-17" $ok "Pulse 1 [$m0] Triangle [$m2]"
  # The locked lane draws steps: two points ramping Duty 0 -> 3.
  ClickL (BarX 2) 237; DragL (BarX 2) 237 (BarX 2) 268; ClickL (BarX 5) 237; DragL (BarX 5) 237 (BarX 5) 214
  Shot "au17_locked_stepped"
  Stop-App
}

# ─────────── AU-18: the mixer hides the Detail View; Shift+Tab shows / hides it there (A1) ───────────
if (Want "AU-18") {
  $f = Fixture "v7_studio_demo.chipdaw" "18"; $d = Launch-With $f "18"
  K 0x09                                              # Tab: the mixer
  ClickL 509 213                                      # a strip's name: selects it
  Shot "au18_mixer_hidden"
  ShiftTab
  $shown = Last $d "^dev: mixer detail shown"
  Shot "au18_mixer_shown"
  [In]::SetForegroundWindow($global:AppH) | Out-Null; Start-Sleep -Milliseconds 300
  ClickL 509 213                                      # the strip again (keyboard back to the app)
  ShiftTab
  $hidden = Last $d "^dev: mixer detail hidden"
  K 0x09                                              # back to the arrangement
  ClickL 900 172                                      # a clip: selected (the arrangement unchanged)
  ShiftTab                                            # the arrangement's Shift+Tab: the Clip view
  $arr = Last $d "^dev: detail (clip|device)"
  Check "AU-18" ($shown -and $hidden -and $arr) "mixer [$shown] [$hidden] arrangement [$arr]"
  Stop-App
}

""
$results
$results | Set-Content -Encoding utf8 (Join-Path $Work "results.txt")
"work: $Work"
