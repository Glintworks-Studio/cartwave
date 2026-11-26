# usage: . .\pixels.ps1   (dot-source)
# Pixel checks on screenshots with System.Drawing (no Python imaging needed).
#   Pixel-Diff a.png b.png      -> the number of pixels that differ (any channel)
#   Mean-Luma a.png x0 y0 x1 y1 -> the mean luminance (0-255) of a rectangle
Add-Type -AssemblyName System.Drawing

function Read-Pixels([string]$Path) {
  $img = [System.Drawing.Bitmap]::FromFile($Path)
  $rect = New-Object System.Drawing.Rectangle 0, 0, $img.Width, $img.Height
  $d = $img.LockBits($rect, 'ReadOnly', 'Format32bppArgb')
  $n = $d.Stride * $img.Height
  $b = New-Object byte[] $n
  [Runtime.InteropServices.Marshal]::Copy($d.Scan0, $b, 0, $n)
  $img.UnlockBits($d)
  $r = @{ W = $img.Width; H = $img.Height; Stride = $d.Stride; Bytes = $b }
  $img.Dispose()
  $r
}

# -Skip x0,y0,x1,y1: a region left out (a live read-out, e.g. MEM).
function Pixel-Diff([string]$PathA, [string]$PathB, [int[]]$Skip = @()) {
  # (PowerShell names are case-insensitive: $pa/$pb, not $a/$A.)
  $pa = Read-Pixels $PathA; $pb = Read-Pixels $PathB
  if ($pa.W -ne $pb.W -or $pa.H -ne $pb.H) { return -1 }
  $x = $pa.Bytes; $y = $pb.Bytes
  $diff = 0
  for ($i = 0; $i -lt $x.Length; $i += 4) {
    if ($x[$i] -ne $y[$i] -or $x[$i + 1] -ne $y[$i + 1] -or $x[$i + 2] -ne $y[$i + 2]) {
      if ($Skip.Count -eq 4) {
        $px = ($i / 4) % $pa.W; $py = [math]::Floor(($i / 4) / $pa.W)
        if ($px -ge $Skip[0] -and $px -lt $Skip[2] -and $py -ge $Skip[1] -and $py -lt $Skip[3]) { continue }
      }
      $diff++
    }
  }
  $diff
}

function Mean-Luma([string]$Path, [int]$X0, [int]$Y0, [int]$X1, [int]$Y1) {
  $p = Read-Pixels $Path
  $sum = 0.0; $n = 0
  for ($y = $Y0; $y -lt $Y1; $y++) {
    for ($x = $X0; $x -lt $X1; $x++) {
      $o = $y * $p.Stride + $x * 4
      $sum += 0.114 * $p.Bytes[$o] + 0.587 * $p.Bytes[$o + 1] + 0.299 * $p.Bytes[$o + 2]
      $n++
    }
  }
  [math]::Round($sum / [math]::Max($n, 1), 2)
}
