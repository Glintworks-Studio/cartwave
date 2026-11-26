# usage: at_b4a.ps1 [-Exe .\target\release\app.exe] [-Work <dir>] [-Only B4A-01,B4A-05]
# Batch 4a scenarios (BATCH4A_PROMPT.md §3–7, UX.md AT-13), the Batch 3 harness method:
# synthetic input into the real app, saved files, settings.json and dev-mode stderr lines,
# checked by check_b4a.py. Client px at 100%: top bar 76, Overview 76..108, ruler 108..136,
# rows from 136 every 73 px; browser 0..241; headers 241..621; lanes 621.. at 92 px per bar.
param([string]$Exe = ".\target\release\app.exe",
  [string]$Work = (Join-Path $env:TEMP ("chipdaw-at-b4a-" + (Get-Date -Format "MMdd-HHmmss"))),
  [string[]]$Only = @())
$global:Work = $Work
New-Item -ItemType Directory -Force $Work | Out-Null
. (Join-Path $PSScriptRoot "..\..\batch3\tools\lib.ps1")
Set-Location $Repo
$global:Exe = (Resolve-Path $Exe).Path
python "$Repo\docs\batch3\tools\fixtures.py" $Work | Out-Null
python "$Repo\docs\batch4a\tools\fixtures_b4a.py" $Work | Out-Null
foreach ($k in "CHIPDAW_OVERVIEW", "CHIPDAW_HIDE_STATUS", "CHIPDAW_FAKE_PROBLEM") { Remove-Item "Env:$k" -ErrorAction SilentlyContinue }
$obs = [ordered]@{ work = $Work }
function ClickL($x, $y) { MoveL $x $y; Start-Sleep -Milliseconds 150; Down; Up; Start-Sleep -Milliseconds 300 }
function DragL($x0, $y0, $x1, $y1) { MoveL $x0 $y0; Start-Sleep -Milliseconds 150; $a = P $x0 $y0; $b = P $x1 $y1; Down; for ($i = 1; $i -le 16; $i++) { Move-To ([int]($a[0] + ($b[0] - $a[0]) * $i / 16)) ([int]($a[1] + ($b[1] - $a[1]) * $i / 16)); Start-Sleep -Milliseconds 15 }; Up; Start-Sleep -Milliseconds 300 }
$Only = @($Only | ForEach-Object { $_ -split "," } | Where-Object { $_ })
function Want($id) { $Only.Count -eq 0 -or $Only -contains $id }
function Fx($name, $as) { $p = Join-Path $Work "$as.chipdaw"; Copy-Item (Join-Path $Work "$name.chipdaw") $p -Force; $p }
function Keep($proj, $as) { Copy-Item $proj (Join-Path $Work "$as.chipdaw") -Force }
function KeepSettings($d, $as) { Copy-Item (Join-Path $d "settings.json") (Join-Path $Work "$as.settings.json") -Force -ErrorAction SilentlyContinue }
function DevLines($d, $pat) { @(StdErr $d | Where-Object { $_ -match $pat }) }
$RowsTop = 136; $Pitch = 73
function RowY($i) { $RowsTop + $Pitch * $i }
function BarX($bar) { 621 + 92 * ($bar - 1) }

# ─────────── B4A-01: Return and Backspace never reach the song behind a dialog (MC-D9) ───────────
if (Want "B4A-01") {
  $p = Fx "fx_basic" "b401"; $d = Join-Path $Work "d401"
  Launch -Project $p -Data $d
  ClickL ((BarX 2) + 10) ((RowY 0) + 40)          # select Lead A
  CtrlShift 0x52; Start-Sleep -Milliseconds 500    # Export Audio… (Ctrl+Shift+R)
  Shot "b401_export"
  K 0x08; K 0x2E                                   # Backspace, Delete
  # Return would run the export: the dialog consumes it. Close with Esc first.
  K 0x1B; Start-Sleep -Milliseconds 300
  Save; Keep $p "b401_after_export"
  ClickL ((BarX 2) + 10) ((RowY 0) + 40)
  Ctrl 0xBC; Start-Sleep -Milliseconds 500         # Settings… (Ctrl+,): a modal panel
  K 0x0D; K 0x08; K 0x2E
  Shot "b401_settings"
  K 0x1B; Start-Sleep -Milliseconds 300
  Save; Keep $p "b401_after_modal"
  $obs.b401_piano = @(DevLines $d "^dev: open piano roll")
  Stop-App
}


# The colour grid's cell (row r, column c) for a submenu opened from a menu at (mx, my) whose
# "Colour" row is the n-th (0-based) row (the polish harness's Cell).
function Cell($mx, $my, $n, $r, $c, $rowsAbove = 0) {
  $sx = $mx + 240 - 4; $sy = $my + $n * 28
  @(($sx + 4 + 8 + $c * 20 + 8), ($sy + 4 + $rowsAbove * 28 + 8 + $r * 20 + 8))
}

# ─────────── B4A-02: colour previews never leak (MC-D15) ───────────
if (Want "B4A-02") {
  $p = Fx "fx_basic" "b402"; $d = Join-Path $Work "d402"
  Launch -Project $p -Data $d
  $mx = 400; $my = (RowY 2) + 5
  # (1) hover a colour, Esc: the old colour is back, nothing unsaved.
  $obs.b402_title0 = Title
  RightL $mx $my; Start-Sleep -Milliseconds 300
  MoveL ($mx + 30) ($my + 4 + 28 + 14); Start-Sleep -Milliseconds 500
  $c = Cell $mx $my 1 1 8; MoveL $c[0] $c[1]; Start-Sleep -Milliseconds 400
  Shot "b402_hover"
  K 0x1B; Start-Sleep -Milliseconds 300
  $obs.b402_title_esc = Title
  Save; Keep $p "b402_esc"
  # (2) hover A, pick B, undo: the original colour.
  RightL $mx $my; Start-Sleep -Milliseconds 300
  MoveL ($mx + 30) ($my + 4 + 28 + 14); Start-Sleep -Milliseconds 500
  $a = Cell $mx $my 1 1 8; MoveL $a[0] $a[1]; Start-Sleep -Milliseconds 400
  $b = Cell $mx $my 1 2 3; ClickL $b[0] $b[1]; Start-Sleep -Milliseconds 300
  Save; Keep $p "b402_picked"
  Ctrl 0x5A; Save; Keep $p "b402_undone"
  Stop-App
}


# ─────────── B4A-03: Ctrl-click toggles a clip, Shift-click selects a range (R28) ───────────
if (Want "B4A-03") {
  $p = Fx "fx_basic" "b403"; $d = Join-Path $Work "d403"
  Launch -Project $p -Data $d
  $lead = @(((BarX 2) + 10), ((RowY 0) + 40)); $harmA = @(((BarX 1) + 40), ((RowY 1) + 40))
  $harmB = @(((BarX 3) + 40), ((RowY 1) + 40)); $bass = @(((BarX 6) + 10), ((RowY 2) + 40))
  ClickL $lead[0] $lead[1]; Start-Sleep -Milliseconds 500
  CtrlClickL $harmB[0] $harmB[1]; Start-Sleep -Milliseconds 500
  Save; Keep $p "b403_ctrl_click"                      # no copies from a Ctrl-click
  K 0x2E; Save; Keep $p "b403_del_two"                 # Lead A + Harm B deleted
  Ctrl 0x5A; Start-Sleep -Milliseconds 300
  ClickL $lead[0] $lead[1]; Start-Sleep -Milliseconds 500
  CtrlClickL $harmB[0] $harmB[1]; Start-Sleep -Milliseconds 500
  CtrlClickL $lead[0] $lead[1]; Start-Sleep -Milliseconds 500   # toggles Lead A off
  K 0x2E; Save; Keep $p "b403_del_toggled"             # only Harm B deleted
  Ctrl 0x5A; Start-Sleep -Milliseconds 300
  ClickL $harmA[0] $harmA[1]; Start-Sleep -Milliseconds 500
  ShiftClickL $bass[0] $bass[1]; Start-Sleep -Milliseconds 500  # range: tracks 2..3, bars 1..8
  K 0x2E; Save; Keep $p "b403_del_range"
  Ctrl 0x5A; Start-Sleep -Milliseconds 300
  ClickL $lead[0] $lead[1]; Start-Sleep -Milliseconds 500
  KeyDown 0x11; DragL $lead[0] $lead[1] ($lead[0] + 92 * 4) $lead[1]; KeyUp 0x11   # Ctrl-drag copies (A6)
  Save; Keep $p "b403_ctrl_drag"
  Stop-App
}


# A menu item's centre: a menu opened at (mx, my); `rows` items above (28 px) and `seps` separators (8 px).
function MenuItemAt($mx, $my, $rows, $seps = 0) { @(($mx + 40), ($my + 4 + 28 * $rows + 8 * $seps + 14)) }
function Badge($i) { @(318, ((RowY $i) + 49)) }   # the badge's left part (name column x 309 + 9)
function NameAt($i) { @(420, ((RowY $i) + 22)) }
$MainY = 823                                       # the Main row's top (client px at 1440 x 920)

# ─────────── B4A-04 / AT-13: insert a track (Ctrl+Shift+T), drag it to the top, delete it, colour ───────────
if (Want "B4A-04") {
  $p = Fx "fx_basic" "b404"; $d = Join-Path $Work "d404"
  Launch -Project $p -Data $d
  $n = NameAt 0; ClickL $n[0] $n[1]; Start-Sleep -Milliseconds 400
  CtrlShift 0x54; Start-Sleep -Milliseconds 500        # Ctrl+Shift+T: a studio track below track 1
  Save; Keep $p "b404_inserted"
  Shot "insert-track"
  $a = NameAt 1; $b = NameAt 0
  DragL $a[0] $a[1] $a[0] ($b[1] - 20); Start-Sleep -Milliseconds 400   # drag it above track 1
  Save; Keep $p "b404_moved"
  K 0x2E; Start-Sleep -Milliseconds 400                 # Delete: the selected (moved) track
  Save; Keep $p "b404_deleted"
  # Colour from the header's menu (AT-13's last step; the polish grid).
  $mx = 400; $my = (RowY 2) + 5
  RightL $mx $my; Start-Sleep -Milliseconds 300
  MoveL ($mx + 30) ($my + 4 + 28 + 14); Start-Sleep -Milliseconds 500
  $c = Cell $mx $my 1 1 8; ClickL $c[0] $c[1]; Start-Sleep -Milliseconds 300
  Save; Keep $p "b404_colour"
  $obs.b404_dev = @(DevLines $d "^dev: (insert|move) track")
  Stop-App
}

# ─────────── B4A-05: lock with reset ("Reset and lock"), one undo, unlock restores ───────────
if (Want "B4A-05") {
  $p = Fx "fx4_studio" "b405"; $d = Join-Path $Work "d405"
  Launch -Project $p -Data $d
  $bd = Badge 0; ClickL $bd[0] $bd[1]; Start-Sleep -Milliseconds 500
  Shot "lock-menu"
  $obs.b405_menu = @(DevLines $d "^dev: menu Badge")
  $mi = MenuItemAt 309 ((RowY 0) + 58 + 2) 1 1; ClickL $mi[0] $mi[1]  # "2A03 Pulse 1" (Studio, separator, P1)
  Start-Sleep -Milliseconds 800
  $obs.b405_dialog = Dialog-Text
  pwsh -NoProfile -File "$Repo\docs\batch4a\tools\screen.ps1" -Out "$Work\lock-reset-dialog.png" | Out-Null
  Press-Button "Reset and lock" | Out-Null; Start-Sleep -Milliseconds 500
  Save; Keep $p "b405_locked"
  Shot "locked-header"
  Ctrl 0x5A; Start-Sleep -Milliseconds 300; Save; Keep $p "b405_undone"
  Ctrl 0x59; Start-Sleep -Milliseconds 300; Save; Keep $p "b405_redone"
  $bd = Badge 0; ClickL $bd[0] $bd[1]; Start-Sleep -Milliseconds 500
  $mi = MenuItemAt 309 ((RowY 0) + 58 + 2) 0 0; ClickL $mi[0] $mi[1]  # "Studio (unlocked)"
  Start-Sleep -Milliseconds 500; Save; Keep $p "b405_unlocked"
  # A taken channel is disabled: track 2's badge lists 2A03 Pulse 1 (taken by nobody now) and P2.
  $obs.b405_dev = @(DevLines $d "^dev: lock track")
  Stop-App
}

# ─────────── B4A-06: profile switching asks; Unlock / Switch anyway / Cancel ───────────
if (Want "B4A-06") {
  foreach ($ans in @("unlock", "fit", "cancel")) {
    $p = Fx "fx4_studio" "b406_$ans"; $d = Join-Path $Work "d406$ans"
    $env:CHIPDAW_WARN_ANSWER = $ans
    Launch -Project $p -Data $d
    Invoke-Menu "File" 8; Start-Sleep -Milliseconds 400
    $obs."b406_click_$ans" = Click-Text "NES (2A03)"
    Start-Sleep -Milliseconds 500
    if ($ans -eq "unlock") { Shot "budget" }
    K 0x1B; Start-Sleep -Milliseconds 300
    Save; Keep $p "b406_$ans"
    $obs."b406_warn_$ans" = @(DevLines $d "^dev: warning")
    Stop-App
    Remove-Item Env:CHIPDAW_WARN_ANSWER
  }
}

# ─────────── B4A-07: a chord on a locked track asks; Fit / Unlock / Cancel ───────────
if (Want "B4A-07") {
  foreach ($ans in @("fit", "unlock", "cancel")) {
    $p = Fx "fx4_chord" "b407_$ans"; $d = Join-Path $Work "d407$ans"
    $env:CHIPDAW_WARN_ANSWER = $ans
    Launch -Project $p -Data $d
    $cl = @(((BarX 1) + 40), ((RowY 0) + 40)); DblL $cl[0] $cl[1]; Start-Sleep -Milliseconds 800   # open the clip
    K 0x42; Start-Sleep -Milliseconds 300                         # B: draw mode
    $ln = DevLines $d "^dev: piano roll bounds"
    $obs."b407_bounds_$ans" = @($ln)
    $bx = 0; $by = 0; $bw = 0; $bh = 0
    if ($ln.Count -gt 0) { $f = ($ln[-1] -split " "); $bx = [double]$f[4]; $by = [double]$f[5]; $bw = [double]$f[6]; $bh = [double]$f[7] }
    # Two notes at the same time on two rows: a chord.
    $x = $bx + 120; ClickL $x ($by + $bh * 0.45); Start-Sleep -Milliseconds 700
    ClickL $x ($by + $bh * 0.30); Start-Sleep -Milliseconds 900
    if ($ans -eq "fit") { Shot "profile-warning-after" }
    K 0x1B; Start-Sleep -Milliseconds 300
    Save; Keep $p "b407_$ans"
    $obs."b407_warn_$ans" = @(DevLines $d "^dev: warning")
    Stop-App
    Remove-Item Env:CHIPDAW_WARN_ANSWER
  }
}

# ─────────── B4A-08: the Main gain (wheel), PEAK resets, saved ───────────
if (Want "B4A-08") {
  $p = Fx "fx_basic" "b408"; $d = Join-Path $Work "d408"
  Launch -Project $p -Data $d
  Shot "main-track"
  MoveL 589 ($MainY + 30); Start-Sleep -Milliseconds 200
  Wheel -1; Start-Sleep -Milliseconds 200; Wheel -1; Start-Sleep -Milliseconds 200; Wheel -1; Start-Sleep -Milliseconds 300
  $obs.b408_title = Title
  Save; Keep $p "b408_minus3"
  MoveL 520 ($MainY + 22); Start-Sleep -Milliseconds 300
  $obs.b408_help = @(DevLines $d "^dev: help .*PEAK")
  Stop-App
}

# ─────────── B4A-09: Insert Track ▸ a type from the header menu; Voices; Duplicate ───────────
if (Want "B4A-09") {
  $p = Fx "fx4_studio" "b409"; $d = Join-Path $Work "d409"
  Launch -Project $p -Data $d
  $mx = 400; $my = (RowY 0) + 5
  RightL $mx $my; Start-Sleep -Milliseconds 300
  $obs.b409_menu = @(DevLines $d "^dev: menu Track")
  # Rename, Colour, sep, Insert Track ▸ (row 2 after one separator): its submenu, then "Noise" (3rd).
  $it = MenuItemAt $mx $my 2 1; MoveL $it[0] $it[1]; Start-Sleep -Milliseconds 500
  $sub = DevLines $d "^dev: submenu InsertTrack"
  $obs.b409_sub = @($sub)
  if ($sub.Count -gt 0) { $f = ($sub[-1] -split " "); $sx = [double]$f[4]; $sy = [double]$f[5]; $n = MenuItemAt $sx $sy 2 0; ClickL $n[0] $n[1] }
  Start-Sleep -Milliseconds 500
  Save; Keep $p "b409_noise"
  # Voices ▸ 4 on track 1 (studio): Rename, Colour, sep, Insert, Duplicate, Delete, sep, Lock, Voices.
  RightL $mx $my; Start-Sleep -Milliseconds 300
  $it = MenuItemAt $mx $my 6 2; MoveL $it[0] $it[1]; Start-Sleep -Milliseconds 500
  $sub = DevLines $d "^dev: submenu Voices"
  if ($sub.Count -gt 0) { $f = ($sub[-1] -split " "); $sx = [double]$f[4]; $sy = [double]$f[5]; $n = MenuItemAt $sx $sy 4 0; ClickL $n[0] $n[1] }
  Start-Sleep -Milliseconds 500
  Save; Keep $p "b409_voices"
  $n = NameAt 0; ClickL $n[0] $n[1]; Start-Sleep -Milliseconds 300
  Ctrl 0x44; Start-Sleep -Milliseconds 500              # Ctrl+D with the track selected: Duplicate Track
  Save; Keep $p "b409_dup"
  Stop-App
}

$obs | ConvertTo-Json -Depth 5 | Set-Content -Encoding utf8 (Join-Path $Work "obs.json")
python "$Repo\docs\batch4a\tools\check_b4a.py" $Work
