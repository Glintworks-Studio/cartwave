# Presets + Browser batch: regress_b4c.ps1 (the rename's states plus the mixer), the same method:
# frozen levels, pinned playhead, audio off, 1440x920 logical, the Overview and the status bar
# hidden, the mouse parked. usage: regress_pb.ps1 -Exe <cartwave.exe> -Out <dir> [-Scale 100|150] [-Only a,b]
# The rename's visual regression gate (RENAME_PROMPT.md §3.4): polish 2's regress.ps1 -New
# states (the method of every gate since: frozen levels, pinned playhead, audio off, 1440x920
# logical, the Overview and the status bar hidden, the mouse parked), plus the states that show
# the app's name: About, the shortcut sheet, Settings > Files (the data folder) and the Help menu.
#
# -Prefix picks the environment names the run uses: CARTWAVE_ (this build), CHIPDAW_ (the
# baseline build, which knows only those; and this build's alias run, gate 5).
# The baseline's app.exe is shot as a copy named cartwave.exe, so every by-name process lookup
# in the shared helpers finds both builds.
# -Shift: logical px the main area moved right (180 on this branch: the 420 px browser; 0 for
# the baseline), so a click on the arrangement lands on the same clip or track in both builds.
param([string]$Exe = ".\target\release\cartwave.exe", [string]$Out = "regress", [int]$Scale = 100, [string]$Prefix = "CARTWAVE_", [string[]]$Only = @(), [int]$Shift = 0)
$Only = @($Only | ForEach-Object { $_ -split "," } | Where-Object { $_ })
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
  [DllImport("dwmapi.dll")] public static extern int DwmGetWindowAttribute(IntPtr h, int a, out R r, int s);
  [DllImport("user32.dll")] public static extern bool GetWindowRect(IntPtr h, out R r);
  [DllImport("user32.dll")] public static extern bool SetWindowPos(IntPtr h, IntPtr after, int x, int y, int cx, int cy, uint f); }
"@
function Save-Window([IntPtr]$h, [string]$path) {
  $r = New-Object Wr+R; [Wr]::DwmGetWindowAttribute($h, 9, [ref]$r, 16) | Out-Null
  $bmp = New-Object System.Drawing.Bitmap ($r.Rr - $r.L), ($r.B - $r.T)
  $g = [System.Drawing.Graphics]::FromImage($bmp)
  $g.CopyFromScreen($r.L, $r.T, 0, 0, $bmp.Size)
  $bmp.Save($path, [System.Drawing.Imaging.ImageFormat]::Png); $g.Dispose(); $bmp.Dispose()
}
# The window's visible frame at the screen's top-left: at 150 % the window is taller than the
# work area, and Windows otherwise places it at a different height in each run (the taskbar
# then covers a different band; the frame's top can even leave the screen).
function Pin-Window([IntPtr]$h) {
  $f = New-Object Wr+R; [Wr]::DwmGetWindowAttribute($h, 9, [ref]$f, 16) | Out-Null
  $w = New-Object Wr+R; [Wr]::GetWindowRect($h, [ref]$w) | Out-Null
  # SWP_NOSIZE | SWP_NOZORDER | SWP_NOACTIVATE
  [Wr]::SetWindowPos($h, [IntPtr]::Zero, $w.L - $f.L, $w.T - $f.T, 0, 0, 0x0001 -bor 0x0004 -bor 0x0010) | Out-Null
  Start-Sleep -Milliseconds 400
}
function P($x, $y) { Move-To ([int]($x * $S)) ([int]($y * $S)) }
function Click($x, $y) { P $x $y; Down; Up; Start-Sleep -Milliseconds 300 }
function Park { P 1300 840; Start-Sleep -Milliseconds 700 }
function Settings { KeyDown 0x11; Key 0xBC; KeyUp 0x11; Start-Sleep -Milliseconds 600 }

# name, extra env (names without the prefix), extra settings json, an action after launch
$States = @(
  @("stopped", @{}, "", $null),
  @("playing", @{ PLAYHEAD = "7680" }, "", { Key 0x20; Start-Sleep -Milliseconds 500 }),
  @("loop_off", @{}, "", { Click 293 38 }),
  @("solo_mute", @{}, "", { Click (241 + $Shift + 300) (104 + 2 * 73 + 22); Start-Sleep -Milliseconds 700; Click (241 + $Shift + 268) (104 + 3 * 73 + 22) }),
  @("browser_closed", @{}, ', "browser_open": false', $null),
  @("inspector", @{}, "", { Click (700 + $Shift) 140 }),
  @("piano_roll", @{ OPEN_CLIP = "1" }, "", $null),
  @("settings", @{}, "", { Settings }),
  @("look_feel", @{}, "", { Settings; Click 611 160; Start-Sleep -Milliseconds 400 }),
  @("settings_files", @{}, "", { Settings; Click-Text "Files" | Out-Null; Start-Sleep -Milliseconds 400 }),
  @("about", @{}, "", { Invoke-Menu "Help" 2; Start-Sleep -Milliseconds 400 }),
  @("shortcuts", @{}, "", { Invoke-Menu "Help" 0; Start-Sleep -Milliseconds 400 }),
  @("menu_edit", @{}, "", { Open-Menu "Edit"; Start-Sleep -Milliseconds 600 }),
  @("menu_view", @{}, "", { Open-Menu "View"; Start-Sleep -Milliseconds 600 }),
  @("menu_help", @{}, "", { Open-Menu "Help"; Start-Sleep -Milliseconds 600 }),
  # Batch 4c: the mixer view (its strips read the meters and clip marks this batch changed).
  @("mixer", @{}, "", { Key 0x09; Start-Sleep -Milliseconds 600 })
)
$Hooks = "AUTOPLAY", "PLAYHEAD", "OPEN_CLIP", "PROJECT", "DATA_DIR", "DEV", "AUDIO", "FROZEN_METERS", "HIDE_STATUS"
foreach ($st in $States) {
  $name, $envs, $extra, $act = $st
  if ($Only.Count -and $Only -notcontains $name) { continue }
  Get-Process -Name cartwave -ErrorAction SilentlyContinue | Stop-Process -Force
  Start-Sleep -Milliseconds 300
  # A name-free folder: Settings > Files shows it, and it must read the same in both builds.
  $d = Join-Path $env:TEMP "rn-regress-data"
  Remove-Item -Recurse -Force $d -ErrorAction SilentlyContinue
  New-Item -ItemType Directory -Force $d | Out-Null
  "{ ""ui_scale"": $Scale, ""show_overview"": false $extra }" | Set-Content -Encoding ascii (Join-Path $d "settings.json")
  foreach ($p in "CARTWAVE_", "CHIPDAW_") { foreach ($k in $Hooks) { Remove-Item "Env:$p$k" -ErrorAction SilentlyContinue } }
  Set-Item "Env:${Prefix}DATA_DIR" $d; Set-Item "Env:${Prefix}DEV" "1"; Set-Item "Env:${Prefix}AUDIO" "0"
  Set-Item "Env:${Prefix}FROZEN_METERS" "1"; Set-Item "Env:${Prefix}HIDE_STATUS" "1"
  foreach ($k in $envs.Keys) { Set-Item "Env:$Prefix$k" $envs[$k] }
  $p = Start-Process $Exe -ArgumentList "--size", "1440x920" -PassThru -RedirectStandardError (Join-Path $Out "$name.stderr.txt")
  for ($i = 0; $i -lt 50; $i++) { Start-Sleep -Milliseconds 100; if ((Get-Process -Id $p.Id).MainWindowHandle -ne 0) { break } }
  Start-Sleep -Milliseconds 1500
  $global:AppH = (Get-Process -Id $p.Id).MainWindowHandle
  Pin-Window $global:AppH
  [In]::SetForegroundWindow($global:AppH) | Out-Null
  Park
  if ($act) { & $act; if ($name -notlike "menu_*") { Park } }
  Start-Sleep -Milliseconds 800
  Save-Window $global:AppH (Join-Path $Out "$name.png")
  if ($name -like "menu_*") { MKey 0x1B; MKey 0x1B }
  Stop-Process -Id $p.Id -Force -ErrorAction SilentlyContinue
}
foreach ($p in "CARTWAVE_", "CHIPDAW_") { foreach ($k in $Hooks) { Remove-Item "Env:$p$k" -ErrorAction SilentlyContinue } }
"shots -> $Out"
