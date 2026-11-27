# usage: polish_shots.ps1 [-Exe .\target\release\cartwave.exe] [-Out docs\screens\polish]
# Screenshots for POLISH.md: the colour grid (a track's Colour ▸, hovering a cell),
# Settings ▸ Editing (Track and Clip Colors), and the window at its minimum size
# (1536x816 logical, a 1080p laptop at 125% without its taskbar; then the true minimum).
param([string]$Exe = ".\target\release\cartwave.exe", [string]$Out = "docs\screens\polish")
$Repo = (Resolve-Path (Join-Path $PSScriptRoot "..\..\..")).Path
$ErrorActionPreference = "Continue"
. "$Repo\docs\perf\tools\input.ps1" 2>$null
. "$Repo\docs\shell\tools\dialog.ps1" 2>$null
$ErrorActionPreference = "Stop"
$Exe = (Resolve-Path $Exe).Path
New-Item -ItemType Directory -Force $Out | Out-Null
$Out = (Resolve-Path $Out).Path
function Start-App([string[]]$AppArgs) {
  Get-Process -Name cartwave -ErrorAction SilentlyContinue | Stop-Process -Force
  Start-Sleep -Milliseconds 300
  $d = Join-Path $env:TEMP "chipdaw-polish-shots"
  Remove-Item -Recurse -Force $d -ErrorAction SilentlyContinue
  New-Item -ItemType Directory -Force $d | Out-Null
  $env:CARTWAVE_DATA_DIR = $d; $env:CARTWAVE_DEV = "1"; $env:CARTWAVE_AUDIO = "0"
  $p = Start-Process $Exe -ArgumentList $AppArgs -PassThru -RedirectStandardError (Join-Path $d "stderr.txt")
  for ($i = 0; $i -lt 50; $i++) { Start-Sleep -Milliseconds 100; if ((Get-Process -Id $p.Id).MainWindowHandle -ne 0) { break } }
  Start-Sleep -Milliseconds 1500
  pwsh -NoProfile -File "$Repo\docs\perf\tools\shot.ps1" -Out (Join-Path $d "probe.png") | Out-Null
  $global:AppH = (Get-Process -Id $p.Id).MainWindowHandle
  [In]::SetForegroundWindow($global:AppH) | Out-Null
  $p
}
function Shot($name) { pwsh -NoProfile -File "$Repo\docs\perf\tools\shot.ps1" -Out (Join-Path $Out $name) | Out-Null }
function RightClick($x, $y) { Move-To $x $y; [In]::mouse_event(0x0008, 0, 0, 0, [UIntPtr]::Zero); Start-Sleep -Milliseconds 40; [In]::mouse_event(0x0010, 0, 0, 0, [UIntPtr]::Zero); Start-Sleep -Milliseconds 400 }

# The colour grid: right-click the Triangle's header, hover Colour, then a cell.
$p = Start-App @("--size", "1440x920")
RightClick 400 (104 + 2 * 73 + 20)
Move-To 430 (104 + 2 * 73 + 20 + 4 + 28 + 14); Start-Sleep -Milliseconds 500   # "Colour" opens the grid
Move-To (400 + 236 + 4 + 8 + 4 * 20 + 8) (104 + 2 * 73 + 20 + 28 + 8 + 2 * 20 + 8); Start-Sleep -Milliseconds 500
Shot "color-grid-raw.png"
Stop-Process -Id $p.Id -Force

# Settings ▸ Editing.
$p = Start-App @("--size", "1440x920")
KeyDown 0x11; Key 0xBC; KeyUp 0x11; Start-Sleep -Milliseconds 600
Click-Text "Editing" | Out-Null; Start-Sleep -Milliseconds 500
Shot "color-settings-raw.png"
Stop-Process -Id $p.Id -Force

# The minimum window: 1536x816, then smaller than allowed (clamped to the minimum).
$p = Start-App @("--size", "1536x816")
Move-To 1200 760; Start-Sleep -Milliseconds 800   # empty canvas: no tooltip in the shot
Shot "min-window-1536x816.png"
Stop-Process -Id $p.Id -Force
$p = Start-App @("--size", "800x600")
Move-To 1100 700; Start-Sleep -Milliseconds 800
Shot "min-window-minimum.png"
Stop-Process -Id $p.Id -Force
"shots -> $Out"
