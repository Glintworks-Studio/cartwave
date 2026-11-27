# usage: at_dp.ps1 [-Exe .\target\release\cartwave.exe] [-Work <dir>] [-Only DP-01,DP-03]
# Design pass scenarios (DESIGN_PASS_PROMPT.md §4.5), the same method as the Batch 3
# harness (docs/batch3/tools): synthetic input into the real app, dev-mode stderr
# lines and saved files checked afterwards by check_dp.py.
#   DP-01  TAP: four taps 500 ms apart -> 120 BPM, one undo step
#   DP-02  the spectrum style persists across a restart
#   DP-03  preview on select (click, arrow keys, the Preview key, during playback)
#   DP-04  the disabled controls say why (Mixer, Record, pan, the triangle's fader)
#   DP-05  the bit depth is saved in the project and shown in the display
# Client px at 100% (Option D): top bar 0..75, ruler 76..103, rows from 104 every 73 px,
# browser 0..239, track headers 241..620, lanes from 621 (92 px per bar).
param([string]$Exe = ".\target\release\cartwave.exe",
  [string]$Work = (Join-Path $env:TEMP ("chipdaw-at-dp-" + (Get-Date -Format "MMdd-HHmmss"))),
  [string[]]$Only = @())
$global:Work = $Work
New-Item -ItemType Directory -Force $Work | Out-Null
. (Join-Path $PSScriptRoot "..\..\batch3\tools\lib.ps1")
Set-Location $Repo
$global:Exe = (Resolve-Path $Exe).Path
python "$Repo\docs\batch3\tools\fixtures.py" $Work | Out-Null
$obs = [ordered]@{ work = $Work }
$Only = @($Only | ForEach-Object { $_ -split "," } | Where-Object { $_ })
function Want($id) { $Only.Count -eq 0 -or $Only -contains $id }
function Fx($name, $as) { $p = Join-Path $Work "$as.chipdaw"; Copy-Item (Join-Path $Work "$name.chipdaw") $p -Force; $p }
function Keep($proj, $as) { Copy-Item $proj (Join-Path $Work "$as.chipdaw") -Force }
function DevLines($d, $pat) { @(StdErr $d | Where-Object { $_ -match $pat }) }
function Ui-Texts {
  $c = New-Object $global:Cond([System.Windows.Automation.AutomationElement]::ControlTypeProperty, [System.Windows.Automation.ControlType]::Text)
  (Main-Window).FindAll([System.Windows.Automation.TreeScope]::Descendants, $c) | ForEach-Object { $_.Current.Name }
}
# Hover a point (client px) long enough for its tooltip, then list the texts.
function Hover-Texts($x, $y) {
  MoveL ($x - 3) $y; Start-Sleep -Milliseconds 120; MoveL $x $y; Start-Sleep -Milliseconds 1100
  @(Ui-Texts)
}

# ─────────── DP-01: TAP ───────────
if (Want "DP-01") {
  $p = Fx "fx_basic" "dp01"; $d = Join-Path $Work "d01"
  Launch -Project $p -Data $d
  ClickL 1300 820                                              # focus the arrangement (empty space)
  $t0 = Get-Date
  for ($i = 0; $i -lt 4; $i++) {
    # The key itself takes ~80 ms (input.ps1); pace the presses 500 ms apart.
    while (((Get-Date) - $t0).TotalMilliseconds -lt 500 * $i) { Start-Sleep -Milliseconds 5 }
    Key 0x54                                                   # T
  }
  Start-Sleep -Milliseconds 400; Shot "dp01_tapped"
  Save; Keep $p "dp01_tapped"
  Ctrl 0x5A; Save; Keep $p "dp01_undone"                       # one Ctrl+Z undoes the whole sequence
  Ctrl 0x59; Save; Keep $p "dp01_redone"
  # The TAP key works too (a new sequence after > 2 s).
  Start-Sleep -Milliseconds 2200
  for ($i = 0; $i -lt 3; $i++) { ClickL 999 38; Start-Sleep -Milliseconds 300 }
  $obs.dp01_taps = @(DevLines $d "^dev: tap bpm")
  Stop-App
}

# ─────────── DP-02: the spectrum style persists ───────────
if (Want "DP-02") {
  $d = Join-Path $Work "d02"
  Launch -Data $d
  RightL 1220 38; Shot "dp02_menu"
  $obs.dp02_menu = @(DevLines $d "^dev: menu Spectrum")
  ClickL 1100 (38 + 4 + 28 + 14)                               # the second row: LED Segments (the menu opens to the left here)
  Start-Sleep -Milliseconds 300
  Stop-App
  $obs.dp02_first = @(DevLines $d "^dev: spectrum style")
  Copy-Item (Join-Path $d "settings.json") (Join-Path $Work "dp02_settings.json") -ErrorAction SilentlyContinue
  Launch -Data $d                                              # the same data dir: a restart
  $obs.dp02_second = @(DevLines $d "^dev: spectrum style")
  RightL 1220 38; Shot "dp02_menu_again"; K 0x1B
  Stop-App
}

# ─────────── DP-03: preview on select ───────────
if (Want "DP-03") {
  $d = Join-Path $Work "d03"
  Launch -Data $d
  # LEADS: "Square Lead" is the first preset row (client y 425), "Bright Lead" the next.
  MoveL 60 425; Down; Start-Sleep -Milliseconds 300; Up; Start-Sleep -Milliseconds 300
  KE 0x28; Start-Sleep -Milliseconds 900                       # Down: the next preset, auditioned
  Shot "dp03_arrow"
  $obs.dp03_on = @(DevLines $d "^dev: audition")
  ClickL 138 95; Start-Sleep -Milliseconds 300; Shot "dp03_key_off"   # the Preview key: off
  MoveL 60 425; Down; Up; Start-Sleep -Milliseconds 300
  $obs.dp03_off = @(DevLines $d "^dev: audition")
  ClickL 138 95                                                # back on
  K 0x20; Start-Sleep -Milliseconds 300                        # Space: play
  MoveL 60 425; Down; Up; Start-Sleep -Milliseconds 500
  $obs.dp03_playing = @(Ui-Texts | Where-Object { $_ -match "song is stopped" })
  Shot "dp03_playing"
  K 0x20
  Stop-App
  Copy-Item (Join-Path $d "settings.json") (Join-Path $Work "dp03_settings.json") -ErrorAction SilentlyContinue
}

# ─────────── DP-04: disabled controls say why ───────────
if (Want "DP-04") {
  $d = Join-Path $Work "d04"
  New-Item -ItemType Directory -Force $d | Out-Null
  '{ "browser_open": false }' | Set-Content -Encoding ascii (Join-Path $d "settings.json")
  Launch -Data $d
  # Polish 2: the reasons are the status bar's hover help (dev lines "dev: help …"); pan is
  # editable (heard with stereo mixing) and every track's fader works (a dB gain, P2-D4).
  foreach ($pt in @(@(84, 38), @(245, 38), @(284, 164), @(348, (104 + 2 * 73 + 20)))) { MoveL ($pt[0] - 3) $pt[1]; Start-Sleep -Milliseconds 150; MoveL $pt[0] $pt[1]; Start-Sleep -Milliseconds 500 }
  Shot "dp04_after"
  $help = @(DevLines $d "^dev: help")
  # Batch 4b: the Mixer key works (its help says what it shows) and pan is heard (B4B-D31).
  $obs.dp04_mixer = @($help | Where-Object { $_ -match "Mixer view" })
  $obs.dp04_record = @($help | Where-Object { $_ -match "MIDI input" })
  $obs.dp04_pan = @($help | Where-Object { $_ -match "^dev: help Pan" })
  $obs.dp04_tri = @($help | Where-Object { $_ -match "^dev: help Volume" })
  Stop-App
}

# ─────────── DP-05: bit depth ───────────
if (Want "DP-05") {
  $p = Fx "fx_basic" "dp05"; $d = Join-Path $Work "d05"
  Launch -Project $p -Data $d
  Shot "dp05_before"                                           # a version 4 file: 16-bit
  Invoke-Menu "File" 8; Start-Sleep -Milliseconds 400          # Project Settings…
  $obs.dp05_clicked = Click-Text "24-bit"; Start-Sleep -Milliseconds 300
  Shot "dp05_settings"
  K 0x1B; Start-Sleep -Milliseconds 300
  Shot "dp05_after"
  Save; Keep $p "dp05_saved"
  $obs.dp05_dev = @(DevLines $d "^dev: project format")
  Stop-App
}

$obs | ConvertTo-Json -Depth 6 | Set-Content -Encoding utf8 (Join-Path $Work "obs.json")
python "$PSScriptRoot\check_dp.py" $Work
