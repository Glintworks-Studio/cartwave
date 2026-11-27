# usage: at_b4b.ps1 [-Exe .\target\release\app.exe] [-Work <dir>] [-Only B4B-M1,B4B-M2]
# Batch 4b scenarios (BATCH4B_PROMPT.md §3–7, UX.md AT-17): synthetic input into the real app,
# real OS file drags (drag_midi.ps1: OLE DoDragDrop, as from Explorer), saved files,
# settings.json and dev-mode stderr lines, checked by check_b4b.py. Client px at 100 %: top bar
# 76, Overview 76..108, ruler 108..136, rows from 136 every 73 px; browser 0..241; headers
# 241..621; lanes 621.. at 92 px per bar. Dark menus off (Settings > Display): the native menu's
# keyboard path (Alt+F, I, M) needs the system-drawn items.
param([string]$Exe = ".\target\release\app.exe",
  [string]$Work = (Join-Path $env:TEMP ("chipdaw-at-b4b-" + (Get-Date -Format "MMdd-HHmmss"))),
  [string[]]$Only = @())
$global:Work = $Work
New-Item -ItemType Directory -Force $Work | Out-Null
. (Join-Path $PSScriptRoot "..\..\batch3\tools\lib.ps1")
Set-Location $Repo
$global:Exe = (Resolve-Path $Exe).Path
python "$Repo\docs\batch4b\tools\fixtures_b4b.py" $Work | Out-Null
foreach ($k in "CHIPDAW_OVERVIEW", "CHIPDAW_HIDE_STATUS", "CHIPDAW_FAKE_PROBLEM", "CHIPDAW_WARN_ANSWER", "CHIPDAW_IMPORT_FROM") { Remove-Item "Env:$k" -ErrorAction SilentlyContinue }
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
  $env:CHIPDAW_IMPORT_FROM = $Mid; $env:CHIPDAW_WARN_ANSWER = "unlock"
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
  Remove-Item Env:CHIPDAW_IMPORT_FROM, Env:CHIPDAW_WARN_ANSWER -ErrorAction SilentlyContinue
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
  $env:CHIPDAW_WARN_ANSWER = "fitarp"
  Launch -Project $p -Data $d; Place
  DragMid ("{0},{1};{2},{3};{4},{5}" -f (BarX 6), (RowY 1 + 30), (BarX 3 + 10), (RowY 1 + 30), (BarX 3 + 20), (RowY 1 + 30))
  Save; Keep $p "m3_after"
  $obs.m3_drag = DevLines $d "midi drag at"
  $obs.m3_import = DevLines $d "midi import"
  $obs.m3_warn = DevLines $d "dev: warning"
  Stop-App
  Remove-Item Env:CHIPDAW_WARN_ANSWER -ErrorAction SilentlyContinue
}

# ─────────── B4B-M4: a drag that leaves (Esc) changes nothing and asks nothing ───────────
if (Want "B4B-M4") {
  $p = Fx "fx_board" "m4"; $d = Join-Path $Work "dm4"
  $env:CHIPDAW_WARN_ANSWER = "unlock"
  Launch -Project $p -Data $d; Place
  DragMid ("{0},{1};{2},{3}" -f (BarX 5), (RowY 1 + 30), (BarX 4), (RowY 1 + 30)) -Cancel
  Save; Keep $p "m4_after"
  $obs.m4_drag = DevLines $d "midi drag at"
  $obs.m4_import = DevLines $d "midi import"
  Stop-App
  Remove-Item Env:CHIPDAW_WARN_ANSWER -ErrorAction SilentlyContinue
}

$obs | ConvertTo-Json -Depth 5 | Set-Content -Encoding utf8 (Join-Path $Work "obs.json")
python "$Repo\docs\batch4b\tools\check_b4b.py" $Work
