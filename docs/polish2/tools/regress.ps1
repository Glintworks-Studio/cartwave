# usage: regress.ps1 -Exe <app.exe> -Out <dir> [-Scale 100|150] [-New]
# Polish 2 (POLISH2_PROMPT.md §9.1): the polish pass's gate with the Overview hidden
# (settings) and the status bar hidden (dev hook CHIPDAW_HIDE_STATUS=1). -New: the
# polish 2 build (its M / S keys moved: E1 header) and the extra states it adds.
# The polish pass's visual regression gate (POLISH_PROMPT.md §4.1): fixed states, each a
# window screenshot, with every level frozen (CHIPDAW_FROZEN_METERS=1), the playhead pinned
# while playing (CHIPDAW_PLAYHEAD), audio off, the window at 1440x920 logical (--size) and
# the mouse parked on empty canvas. Run it on the "before" and "after" builds; diff.py
# compares them.
param([string]$Exe = ".\target\release\app.exe", [string]$Out = "regress", [int]$Scale = 100, [switch]$New)
$Repo = (Resolve-Path (Join-Path $PSScriptRoot "..\..\..")).Path
$ErrorActionPreference = "Continue"
. "$Repo\docs\perf\tools\input.ps1" 2>$null
. "$Repo\docs\shell\tools\dialog.ps1" 2>$null
$ErrorActionPreference = "Stop"
$Exe = (Resolve-Path $Exe).Path
New-Item -ItemType Directory -Force $Out | Out-Null
$S = $Scale / 100.0
Add-Type -AssemblyName System.Drawing
Add-Type @"
using System; using System.Runtime.InteropServices;
public class Wr { [StructLayout(LayoutKind.Sequential)] public struct R { public int L, T, Rr, B; }
  [DllImport("dwmapi.dll")] public static extern int DwmGetWindowAttribute(IntPtr h, int a, out R r, int s); }
"@
# A screenshot of the window's frame bounds (shot.ps1's method), by handle: with a menu
# open, the process's "main window" can be the popup.
function Save-Window([IntPtr]$h, [string]$path) {
  $r = New-Object Wr+R; [Wr]::DwmGetWindowAttribute($h, 9, [ref]$r, 16) | Out-Null
  $bmp = New-Object System.Drawing.Bitmap ($r.Rr - $r.L), ($r.B - $r.T)
  $g = [System.Drawing.Graphics]::FromImage($bmp)
  $g.CopyFromScreen($r.L, $r.T, 0, 0, $bmp.Size)
  $bmp.Save($path, [System.Drawing.Imaging.ImageFormat]::Png); $g.Dispose(); $bmp.Dispose()
}
function P($x, $y) { Move-To ([int]($x * $S)) ([int]($y * $S)) }
function Click($x, $y) { P $x $y; Down; Up; Start-Sleep -Milliseconds 300 }
function Park { P 1300 840; Start-Sleep -Milliseconds 700 }

# name, extra env, extra settings json, an action after launch
$States = @(
  @("stopped", @{}, "", $null),
  @("playing", @{ CHIPDAW_PLAYHEAD = "7680" }, "", { Key 0x20; Start-Sleep -Milliseconds 500 }),
  @("loop_off", @{}, "", { Click 293 38 }),
  @("solo_mute", @{}, "", { if ($New) { Click (241 + 300) (104 + 2 * 73 + 22); Click (241 + 268) (104 + 3 * 73 + 22) } else { Click (241 + 356) (104 + 2 * 73 + 28); Click (241 + 324) (104 + 3 * 73 + 28) } }),
  @("browser_closed", @{}, ', "browser_open": false', $null),
  @("inspector", @{}, "", { Click 700 140 }),
  @("piano_roll", @{ CHIPDAW_OPEN_CLIP = "1" }, "", $null),
  @("settings", @{}, "", { KeyDown 0x11; Key 0xBC; KeyUp 0x11; Start-Sleep -Milliseconds 600 }),
  @("menu_edit", @{}, "", { Open-Menu "Edit"; Start-Sleep -Milliseconds 600 }),
  @("menu_view", @{}, "", { Open-Menu "View"; Start-Sleep -Milliseconds 600 })
)
if ($New) {
  # Settings > Look & Feel (the new section; after only).
  $States += ,@("look_feel", @{}, "", { KeyDown 0x11; Key 0xBC; KeyUp 0x11; Start-Sleep -Milliseconds 600; Click 640 168; Start-Sleep -Milliseconds 400 })
}
foreach ($st in $States) {
  $name, $envs, $extra, $act = $st
  Get-Process -Name app -ErrorAction SilentlyContinue | Stop-Process -Force
  Start-Sleep -Milliseconds 300
  $d = Join-Path $env:TEMP "chipdaw-regress-data"
  Remove-Item -Recurse -Force $d -ErrorAction SilentlyContinue
  New-Item -ItemType Directory -Force $d | Out-Null
  "{ ""ui_scale"": $Scale, ""show_overview"": false $extra }" | Set-Content -Encoding ascii (Join-Path $d "settings.json")
  foreach ($k in "CHIPDAW_AUTOPLAY", "CHIPDAW_PLAYHEAD", "CHIPDAW_OPEN_CLIP", "CHIPDAW_PROJECT") { Remove-Item "Env:$k" -ErrorAction SilentlyContinue }
  $env:CHIPDAW_DATA_DIR = $d; $env:CHIPDAW_DEV = "1"; $env:CHIPDAW_AUDIO = "0"; $env:CHIPDAW_FROZEN_METERS = "1"; $env:CHIPDAW_HIDE_STATUS = "1"
  foreach ($k in $envs.Keys) { Set-Item "Env:$k" $envs[$k] }
  $p = Start-Process $Exe -ArgumentList "--size", "1440x920" -PassThru -RedirectStandardError (Join-Path $d "stderr.txt")
  for ($i = 0; $i -lt 50; $i++) { Start-Sleep -Milliseconds 100; if ((Get-Process -Id $p.Id).MainWindowHandle -ne 0) { break } }
  Start-Sleep -Milliseconds 1500
  pwsh -NoProfile -File "$Repo\docs\perf\tools\shot.ps1" -Out (Join-Path $d "probe.png") | Out-Null
  $global:AppH = (Get-Process -Id $p.Id).MainWindowHandle
  [In]::SetForegroundWindow($global:AppH) | Out-Null
  Park
  if ($act) { & $act; if ($name -notlike "menu_*") { Park } }
  Start-Sleep -Milliseconds 800
  Save-Window $global:AppH (Join-Path $Out "$name.png")
  if ($name -like "menu_*") { MKey 0x1B; MKey 0x1B }
  Stop-Process -Id $p.Id -Force -ErrorAction SilentlyContinue
}
"shots -> $Out"
