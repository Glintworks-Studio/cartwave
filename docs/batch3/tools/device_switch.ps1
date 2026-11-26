# usage: device_switch.ps1 [-Work <dir>]
# Settings > Audio while playing: switch the output to the HDMI endpoint and back to the MOTU
# through the panel (UI Automation clicks on the device names), twice. The app logs each
# switch's gap ("audio-switch: gap_ms=…", the longest interval between callbacks).
param([string]$Work = (Join-Path $env:TEMP "chipdaw-b3-devswitch"))
$global:Work = $Work
New-Item -ItemType Directory -Force $Work | Out-Null
. (Join-Path $PSScriptRoot "lib.ps1")
Set-Location $Repo
$global:Exe = (Resolve-Path ".\target\release\app.exe").Path
$p = Join-Path $Work "demo.chipdaw"; Copy-Item "$Repo\docs\songs\v3\demo_band.chipdaw" $p -Force
$d = Join-Path $Work "d"
New-Item -ItemType Directory -Force $d | Out-Null
# Audio on this time (Launch turns it off for the UI tests).
Remove-Item Env:CHIPDAW_AUDIO -ErrorAction SilentlyContinue
$env:CHIPDAW_DATA_DIR = $d; $env:CHIPDAW_DEV = "1"; $env:CHIPDAW_PROJECT = $p
$script:proc = Start-Process $global:Exe -PassThru -RedirectStandardError (Join-Path $d "stderr.txt")
for ($i = 0; $i -lt 50; $i++) { Start-Sleep -Milliseconds 100; if ((Get-Process -Id $script:proc.Id).MainWindowHandle -ne 0) { break } }
Start-Sleep -Seconds 2
$global:AppH = (Get-Process -Id $script:proc.Id).MainWindowHandle
[In]::SetForegroundWindow($global:AppH) | Out-Null
K 0x20; Start-Sleep -Seconds 2                                  # play
Ctrl 0xBC; Start-Sleep -Milliseconds 600                        # Settings (Audio tab)
foreach ($round in 1, 2) {
  "hdmi: " + (Click-Text "VX3211-2K (NVIDIA High Definition Audio)"); Start-Sleep -Seconds 4
  "motu: " + (Click-Text "Out 1-2 (2- MOTU M Series)"); Start-Sleep -Seconds 4
}
Shot "device_switch"
K 0x1B; K 0x20
Stop-App
Get-Content (Join-Path $d "stderr.txt") | Where-Object { $_ -match "audio-switch" }
