# usage: screens_ux.ps1 [-Exe .\target\release\app.exe] [-Part 1|2|3]
# UX audit inventory screenshots (docs/ux-audit/INVENTORY.md). Synthetic input as in
# docs/stabilize/tools/scenario_v11.ps1. Window 1440x900 at 100%, the built-in demo song,
# audio off, a throw-away data dir (so the user's settings.json is not touched).
# Coordinates are client px (screenshot px minus the 1 px border and the 31 px title bar).
param([string]$Exe = ".\target\release\app.exe",
  [string]$Repo = (Resolve-Path (Join-Path $PSScriptRoot "..\..\..")).Path,
  [string]$Work = (Join-Path $env:TEMP "chipdaw-ux"),
  [int]$Part = 1)
$env:CHIPDAW_DEV = "1"   # F-02: test hooks (CHIPDAW_PROJECT, _STRESS, …) and F3-F7 need dev mode
$ErrorActionPreference = "Stop"
Set-Location $Repo
$Out = Join-Path $Repo "docs\ux-audit\screens"
New-Item -ItemType Directory -Force "$Work\data", $Out | Out-Null
Get-Process -Name app -ErrorAction SilentlyContinue | Stop-Process -Force
Remove-Item "$Work\data\*" -ErrorAction SilentlyContinue
$env:CHIPDAW_DATA_DIR = "$Work\data"; $env:CHIPDAW_AUDIO = "0"
$p = Start-Process (Resolve-Path $Exe).Path -PassThru
Start-Sleep -Seconds 3
. "$Repo\docs\perf\tools\input.ps1"
function Ctrl($vk) { KeyDown 0x11; Key $vk; KeyUp 0x11; Start-Sleep -Milliseconds 250 }
function CtrlShift($vk) { KeyDown 0x11; KeyDown 0x10; Key $vk; KeyUp 0x10; KeyUp 0x11; Start-Sleep -Milliseconds 300 }
function CtrlAlt($vk) { KeyDown 0x11; KeyDown 0x12; Key $vk; KeyUp 0x12; KeyUp 0x11; Start-Sleep -Milliseconds 300 }
function Click($x, $y) { Move-To $x $y; Down; Up; Start-Sleep -Milliseconds 250 }
function DClick($x, $y) { Move-To $x $y; Down; Up; Down; Up; Start-Sleep -Milliseconds 500 }
function RClick($x, $y) { Move-To $x $y; [In]::mouse_event(0x0008, 0, 0, 0, [UIntPtr]::Zero); Start-Sleep -Milliseconds 40; [In]::mouse_event(0x0010, 0, 0, 0, [UIntPtr]::Zero); Start-Sleep -Milliseconds 300 }
function Shot($n) { pwsh -NoProfile -File "$Repo\docs\perf\tools\shot.ps1" -Out "$Out\$n.png" | Out-Null; [In]::SetForegroundWindow($global:AppH) | Out-Null; Start-Sleep -Milliseconds 200 }
function Esc { Key 0x1B; Start-Sleep -Milliseconds 250 }

if ($Part -eq 1) {
  Move-To 1300 700
  Shot "01-main-idle"
  Click 1099 133;               Shot "02-clip-selected-inspector"
  RClick 1099 133;              Shot "03-context-menu-clip"
  Esc
  RClick 999 448;               Shot "04-context-menu-empty"
  Esc
  Drag-Path 999 390 1250 510 20; Up; Start-Sleep -Milliseconds 300
  RClick 1100 450;              Shot "05-context-menu-time-selection"
  Esc
  RClick 400 128;               Shot "06-right-click-track-header"
  Click 700 248;                Shot "07-inspector-triangle"
  Click 850 313;                Shot "08-inspector-noise-kit"
  Click 940 373;                Shot "09-inspector-dpcm"
  DClick 1099 133;              Shot "10-piano-roll"
  RClick 1000 700;              Shot "11-right-click-piano-roll"
  Esc
}
if ($Part -eq 2) {
  CtrlShift 0x52;               Shot "12-export-dialog"
  Esc;                          Shot "13-export-dialog-after-esc"
}
if ($Part -eq 3) {
  Click 514 17;                 Shot "14-chip-2a03-only"
  Click 514 17
  CtrlAlt 0x42;                 Shot "15-browser-closed"
  CtrlAlt 0x42
  Ctrl 0x4E;                    Shot "16-new-empty-project"
  Ctrl 0x5A;                    Shot "17-notice-nothing-to-undo"
}
"done part $Part"
