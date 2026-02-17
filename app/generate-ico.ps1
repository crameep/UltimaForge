# Generate multi-resolution .ico file from PNG source
# This creates a proper Windows icon with multiple embedded PNG frames

param(
    [string]$SourceImage = "../branding/sidebar-logo.png",
    [string]$OutputIco = "src-tauri/icons/icon.ico"
)

Add-Type -AssemblyName System.Drawing

Write-Host "Generating icon.ico from $SourceImage..." -ForegroundColor Cyan

# Load source image
$sourceImg = [System.Drawing.Image]::FromFile((Resolve-Path $SourceImage))

# Icon sizes to include (standard Windows sizes)
$sizes = @(256, 128, 64, 48, 32, 16)

# Function to resize image
function Resize-Image {
    param([int]$size, [System.Drawing.Image]$source)

    $bitmap = New-Object System.Drawing.Bitmap($size, $size)
    $graphics = [System.Drawing.Graphics]::FromImage($bitmap)
    $graphics.InterpolationMode = [System.Drawing.Drawing2D.InterpolationMode]::HighQualityBicubic
    $graphics.SmoothingMode = [System.Drawing.Drawing2D.SmoothingMode]::HighQuality
    $graphics.CompositingQuality = [System.Drawing.Drawing2D.CompositingQuality]::HighQuality
    $graphics.DrawImage($source, 0, 0, $size, $size)
    $graphics.Dispose()

    return $bitmap
}

# Function to save bitmap as PNG to memory
function Get-PngBytes {
    param([System.Drawing.Bitmap]$bitmap)

    $ms = New-Object System.IO.MemoryStream
    $bitmap.Save($ms, [System.Drawing.Imaging.ImageFormat]::Png)
    $bytes = $ms.ToArray()
    $ms.Dispose()

    return $bytes
}

# Create ICO file
$icoStream = New-Object System.IO.FileStream($OutputIco, [System.IO.FileMode]::Create)
$writer = New-Object System.IO.BinaryWriter($icoStream)

try {
    # Write ICO header
    $writer.Write([UInt16]0)           # Reserved (must be 0)
    $writer.Write([UInt16]1)           # Type (1 = icon)
    $writer.Write([UInt16]$sizes.Count) # Number of images

    # Prepare image data
    $imageDataList = @()

    foreach ($size in $sizes) {
        Write-Host "  Creating ${size}x${size}..." -NoNewline

        $resized = Resize-Image -size $size -source $sourceImg
        $pngBytes = Get-PngBytes -bitmap $resized
        $resized.Dispose()

        $imageDataList += @{
            Size = $size
            Data = $pngBytes
        }

        Write-Host " OK ($($pngBytes.Length) bytes)" -ForegroundColor Green
    }

    # Calculate offset for first image data (header + all directory entries)
    $dataOffset = 6 + ($sizes.Count * 16)

    # Write directory entries
    foreach ($imgData in $imageDataList) {
        $size = $imgData.Size
        $data = $imgData.Data

        # ICO directory entry
        $widthByte = if ($size -eq 256) { 0 } else { $size }
        $heightByte = if ($size -eq 256) { 0 } else { $size }

        $writer.Write([byte]$widthByte)                    # Width (0 means 256)
        $writer.Write([byte]$heightByte)                   # Height (0 means 256)
        $writer.Write([byte]0)                             # Color palette (0 = no palette)
        $writer.Write([byte]0)                             # Reserved
        $writer.Write([UInt16]1)                           # Color planes
        $writer.Write([UInt16]32)                          # Bits per pixel (32 = RGBA)
        $writer.Write([UInt32]$data.Length)                # Size of image data
        $writer.Write([UInt32]$dataOffset)                 # Offset to image data

        $dataOffset += $data.Length
    }

    # Write image data
    foreach ($imgData in $imageDataList) {
        $icoStream.Write($imgData.Data, 0, $imgData.Data.Length)
    }

    Write-Host ""
    Write-Host "Successfully created icon.ico with $($sizes.Count) sizes" -ForegroundColor Green
    Write-Host "Total file size: $($icoStream.Length) bytes" -ForegroundColor Green

} finally {
    $writer.Close()
    $icoStream.Close()
    $sourceImg.Dispose()
}
