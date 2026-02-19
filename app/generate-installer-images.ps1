# Generate NSIS installer branding images from brand assets
# Creates header and sidebar images for the installer

param(
    [string]$LogoPath = "../branding/sidebar-logo.png",
    [string]$BrandConfig = "../branding/brand.json",
    [string]$OutputDir = "src-tauri/installer-assets"
)

Add-Type -AssemblyName System.Drawing

Write-Host "Generating NSIS installer images..." -ForegroundColor Cyan
Write-Host ""

# Load brand colors from brand.json
$brandData = Get-Content $BrandConfig | ConvertFrom-Json
$primaryColor = $brandData.theme.colors.primary
$bgColor = $brandData.theme.colors.background

# Parse hex colors
function Parse-HexColor {
    param([string]$hex)
    $hex = $hex.TrimStart('#')
    $r = [Convert]::ToInt32($hex.Substring(0,2), 16)
    $g = [Convert]::ToInt32($hex.Substring(2,2), 16)
    $b = [Convert]::ToInt32($hex.Substring(4,2), 16)
    return [System.Drawing.Color]::FromArgb($r, $g, $b)
}

$primaryBrush = New-Object System.Drawing.SolidBrush(Parse-HexColor $primaryColor)
$bgBrush = New-Object System.Drawing.SolidBrush(Parse-HexColor $bgColor)

# Load logo
$logo = [System.Drawing.Image]::FromFile((Resolve-Path $LogoPath))

# Create output directory
if (!(Test-Path $OutputDir)) {
    New-Item -ItemType Directory -Path $OutputDir | Out-Null
}

# === NSIS Header Image (150x57) ===
Write-Host "Creating NSIS header (150x57)..." -NoNewline

$headerWidth = 150
$headerHeight = 57
$header = New-Object System.Drawing.Bitmap($headerWidth, $headerHeight)
$graphics = [System.Drawing.Graphics]::FromImage($header)
$graphics.SmoothingMode = [System.Drawing.Drawing2D.SmoothingMode]::AntiAlias

# Fill background with primary color
$graphics.FillRectangle($primaryBrush, 0, 0, $headerWidth, $headerHeight)

# Draw logo (small, centered vertically, on the left)
$logoSize = 40
$logoX = 8
$logoY = ($headerHeight - $logoSize) / 2
$graphics.DrawImage($logo, $logoX, $logoY, $logoSize, $logoSize)

$graphics.Dispose()

# Save as BMP
$headerPath = "$OutputDir/nsis-header.bmp"
$header.Save($headerPath, [System.Drawing.Imaging.ImageFormat]::Bmp)
$header.Dispose()

Write-Host " OK" -ForegroundColor Green
Write-Host "  Saved: $headerPath"

# === NSIS Sidebar Image (164x314) ===
Write-Host "Creating NSIS sidebar (164x314)..." -NoNewline

$sidebarWidth = 164
$sidebarHeight = 314
$sidebar = New-Object System.Drawing.Bitmap($sidebarWidth, $sidebarHeight)
$graphics = [System.Drawing.Graphics]::FromImage($sidebar)
$graphics.SmoothingMode = [System.Drawing.Drawing2D.SmoothingMode]::AntiAlias
$graphics.CompositingQuality = [System.Drawing.Drawing2D.CompositingQuality]::HighQuality

# Fill background with gradient (primary to darker)
$darkPrimary = [System.Drawing.Color]::FromArgb(
    [Math]::Max(0, (Parse-HexColor $primaryColor).R - 40),
    [Math]::Max(0, (Parse-HexColor $primaryColor).G - 40),
    [Math]::Max(0, (Parse-HexColor $primaryColor).B - 40)
)
$gradient = New-Object System.Drawing.Drawing2D.LinearGradientBrush(
    (New-Object System.Drawing.Point(0, 0)),
    (New-Object System.Drawing.Point(0, $sidebarHeight)),
    (Parse-HexColor $primaryColor),
    $darkPrimary
)
$graphics.FillRectangle($gradient, 0, 0, $sidebarWidth, $sidebarHeight)

# Draw logo (centered, larger)
$logoSize = 120
$logoX = ($sidebarWidth - $logoSize) / 2
$logoY = 40
$graphics.DrawImage($logo, $logoX, $logoY, $logoSize, $logoSize)

$graphics.Dispose()
$gradient.Dispose()

# Save as BMP
$sidebarPath = "$OutputDir/nsis-sidebar.bmp"
$sidebar.Save($sidebarPath, [System.Drawing.Imaging.ImageFormat]::Bmp)
$sidebar.Dispose()

Write-Host " OK" -ForegroundColor Green
Write-Host "  Saved: $sidebarPath"

# Cleanup
$logo.Dispose()
$primaryBrush.Dispose()
$bgBrush.Dispose()

Write-Host ""
Write-Host "Installer images generated successfully!" -ForegroundColor Green
Write-Host ""
Write-Host "Next: Run build (option 7) to create branded installer" -ForegroundColor Yellow
