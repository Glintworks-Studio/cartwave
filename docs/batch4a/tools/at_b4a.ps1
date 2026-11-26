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

$obs | ConvertTo-Json -Depth 5 | Set-Content -Encoding utf8 (Join-Path $Work "obs.json")
python "$Repo\docs\batch4a\tools\check_b4a.py" $Work
