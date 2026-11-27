# usage: at_polish.ps1 [-Exe .\target\release\cartwave.exe] [-Work <dir>] [-Only PL-01,PL-04]
# Polish pass scenarios (UX.md §6, PL-01…PL-05), the Batch 3 harness method: synthetic input
# into the real app, saved files and dev-mode stderr lines checked by check_polish.py.
# Client px at 100% (Option D): rows from 104 every 73 px, headers 241..620 with the browser
# open, lanes from 621 at 92 px per bar.
param([string]$Exe = ".\target\release\cartwave.exe",
  [string]$Work = (Join-Path $env:TEMP ("chipdaw-at-polish-" + (Get-Date -Format "MMdd-HHmmss"))),
  [string[]]$Only = @())
$global:Work = $Work
New-Item -ItemType Directory -Force $Work | Out-Null
. (Join-Path $PSScriptRoot "..\..\batch3\tools\lib.ps1")
Set-Location $Repo
$global:Exe = (Resolve-Path $Exe).Path
python "$Repo\docs\batch3\tools\fixtures.py" $Work | Out-Null
$obs = [ordered]@{ work = $Work }
$Only = @($Only | ForEach-Object { $_ -split "," } | Where-Object { $_ })
function Want($id) { $Only.Count -eq 0 -or $Only -contains $id }
function Fx($name, $as) { $p = Join-Path $Work "$as.chipdaw"; Copy-Item (Join-Path $Work "$name.chipdaw") $p -Force; $p }
function Keep($proj, $as) { Copy-Item $proj (Join-Path $Work "$as.chipdaw") -Force }
function DevLines($d, $pat) { @(StdErr $d | Where-Object { $_ -match $pat }) }
function Bx($b) { 621 + ($b - 1) * 92 }
function Ly($i) { 141 + 73 * $i }
# The colour grid's cell (row r, column c) for a submenu opened from a menu at (mx, my)
# whose "Colour" row is the n-th (0-based) row: the submenu sits at the menu's right edge.
function Cell($mx, $my, $n, $r, $c, $rowsAbove = 0) {
  $sx = $mx + 240 - 4; $sy = $my + $n * 28
  @(($sx + 4 + 8 + $c * 20 + 8), ($sy + 4 + $rowsAbove * 28 + 8 + $r * 20 + 8))
}

# ─────────── PL-01: a track's colour from the grid ───────────
if (Want "PL-01") {
  $p = Fx "fx_basic" "pl01"; $d = Join-Path $Work "d01"
  Launch -Project $p -Data $d
  $mx = 400; $my = Ly 2
  RightL $mx $my; Start-Sleep -Milliseconds 300
  MoveL ($mx + 30) ($my + 4 + 28 + 14); Start-Sleep -Milliseconds 500       # "Colour": the grid opens
  $c = Cell $mx $my 1 1 8                                                   # row 2, column 9
  MoveL $c[0] $c[1]; Start-Sleep -Milliseconds 400; Shot "pl01_hover"
  ClickL $c[0] $c[1]; Start-Sleep -Milliseconds 300
  $obs.pl01_title = Title
  Save; Keep $p "pl01_after"
  Ctrl 0x5A; Save; Keep $p "pl01_undone"
  $obs.pl01_menu = @(DevLines $d "^dev: menu Track")
  Stop-App
}

# ─────────── PL-02: two clips at once ───────────
if (Want "PL-02") {
  $p = Fx "fx_basic" "pl02"; $d = Join-Path $Work "d02"
  Launch -Project $p -Data $d
  ClickL (Bx 1.5) (Ly 1); ShiftClickL (Bx 3.5) (Ly 1)                       # Harm A + Harm B
  $mx = (Bx 1.5); $my = (Ly 1)
  RightL $mx $my; Start-Sleep -Milliseconds 300
  $obs.pl02_menu = @(DevLines $d "^dev: menu Clip")
  # The clip menu's "Colour" row: after Cut, Copy, Paste, Duplicate, Delete, a separator,
  # Split, Consolidate, Rename (9 rows + 8 px).
  $cy = $my + 4 + 8 * 28 + 8 + 14
  MoveL ($mx + 30) $cy; Start-Sleep -Milliseconds 500
  # The submenu: "Track Colour" (one row) above the grid.
  $sx = $mx + 240 - 4; $sy = $cy - 14 - 4
  ClickL ($sx + 4 + 8 + 3 * 20 + 8) ($sy + 4 + 28 + 8 + 2 * 20 + 8)        # row 3, column 4
  Save; Keep $p "pl02_after"
  # Clips keep their own colour when their track's changes (independent colours).
  $mx = 400; $my = Ly 1
  RightL $mx $my; Start-Sleep -Milliseconds 300
  MoveL ($mx + 30) ($my + 4 + 28 + 14); Start-Sleep -Milliseconds 500
  $c = Cell $mx $my 1 0 0                                                   # row 1, column 1
  ClickL $c[0] $c[1]; Start-Sleep -Milliseconds 300
  Save; Keep $p "pl02_track"
  Stop-App
}

# ─────────── PL-03: random colours for new clips ───────────
if (Want "PL-03") {
  $p = Fx "fx_basic" "pl03"; $d = Join-Path $Work "d03"
  New-Item -ItemType Directory -Force $d | Out-Null
  '{ "new_clip_colour": "random" }' | Set-Content -Encoding ascii (Join-Path $d "settings.json")
  Launch -Project $p -Data $d
  DragL (Bx 5) (Ly 4) (Bx 7) (Ly 4)                                         # bars 5-6 on the empty DPCM track
  CtrlShift 0x4D; Start-Sleep -Milliseconds 300                             # Insert MIDI Clip
  Save; Keep $p "pl03_after"
  Stop-App
}

# ─────────── PL-04: the lanes scroll in a short window ───────────
if (Want "PL-04") {
  $p = Fx "fx_basic" "pl04"; $d = Join-Path $Work "d04"
  Launch -Project $p -Data $d -AppArgs @("--size", "1536x816")
  ClickL (Bx 2) (Ly 0)                                                      # a clip: the inspector opens
  Start-Sleep -Milliseconds 400; Shot "pl04_before"
  MoveL 400 (Ly 2); Wheel -3; Start-Sleep -Milliseconds 400                 # over the headers
  Shot "pl04_after"
  $obs.pl04_scroll = @(DevLines $d "^dev: lane scroll")
  Stop-App
}

# ─────────── PL-05: the minimum window ───────────
if (Want "PL-05") {
  $d = Join-Path $Work "d05"
  Launch -Data $d -AppArgs @("--size", "800x600")
  Add-Type @"
using System; using System.Runtime.InteropServices;
public class Cl { [StructLayout(LayoutKind.Sequential)] public struct R { public int L, T, Rr, B; }
  [DllImport("user32.dll")] public static extern bool GetClientRect(IntPtr h, out R r);
  [DllImport("user32.dll")] public static extern uint GetDpiForWindow(IntPtr h); }
"@ -ErrorAction SilentlyContinue
  $r = New-Object Cl+R; [Cl]::GetClientRect($global:AppH, [ref]$r) | Out-Null
  $dpi = [Cl]::GetDpiForWindow($global:AppH)
  $obs.pl05_client = @($r.Rr, $r.B, $dpi)
  Shot "pl05_minimum"
  Stop-App
}

$obs | ConvertTo-Json -Depth 6 | Set-Content -Encoding utf8 (Join-Path $Work "obs.json")
python "$PSScriptRoot\check_polish.py" $Work
