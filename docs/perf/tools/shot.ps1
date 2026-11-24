# usage: shot.ps1 [-ProcName app] [-Out shot.png] [-Delay <ms>]
# Moves the process's main window to the top-left and saves a PNG of it (DWM frame bounds).
param([string]$ProcName = "app", [string]$Out = "shot.png", [int]$Delay = 0)
Add-Type -AssemblyName System.Drawing
Add-Type @"
using System;
using System.Runtime.InteropServices;
public class W {
  [DllImport("user32.dll")] public static extern bool SetProcessDPIAware();
  [DllImport("user32.dll")] public static extern bool SetForegroundWindow(IntPtr h);
  [StructLayout(LayoutKind.Sequential)] public struct RECT { public int L, T, R, B; }
  [DllImport("user32.dll")] public static extern bool SetWindowPos(IntPtr h, IntPtr a, int x, int y, int cx, int cy, uint f);
  [DllImport("dwmapi.dll")] public static extern int DwmGetWindowAttribute(IntPtr h, int a, out RECT r, int s);
}
"@
[W]::SetProcessDPIAware() | Out-Null
if ($Delay -gt 0) { Start-Sleep -Milliseconds $Delay }
$p = Get-Process -Name $ProcName -ErrorAction Stop | Where-Object { $_.MainWindowHandle -ne 0 } | Select-Object -First 1
$h = $p.MainWindowHandle
[W]::SetForegroundWindow($h) | Out-Null
# Move to top-left (SWP_NOSIZE | SWP_NOZORDER) so tall windows fit above the taskbar.
[W]::SetWindowPos($h, [IntPtr]::Zero, 0, 0, 0, 0, 0x0001 -bor 0x0004) | Out-Null
Start-Sleep -Milliseconds 400
$r = New-Object W+RECT
# DWMWA_EXTENDED_FRAME_BOUNDS = 9 (excludes the invisible resize border)
[W]::DwmGetWindowAttribute($h, 9, [ref]$r, 16) | Out-Null
$w = $r.R - $r.L; $hh = $r.B - $r.T
$bmp = New-Object System.Drawing.Bitmap $w, $hh
$g = [System.Drawing.Graphics]::FromImage($bmp)
$g.CopyFromScreen($r.L, $r.T, 0, 0, $bmp.Size)
$bmp.Save($Out, [System.Drawing.Imaging.ImageFormat]::Png)
"$w x $hh -> $Out"
