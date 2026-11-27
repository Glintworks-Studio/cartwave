# usage: sample_drag.ps1 -Exe <cartwave.exe> -Out <folded.txt> [-DragX 760 -DragY 140] [-Samples 2000]
# Samples the UI (main) thread's stacks during the 200-clip drag (docs/perf/tools/Sampler.cs,
# a non-ETW sampler) and writes folded stacks. Used to attribute the drag's CPU (POLISH.md).
param([string]$Exe, [string]$Out, [int]$DragX = 760, [int]$DragY = 140, [int]$Samples = 2000)
$Repo = (Resolve-Path (Join-Path $PSScriptRoot "..\..\..")).Path
. "$Repo\docs\perf\tools\input.ps1"
Add-Type -Path "$Repo\docs\perf\tools\Sampler.cs"
Get-Process cartwave -ErrorAction SilentlyContinue | Stop-Process -Force
$d = Join-Path $env:TEMP "chipdaw-sample"; New-Item -ItemType Directory -Force $d | Out-Null
$env:CARTWAVE_DATA_DIR = $d; $env:CARTWAVE_DEV = "1"; $env:CARTWAVE_AUDIO = "1"; $env:CARTWAVE_STRESS = "1"
$p = Start-Process (Resolve-Path $Exe).Path -PassThru
for ($i = 0; $i -lt 50; $i++) { Start-Sleep -Milliseconds 100; $p.Refresh(); if ($p.MainWindowHandle -ne 0) { break } }
Start-Sleep 3
pwsh -NoProfile -File "$Repo\docs\perf\tools\shot.ps1" -Out (Join-Path $d "probe.png") | Out-Null   # top-left
$p.Refresh()
$global:AppH = $p.MainWindowHandle
[In]::SetForegroundWindow($global:AppH) | Out-Null
$p.Refresh()
$tid = ($p.Threads | Sort-Object StartTime | Select-Object -First 1).Id
Move-To $DragX $DragY; Down
$job = Start-ThreadJob -ScriptBlock {
  param($h, $x, $y)
  Add-Type @"
using System; using System.Runtime.InteropServices;
public class M { [DllImport("user32.dll")] public static extern bool SetCursorPos(int x, int y);
  [StructLayout(LayoutKind.Sequential)] public struct P { public int X, Y; }
  [DllImport("user32.dll")] public static extern bool ClientToScreen(IntPtr h, ref P p); }
"@
  $o = New-Object M+P; [M]::ClientToScreen($h, [ref]$o) | Out-Null
  $sw = [Diagnostics.Stopwatch]::StartNew(); $i = 0
  while ($sw.Elapsed.TotalSeconds -lt 14) { [M]::SetCursorPos($o.X + $x + [int](120 * [Math]::Sin($i / 12.0)), $o.Y + $y + [int](64 * [Math]::Sin($i / 30.0))) | Out-Null; $i++; Start-Sleep -Milliseconds 8 }
} -ArgumentList $global:AppH, $DragX, $DragY
Start-Sleep 1
$sym = Split-Path (Resolve-Path $Exe).Path
$folded = [Sampler]::Run([int]$p.Id, [int]$tid, $Samples, 5, $sym)
Wait-Job $job | Out-Null
Up
$folded | Set-Content -Encoding utf8 $Out
Stop-Process -Id $p.Id -Force
"$Out"
