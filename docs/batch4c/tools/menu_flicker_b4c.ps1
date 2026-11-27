# usage: menu_flicker_b4c.ps1 -Out <dir> [-Exe .\target\release\cartwave.exe] [-DarkMenus] [-Play] [-Edge corner|right|bottom|left|top|topleft] [-Seconds 4]
# Batch 4c §3 (K1), from batch 4b's menu_flicker.ps1. New: -Play (audio on, the demo song
# playing from launch: the CEO saw the flicker while the song plays), the right, bottom and top
# edges, and the app's frame log (CARTWAVE_FRAMELOG, <Out>\frames.csv) for lining frames up
# with the capture. flicker_b4c.py also flags a bar that is misdrawn (not light: missing text).
# Batch 4b §3.4 (K1): does the Windows menu bar (File, Edit, …) flash while the window is
# resized? Launches the app (Settings > Display > Dark menus on or off), starts ffmpeg grabbing
# the menu-bar band (ddagrab, every composed frame up to 120 fps; or gdigrab 60), PNG frames, then resizes the window by a real mouse drag
# on its bottom-right corner (Windows' own sizing loop, as a user does), back and forth for
# -Seconds. flicker.py counts the frames where the band turns light.
param([string]$Out, [string]$Exe = ".\target\release\cartwave.exe", [switch]$DarkMenus, [int]$Seconds = 4, [string]$Grabber = "ddagrab", [string]$Edge = "corner", [switch]$SelfTest, [switch]$Play)
$Repo = (Resolve-Path (Join-Path $PSScriptRoot "..\..\..")).Path
New-Item -ItemType Directory -Force $Out | Out-Null
Get-ChildItem $Out -Filter "f_*.png" -ErrorAction SilentlyContinue | Remove-Item -Force
$d = Join-Path $Out "data"
New-Item -ItemType Directory -Force $d | Out-Null
$dm = if ($DarkMenus) { "true" } else { "false" }
"{ ""dark_menus"": $dm, ""browser_open"": true }" | Set-Content -Encoding ascii (Join-Path $d "settings.json")
Add-Type @"
using System; using System.Runtime.InteropServices;
public class Fl {
  [StructLayout(LayoutKind.Sequential)] public struct R { public int l, t, r, b; }
  [StructLayout(LayoutKind.Sequential)] public struct P { public int x, y; }
  [DllImport("user32.dll")] public static extern bool SetProcessDPIAware();
  [DllImport("user32.dll")] public static extern bool GetWindowRect(IntPtr h, out R r);
  [DllImport("user32.dll")] public static extern bool ClientToScreen(IntPtr h, ref P p);
  [DllImport("user32.dll")] public static extern bool SetForegroundWindow(IntPtr h);
  [DllImport("user32.dll")] public static extern bool SetCursorPos(int x, int y);
  [DllImport("user32.dll")] public static extern void mouse_event(uint f, uint dx, uint dy, uint d, UIntPtr e);
  [DllImport("user32.dll")] public static extern bool SetWindowPos(IntPtr h, IntPtr a, int x, int y, int w, int hh, uint f);
}
"@
[Fl]::SetProcessDPIAware() | Out-Null
Get-Process -Name cartwave -ErrorAction SilentlyContinue | Stop-Process -Force
$env:CARTWAVE_DATA_DIR = $d; $env:CARTWAVE_DEV = "1"
Remove-Item Env:CARTWAVE_PROJECT, Env:CARTWAVE_AUTOPLAY, Env:CARTWAVE_AUDIO -ErrorAction SilentlyContinue
if ($Play) { $env:CARTWAVE_AUTOPLAY = "1"; $env:CARTWAVE_FRAMELOG = (Join-Path $Out "frames.csv") } else { $env:CARTWAVE_AUDIO = "0"; Remove-Item Env:CARTWAVE_FRAMELOG -ErrorAction SilentlyContinue }
$p = Start-Process (Resolve-Path $Exe).Path -PassThru -RedirectStandardError (Join-Path $d "stderr.txt")
Start-Sleep -Milliseconds 2500
$h = (Get-Process -Id $p.Id).MainWindowHandle
[Fl]::SetWindowPos($h, [IntPtr]::Zero, 40, 40, 1400, 860, 0x0004) | Out-Null
Start-Sleep -Milliseconds 600
[Fl]::SetForegroundWindow($h) | Out-Null
$r = New-Object Fl+R; [Fl]::GetWindowRect($h, [ref]$r) | Out-Null
$c = New-Object Fl+P; [Fl]::ClientToScreen($h, [ref]$c) | Out-Null
# The menu bar: the band just above the client area (its height measured from the title bar's
# bottom: the window's top + the caption); grab from the title bar's middle to the client top.
$y0 = $r.t + 8; $hgt = $c.y - $y0
$x0 = $r.l + 8; $w = $r.r - $r.l - 16
if ($Edge -ne "corner") { $x0 = $r.l + 210; $w = $r.r - $r.l - 220 }
"band x $x0 y $y0 w $w h $hgt (client top $($c.y))" | Set-Content (Join-Path $Out "band.txt")
if ($Grabber -eq "gdigrab") {
  $fa = @("-hide_banner", "-loglevel", "error", "-f", "gdigrab", "-framerate", "60", "-offset_x", "$x0", "-offset_y", "$y0", "-video_size", "${w}x${hgt}", "-draw_mouse", "0", "-t", "$($Seconds + 2)", "-i", "desktop", (Join-Path $Out "f_%04d.png"))
} else {
  # Desktop Duplication: every frame DWM composes, up to 120 per second.
  $fa = @("-hide_banner", "-loglevel", "error", "-f", "lavfi", "-i", "ddagrab=output_idx=0:framerate=120:offset_x=${x0}:offset_y=${y0}:video_size=${w}x${hgt},hwdownload,format=bgra", "-t", "$($Seconds + 2)", (Join-Path $Out "f_%04d.png"))
}
$ff = Start-Process ffmpeg -ArgumentList $fa -PassThru -WindowStyle Hidden
Start-Sleep -Milliseconds 800
# The drag: the bottom-right corner, back and forth (Windows' sizing loop).
# -SelfTest: the detector's positive control. Instead of resizing, a white top-most window covers
# the bar twice for about 50 ms each: flicker.py must count those frames.
if ($SelfTest) {
  Add-Type -AssemblyName System.Windows.Forms, System.Drawing
  for ($k = 0; $k -lt 2; $k++) {
    Start-Sleep -Milliseconds 700
    $fw = New-Object System.Windows.Forms.Form
    $fw.FormBorderStyle = "None"; $fw.TopMost = $true; $fw.ShowInTaskbar = $false; $fw.BackColor = [System.Drawing.Color]::White
    $fw.StartPosition = "Manual"; $fw.Location = New-Object System.Drawing.Point($x0, ($c.y - 18)); $fw.Size = New-Object System.Drawing.Size(400, 18)
    $fw.Show(); [System.Windows.Forms.Application]::DoEvents(); Start-Sleep -Milliseconds 50; $fw.Close()
  }
  $ff.WaitForExit(20000) | Out-Null
  Stop-Process -Id $p.Id -Force -ErrorAction SilentlyContinue
  python "$Repo\docs\batch4c\tools\flicker_b4c.py" $Out
  return
}
# -Edge: corner (bottom-right), left (the left edge: the window moves) or topleft.
$gx = $r.r - 4; $gy = $r.b - 4
if ($Edge -eq "left") { $gx = $r.l + 4; $gy = [int](($r.t + $r.b) / 2) }
if ($Edge -eq "topleft") { $gx = $r.l + 4; $gy = $r.t + 4 }
if ($Edge -eq "right") { $gx = $r.r - 4; $gy = [int](($r.t + $r.b) / 2) }
if ($Edge -eq "bottom") { $gx = [int](($r.l + $r.r) / 2); $gy = $r.b - 4 }
if ($Edge -eq "top") { $gx = [int](($r.l + $r.r) / 2); $gy = $r.t + 4 }
$sgn = if ($Edge -in @("corner", "right", "bottom")) { 1 } else { -1 }
[Fl]::SetCursorPos($gx, $gy) | Out-Null; Start-Sleep -Milliseconds 200
[Fl]::mouse_event(2, 0, 0, 0, [UIntPtr]::Zero); Start-Sleep -Milliseconds 100
$steps = $Seconds * 60
$widths = @()
for ($i = 0; $i -lt $steps; $i++) {
  $t = $i / 60.0
  $dx = [int](-200 + 200 * [math]::Cos($t * 2 * [math]::PI * 0.7))
  $dy = [int](-60 + 60 * [math]::Cos($t * 2 * [math]::PI * 0.5))
  if ($Edge -in @("left", "right")) { $dy = 0 }
  if ($Edge -in @("top", "bottom")) { $dx = 0 }
  [Fl]::SetCursorPos($gx + $sgn * $dx, $gy + $sgn * $dy) | Out-Null
  [Fl]::mouse_event(1, 0, 0, 0, [UIntPtr]::Zero)
  Start-Sleep -Milliseconds 16
  if ($i % 30 -eq 0) { $q = New-Object Fl+R; [Fl]::GetWindowRect($h, [ref]$q) | Out-Null; $widths += @($q.r - $q.l) }
}
"window widths during the drag: $($widths -join ' ')" | Add-Content (Join-Path $Out "band.txt")
[Fl]::mouse_event(4, 0, 0, 0, [UIntPtr]::Zero)
$ff.WaitForExit(20000) | Out-Null
Stop-Process -Id $p.Id -Force -ErrorAction SilentlyContinue
foreach ($k in "CARTWAVE_AUTOPLAY", "CARTWAVE_FRAMELOG", "CARTWAVE_AUDIO") { Remove-Item "Env:$k" -ErrorAction SilentlyContinue }
python "$Repo\docs\batch4c\tools\flicker_b4c.py" $Out
