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

$obs | ConvertTo-Json -Depth 5 | Set-Content -Encoding utf8 (Join-Path $Work "obs.json")
python "$Repo\docs\batch4a\tools\check_b4a.py" $Work
