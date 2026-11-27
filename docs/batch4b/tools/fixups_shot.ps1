# usage: fixups_shot.ps1 -Out <png> [-Scale 100] [-Exe .\target\release\cartwave.exe]
# Batch 4b §9.3: the `fixups` board's view. fx_board12 (12 tracks: the board's 9-12 are the last
# four), browser closed, window 1440 × 456 logical like the board, lanes scrolled to the bottom so
# tracks 9-12 show, and a clip on track 9 selected (the board's "Hook A": the selection outline).
param([string]$Out, [int]$Scale = 100, [string]$Exe = ".\target\release\cartwave.exe")
$Repo = (Resolve-Path (Join-Path $PSScriptRoot "..\..\..")).Path
$d = Join-Path $env:TEMP ("chipdaw-fixups-" + [guid]::NewGuid().ToString("N").Substring(0, 8))
New-Item -ItemType Directory -Force $d | Out-Null
python "$Repo\docs\batch4b\tools\fixtures_b4b.py" $d | Out-Null
"{ ""ui_scale"": $Scale, ""browser_open"": false }" | Set-Content -Encoding ascii (Join-Path $d "settings.json")
. "$Repo\docs\perf\tools\input.ps1" 2>$null
Get-Process -Name cartwave -ErrorAction SilentlyContinue | Stop-Process -Force
$env:CARTWAVE_DATA_DIR = $d; $env:CARTWAVE_DEV = "1"; $env:CARTWAVE_AUDIO = "0"; $env:CARTWAVE_FROZEN_METERS = "1"
$env:CARTWAVE_PROJECT = (Join-Path $d "fx_board12.chipdaw")
$p = Start-Process (Resolve-Path $Exe).Path -ArgumentList "--size", "1440x720" -PassThru -RedirectStandardError (Join-Path $d "stderr.txt")
Start-Sleep -Milliseconds 2500
pwsh -NoProfile -File "$Repo\docs\perf\tools\shot.ps1" -Out (Join-Path $d "_place.png") | Out-Null
$global:AppH = (Get-Process -Id $p.Id).MainWindowHandle
[In]::SetForegroundWindow($global:AppH) | Out-Null; Start-Sleep -Milliseconds 300
$k = $Scale / 100
# Scroll the rows to the bottom (the wheel over the headers).
Move-To ([int](200 * $k)) ([int](300 * $k)); Start-Sleep -Milliseconds 200
for ($i = 0; $i -lt 40; $i++) { [In]::mouse_event(0x0800, 0, 0, -120, [UIntPtr]::Zero); Start-Sleep -Milliseconds 30 }
Start-Sleep -Milliseconds 400
pwsh -NoProfile -File "$Repo\docs\perf\tools\shot.ps1" -Out ($Out -replace "\.png$", "_unselected.png") | Out-Null
# Select the first clip of track 9 ("Intro Lead" there): its row is the 4th from the bottom of
# the lanes (above the Main row); click its header bar (16 px tall) near its left.
$rows = Get-Content (Join-Path $d "stderr.txt") -ErrorAction SilentlyContinue
pwsh -NoProfile -File "$Repo\docs\perf\tools\shot.ps1" -Out (Join-Path $d "_probe.png") | Out-Null
python -c "
import sys
sys.path.insert(0, r'$Repo\docs\polish2\tools')
from diff import client_top
from PIL import Image
im = Image.open(r'$(Join-Path $d '_probe.png')').convert('RGB'); t = client_top(im, 100); px = im.load()
# The first clip header colour (#C9D24A, lime-ish, track 9) under the lanes' left: its top row.
k = $k
for y in range(int(140 * k) + t, im.height):
    p = px[int(400 * k), y]
    if abs(p[0] - 201) < 12 and abs(p[1] - 210) < 12 and abs(p[2] - 74) < 16:
        print(y - t); break
" | Set-Variable cy
if ($cy) { Move-To ([int](420 * $k)) ([int]$cy + [int](6 * $k)); Start-Sleep -Milliseconds 200; [In]::mouse_event(2, 0, 0, 0, [UIntPtr]::Zero); [In]::mouse_event(4, 0, 0, 0, [UIntPtr]::Zero); Start-Sleep -Milliseconds 600 }
Move-To ([int](1300 * $k)) ([int](60 * $k)); Start-Sleep -Milliseconds 400
pwsh -NoProfile -File "$Repo\docs\perf\tools\shot.ps1" -Out $Out | Out-Null
Stop-Process -Id $p.Id -Force -ErrorAction SilentlyContinue
Remove-Item Env:CARTWAVE_FROZEN_METERS, Env:CARTWAVE_PROJECT -ErrorAction SilentlyContinue
