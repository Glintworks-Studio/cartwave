# usage: render_mockup.ps1 [-Out mockup.png] [-Scale 1]
# Renders docs/design/mockup-D.dc.html at 1440x400 CSS px with headless
# Chrome, through the support.js shim next to this script. Web fonts come
# from Google Fonts as the mockup links them (the virtual-time budget lets
# them load before the capture).
param([string]$Out = "mockup.png", [int]$Scale = 1)
$Repo = (Resolve-Path (Join-Path $PSScriptRoot "..\..\..")).Path
$tmp = Join-Path $env:TEMP "chipdaw-mockup-render"
New-Item -ItemType Directory -Force $tmp | Out-Null
Copy-Item "$Repo\docs\design\mockup-D.dc.html" "$tmp\mockup.html" -Force
Copy-Item "$PSScriptRoot\support.js" "$tmp\support.js" -Force
$chrome = "C:\Program Files\Google\Chrome\Application\chrome.exe"
$outAbs = [System.IO.Path]::GetFullPath($Out)
& $chrome --headless=new --disable-gpu --hide-scrollbars --force-device-scale-factor=$Scale `
  --window-size=1440,400 --virtual-time-budget=8000 --screenshot="$outAbs" "file:///$($tmp -replace '\\','/')/mockup.html" 2>$null | Out-Null
Start-Sleep -Milliseconds 500
"rendered -> $outAbs"
