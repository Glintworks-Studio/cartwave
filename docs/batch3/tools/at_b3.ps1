# usage: at_b3.ps1 [-Exe .\target\release\app.exe] [-Work <dir>] [-Only AT-07,AT-11] [-Scale 100]
# Acceptance tests for Batch 3 (UX.md §6): AT-07..12, 14, 22 and AT-23..AT-31 (the other M/S
# lines this batch implemented). Synthetic input into the real app (lib.ps1); every test
# saves the project it edited, and check_b3.py checks the saved files, the dev-mode stderr
# lines (context-menu items, zoom ranges, piano-roll bounds) and the screenshots' pixels.
# Fixtures: fixtures.py (format v4, known clips and notes). Window 1440x900 logical, audio off.
# Coordinates are logical client px at 100% (Option D layout, design pass): bar b starts at
# x = 621 + (b-1)*92, lane i is centred at y = 141 + 73*i, the ruler is y 76..103, the track
# headers at x 241..620 (their S keys at x 597), the browser's rows 16 px lower than before.
param([string]$Exe = ".\target\release\app.exe",
  [string]$Work = (Join-Path $env:TEMP ("chipdaw-at-b3-" + (Get-Date -Format "MMdd-HHmmss"))),
  [string[]]$Only = @(), [int]$Scale = 100)
$global:Work = $Work
New-Item -ItemType Directory -Force $Work | Out-Null
. (Join-Path $PSScriptRoot "lib.ps1")
Set-Location $Repo
$global:Exe = (Resolve-Path $Exe).Path
python "$PSScriptRoot\fixtures.py" $Work | Out-Null
$obs = [ordered]@{ work = $Work; scale = $Scale }
# (-Only through `pwsh -File` arrives as one "AT-07,AT-11" string.)
$Only = @($Only | ForEach-Object { $_ -split "," } | Where-Object { $_ })
function Want($id) { $Only.Count -eq 0 -or $Only -contains $id }
function Bx($b) { 621 + ($b - 1) * 92 }
function Ly($i) { 141 + 73 * $i }
function Fx($name, $as) { $p = Join-Path $Work "$as.chipdaw"; Copy-Item (Join-Path $Work "$name.chipdaw") $p -Force; $p }
function Keep($proj, $as) { Copy-Item $proj (Join-Path $Work "$as.chipdaw") -Force }
function DevLines($d, $pat) { @(StdErr $d | Where-Object { $_ -match $pat }) }
# The piano roll's bounds (client px) from its dev line.
function PianoAt($d) {
  $l = DevLines $d "^dev: piano roll bounds" | Select-Object -Last 1
  if (-not $l) { return $null }
  $v = ($l -replace "^dev: piano roll bounds ", "" -replace " scale.*$", "").Split(" ") | ForEach-Object { [double]$_ }
  @{ X = $v[0]; Y = $v[1]; W = $v[2]; H = $v[3] }
}
# Hover the app label with this text (UI Automation) and wait for its tooltip.
function Hover-Text([string]$Text) {
  $c = New-Object System.Windows.Automation.AndCondition(
    (New-Object $global:Cond([System.Windows.Automation.AutomationElement]::ControlTypeProperty, [System.Windows.Automation.ControlType]::Text)),
    (New-Object $global:Cond([System.Windows.Automation.AutomationElement]::NameProperty, $Text)))
  $e = (Main-Window).FindAll([System.Windows.Automation.TreeScope]::Descendants, $c) | Select-Object -Last 1
  if (-not $e) { return $false }
  $r = $e.Current.BoundingRectangle
  [In]::SetCursorPos([int]($r.X + $r.Width / 2), [int]($r.Y + $r.Height / 2)) | Out-Null
  Start-Sleep -Milliseconds 120
  [In]::SetCursorPos([int]($r.X + $r.Width / 2) + 2, [int]($r.Y + $r.Height / 2)) | Out-Null
  Start-Sleep -Milliseconds 1100
  $true
}
function Ui-Texts {
  $c = New-Object $global:Cond([System.Windows.Automation.AutomationElement]::ControlTypeProperty, [System.Windows.Automation.ControlType]::Text)
  (Main-Window).FindAll([System.Windows.Automation.TreeScope]::Descendants, $c) | ForEach-Object { $_.Current.Name }
  $e = New-Object $global:Cond([System.Windows.Automation.AutomationElement]::ControlTypeProperty, [System.Windows.Automation.ControlType]::Edit)
  (Main-Window).FindAll([System.Windows.Automation.TreeScope]::Descendants, $e) | ForEach-Object {
    $v = $null
    if ($_.TryGetCurrentPattern([System.Windows.Automation.ValuePattern]::Pattern, [ref]$v)) { "field:" + $v.Current.Value }
  }
}

# ─────────── AT-07 (F-07, A9): copy two clips, paste at bar 9 ───────────
if (Want "AT-07") {
  $p = Fx "fx_basic" "at07"; $d = Join-Path $Work "d07"
  Launch -Project $p -Data $d -Scale $Scale
  ClickL (Bx 1.5) (Ly 1); ShiftClickL (Bx 3.5) (Ly 1)          # Harm A + Harm B
  Ctrl 0x43                                                     # Ctrl+C
  ClickL (Bx 9) (Ly 1)                                          # insert marker: bar 9, Pulse 2
  Ctrl 0x56                                                     # Ctrl+V
  Shot "at07_pasted"; Save; Keep $p "at07_out"; Stop-App
}

# ─────────── AT-08 (F-08, A13, T2): rename a clip and tracks in place ───────────
if (Want "AT-08") {
  $p = Fx "fx_basic" "at08"; $d = Join-Path $Work "d08"
  Launch -Project $p -Data $d -Scale $Scale
  ClickL (Bx 2) (Ly 0)                                          # Lead A
  Ctrl 0x52; Shot "at08_rename_clip"                            # Ctrl+R: the field over the clip
  TypeText "Verse"; K 0x0D
  ClickL 300 ((Ly 2) - 10)                                      # the triangle's header: select
  Ctrl 0x52; TypeText "Bass Line"; K 0x0D                       # Ctrl+R on a track
  DblL 300 ((Ly 0) - 10)                                        # double-click a track name
  TypeText "Lead"; Shot "at08_rename_track"
  K 0x09                                                        # Tab: next track (R9)
  TypeText "Harmony"; K 0x0D
  Shot "at08_renamed"; Save; Keep $p "at08_out"
  Ctrl 0x5A                                                     # one undo: the last rename
  Save; Keep $p "at08_undo"; Stop-App
}

# ─────────── AT-09 (F-09, F-23, P1–P4): double-click and Draw Mode ───────────
if (Want "AT-09") {
  $p = Fx "fx_empty_clip" "at09"; $d = Join-Path $Work "d09"
  $env:CHIPDAW_OPEN_CLIP = "1"
  Launch -Project $p -Data $d -Scale $Scale
  Remove-Item Env:CHIPDAW_OPEN_CLIP
  $pr = PianoAt $d; $obs.t09_piano = [bool]$pr
  if ($pr) {
    # C4 sits 11 rows below the grid top (open_piano centres middle C); bar 1 after the keys.
    $x = $pr.X / $global:S + 64 + 6; $y = $pr.Y / $global:S + 18 + 11 * 12 + 6
    DblL $x $y; Save; Keep $p "at09_a"; Shot "at09_created"
    DblL $x $y; Save; Keep $p "at09_b"
    ClickL ($x + 200) $y                                        # Draw Mode off: a click draws nothing
    Save; Keep $p "at09_click_off"
    K 0x42; Shot "at09_drawmode"                                # B: Draw Mode
    ClickL $x $y; Save; Keep $p "at09_c"
    K 0x42
  }
  Stop-App
}

# ─────────── AT-10 (F-10): right-click menus (dev lines list their items) ───────────
if (Want "AT-10") {
  $p = Fx "fx_basic" "at10"; $d = Join-Path $Work "d10"
  $env:CHIPDAW_OPEN_CLIP = "1"
  Launch -Project $p -Data $d -Scale $Scale
  Remove-Item Env:CHIPDAW_OPEN_CLIP
  RightL 300 ((Ly 1) - 10); Shot "at10_track"; K 0x1B
  RightL 900 90; Shot "at10_ruler"; K 0x1B
  $pr = PianoAt $d
  if ($pr) {
    # Note 6 of Lead A: pitch 65 (11 rows down), beat 6.
    $nx = $pr.X / $global:S + 64 + 320 + 12; $ny = $pr.Y / $global:S + 18 + 11 * 12 + 6
    RightL $nx $ny; Shot "at10_note"; K 0x1B
    ClickL $nx ($ny + 36)                                       # a click on empty space deselects
    RightL $nx ($ny + 36); Shot "at10_piano_empty"; K 0x1B
  }
  K 0x1B                                                        # close the piano roll
  RightL (Bx 2) (Ly 0); Shot "at10_clip"; K 0x1B
  RightL (Bx 9.5) (Ly 5); Shot "at10_empty"
  MoveL ((Bx 9.5) + 30) ((Ly 5) + 4 + 5 * 28 + 8 + 14 - 0); Start-Sleep -Milliseconds 400; Shot "at10_grid_sub"; K 0x1B
  $obs.t10_menus = @(DevLines $d "^dev: menu ")
  Stop-App
}

# ─────────── AT-11 (F-11, A12): split at the insert marker, consolidate ───────────
if (Want "AT-11") {
  $p = Fx "fx_basic" "at11"; $d = Join-Path $Work "d11"
  Launch -Project $p -Data $d -Scale $Scale
  ClickL (Bx 3) (Ly 0)                                          # Lead A; insert marker at bar 3
  Ctrl 0x45; Shot "at11_split"; Save; Keep $p "at11_split"      # Ctrl+E
  ClickL (Bx 1.5) (Ly 0); ShiftClickL (Bx 3.5) (Ly 0)
  Ctrl 0x4A; Shot "at11_consolidated"; Save; Keep $p "at11_join" # Ctrl+J
  Stop-App
}

# ─────────── AT-12 (F-12, A14): Z, X, double-click the ruler ───────────
if (Want "AT-12") {
  $p = Fx "fx_basic" "at12"; $d = Join-Path $Work "d12"
  Launch -Project $p -Data $d -Scale $Scale
  DragL (Bx 5) (Ly 4) (Bx 7) (Ly 4); Shot "at12_timesel"        # bars 5–6 on the empty DPCM track
  K 0x5A; Shot "at12_zoomed"                                    # Z
  K 0x58; Shot "at12_back"                                      # X
  K 0x1B                                                        # Esc: no selection
  DblL 900 90; Shot "at12_fit"                                  # double-click the ruler
  $obs.t12_zoom = @(DevLines $d "^dev: zoom ")
  Stop-App
}

# ─────────── AT-14 (F-14, A22): the getting-started guide ───────────
if (Want "AT-14") {
  $d = Join-Path $Work "d14"
  Launch -Data $d -Scale $Scale
  Shot "at14_demo"
  Ctrl 0x4E                                                     # Ctrl+N: an empty project (clean: no prompt)
  Start-Sleep -Milliseconds 500; Shot "at14_guide"
  DblL (Bx 2) (Ly 0); Shot "at14_after_clip"                    # the first clip: the guide goes
  Invoke-Menu "Help" 1; Shot "at14_help"                        # Help > Getting Started
  K 0x1B; Shot "at14_closed"
  Stop-App
}

# ─────────── AT-22 (F-22, A11, A3): Delete on a time selection across tracks ───────────
if (Want "AT-22") {
  $p = Fx "fx_two" "at22"; $d = Join-Path $Work "d22"
  Launch -Project $p -Data $d -Scale $Scale
  DragL (Bx 2) (Ly 2) (Bx 4) (Ly 2)                             # bars 2–3 on the empty triangle track
  ShiftClickL (Bx 3.5) (Ly 0)                                   # Shift-click extends it up over Pulse 1 (A3)
  Shot "at22_timesel"
  KE 0x2E; Shot "at22_deleted"; Save; Keep $p "at22_out"        # Del
  Stop-App
}

# ─────────── AT-23 (A15, A17): arrows nudge by the grid; Cmd+2 widens it ───────────
if (Want "AT-23") {
  $p = Fx "fx_basic" "at23"; $d = Join-Path $Work "d23"
  Launch -Project $p -Data $d -Scale $Scale
  ClickL (Bx 3.5) (Ly 1)                                        # Harm B (bar 3)
  KE 0x27; KE 0x27                                              # → → : 2 × 1/16
  Save; Keep $p "at23_a"
  Ctrl 0x32                                                     # Ctrl+2: 1/16 → 1/8
  KE 0x27; Save; Keep $p "at23_b"
  KE 0x24                                                       # Home
  Stop-App
}

# ─────────── AT-24 (A10, R46, R47): Duplicate a time selection, Insert Silence, Delete Time ───────────
if (Want "AT-24") {
  $p = Fx "fx_basic" "at24"; $d = Join-Path $Work "d24"
  Launch -Project $p -Data $d -Scale $Scale
  DragL (Bx 2) (Ly 4) (Bx 3) (Ly 4); ShiftClickL (Bx 2.5) (Ly 0)  # bar 2, Pulse 1 .. DPCM
  Ctrl 0x49; Save; Keep $p "at24_silence"                       # Ctrl+I: one bar of silence at bar 2
  CtrlShiftExt 0x2E; Save; Keep $p "at24_deltime"               # Ctrl+Shift+Del: and gone again
  Stop-App
}

# ─────────── AT-25 (T1, T5, I1): select a track; exclusive solo, Ctrl-click adds ───────────
if (Want "AT-25") {
  $p = Fx "fx_basic" "at25"; $d = Join-Path $Work "d25"
  Launch -Project $p -Data $d -Scale $Scale
  ClickL 300 ((Ly 1) - 10); Start-Sleep -Milliseconds 300       # select Pulse 2: the inspector opens
  $obs.t25_texts = @(Ui-Texts)
  Shot "at25_track_selected"
  $obs.t25_tip = Hover-Text "Make Unique"; Shot "at25_tooltip_disabled"
  ClickL 597 ((Ly 0) - 1); ClickL 597 ((Ly 1) - 1)               # S on Pulse 1, then S on Pulse 2
  CtrlClickL 597 ((Ly 2) - 1)                                   # Ctrl-click S on the triangle: adds
  Save; Keep $p "at25_out"; Stop-App
}

# ─────────── AT-26 (T3, A18): track and clip colour from the menus ───────────
if (Want "AT-26") {
  $p = Fx "fx_basic" "at26"; $d = Join-Path $Work "d26"
  Launch -Project $p -Data $d -Scale $Scale
  $mx = 300; $my = (Ly 3) - 10
  RightL $mx $my
  MoveL ($mx + 30) ($my + 4 + 28 + 14); Start-Sleep -Milliseconds 400   # hover Colour: the submenu
  Shot "at26_colour_sub"
  # The colour grid (polish pass): the submenu opens at the menu's right edge, 28 px down;
  # cells are 16 px on a 20 px pitch from 12 px in. Sky #4FA8E0: row 3, column 9.
  ClickL ($mx + 236 + 12 + 8 * 20 + 8) ($my + 28 + 12 + 2 * 20 + 8)      # Sky
  $cx = Bx 2; $cy = Ly 0
  RightL $cx $cy
  MoveL ($cx + 30) ($cy + 4 + 5 * 28 + 8 + 3 * 28 + 14); Start-Sleep -Milliseconds 400
  # Under "Track Colour" (one row): Coral #E0705A, row 3, column 1.
  ClickL ($cx + 236 + 12 + 0 * 20 + 8) ($cy + 5 * 28 + 8 + 3 * 28 + 4 + 28 + 8 + 2 * 20 + 8)   # Coral
  Shot "at26_coloured"; Save; Keep $p "at26_out"; Stop-App
}

# ─────────── AT-27 (P7, P9, P11): Shift+→, chop, join, zoom in the piano roll ───────────
if (Want "AT-27") {
  $p = Fx "fx_basic" "at27"; $d = Join-Path $Work "d27"
  $env:CHIPDAW_OPEN_CLIP = "1"
  Launch -Project $p -Data $d -Scale $Scale
  Remove-Item Env:CHIPDAW_OPEN_CLIP
  $pr = PianoAt $d
  if ($pr) {
    ClickL ($pr.X / $global:S + 400) ($pr.Y / $global:S + 40)  # focus the piano roll (empty space)
    Ctrl 0x41                                                   # select all notes
    ShiftKey 0x27; Save; Keep $p "at27_longer"                  # Shift+→: one grid step longer
    Ctrl 0x45; Save; Keep $p "at27_chopped"                     # Ctrl+E: chop on the grid
    Ctrl 0x4A; Save; Keep $p "at27_joined"                      # Ctrl+J: join
    K 0x5A; Shot "at27_zoom_z"                                  # Z
  }
  Stop-App
}

# ─────────── AT-28 (E1–E4): the export dialog ───────────
if (Want "AT-28") {
  $p = Fx "fx_basic" "at28"; $d = Join-Path $Work "d28"
  $env:CHIPDAW_EXPORT_TO = (Join-Path $Work "at28_export.wav")
  Launch -Project $p -Data $d -Scale $Scale
  CtrlShift 0x52; Start-Sleep -Milliseconds 400; Shot "at28_dialog"
  $obs.t28_open = [bool]((Ui-Texts) -contains "EXPORT")
  K 0x1B; $obs.t28_after_esc = [bool]((Ui-Texts) -contains "EXPORT")
  CtrlShift 0x52; Start-Sleep -Milliseconds 300
  $obs.t28_pick = Click-Text "24-bit"; Start-Sleep -Milliseconds 300
  $obs.t28_tip = Hover-Text "Selection"; Shot "at28_disabled_tip"
  K 0x1B
  CtrlShift 0x52; Start-Sleep -Milliseconds 300; Shot "at28_reopened"
  K 0x0D; Start-Sleep -Seconds 4                                # Enter: export
  $obs.t28_wav = Test-Path (Join-Path $Work "at28_export.wav")
  Copy-Item (Join-Path $d "settings.json") (Join-Path $Work "at28_settings.json") -ErrorAction SilentlyContinue
  Remove-Item Env:CHIPDAW_EXPORT_TO
  Stop-App
}

# ─────────── AT-29 (B1, B4): Esc clears the search; a click on a layout only selects it ───────────
if (Want "AT-29") {
  $p = Fx "fx_basic" "at29"; $d = Join-Path $Work "d29"
  Launch -Project $p -Data $d -Scale $Scale
  ClickL 120 126; TypeText "ab"
  $obs.t29_typed = @(Ui-Texts | Where-Object { $_ -like "field:*" })
  K 0x1B; Start-Sleep -Milliseconds 300
  $obs.t29_after_esc = @(Ui-Texts | Where-Object { $_ -like "field:*" })
  ClickL 100 193                                                # "Classic 2A03 Band": select only
  Save; Keep $p "at29_click"; Shot "at29_layout_selected"
  DblL 100 193                                                  # double-click: load (it asks: 2A03)
  $obs.t29_dialog = Dialog-Text
  $obs.t29_keep = Press-Button "Keep 2A03 + VRC6"
  Save; Keep $p "at29_dbl"; Stop-App
}

# ─────────── AT-30 (A18, A19, A20 + menus): the native menus' new items (screenshots) ───────────
if (Want "AT-30") {
  $p = Fx "fx_basic" "at30"; $d = Join-Path $Work "d30"
  Launch -Project $p -Data $d -Scale $Scale
  Open-Menu "Edit"; Start-Sleep -Milliseconds 300; Shot "at30_menu_edit"; Close-Menu
  Open-Menu "View"; Start-Sleep -Milliseconds 300; Shot "at30_menu_view"; Close-Menu
  Open-Menu "Help"; Start-Sleep -Milliseconds 300; Shot "at30_menu_help"; Close-Menu
  Invoke-Menu "Help" 0; Start-Sleep -Milliseconds 400; Shot "at30_sheet"
  $obs.t30_sheet = @(Ui-Texts)
  K 0x1B; Stop-App
}

# ─────────── AT-31 (B5, I3): a user preset's menu ───────────
if (Want "AT-31") {
  $p = Fx "fx_basic" "at31"; $d = Join-Path $Work "d31"
  Launch -Project $p -Data $d -Scale $Scale
  ClickL (Bx 2) (Ly 0)
  $obs.t31_save = Click-Text "Save as Preset"; Start-Sleep -Milliseconds 500
  $obs.t31_presets = Test-Path (Join-Path $d "user_presets.json")
  # Find the user preset ("Square Lead 2": a unique name) with the search.
  ClickL 120 126; TypeText "Square Lead 2"; K 0x0D
  Shot "at31_search"
  RightL 100 193; Shot "at31_preset_menu"; K 0x1B
  $obs.t31_menu = @(DevLines $d "^dev: menu Preset")
  Stop-App
}

$obs | ConvertTo-Json -Depth 6 | Set-Content -Encoding utf8 (Join-Path $Work "obs.json")
python "$PSScriptRoot\check_b3.py" $Work
