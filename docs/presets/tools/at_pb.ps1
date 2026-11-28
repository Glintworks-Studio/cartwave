# usage: at_pb.ps1 [-Exe .\target\release\cartwave.exe] [-Work <dir>] [-Only PB-01,PB-05]
# The Presets + Browser + device panel scenarios (PRESETS_PROMPT.md §8.4), PASS/FAIL each, the
# Batch 3 harness method: synthetic input into the real app, dev-mode stderr lines, screenshots.
# Client px at 100 % (1440 x 920): top bar 0..76, Overview 76..108, ruler 108..136, rows from
# 136 every 73 px; the browser 0..421 (header 76..116, search 116..144, sidebar and content
# from 152: the content's title 152..180, rows from 180 every 24); headers 421..801, lanes 801..;
# with a selection the bottom pane 540..896 holds the CLIP section (12..212) and the device
# (224..1104, 552..884); the status bar 896..920.
param([string]$Exe = ".\target\release\cartwave.exe",
  [string]$Work = (Join-Path $env:TEMP ("cartwave-at-pb-" + (Get-Date -Format "MMdd-HHmmss"))),
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
function ClickL($x, $y) { MoveL $x $y; Start-Sleep -Milliseconds 150; Down; Up; Start-Sleep -Milliseconds 350 }
function DblL($x, $y) { MoveL $x $y; Start-Sleep -Milliseconds 150; Down; Up; Start-Sleep -Milliseconds 60; Down; Up; Start-Sleep -Milliseconds 500 }
function DragL($x0, $y0, $x1, $y1) { MoveL $x0 $y0; Start-Sleep -Milliseconds 150; $a = P $x0 $y0; $b = P $x1 $y1; Down; for ($i = 1; $i -le 24; $i++) { Move-To ([int]($a[0] + ($b[0] - $a[0]) * $i / 24)) ([int]($a[1] + ($b[1] - $a[1]) * $i / 24)); Start-Sleep -Milliseconds 20 }; Start-Sleep -Milliseconds 200; Up; Start-Sleep -Milliseconds 500 }
function Lines($d, $pat) { @(StdErr $d | Where-Object { $_ -match $pat }) }
function Last($d, $pat) { (Lines $d $pat | Select-Object -Last 1) }
$results = @()
function Check($id, $ok, $what) { $line = "{0} {1}  {2}" -f $(if ($ok) { "PASS" } else { "FAIL" }), $id, $what; $script:results += $line; $line }
function RowY($i) { 136 + 73 * $i }
function Header($i) { ClickL 520 ((RowY $i) + 24) }
function BRow($k) { 180 + 24 * $k + 12 }      # the browser content's k-th row (centre)
function Side($name) { $ys = @{ Favourites = 192; Sounds = 244; Drums = 268; Instruments = 292; Hardware = 316; Clips = 340; Samples = 364; User = 416; Project = 440 }; ClickL 75 $ys[$name] }
$PrevX = 515; $FieldX = 664; $NextX = 813; $HeadY = 572
$VoiceX = 452; $VoiceY = 625
# A new studio project (File > New: Pulse, Pulse, Triangle, Noise; no profile).
function NewProject { Ctrl 0x4E; Start-Sleep -Milliseconds 800 }
function Launch-Fresh($n) { $d = Join-Path $Work "d$n"; Remove-Item -Recurse -Force $d -ErrorAction SilentlyContinue; Launch -Data $d; Place; $d }

# ─────────── PB-01: the Voice selector switches a studio track's voice; one undo step ───────────
if (Want "PB-01") {
  $d = Launch-Fresh "01"; NewProject
  Header 0
  $dev0 = Last $d "^dev: device "
  ClickL $VoiceX $VoiceY
  $menu = Last $d "^dev: menu Voice"
  ClickL 452 767                                   # VRC6 Saw (the menu's fifth row)
  $sw = Last $d "^dev: voice track 0 -> Saw"
  Shot "pb01_saw"
  $devSaw = Last $d "^dev: device "
  Ctrl 0x5A; Start-Sleep -Milliseconds 500          # Undo
  $devUndo = Last $d "^dev: device "
  Ctrl 0x59; Start-Sleep -Milliseconds 500          # Redo
  $devRedo = Last $d "^dev: device "
  Check "PB-01" ($menu -match "Pulse\|Triangle\|Noise\|VRC6 Pulse\|VRC6 Saw" -and $sw -and $devSaw -match "Chip Synth Saw studio" -and $devSaw -match "RATE\|UNISON" -and $devUndo -match "Chip Synth Pulse studio" -and $devRedo -match "Chip Synth Saw studio") "menu [$menu] switch [$sw] after [$devSaw] undo [$devUndo] redo [$devRedo]"
  Stop-App
}

# ─────────── PB-02: a locked track: the real channel's controls only, displays and knobs dimmed, labels readable ───────────
if (Want "PB-02") {
  $d = Launch-Fresh "02"
  Header 0; Start-Sleep -Milliseconds 400
  $locked = Last $d "^dev: device "
  Shot "pb02_locked"
  NewProject; Header 0; Start-Sleep -Milliseconds 400
  $studio = Last $d "^dev: device "
  Shot "pb02_studio"
  $m = python "$Repo\docs\presets\tools\check_pb.py" dim "$Work\pb02_locked.png" "$Work\pb02_studio.png"
  $noStudioOnly = ($locked -notmatch "UNISON|DETUNE|DRIFT") -and ($studio -match "UNISON\|DETUNE\|DRIFT")
  Check "PB-02" ($locked -match "Pulse locked" -and $noStudioOnly -and $locked -match "DUTY" -and ($m -match "^ok")) "locked [$locked] studio [$studio] pixels [$m]"
  Stop-App
}

# ─────────── PB-03: folders open and close; PB-04: search across everything; PB-05: select previews ───────────
if ((Want "PB-03") -or (Want "PB-04") -or (Want "PB-05")) {
  $d = Launch-Fresh "03"; NewProject
  Header 0
  Side "Sounds"
  ClickL 260 (BRow 0)                               # Leads: open
  $open = Last $d "^dev: browser open Sounds/Leads$"
  $rowsOpen = Last $d "^dev: browser Sounds rows"
  ClickL 260 (BRow 0)                               # Leads: close
  $close = Last $d "^dev: browser close Sounds/Leads$"
  Check "PB-03" ($open -and $close -and $rowsOpen -match "rows 15 ") "[$open] [$rowsOpen] [$close]"
  ClickL 220 130; TypeText "kick"; Start-Sleep -Milliseconds 500
  $q = Last $d "^dev: browser .* query `"kick`""
  Shot "pb04_search"
  $n = if ($q -match "items (\d+)") { [int]$Matches[1] } else { 0 }
  K 0x1B; Start-Sleep -Milliseconds 300
  $cleared = Last $d "^dev: browser .* query `"`""
  Check "PB-04" ($n -ge 6 -and $cleared) "search kick: $n items [$q]; Esc [$cleared]"
  ClickL 260 (BRow 0); ClickL 260 (BRow 1)          # Leads, Square
  ClickL 300 (BRow 2)                               # Bright Square Lead: select
  $aud = Last $d "^dev: audition preset"
  Check "PB-05" ($aud -match "audition preset \d+ lane 0") "select previews: [$aud]"
  Stop-App
}

# ─────────── PB-06 double-click loads; PB-07 drag onto a track; PB-08 drag onto empty space makes a track ───────────
if ((Want "PB-06") -or (Want "PB-07") -or (Want "PB-08")) {
  $d = Launch-Fresh "06"; NewProject
  Header 0
  Side "Sounds"; ClickL 260 (BRow 0); ClickL 260 (BRow 1)
  DblL 300 (BRow 2)                                 # Bright Square Lead -> track 0
  $l6 = Last $d "^dev: preset \d+ `"Bright Square Lead`" on track 0"
  Check "PB-06" ($l6 -match "voice Pulse lock None") "double-click [$l6]"
  DragL 300 (BRow 3) 600 ((RowY 1) + 36)            # Late Vibrato Lead -> track 1's header
  $l7 = Last $d "^dev: preset \d+ `"Late Vibrato Lead`" on track 1"
  Check "PB-07" ($l7) "drag onto a header [$l7]"
  ClickL 1200 ((RowY 1) + 36)                       # keep a selection (the bottom pane open)
  DragL 300 (BRow 4) 1100 455                       # Duty Sweep Lead -> empty space under the tracks
  $l8 = Last $d "^dev: new track 4 from preset"
  $l8b = Last $d "^dev: preset \d+ `"Duty Sweep Lead`" on track 4"
  Shot "pb08_new_track"
  Check "PB-08" ($l8 -and $l8b) "drag onto empty space [$l8] [$l8b]"
  Stop-App
}

# ─────────── PB-09: loading from Hardware locks the track and sets the profile; another chip asks ───────────
if (Want "PB-09") {
  $d = Launch-Fresh "09"; NewProject
  Header 0
  Side "Hardware"
  ClickL 260 (BRow 0); ClickL 260 (BRow 1)          # NES (2A03), Pulse
  DblL 300 (BRow 2)                                 # Duty Lead: no question (no profile yet)
  $l1 = Last $d "^dev: preset \d+ `"Duty Lead`" on track 0"
  $asked1 = @(Lines $d "^dev: warning").Count
  ClickL 260 (BRow 0)                               # close NES
  ClickL 260 (BRow 1); ClickL 260 (BRow 3)          # Famicom + VRC6, VRC6 Saw
  Header 1
  MoveL 300 (BRow 4); Down; Up; Start-Sleep -Milliseconds 60; Down; Up   # Saw Brass (double-click: a dialog follows)
  $txt = (Find-Dialog 4000) -ne $null
  $q = Last $d "^dev: warning A preset for another chip setup"
  Press-Button "Switch to Famicom + VRC6" | Out-Null; Start-Sleep -Milliseconds 800
  $l2 = Last $d "^dev: preset \d+ `"Saw Brass`" on track 1"
  Shot "pb09_hardware"
  Check "PB-09" ($l1 -match "lock Some\(0\).*profile Base2A03" -and $asked1 -eq 0 -and $q -and $l2 -match "voice Saw lock Some\(7\).*profile WithVrc6") "None -> NES, no question [$l1]; NES -> VRC6 asks [$q] then [$l2]"
  Stop-App
}

# ─────────── PB-10: ◀ ▶ step while playing, loaded at once, one undo step for the run ───────────
if (Want "PB-10") {
  $d = Launch-Fresh "10"
  Header 0                                          # Pulse 1 (locked, Square Lead: Sounds/Leads/Classic)
  K 0x20; Start-Sleep -Milliseconds 600              # play
  ClickL $NextX $HeadY; ClickL $NextX $HeadY; ClickL $NextX $HeadY
  $steps = @(Lines $d "^dev: step preset \+1")
  $playing = (Last $d "^dev: (play|transport)")
  K 0x20; Start-Sleep -Milliseconds 400              # stop
  $after = Last $d "^dev: device "
  Ctrl 0x5A; Start-Sleep -Milliseconds 600
  $undo = Last $d "^dev: device "
  $u = @($steps | ForEach-Object { if ($_ -match "undo steps \((\d+), 0\)") { $Matches[1] } }) | Select-Object -Unique
  Check "PB-10" ($steps.Count -eq 3 -and $u.Count -eq 1 -and $after -notmatch 'preset "Square Lead"' -and $undo -match 'preset \("Square Lead", false\)') "steps $($steps.Count) undo positions [$($u -join ',')]; after [$after]; one undo [$undo]"
  Stop-App
}

# ─────────── PB-11: a studio preset on a locked track asks: Unlock track · Cancel ───────────
if (Want "PB-11") {
  $d = Launch-Fresh "11"
  Header 0                                          # Pulse 1, locked
  Side "Sounds"; ClickL 260 (BRow 0); ClickL 260 (BRow 4)   # Leads, Detuned
  MoveL 300 (BRow 5); Down; Up; Start-Sleep -Milliseconds 60; Down; Up   # Supersquare
  $dlg = (Find-Dialog 4000) -ne $null
  Press-Button "Cancel" | Out-Null; Start-Sleep -Milliseconds 600
  $c = Last $d "^dev: device "
  MoveL 300 (BRow 5); Down; Up; Start-Sleep -Milliseconds 60; Down; Up
  $dlg2 = (Find-Dialog 4000) -ne $null
  Press-Button "Unlock track" | Out-Null; Start-Sleep -Milliseconds 800
  $u = Last $d "^dev: device "
  $q = Last $d "^dev: warning A studio preset on a locked track"
  Check "PB-11" ($dlg -and $q -and $c -match "Pulse locked" -and $dlg2 -and $u -match "Pulse studio preset \(`"Supersquare`"") "asked [$q]; Cancel [$c]; Unlock [$u]"
  Stop-App
}

# ─────────── PB-12: Shift+Tab switches the bottom pane between the Clip and the Device view ───────────
if (Want "PB-12") {
  $d = Launch-Fresh "12"
  DblL 830 ((RowY 0) + 40)                          # open Intro Lead in the piano roll
  $open = Last $d "^dev: open piano roll|^dev: CARTWAVE_OPEN_CLIP|piano"
  KeyDown 0x10; Key 0x09; KeyUp 0x10; Start-Sleep -Milliseconds 500
  $dev = Last $d "^dev: detail device"
  Shot "pb12_device"
  KeyDown 0x10; Key 0x09; KeyUp 0x10; Start-Sleep -Milliseconds 500
  $clip = Last $d "^dev: detail clip"
  Header 2; Start-Sleep -Milliseconds 300            # a track header: its Device view
  $hdr = Last $d "^dev: device .*Triangle"
  Check "PB-12" ($dev -and $clip -and $hdr) "[$dev] [$clip] header [$hdr]"
  Stop-App
}

# ─────────── PB-13: the preset field opens the picker (the browser's folders), Show in Browser ───────────
if (Want "PB-13") {
  $d = Launch-Fresh "13"
  Header 0
  ClickL $FieldX $HeadY
  $pk = Last $d "^dev: picker open"
  Shot "pb13_picker"
  ClickL $FieldX 519                                # Show in Browser (the picker opens above the field)
  $sib = Last $d "^dev: show in browser Some"
  $rev = Last $d "^dev: browser Sounds rows"
  Check "PB-13" ($pk -match "For this channel \(2A03 Pulse 1\)\|Sounds\|Drums\|Instruments\|Hardware" -and $sib -and $rev) "picker [$pk] show [$sib] browser [$rev]"
  Stop-App
}

$results | Set-Content (Join-Path $Work "pb-summary.txt")
"PB: {0} PASS, {1} FAIL" -f @($results | Where-Object { $_ -like "PASS*" }).Count, @($results | Where-Object { $_ -like "FAIL*" }).Count
