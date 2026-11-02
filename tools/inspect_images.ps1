Add-Type -AssemblyName System.Drawing

$p1 = 'C:\Users\Rishabh Kumar\.gemini\antigravity\brain\649d802d-14da-4412-ba4f-48f5aff8b3b9\.user_uploaded\media_1793552625287_668c8e10.png'
$img1 = [System.Drawing.Bitmap]::FromFile($p1)
Write-Host "Img1 (Logo): $($img1.Width) x $($img1.Height), PixelFormat: $($img1.PixelFormat)"
$c0 = $img1.GetPixel(0, 0)
Write-Host "Img1 (0,0): A=$($c0.A), R=$($c0.R), G=$($c0.G), B=$($c0.B)"
$img1.Dispose()

$p2 = 'C:\Users\Rishabh Kumar\.gemini\antigravity\brain\649d802d-14da-4412-ba4f-48f5aff8b3b9\.user_uploaded\media_1793552625249_8b1bdefb.png'
$img2 = [System.Drawing.Bitmap]::FromFile($p2)
Write-Host "Img2 (XDNOT): $($img2.Width) x $($img2.Height), PixelFormat: $($img2.PixelFormat)"
$c2 = $img2.GetPixel(0, 0)
Write-Host "Img2 (0,0): A=$($c2.A), R=$($c2.R), G=$($c2.G), B=$($c2.B)"
$img2.Dispose()
