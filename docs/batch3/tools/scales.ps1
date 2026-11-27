# usage: scales.ps1 [-Work <dir>] [-Scales 100,125,150,200]
# Interface scale (Settings > Display) at each scale: starts the app with the demo song and a
# clip selected (so the inspector shows), saves a screenshot, and reports the window's client
# size (logical 1440x900 x the scale) and the stderr scale line from the piano roll bounds.
# Then switches the scale live through the Settings panel (100 -> 150) and screenshots again.
param([string]$Work = (Join-Path $env:TEMP "chipdaw-scales"), [string]$Scales = "100,125,150,200")
$global:Work = $Work
New-Item -ItemType Directory -Force $Work | Out-Null
. (Join-Path $PSScriptRoot "lib.ps1")
Set-Location $Repo
$global:Exe = (Resolve-Path ".\target\release\cartwave.exe").Path
Add-Type @"
using System; using System.Runtime.InteropServices;
public class CR { [StructLayout(LayoutKind.Sequential)] public struct R { public int L, T, Rt, B; }
  [DllImport("user32.dll")] public static extern bool GetClientRect(IntPtr h, out R r); }
"@
foreach ($sc in $Scales.Split(",")) {
  $sc = [int]$sc
  $p = Join-Path $Work "demo_$sc.chipdaw"; Copy-Item "$Repo\docs\songs\v3\demo_band.chipdaw" $p -Force
  $env:CARTWAVE_OPEN_CLIP = "1"
  Launch -Project $p -Data (Join-Path $Work "d$sc") -Scale $sc
  Remove-Item Env:CARTWAVE_OPEN_CLIP
  $r = New-Object CR+R; [CR]::GetClientRect($global:AppH, [ref]$r) | Out-Null
  "scale $sc%: client $($r.Rt)x$($r.B) (logical $([math]::Round($r.Rt / ($sc / 100)))x$([math]::Round($r.B / ($sc / 100))))"
  (StdErr (Join-Path $Work "d$sc") | Where-Object { $_ -match "piano roll bounds" } | Select-Object -Last 1)
  Shot "scale_$sc"
  Stop-App
}
# Live switch: 100% -> 150% from the Settings panel.
$p = Join-Path $Work "demo_live.chipdaw"; Copy-Item "$Repo\docs\songs\v3\demo_band.chipdaw" $p -Force
Launch -Project $p -Data (Join-Path $Work "dlive") -Scale 100
Ctrl 0xBC; Start-Sleep -Milliseconds 400
Click-Text "Display" | Out-Null; Start-Sleep -Milliseconds 400
Click-Text "150%" | Out-Null; Start-Sleep -Milliseconds 1200
$r = New-Object CR+R; [CR]::GetClientRect($global:AppH, [ref]$r) | Out-Null
"live switch to 150%: client $($r.Rt)x$($r.B)"
Shot "scale_live_150"
Get-Content (Join-Path $Work "dlive\settings.json") | Select-String "ui_scale"
Stop-App
