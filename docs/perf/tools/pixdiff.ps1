# usage: pixdiff.ps1 -A <png> -B <png> [-DiffOut <png>]
param([string]$A, [string]$B, [string]$DiffOut = "")
# Pixel diff of two screenshots, excluding the title bar (32 px) and the caret-blink field area is up to caller.
Add-Type -AssemblyName System.Drawing
$imgA = [System.Drawing.Bitmap]::FromFile($A); $imgB = [System.Drawing.Bitmap]::FromFile($B)
if ($imgA.Width -ne $imgB.Width -or $imgA.Height -ne $imgB.Height) { "size mismatch $($imgA.Width)x$($imgA.Height) vs $($imgB.Width)x$($imgB.Height)"; exit 1 }
$rect = New-Object System.Drawing.Rectangle 0, 0, $imgA.Width, $imgA.Height
$da = $imgA.LockBits($rect, 'ReadOnly', 'Format32bppArgb'); $db = $imgB.LockBits($rect, 'ReadOnly', 'Format32bppArgb')
$n = $da.Stride * $imgA.Height
$ba = New-Object byte[] $n; $bb = New-Object byte[] $n
[Runtime.InteropServices.Marshal]::Copy($da.Scan0, $ba, 0, $n); [Runtime.InteropServices.Marshal]::Copy($db.Scan0, $bb, 0, $n)
$imgA.UnlockBits($da); $imgB.UnlockBits($db)
$diffPx = 0; $maxd = 0; $sum = 0
$start = $da.Stride * 32
for ($i = $start; $i -lt $n; $i += 4) {
  $d = [math]::Max([math]::Max([math]::Abs($ba[$i] - $bb[$i]), [math]::Abs($ba[$i+1] - $bb[$i+1])), [math]::Abs($ba[$i+2] - $bb[$i+2]))
  if ($d -gt 0) { $diffPx++; $sum += $d; if ($d -gt $maxd) { $maxd = $d } }
}
"pixels differing: $diffPx of $(($n - $start) / 4)  max channel diff: $maxd  mean diff (of differing): {0:N2}" -f $(if ($diffPx) { $sum / $diffPx } else { 0 })

