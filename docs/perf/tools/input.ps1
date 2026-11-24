# usage: . .\input.ps1   (dot-source once the app window is open)
# Helpers to drive the app with synthetic input (client coords, physical px).
# Capture/TraceCmd talk to capture_server.ps1 through <Work>\capq, where Work is the
# caller's $Work, else $env:CHIPDAW_PERF_WORK, else %TEMP%\chipdaw-perf.
if (-not $Work) { $Work = $(if ($env:CHIPDAW_PERF_WORK) { $env:CHIPDAW_PERF_WORK } else { Join-Path $env:TEMP "chipdaw-perf" }) }
$global:CapQ = Join-Path $Work "capq"
Add-Type @"
using System;
using System.Runtime.InteropServices;
public class In {
  [DllImport("user32.dll")] public static extern bool SetProcessDPIAware();
  [DllImport("user32.dll")] public static extern bool SetCursorPos(int x, int y);
  [DllImport("user32.dll")] public static extern void mouse_event(uint f, int dx, int dy, int d, UIntPtr e);
  [DllImport("user32.dll")] public static extern void keybd_event(byte vk, byte scan, uint f, UIntPtr e);
  [DllImport("user32.dll")] public static extern uint MapVirtualKey(uint c, uint t);
  [DllImport("user32.dll")] public static extern bool SetForegroundWindow(IntPtr h);
  [StructLayout(LayoutKind.Sequential)] public struct POINT { public int X, Y; }
  [DllImport("user32.dll")] public static extern bool ClientToScreen(IntPtr h, ref POINT p);
}
"@
[In]::SetProcessDPIAware() | Out-Null
$global:AppH = (Get-Process -Name app | Where-Object { $_.MainWindowHandle -ne 0 } | Select-Object -First 1).MainWindowHandle
[In]::SetForegroundWindow($global:AppH) | Out-Null
function Pt($x, $y) { $p = New-Object In+POINT; $p.X = $x; $p.Y = $y; [In]::ClientToScreen($global:AppH, [ref]$p) | Out-Null; $p }
function Move-To($x, $y) { $p = Pt $x $y; [In]::SetCursorPos($p.X, $p.Y) | Out-Null; Start-Sleep -Milliseconds 30 }
function Down { [In]::mouse_event(0x0002, 0, 0, 0, [UIntPtr]::Zero); Start-Sleep -Milliseconds 40 }
function Up { [In]::mouse_event(0x0004, 0, 0, 0, [UIntPtr]::Zero); Start-Sleep -Milliseconds 40 }
function Wheel($n) { [In]::mouse_event(0x0800, 0, 0, 120 * $n, [UIntPtr]::Zero); Start-Sleep -Milliseconds 40 }
function Key($vk) { [In]::keybd_event($vk, 0, 0, [UIntPtr]::Zero); Start-Sleep -Milliseconds 20; [In]::keybd_event($vk, 0, 2, [UIntPtr]::Zero); Start-Sleep -Milliseconds 60 }
function KeyDown($vk) { [In]::keybd_event($vk, 0, 0, [UIntPtr]::Zero); Start-Sleep -Milliseconds 20 }
function KeyUp($vk) { [In]::keybd_event($vk, 0, 2, [UIntPtr]::Zero); Start-Sleep -Milliseconds 20 }
function Drag-Path($x0, $y0, $x1, $y1, $steps = 20) {
  Move-To $x0 $y0; Down
  for ($i = 1; $i -le $steps; $i++) { Move-To ([int]($x0 + ($x1 - $x0) * $i / $steps)) ([int]($y0 + ($y1 - $y0) * $i / $steps)) }
}
$global:Shot = Join-Path $PSScriptRoot "shot.ps1"
function KeyExt($vk) { $sc = [byte][In]::MapVirtualKey($vk, 0); [In]::keybd_event($vk, $sc, 1, [UIntPtr]::Zero); Start-Sleep -Milliseconds 20; [In]::keybd_event($vk, $sc, 3, [UIntPtr]::Zero); Start-Sleep -Milliseconds 60 }
function Capture($procId, $secs, $name) {
  $Q = $global:CapQ
  Remove-Item "$Q\$name.done", "$Q\$name.log" -ErrorAction SilentlyContinue
  "$procId|$secs|$name" | Set-Content "$Q\$name.req"
  for ($i = 0; $i -lt ($secs + 120); $i++) { if (Test-Path "$Q\$name.done") { break }; Start-Sleep -Seconds 1 }
  Get-Content "$Q\$name.log" -ErrorAction SilentlyContinue | Select-Object -Last 4
}
function TraceCmd($verb, $name) {
  $Q = $global:CapQ
  $id = "$verb-$name"
  Remove-Item -LiteralPath "$Q\$id.done" -ErrorAction SilentlyContinue
  "$verb|$name" | Set-Content "$Q\$id.req"
  for ($i = 0; $i -lt 600; $i++) { if (Test-Path "$Q\$id.done") { break }; Start-Sleep -Milliseconds 200 }
  Get-Content "$Q\$id.log" -ErrorAction SilentlyContinue | Select-Object -Last 3
}
