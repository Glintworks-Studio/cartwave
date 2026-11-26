# usage: at_p2.ps1 [-Exe .\target\release\app.exe] [-Work <dir>] [-Only P2-01,P2-11]
# Polish 2 scenarios (UX.md §6, P2-01…P2-11), the Batch 3 harness method: synthetic input into
# the real app (default settings: the Overview and the status bar shown), saved files,
# settings.json and dev-mode stderr lines, checked by check_p2.py.
# Client px at 100%: top bar 76, Overview 76..108, ruler 108..136, rows from 136 every 73 px;
# browser 0..241; headers 241..621 (left) or 1060..1440 (right); lanes 621.. at 92 px per bar.
param([string]$Exe = ".\target\release\app.exe",
  [string]$Work = (Join-Path $env:TEMP ("chipdaw-at-p2-" + (Get-Date -Format "MMdd-HHmmss"))),
  [string[]]$Only = @())
$global:Work = $Work
New-Item -ItemType Directory -Force $Work | Out-Null
. (Join-Path $PSScriptRoot "..\..\batch3\tools\lib.ps1")
Set-Location $Repo
$global:Exe = (Resolve-Path $Exe).Path
python "$Repo\docs\batch3\tools\fixtures.py" $Work | Out-Null
foreach ($k in "CHIPDAW_OVERVIEW", "CHIPDAW_HIDE_STATUS", "CHIPDAW_FAKE_PROBLEM") { Remove-Item "Env:$k" -ErrorAction SilentlyContinue }
$obs = [ordered]@{ work = $Work }
# Clicks and drags settle on their spot first (hover state), so a click right after a layout
# change (a width drag, a settings change) isn't lost.
function ClickL($x, $y) { MoveL $x $y; Start-Sleep -Milliseconds 150; Down; Up; Start-Sleep -Milliseconds 250 }
function DragL($x0, $y0, $x1, $y1) { MoveL $x0 $y0; Start-Sleep -Milliseconds 150; $a = P $x0 $y0; $b = P $x1 $y1; Down; for ($i = 1; $i -le 16; $i++) { Move-To ([int]($a[0] + ($b[0] - $a[0]) * $i / 16)) ([int]($a[1] + ($b[1] - $a[1]) * $i / 16)); Start-Sleep -Milliseconds 15 }; Up; Start-Sleep -Milliseconds 300 }
$Only = @($Only | ForEach-Object { $_ -split "," } | Where-Object { $_ })
function Want($id) { $Only.Count -eq 0 -or $Only -contains $id }
function Fx($name, $as) { $p = Join-Path $Work "$as.chipdaw"; Copy-Item (Join-Path $Work "$name.chipdaw") $p -Force; $p }
function Keep($proj, $as) { Copy-Item $proj (Join-Path $Work "$as.chipdaw") -Force }
function KeepSettings($d, $as) { Copy-Item (Join-Path $d "settings.json") (Join-Path $Work "$as.settings.json") -Force -ErrorAction SilentlyContinue }
function DevLines($d, $pat) { @(StdErr $d | Where-Object { $_ -match $pat }) }
function Settings($d, $json) { New-Item -ItemType Directory -Force $d | Out-Null; $json | Set-Content -Encoding ascii (Join-Path $d "settings.json") }

# Types into a text field: digits, letters, '-', '+', '.', ' ' (US layout).
function TypeKeys($t) {
  foreach ($ch in $t.ToCharArray()) {
    switch ($ch) {
      '-' { Key 0xBD }
      '+' { KeyDown 0x10; Key 0xBB; KeyUp 0x10 }
      '.' { Key 0xBE }
      ' ' { Key 0x20 }
      default { $vk = [int][char]::ToUpper($ch); if ([char]::IsUpper($ch)) { KeyDown 0x10; Key $vk; KeyUp 0x10 } else { Key $vk } }
    }
  }
  Start-Sleep -Milliseconds 200
}
function SelectAllType($t) { Ctrl 0x41; TypeKeys $t }

# The header's geometry (header_layout.rs), in client px for a layout.
$RowsTop = 136; $Pitch = 73
function RowY($i) { $RowsTop + $Pitch * $i }
function Lay($right, $byClips, $w = 380) {
  $hl = if ($right) { 1440 - $w } else { 241 }
  $lanesL = if ($right) { 241 } else { 241 + $w }
  $mir = ($right -ne $byClips)
  $ctrl = if ($mir) { 12 } else { $w - 132 }
  $ca = $hl + $ctrl + 8
  [pscustomobject]@{ hl = $hl; w = $w; lanes = $lanesL; colA = $ca; colB = $ca + 72; mir = $mir; right = $right
    edge = $(if ($right) { $hl + 1 } else { $hl + $w - 2 }) }
}
function KeyM($L, $i) { @(($L.colA + 12), ((RowY $i) + 22)) }
function SKey($L, $i) { @(($L.colA + 44), ((RowY $i) + 22)) }
function DbVal($L, $i) { @(($L.colB + 20), ((RowY $i) + 58)) }
function PanVal($L, $i) { @(($L.colA + 28), ((RowY $i) + 48)) }
function Well($L, $i, $y) { @(($L.colB + 20), ((RowY $i) + 8 + $y)) }
function Edge($L, $i) { @(($L.hl + 200), ((RowY $i) + 71)) }

# ─────────── P2-01: type a volume (dB) ───────────
if (Want "P2-01") {
  $p = Fx "fx_basic" "p201"; $d = Join-Path $Work "d201"
  Launch -Project $p -Data $d
  $L = Lay $false $false
  $v = DbVal $L 0
  $obs.p201_title0 = Title
  DblL $v[0] $v[1]; SelectAllType "-3"; K 0x0D; $obs.p201_title = Title; Save; Keep $p "p201_typed"
  Ctrl 0x5A; Save; Keep $p "p201_undone"
  DblL $v[0] $v[1]; SelectAllType "-9"; K 0x1B; Save; Keep $p "p201_esc"
  DblL $v[0] $v[1]; SelectAllType "-inf"; K 0x0D; Save; Keep $p "p201_inf"
  $w = DbVal $L 1
  DblL $w[0] $w[1]; SelectAllType "+2.5"; ClickL 900 820; Save; Keep $p "p201_away"
  Shot "p201"
  Stop-App
}

# ─────────── P2-02: type a pan value ───────────
if (Want "P2-02") {
  $p = Fx "fx_basic" "p202"; $d = Join-Path $Work "d202"
  Launch -Project $p -Data $d
  $L = Lay $false $false
  $v = PanVal $L 1
  DblL $v[0] $v[1]; SelectAllType "25L"; K 0x0D; Save; Keep $p "p202_left"
  DblL $v[0] $v[1]; SelectAllType "30R"; K 0x0D; Save; Keep $p "p202_right"
  Ctrl 0x5A; Save; Keep $p "p202_undone"
  $v = PanVal $L 2
  DblL $v[0] $v[1]; SelectAllType "-12"; K 0x0D; Save; Keep $p "p202_minus"
  Stop-App
}

# ─────────── P2-03: fader and pan drags, one undo step each ───────────
if (Want "P2-03") {
  $p = Fx "fx_basic" "p203"; $d = Join-Path $Work "d203"
  Launch -Project $p -Data $d
  $L = Lay $false $false
  $a = Well $L 0 4; DragL $a[0] $a[1] $a[0] ($a[1] + 20); Save; Keep $p "p203_fader"
  Ctrl 0x5A; Save; Keep $p "p203_undone"
  $x0 = $L.colA + 28; $y = (RowY 0) + 60
  DragL $x0 $y ($x0 + 14) $y; Save; Keep $p "p203_pan"
  Stop-App
}

# ─────────── P2-04: row heights (edge drag, Alt+wheel) ───────────
if (Want "P2-04") {
  $p = Fx "fx_basic" "p204"; $d = Join-Path $Work "d204"
  Launch -Project $p -Data $d
  $L = Lay $false $false
  $e = Edge $L 0; DragL $e[0] $e[1] $e[0] ($e[1] + 72); Save; Keep $p "p204_edge"
  Shot "p204_edge"
  # Row 2 now starts 72 px lower.
  MoveL ($L.hl + 120) ((RowY 1) + 72 + 30); KeyDown 0x12; Start-Sleep -Milliseconds 150; Wheel 1; Start-Sleep -Milliseconds 300; KeyUp 0x12; Start-Sleep -Milliseconds 200; K 0x1B
  Save; Keep $p "p204_wheel"
  Ctrl 0x5A; Save; Keep $p "p204_undone"
  $obs.p204_dev = @(DevLines $d "^dev: row height")
  Stop-App
}

# ─────────── P2-05: header width (the boundary), saved per user ───────────
if (Want "P2-05") {
  $p = Fx "fx_basic" "p205"; $d = Join-Path $Work "d205"
  Launch -Project $p -Data $d
  $L = Lay $false $false
  DragL $L.edge 400 ($L.edge + 140) 400; Start-Sleep -Milliseconds 300
  Shot "p205_520"
  DragL ($L.edge + 140) 400 ($L.edge + 400) 400; Start-Sleep -Milliseconds 300
  KeepSettings $d "p205_max"
  DragL ($L.edge + 220) 400 ($L.edge - 200) 400; Start-Sleep -Milliseconds 300
  KeepSettings $d "p205_min"
  DragL $L.edge 400 ($L.edge + 140) 400; Start-Sleep -Milliseconds 300
  KeepSettings $d "p205_520"
  Stop-App
  Launch -Project $p -Data $d
  Shot "p205_restart"
  $obs.p205_dev = @(DevLines $d "^dev: look")
  Stop-App
}

# ─────────── P2-06: Settings > Look & Feel ───────────
if (Want "P2-06") {
  $p = Fx "fx_basic" "p206"; $d = Join-Path $Work "d206"
  Launch -Project $p -Data $d
  Ctrl 0xBC; Start-Sleep -Milliseconds 600
  ClickL 611 160; Start-Sleep -Milliseconds 500                             # the Look & Feel tab
  Shot "p206_tab"
  ClickL 673 201; Start-Sleep -Milliseconds 600; KeepSettings $d "p206_right"   # Track headers: Right
  ClickL 722 233; Start-Sleep -Milliseconds 600; KeepSettings $d "p206_byclips" # Colour strip: By the clips
  ClickL 673 307; Start-Sleep -Milliseconds 600; KeepSettings $d "p206_browser" # Browser: Right
  K 0x1B; Start-Sleep -Milliseconds 500
  Shot "p206_after"
  $obs.p206_dev = @(DevLines $d "^dev: look")
  Stop-App
}

# ─────────── P2-07: the ruler (click, zoom, scroll, double-click) ───────────
if (Want "P2-07") {
  $p = Fx "fx_basic" "p207"; $d = Join-Path $Work "d207"
  Launch -Project $p -Data $d
  ClickL 900 122; Start-Sleep -Milliseconds 700
  DragL 900 122 900 182; Start-Sleep -Milliseconds 700
  DragL 1100 122 900 122; Start-Sleep -Milliseconds 700
  DblL 900 122
  MoveL 1000 124; Start-Sleep -Milliseconds 300
  $obs.p207_dev = @(DevLines $d "^dev: (ruler|zoom|help Ruler)")
  Stop-App
}

# ─────────── P2-08: W fits the song, again goes back ───────────
if (Want "P2-08") {
  $p = Fx "fx_basic" "p208"; $d = Join-Path $Work "d208"
  Launch -Project $p -Data $d
  ClickL 900 800; K 0x57; K 0x57
  $obs.p208_dev = @(DevLines $d "^dev: zoom")
  Stop-App
}

# ─────────── P2-09: the Overview ───────────
if (Want "P2-09") {
  $p = Fx "fx_basic" "p209"; $d = Join-Path $Work "d209"
  Launch -Project $p -Data $d
  ClickL 1300 92; Start-Sleep -Milliseconds 700
  DragL 700 92 900 92; Start-Sleep -Milliseconds 700
  DragL 800 92 800 140; Start-Sleep -Milliseconds 700
  DblL 800 92
  MoveL 1000 92; Start-Sleep -Milliseconds 300
  Shot "p209_on"
  KeyDown 0x11; KeyDown 0x12; Key 0x4F; KeyUp 0x12; KeyUp 0x11; Start-Sleep -Milliseconds 600
  Shot "p209_off"; KeepSettings $d "p209_off"
  KeyDown 0x11; KeyDown 0x12; Key 0x4F; KeyUp 0x12; KeyUp 0x11; Start-Sleep -Milliseconds 600
  KeepSettings $d "p209_on"
  $obs.p209_dev = @(DevLines $d "^dev: (overview|zoom|look|help Overview)")
  Stop-App
}

# ─────────── P2-10: the status bar ───────────
if (Want "P2-10") {
  $p = Fx "fx_basic" "p210"; $d = Join-Path $Work "d210"
  $env:CHIPDAW_FAKE_PROBLEM = "Audio dropouts: 3 buffer underruns so far. A larger buffer (Settings > Audio) helps."
  Launch -Project $p -Data $d
  $L = Lay $false $false
  $f = Well $L 0 20; MoveL $f[0] $f[1]; Start-Sleep -Milliseconds 400
  $m = KeyM $L 0; MoveL $m[0] $m[1]; Start-Sleep -Milliseconds 300
  $pv = PanVal $L 0; MoveL $pv[0] ($pv[1] + 12); Start-Sleep -Milliseconds 300
  foreach ($pt in @(@(36, 38), @(150, 38), @(197, 38), @(246, 38), @(293, 38), @(600, 38), @(1000, 38), @(1220, 38), @(1380, 38), @(900, 122), @(900, 92))) {
    MoveL $pt[0] $pt[1]; Start-Sleep -Milliseconds 300
  }
  ClickL 900 800; Ctrl 0x53; Start-Sleep -Milliseconds 300
  $f = Well $L 0 20; MoveL $f[0] $f[1]; Start-Sleep -Milliseconds 400
  Shot "p210_status"
  ClickL 22 908; Start-Sleep -Milliseconds 500
  Shot "p210_problems"
  K 0x1B
  $obs.p210_dev = @(DevLines $d "^dev: (help|menu Problems)")
  Remove-Item Env:CHIPDAW_FAKE_PROBLEM
  Stop-App
}

# ─────────── P2-11: hit tests in the four layouts ───────────
if (Want "P2-11") {
  $n = 0
  foreach ($lay in @(@($false, $false, "lo"), @($false, $true, "lc"), @($true, $true, "rc"), @($true, $false, "ro"))) {
    $right, $by, $tag = $lay
    $p = Fx "fx_basic" "p211_$tag"; $d = Join-Path $Work "d211_$tag"
    Settings $d ("{ ""headers_right"": " + "$right".ToLower() + ", ""strip_by_clips"": " + "$by".ToLower() + " }")
    Launch -Project $p -Data $d
    $L = Lay $right $by
    function Gap { Start-Sleep -Milliseconds 700 }
    $c = KeyM $L 0; ClickL $c[0] $c[1]; Gap
    $c = SKey $L 1; ClickL $c[0] $c[1]; Gap
    $c = DbVal $L 2; DblL $c[0] $c[1]; SelectAllType "-6"; K 0x0D; Gap
    $c = PanVal $L 3; DblL $c[0] $c[1]; SelectAllType "10R"; K 0x0D; Gap
    $c = Well $L 4 20; ClickL $c[0] $c[1]; Gap
    # The name: a double-click opens the rename field over it.
    $nx = if ($L.mir) { $L.hl + 160 } else { $L.hl + 90 }
    DblL $nx ((RowY 6) + 22); SelectAllType "Hit"; K 0x0D; Gap
    $c = Edge $L 5; DragL $c[0] $c[1] $c[0] ($c[1] + 24); Gap
    Save; Keep $p "p211_${tag}_after"
    $dir = if ($right) { -40 } else { 40 }
    DragL $L.edge 500 ($L.edge + $dir) 500; Gap; KeepSettings $d "p211_$tag"
    ClickL ($L.lanes + 200) 122; Gap
    ClickL ($L.lanes + 700) 92; Gap
    Shot "p211_$tag"
    $obs["p211_${tag}_dev"] = @(DevLines $d "^dev: (ruler|overview|look|row height)")
    Stop-App
  }
}

$obs | ConvertTo-Json -Depth 5 | Set-Content -Encoding utf8 (Join-Path $Work "obs.json")
python "$Repo\docs\polish2\tools\check_p2.py" $Work
