# usage: at_b4b.ps1 [-Exe .\target\release\cartwave.exe] [-Work <dir>] [-Only B4B-M1,B4B-M2]
# Batch 4b scenarios (BATCH4B_PROMPT.md §3–7, UX.md AT-17): synthetic input into the real app,
# real OS file drags (drag_midi.ps1: OLE DoDragDrop, as from Explorer), saved files,
# settings.json and dev-mode stderr lines, checked by check_b4b.py. Client px at 100 %: top bar
# 76, Overview 76..108, ruler 108..136, rows from 136 every 73 px; browser 0..241; headers
# 241..621; lanes 621.. at 92 px per bar. Dark menus off (Settings > Display): the native menu's
# keyboard path (Alt+F, I, M) needs the system-drawn items.
param([string]$Exe = ".\target\release\cartwave.exe",
  [string]$Work = (Join-Path $env:TEMP ("chipdaw-at-b4b-" + (Get-Date -Format "MMdd-HHmmss"))),
  [string[]]$Only = @())
$global:Work = $Work
New-Item -ItemType Directory -Force $Work | Out-Null
. (Join-Path $PSScriptRoot "..\..\batch3\tools\lib.ps1")
Set-Location $Repo
$global:Exe = (Resolve-Path $Exe).Path
python "$Repo\docs\batch4b\tools\fixtures_b4b.py" $Work | Out-Null
foreach ($k in "CARTWAVE_OVERVIEW", "CARTWAVE_HIDE_STATUS", "CARTWAVE_FAKE_PROBLEM", "CARTWAVE_WARN_ANSWER", "CARTWAVE_IMPORT_FROM") { Remove-Item "Env:$k" -ErrorAction SilentlyContinue }
$obs = [ordered]@{ work = $Work }
$Mid = (Resolve-Path "$Repo\docs\songs\showcase.mid").Path
function Place { pwsh -NoProfile -File "$Repo\docs\perf\tools\shot.ps1" -Out "$global:Work\_place.png" | Out-Null; Start-Sleep -Milliseconds 300 }
$Only = @($Only | ForEach-Object { $_ -split "," } | Where-Object { $_ })
function Want($id) { $Only.Count -eq 0 -or $Only -contains $id }
function Fx($name, $as) { $p = Join-Path $Work "$as.chipdaw"; Copy-Item (Join-Path $Work "$name.chipdaw") $p -Force; $p }
function Keep($proj, $as) { Copy-Item $proj (Join-Path $Work "$as.chipdaw") -Force }
function DevLines($d, $pat) { @(StdErr $d | Where-Object { $_ -match $pat }) }
function NoDark($d) { New-Item -ItemType Directory -Force $d | Out-Null; '{ "dark_menus": false }' | Set-Content -Encoding ascii (Join-Path $d "settings.json") }
$RowsTop = 136; $Pitch = 73
function RowY($i) { $RowsTop + $Pitch * $i }
function BarX($bar) { 621 + 92 * ($bar - 1) }
# An OS drag of the MIDI file from right of the window through the points (client = screen px
# after Place), ending with a drop (or Esc with -Cancel).
function DragMid($pts, [switch]$Cancel, [string]$ShotName = "", [int]$ShotAt = -1) {
  $start = "1700,400"
  # Client px -> screen px (as Move-To does).
  $scr = ($pts -split ";" | Where-Object { $_ } | ForEach-Object { $a = $_ -split ","; $q = Pt ([int]$a[0]) ([int]$a[1]); "$($q.X),$($q.Y)" }) -join ";"
  $arg = @("-NoProfile", "-File", "$Repo\docs\batch4b\tools\drag_midi.ps1", "-File", $Mid, "-Points", ($start + ";" + $scr), "-StepMs", "200")
  if ($Cancel) { $arg += "-Cancel" }
  if ($ShotName) { $arg += @("-ShotAt", "$ShotAt", "-Shot", (Join-Path $Work "$ShotName.png")) }
  pwsh @arg | Out-Null
  Start-Sleep -Milliseconds 900
  [In]::SetForegroundWindow($global:AppH) | Out-Null
  Start-Sleep -Milliseconds 300
}

# ─────────── B4B-M1 / AT-17: File > Import > MIDI File… adds tracks, the old clips unchanged ───────────
if (Want "B4B-M1") {
  $p = Fx "fx_board" "m1"; $d = Join-Path $Work "dm1"; NoDark $d
  $env:CARTWAVE_IMPORT_FROM = $Mid; $env:CARTWAVE_WARN_ANSWER = "unlock"
  Launch -Project $p -Data $d; Place
  [In]::SetForegroundWindow($global:AppH) | Out-Null; Start-Sleep -Milliseconds 400
  KeyDown 0x12; Key 0x46; KeyUp 0x12; Start-Sleep -Milliseconds 800   # Alt+F: File
  Shot "m1_menu"
  K 0x49; Start-Sleep -Milliseconds 800                              # I: Import ▸
  Shot "m1_sub"
  K 0x4D; Start-Sleep -Milliseconds 1500                             # M: MIDI File…
  Save; Keep $p "m1_after"
  $obs.m1_import = DevLines $d "midi import"
  $obs.m1_warn = DevLines $d "dev: warning"
  Stop-App
  Remove-Item Env:CARTWAVE_IMPORT_FROM, Env:CARTWAVE_WARN_ANSWER -ErrorAction SilentlyContinue
}

# ─────────── B4B-M2: a drop on empty space with no profile: new studio tracks, no dialog; the preview follows the cursor ───────────
if (Want "B4B-M2") {
  $p = Fx "fx_studio" "m2"; $d = Join-Path $Work "dm2"
  Launch -Project $p -Data $d; Place
  # Over track 1 at bar 4, then down to empty space at bar 3 (the drop).
  DragMid ("{0},{1};{2},{3};{4},{5};{6},{7}" -f (BarX 6), (RowY 0 + 30), (BarX 4), (RowY 0 + 30), (BarX 3), (RowY 3 + 30), (BarX 3 + 10), (RowY 3 + 30)) -ShotName "m2_preview" -ShotAt 2
  Save; Keep $p "m2_after"
  $obs.m2_drag = DevLines $d "midi drag at"
  $obs.m2_import = DevLines $d "midi import"
  $obs.m2_warn = DevLines $d "dev: warning"
  Stop-App
}

# ─────────── B4B-M3: a drop onto track 2 at bar 3 that conflicts: Fit, chords as arpeggios ───────────
if (Want "B4B-M3") {
  $p = Fx "fx_board" "m3"; $d = Join-Path $Work "dm3"
  $env:CARTWAVE_WARN_ANSWER = "fitarp"
  Launch -Project $p -Data $d; Place
  DragMid ("{0},{1};{2},{3};{4},{5}" -f (BarX 6), (RowY 1 + 30), (BarX 3 + 10), (RowY 1 + 30), (BarX 3 + 20), (RowY 1 + 30))
  Save; Keep $p "m3_after"
  $obs.m3_drag = DevLines $d "midi drag at"
  $obs.m3_import = DevLines $d "midi import"
  $obs.m3_warn = DevLines $d "dev: warning"
  Stop-App
  Remove-Item Env:CARTWAVE_WARN_ANSWER -ErrorAction SilentlyContinue
}

# ─────────── B4B-M4: a drag that leaves (Esc) changes nothing and asks nothing ───────────
if (Want "B4B-M4") {
  $p = Fx "fx_board" "m4"; $d = Join-Path $Work "dm4"
  $env:CARTWAVE_WARN_ANSWER = "unlock"
  Launch -Project $p -Data $d; Place
  DragMid ("{0},{1};{2},{3}" -f (BarX 5), (RowY 1 + 30), (BarX 4), (RowY 1 + 30)) -Cancel
  Save; Keep $p "m4_after"
  $obs.m4_drag = DevLines $d "midi drag at"
  $obs.m4_import = DevLines $d "midi import"
  Stop-App
  Remove-Item Env:CARTWAVE_WARN_ANSWER -ErrorAction SilentlyContinue
}

# The mixer (Tab) at 1440 × 900 with the browser open: strips from x 241 + 24, every 160 px,
# from y 76 + 60; inside a strip (strip-relative): the cap's centre (67, 166 + the dB taper over
# 260 px), the knob (76, 73), M (60, 130), S (92, 130), the dB readout (76, 527), the clip mark
# (110, 150).
$MixX0 = 241 + 24; $MixY0 = 76 + 60
function StripX($i) { $MixX0 + 160 * $i }
function CapY($db) { $MixY0 + 166 + [math]::Round((1 - ($db + 48) / 54) * 260) }

# ─────────── B4B-X1: the mixer's controls are the headers' (values, one undo step each, typed values) ───────────
if (Want "B4B-X1") {
  $p = Fx "fx_studio" "x1"; $d = Join-Path $Work "dx1"
  Launch -Project $p -Data $d; Place
  K 0x09; Start-Sleep -Milliseconds 600                                   # Tab: the mixer
  Shot "x1_mixer"
  DragL ((StripX 0) + 67) (CapY 0) ((StripX 0) + 67) ((CapY 0) + 58)        # fader: down 58 px (about -12 dB)
  Save; Keep $p "x1_fader"
  Ctrl 0x5A; Save; Keep $p "x1_undo"                                       # one Ctrl+Z: back to 0 dB
  DragL ((StripX 0) + 76) ($MixY0 + 73) ((StripX 0) + 76) ($MixY0 + 73 - 60) # knob: up 60 px (pan right)
  ClickL ((StripX 0) + 60) ($MixY0 + 130)                                   # M on track 1
  ClickL ((StripX 1) + 92) ($MixY0 + 130)                                   # S on track 2
  Save; Keep $p "x1_pan_ms"
  DblL ((StripX 0) + 76) ($MixY0 + 527); Start-Sleep -Milliseconds 300      # the readout: type a value
  K 0xBD; TypeText "3"; K 0x0D; Start-Sleep -Milliseconds 300            # "-3" (VK_OEM_MINUS, then 3)
  Save; Keep $p "x1_typed"
  MoveL ((StripX 0) + 67) ((CapY -3) ); Start-Sleep -Milliseconds 300
  K 0x09; Start-Sleep -Milliseconds 600                                   # Tab: back to the arrangement
  $obs.x1_view = DevLines $d "dev: view"
  $obs.x1_help = DevLines $d "dev: help Volume"
  Stop-App
}

# ─────────── B4B-X2: locked strips follow the lock rules (fader parked, no typed value, the reason on hover) ───────────
if (Want "B4B-X2") {
  $p = Fx "fx_board" "x2"; $d = Join-Path $Work "dx2"
  Launch -Project $p -Data $d; Place
  K 0x09; Start-Sleep -Milliseconds 600
  MoveL ((StripX 0) + 67) (CapY 0); Start-Sleep -Milliseconds 300
  DragL ((StripX 0) + 67) (CapY 0) ((StripX 0) + 67) ((CapY 0) + 80)
  DragL ((StripX 0) + 76) ($MixY0 + 73) ((StripX 0) + 76) ($MixY0 + 13)
  DblL ((StripX 0) + 76) ($MixY0 + 527); Start-Sleep -Milliseconds 300
  K 0x1B
  Save; Keep $p "x2_after"
  $obs.x2_help = DevLines $d "Locked to the 2A03 Pulse 1"
  Shot "x2_locked"
  Stop-App
}

# ─────────── B4B-U1: Unison and Detune in the inspector (a studio track), saved; hidden on a locked track ───────────
if (Want "B4B-U1") {
  $p = Fx "fx_studio" "u1"; $d = Join-Path $Work "du1"
  Launch -Project $p -Data $d; Place
  ClickL 700 170; Start-Sleep -Milliseconds 600                            # a clip on track 1: the inspector
  MoveL 1000 857; Start-Sleep -Milliseconds 200; Wheel 1; Wheel 1; Start-Sleep -Milliseconds 300   # UNISON 1 -> 3
  MoveL 1130 857; Start-Sleep -Milliseconds 200
  for ($i = 0; $i -lt 15; $i++) { Wheel 1; Start-Sleep -Milliseconds 30 }                             # DETUNE 0 -> 15
  Start-Sleep -Milliseconds 300
  Shot "u1_inspector"
  Save; Keep $p "u1_after"
  $obs.u1_help = DevLines $d "dev: help (Unison|Detune)"
  Stop-App
  $p = Fx "fx_board" "u1b"; $d = Join-Path $Work "du1b"
  Launch -Project $p -Data $d; Place
  ClickL 700 170; Start-Sleep -Milliseconds 600
  Shot "u1_locked"
  Stop-App
}

# ─────────── B4B-C1: the clip mark lights white past 0 dBFS, holds, and a click resets it ───────────
if (Want "B4B-C1") {
  $p = Fx "fx_clip" "c1"; $d = Join-Path $Work "dc1"
  # Launch with audio (lib.ps1's Launch turns it off): the meters need the engine.
  Get-Process -Name cartwave -ErrorAction SilentlyContinue | Stop-Process -Force; Start-Sleep -Milliseconds 300
  New-Item -ItemType Directory -Force $d | Out-Null
  $env:CARTWAVE_DATA_DIR = $d; $env:CARTWAVE_AUDIO = "1"; $env:CARTWAVE_DEV = "1"; $env:CARTWAVE_PROJECT = $p
  $script:proc = Start-Process $global:Exe -PassThru -RedirectStandardError (Join-Path $d "stderr.txt")
  for ($i = 0; $i -lt 50; $i++) { Start-Sleep -Milliseconds 100; if ((Get-Process -Id $script:proc.Id).MainWindowHandle -ne 0) { break } }
  Start-Sleep -Milliseconds 1500
  $global:AppH = (Get-Process -Id $script:proc.Id).MainWindowHandle
  [In]::SetForegroundWindow($global:AppH) | Out-Null
  $global:S = 1.0
  Place
  $env:CARTWAVE_AUDIO = "0"
  K 0x20; Start-Sleep -Milliseconds 2500; K 0x20; Start-Sleep -Milliseconds 1500   # play 2.5 s, stop, wait (the hold)
  Shot "c1_lit"
  ClickL 589 138; Start-Sleep -Milliseconds 400                                     # the mark of track 1 (241 + 328 + 18, row top + 2)
  Shot "c1_reset"
  K 0x09; Start-Sleep -Milliseconds 600
  K 0x20; Start-Sleep -Milliseconds 2500; K 0x20; Start-Sleep -Milliseconds 1500
  Shot "c1_mixer"
  Stop-App
}

# ─────────── B4B-T1: Tab switches views and leaves no button focused (Space plays, never presses Make Unique) ───────────
if (Want "B4B-T1") {
  $p = Fx "fx_studio" "t1"; $d = Join-Path $Work "dt1"
  Launch -Project $p -Data $d; Place
  ClickL 700 170; Start-Sleep -Milliseconds 600                            # a clip: the inspector shows Make Unique
  Save; Keep $p "t1_before"                                                # as the app writes it
  K 0x09; Start-Sleep -Milliseconds 600                                    # the mixer
  K 0x20; Start-Sleep -Milliseconds 1200; K 0x20; Start-Sleep -Milliseconds 400
  Shot "t1_mixer"
  K 0x09; Start-Sleep -Milliseconds 600                                    # back
  K 0x20; Start-Sleep -Milliseconds 800; K 0x20; Start-Sleep -Milliseconds 400
  Save; Keep $p "t1_after"
  $obs.t1_views = DevLines $d "dev: view (mixer|arrangement)"
  Stop-App
}

# ─────────── Screenshot only (run with -Only B4B-SHOT-DIALOG): the import question, unanswered ───────────
if ($Only -contains "B4B-SHOT-DIALOG") {
  $p = Fx "fx_board" "sd"; $d = Join-Path $Work "dsd"
  Launch -Project $p -Data $d; Place
  DragMid ("{0},{1};{2},{3};{4},{5}" -f (BarX 6), (RowY 1 + 30), (BarX 3 + 10), (RowY 1 + 30), (BarX 3 + 20), (RowY 1 + 30))
  Start-Sleep -Milliseconds 1200
  # The app window's rectangle (the dialog sits over it): a screen grab cropped to it.
  Add-Type -Namespace SD -Name W -MemberDefinition '[DllImport("dwmapi.dll")] public static extern int DwmGetWindowAttribute(IntPtr h, int a, out RECT r, int s); public struct RECT { public int L, T, R, B; }' -ErrorAction SilentlyContinue
  $r = New-Object SD.W+RECT; [SD.W]::DwmGetWindowAttribute($global:AppH, 9, [ref]$r, 16) | Out-Null
  python -c "from PIL import ImageGrab; ImageGrab.grab(bbox=($($r.L), $($r.T), $($r.R), $($r.B)), all_screens=True).save(r'$(Join-Path $Work 'sd_dialog.png')')"
  K 0x1B; Start-Sleep -Milliseconds 500                                     # Cancel
  Stop-App
}

$obs | ConvertTo-Json -Depth 5 | Set-Content -Encoding utf8 (Join-Path $Work "obs.json")
python "$Repo\docs\batch4b\tools\check_b4b.py" $Work
