# usage: screens_v11.ps1 [-Exe .\target\release\app.exe]
# The v1.1 screenshots (docs/screens/v1.1/), 1440x900 at 100%, plus one export run through the
# dialog (44.1 kHz, 24-bit, 8 channel stems), whose files are checked. Synthetic input.
param([string]$Exe = ".\target\release\app.exe",
  [string]$Repo = (Resolve-Path (Join-Path $PSScriptRoot "..\..\..")).Path,
  [string]$Work = (Join-Path $env:TEMP "chipdaw-screens-v11"))
$env:CHIPDAW_DEV = "1"   # F-02: test hooks (CHIPDAW_PROJECT, _STRESS, …) and F3-F7 need dev mode
$ErrorActionPreference = "Stop"
Set-Location $Repo
$out = Join-Path $Repo "docs\screens\v1.1"
New-Item -ItemType Directory -Force $out, $Work | Out-Null
Remove-Item "$Work\*" -Recurse -Force -ErrorAction SilentlyContinue
Copy-Item docs\songs\v2\demo_band.chipdaw "$Work\demo.chipdaw" -Force
Get-Process -Name app -ErrorAction SilentlyContinue | Stop-Process -Force
$env:CHIPDAW_PROJECT = "$Work\demo.chipdaw"; $env:CHIPDAW_DATA_DIR = "$Work\data"; $env:CHIPDAW_AUDIO = "0"
$env:CHIPDAW_EXPORT_TO = "$Work\export\song.wav"
$p = Start-Process (Resolve-Path $Exe).Path -PassThru
Start-Sleep -Seconds 3
. "$Repo\docs\perf\tools\input.ps1"
. "$Repo\docs\shell\tools\dialog.ps1"
function Click($x, $y) { Move-To $x $y; Down; Up; Start-Sleep -Milliseconds 250 }
function Full($n) { pwsh -NoProfile -File "$Repo\docs\perf\tools\shot.ps1" -Out "$Work\$n.png" | Out-Null }
# Crop a screen-space rectangle of the last full shot into docs/screens/v1.1.
function Crop($n, $from, $x0, $y0, $x1, $y1) {
  python -c "from PIL import Image; Image.open(r'$Work\$from.png').crop(($x0,$y0,$x1,$y1)).save(r'$out\$n.png')"
}

# Clock: m:ss.cc, then View > Clock Shows Bars (the clock click left with the app shell).
Move-To 1430 890; Full "f_clock_time"
Crop "clock-time" "f_clock_time" 610 36 830 88
Invoke-Menu "View" 2; Move-To 1430 890; Start-Sleep -Milliseconds 300; Full "f_clock_bars"
Crop "clock-bars" "f_clock_bars" 610 36 830 88
Invoke-Menu "View" 2                       # back to m:ss.cc

# A clip with its own instrument: Cello dropped on "Hook A".
Click 120 110
foreach ($c in "CELLO".ToCharArray()) { Key ([int][char]$c) }
Start-Sleep -Milliseconds 300
Drag-Path 42 237 1100 130 25; Start-Sleep -Milliseconds 300; Up; Start-Sleep -Milliseconds 400
# Knob and macro states: select "Intro Lead" (Square Lead overrides duty and envelope).
Click 700 130
Move-To 1430 640; Start-Sleep -Milliseconds 400
Full "f_main"
Copy-Item "$Work\f_main.png" "$out\main-v11.png"
Crop "clip-own-instrument" "f_main" 598 125 1442 200
Crop "knobs-and-macros" "f_main" 0 700 1442 932

# The export dialog, with group stems shown; then a real export (8 channel stems).
KeyDown 0x11; KeyDown 0x10; Key 0x52; KeyUp 0x10; KeyUp 0x11; Start-Sleep -Milliseconds 400   # Ctrl+Shift+R (the EXPORT key left the top bar)
Click 710 229                              # Groups (a row of lane chips appears)
Move-To 1430 890; Start-Sleep -Milliseconds 300
Full "f_dialog"
Crop "export-dialog" "f_dialog" 355 110 1085 460
Click 525 165                              # 44.1 kHz
Click 748 165                              # 24-bit
Click 620 229                              # 8 channels (the group row goes away)
Click 418 318                              # Export... (CHIPDAW_EXPORT_TO: no dialog)
Start-Sleep -Seconds 4
Move-To 1430 890; Full "f_exported"
Stop-Process -Id $p.Id -Force
Remove-Item Env:CHIPDAW_PROJECT, Env:CHIPDAW_DATA_DIR, Env:CHIPDAW_AUDIO, Env:CHIPDAW_EXPORT_TO

python -I -c @"
import os, struct, sys
d = sys.argv[1]
names = sorted(os.listdir(d))
print(len(names), 'files:', ', '.join(names[:5]), '...')
assert 'song.wav' in names and 'song_intro.wav' in names and 'song_loop.wav' in names and 'song.json' in names, names
stems = [n for n in names if '_stem_' in n]
assert len(stems) == 24, len(stems)
b = open(os.path.join(d, 'song.wav'), 'rb').read()
i = b.index(b'fmt ')
rate, bits = struct.unpack('<I', b[i + 12:i + 16])[0], struct.unpack('<H', b[i + 22:i + 24])[0]
assert (rate, bits) == (44100, 24), (rate, bits)
assert b'smpl' in b, 'loop points'
print('PASS: dialog export wrote', len(names), 'files at', rate, 'Hz', bits, 'bit, with smpl loop points')
"@ "$Work\export"
