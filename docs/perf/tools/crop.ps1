# usage: crop.ps1 -In <png> -Out <png> -X <x> -Y <y> -W <w> -H <h> [-Zoom 2]
# Nearest-neighbour zoomed crop of a screenshot, for inspecting edges and shadows.
param([string]$In, [string]$Out, [int]$X, [int]$Y, [int]$W, [int]$H, [int]$Zoom = 2)
Add-Type -AssemblyName System.Drawing
$src = [System.Drawing.Image]::FromFile($In)
$bmp = New-Object System.Drawing.Bitmap ($W * $Zoom), ($H * $Zoom)
$g = [System.Drawing.Graphics]::FromImage($bmp)
$g.InterpolationMode = [System.Drawing.Drawing2D.InterpolationMode]::NearestNeighbor
$g.PixelOffsetMode = [System.Drawing.Drawing2D.PixelOffsetMode]::Half
$g.DrawImage($src, (New-Object System.Drawing.Rectangle 0, 0, ($W * $Zoom), ($H * $Zoom)), (New-Object System.Drawing.Rectangle $X, $Y, $W, $H), [System.Drawing.GraphicsUnit]::Pixel)
$bmp.Save($Out, [System.Drawing.Imaging.ImageFormat]::Png)
$src.Dispose()
