Add-Type -AssemblyName System.Drawing

$logoPath = 'C:\Users\Rishabh Kumar\.gemini\antigravity\brain\649d802d-14da-4412-ba4f-48f5aff8b3b9\.user_uploaded\media_1793552625287_668c8e10.png'
$xdnotPath = 'C:\Users\Rishabh Kumar\.gemini\antigravity\brain\649d802d-14da-4412-ba4f-48f5aff8b3b9\.user_uploaded\media_1793552625249_8b1bdefb.png'

# 1. Copy XDNOT-Labs logo
Copy-Item -Path $xdnotPath -Destination 'assets\xdnotlabs.png' -Force
Write-Host "Copied assets\xdnotlabs.png"

$srcBmp = [System.Drawing.Bitmap]::FromFile($logoPath)

function Resize-Bitmap([System.Drawing.Bitmap]$source, [int]$width, [int]$height) {
    $dest = New-Object System.Drawing.Bitmap($width, $height, [System.Drawing.Imaging.PixelFormat]::Format32bppArgb)
    $g = [System.Drawing.Graphics]::FromImage($dest)
    $g.InterpolationMode = [System.Drawing.Drawing2D.InterpolationMode]::HighQualityBicubic
    $g.SmoothingMode = [System.Drawing.Drawing2D.SmoothingMode]::HighQuality
    $g.PixelOffsetMode = [System.Drawing.Drawing2D.PixelOffsetMode]::HighQuality
    $g.CompositingQuality = [System.Drawing.Drawing2D.CompositingQuality]::HighQuality
    $g.Clear([System.Drawing.Color]::Transparent)
    $g.DrawImage($source, 0, 0, $width, $height)
    $g.Dispose()
    return $dest
}

# 2. Save 256x256 PNGs
$bmp256 = Resize-Bitmap $srcBmp 256 256
$bmp256.Save('assets\nowbar-256.png', [System.Drawing.Imaging.ImageFormat]::Png)
$bmp256.Save('assets\swift-tab-logo.png', [System.Drawing.Imaging.ImageFormat]::Png)
$bmp256.Save('ui\syro-mark.png', [System.Drawing.Imaging.ImageFormat]::Png)
Write-Host "Saved assets\nowbar-256.png, assets\swift-tab-logo.png, ui\syro-mark.png"

# 3. Save 64x64 raw BGRA
$bmp64 = Resize-Bitmap $srcBmp 64 64
$bgraBytes = New-Object byte[] (64 * 64 * 4)
$idx = 0
for ($y = 0; $y -lt 64; $y++) {
    for ($x = 0; $x -lt 64; $x++) {
        $pixel = $bmp64.GetPixel($x, $y)
        $bgraBytes[$idx]     = $pixel.B
        $bgraBytes[$idx + 1] = $pixel.G
        $bgraBytes[$idx + 2] = $pixel.R
        $bgraBytes[$idx + 3] = $pixel.A
        $idx += 4
    }
}
[System.IO.File]::WriteAllBytes('assets\tray-nobg-64.bgra', $bgraBytes)
Write-Host "Saved assets\tray-nobg-64.bgra (length $($bgraBytes.Length))"
$bmp64.Dispose()

# 4. Generate .ico with PNG frames
function Build-Ico([System.Drawing.Bitmap]$source, [string]$outIcoPath) {
    $sizes = @(16, 20, 24, 32, 40, 48, 64, 96, 128, 256)
    $pngBuffers = New-Object System.Collections.ArrayList
    
    foreach ($sz in $sizes) {
        $resized = Resize-Bitmap $source $sz $sz
        $ms = New-Object System.IO.MemoryStream
        $resized.Save($ms, [System.Drawing.Imaging.ImageFormat]::Png)
        $null = $pngBuffers.Add(@($sz, $ms.ToArray()))
        $resized.Dispose()
        $ms.Dispose()
    }
    
    $fs = [System.IO.File]::Open($outIcoPath, [System.IO.FileMode]::Create, [System.IO.FileAccess]::Write)
    $writer = New-Object System.IO.BinaryWriter($fs)
    
    # ICONDIR
    $writer.Write([uint16]0) # Reserved
    $writer.Write([uint16]1) # Type: 1 = Icon
    $writer.Write([uint16]$pngBuffers.Count) # Count
    
    # Calculate offset
    $offset = 6 + ($pngBuffers.Count * 16)
    
    # ICONDIRENTRY for each
    foreach ($item in $pngBuffers) {
        $sz = [int]$item[0]
        $bytes = [byte[]]$item[1]
        $bW = if ($sz -ge 256) { [byte]0 } else { [byte]$sz }
        $bH = if ($sz -ge 256) { [byte]0 } else { [byte]$sz }
        $writer.Write($bW)
        $writer.Write($bH)
        $writer.Write([byte]0) # Color count
        $writer.Write([byte]0) # Reserved
        $writer.Write([uint16]1) # Planes
        $writer.Write([uint16]32) # Bit count
        $writer.Write([uint32]$bytes.Length) # Bytes in res
        $writer.Write([uint32]$offset) # Offset
        $offset += $bytes.Length
    }
    
    # Write image data
    foreach ($item in $pngBuffers) {
        $bytes = [byte[]]$item[1]
        $writer.Write($bytes)
    }
    
    $writer.Flush()
    $writer.Close()
    $fs.Close()
    Write-Host "Created $outIcoPath"
}

Build-Ico $srcBmp 'assets\nowbar.ico'
Build-Ico $srcBmp 'assets\nowbar-nobg.ico'

$bmp256.Dispose()
$srcBmp.Dispose()
Write-Host "Icon generation complete!"
