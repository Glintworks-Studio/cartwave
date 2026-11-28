# usage: shots_pb.ps1 [-Exe .\target\release\cartwave.exe] [-Out docs\screens\presets] [-Scale 100]
# The batch's screenshots (PRESETS_PROMPT.md §9): the device on a studio and a locked track, the
# browser (folders as on the board, and a search), the preset picker, the DPCM Sampler. Window
# frame captures at 1440 x 920 logical, levels frozen, audio off. The mockup match reads
# device-studio.png and device-locked.png.
param([string]$Exe = ".\target\release\cartwave.exe", [string]$Out = "docs\screens\presets", [int]$Scale = 100)
$Repo = (Resolve-Path (Join-Path $PSScriptRoot "..\..\..")).Path
$global:Work = Join-Path $env:TEMP "pb-shots"
New-Item -ItemType Directory -Force $global:Work, $Out | Out-Null
. (Join-Path $Repo "docs\batch3\tools\lib.ps1")
$global:Exe = (Resolve-Path $Exe).Path
$Out = (Resolve-Path $Out).Path
foreach ($k in "CARTWAVE_OVERVIEW", "CARTWAVE_HIDE_STATUS", "CARTWAVE_BROWSER_W", "CARTWAVE_PROJECT") { Remove-Item "Env:$k" -ErrorAction SilentlyContinue }
$env:CARTWAVE_FROZEN_METERS = "1"
function Place { pwsh -NoProfile -File "$Repo\docs\perf\tools\shot.ps1" -Out "$global:Work\_place.png" | Out-Null; Start-Sleep -Milliseconds 300 }
function ClickL($x, $y) { MoveL $x $y; Start-Sleep -Milliseconds 150; Down; Up; Start-Sleep -Milliseconds 350 }
function DblL($x, $y) { MoveL $x $y; Start-Sleep -Milliseconds 150; Down; Up; Start-Sleep -Milliseconds 60; Down; Up; Start-Sleep -Milliseconds 600 }
function Park { MoveL 1300 120; Start-Sleep -Milliseconds 500 }
function Snap($name) { Park; pwsh -NoProfile -File "$Repo\docs\perf\tools\shot.ps1" -Out (Join-Path $Out "$name.png") | Out-Null }
function Fresh { $d = Join-Path $global:Work "data"; Remove-Item -Recurse -Force $d -ErrorAction SilentlyContinue; Launch -Data $d -Scale $Scale; Place; $d }

# The studio device: a new project's first track (a studio Pulse) with Bright Square Lead, as the board.
$d = Fresh
Ctrl 0x4E; Start-Sleep -Milliseconds 800
ClickL 520 160
ClickL 75 244; ClickL 260 192; ClickL 260 216; DblL 300 240   # Sounds > Leads > Square > Bright Square Lead
Snap "browser"
Snap "device-studio"
ClickL 220 130; TypeText "kick"; Start-Sleep -Milliseconds 500
Snap "browser-search"
Stop-App

# The locked device: the demo's Pulse 1 (2A03 P1, Square Lead), and its picker.
$d = Fresh
ClickL 520 160
Snap "device-locked"
ClickL 664 572; Start-Sleep -Milliseconds 500
Park; pwsh -NoProfile -File "$Repo\docs\perf\tools\shot.ps1" -Out (Join-Path $Out "picker-dropdown.png") | Out-Null
K 0x1B
# The DPCM Sampler: the demo's DPCM track (track 5).
ClickL 1300 400; Start-Sleep -Milliseconds 200
ClickL 520 (136 + 4 * 73 + 24)
Snap "dpcm-sampler"
Stop-App
Remove-Item Env:CARTWAVE_FROZEN_METERS -ErrorAction SilentlyContinue
"shots -> $Out"
