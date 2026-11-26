# usage: shots_p2.ps1 [-Out <dir>]
# Polish 2 §10: the raw screenshots behind docs/screens/polish2 (compose_p2.py assembles them)
# and the mockup-match shots (gate 2). Release build, meters frozen, audio off, 1440x920.
param([string]$Out = (Join-Path $env:TEMP "chipdaw-p2-shots"))
$Repo = (Resolve-Path (Join-Path $PSScriptRoot "..\..\..")).Path
Set-Location $Repo
New-Item -ItemType Directory -Force $Out | Out-Null
$S = "$PSScriptRoot\shot_layout.ps1"
$songs = "$Repo\docs\polish2\songs"
function Shot-Layout($name, $settings = "", $project = "$songs\mockup.chipdaw") {
  pwsh -NoProfile -File $S -Out "$Out\$name.png" -Settings $settings -Project $project | Out-Null
  "$name"
}
# E1 (4 tracks at 72 px, the Overview hidden as on the E1 board) and the four layouts.
Shot-Layout "e1" '"show_overview": false'
Shot-Layout "e1_overview" '"show_overview": true'
Shot-Layout "lay_lo" '"show_overview": false'
Shot-Layout "lay_lc" '"show_overview": false, "strip_by_clips": true'
Shot-Layout "lay_rc" '"show_overview": false, "headers_right": true, "strip_by_clips": true'
Shot-Layout "lay_ro" '"show_overview": false, "headers_right": true'
# Widths.
Shot-Layout "w380" '"show_overview": false'
Shot-Layout "w520" '"show_overview": false, "header_width": 520'
Shot-Layout "w600" '"show_overview": false, "header_width": 600'
Shot-Layout "w380_right" '"show_overview": false, "headers_right": true, "strip_by_clips": true'
# Row heights (the board's 72 / 144 / 240, and 72 / 96 / 144 / 240).
Shot-Layout "rows_board" '"show_overview": false' "$songs\mockup_rows.chipdaw"
Shot-Layout "rows" '"show_overview": false' "$songs\rows.chipdaw"
# The status bar: a persistent problem, then a message and hover help. Launched here (not through
# shot_layout.ps1 -Keep: a child pwsh would wait on the app's inherited output handles).
. (Join-Path $Repo "docs\batch3\tools\lib.ps1")
$global:Work = $Out
$global:Exe = (Resolve-Path (Join-Path $Repo "target\release\app.exe")).Path
$env:CHIPDAW_FAKE_PROBLEM = "Audio dropouts: 3 buffer underruns so far. A larger buffer (Settings > Audio) helps."
$env:CHIPDAW_FROZEN_METERS = "1"
$sp = Join-Path $Out "status.chipdaw"; Copy-Item "$songs\mockup.chipdaw" $sp -Force
Launch -Project $sp -Data (Join-Path $Out "dstatus")
MoveL 1300 860; Start-Sleep -Milliseconds 500
Shot "status_idle"
ClickL 900 800; Ctrl 0x53; Start-Sleep -Milliseconds 300
MoveL 589 (136 + 30); Start-Sleep -Milliseconds 500
Shot "status_msg_help"
ClickL 22 908; Start-Sleep -Milliseconds 600
Shot "status_problems"
Key 0x1B
Remove-Item Env:CHIPDAW_FAKE_PROBLEM
# The ruler zoom: a frame sequence while dragging down on bar 5.
MoveL 990 122; Start-Sleep -Milliseconds 200; Down
for ($i = 0; $i -le 6; $i++) { MoveL 990 (122 + 12 * $i); Start-Sleep -Milliseconds 250; Shot ("ruler_zoom_{0}" -f $i) }
Up
Stop-App
Get-Process -Name app -ErrorAction SilentlyContinue | Stop-Process -Force
"shots -> $Out"
