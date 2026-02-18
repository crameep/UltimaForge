param(
    [string]$Version,
    [string]$Target = "windows",
    [string]$Arch = "x86_64",
    [string]$BinaryPath,
    [string]$Signature,
    [string]$SignatureFile,
    [string]$Notes,
    [string]$NotesFile,
    [string]$BaseUrl = "http://localhost:8080",
    [string]$OutputDir
)

function Read-RequiredValue {
    param(
        [string]$Prompt,
        [string]$Current
    )
    if ($Current -and $Current.Trim().Length -gt 0) {
        return $Current
    }
    return (Read-Host $Prompt)
}

$repoRoot = Resolve-Path (Join-Path $PSScriptRoot "..\..")
if (-not $OutputDir) {
    $OutputDir = Join-Path $repoRoot "updates\launcher"
}

$Version = Read-RequiredValue -Prompt "Version (e.g. 1.2.3)" -Current $Version
$BinaryPath = Read-RequiredValue -Prompt "Path to launcher binary/installer" -Current $BinaryPath

if (-not $Signature -and $SignatureFile) {
    $Signature = Get-Content -Path $SignatureFile -Raw
}

if (-not $Signature -and $env:TAURI_UPDATER_SIGNATURE) {
    $Signature = $env:TAURI_UPDATER_SIGNATURE
}

$Signature = Read-RequiredValue -Prompt "Signature string (Tauri updater signature)" -Current $Signature

if (-not $Notes -and $NotesFile) {
    $Notes = Get-Content -Path $NotesFile -Raw
}

if (-not $Notes) {
    $Notes = ""
}

$BinaryPath = (Resolve-Path $BinaryPath).Path
$BinaryName = Split-Path $BinaryPath -Leaf
$PlatformKey = "$Target-$Arch"
$FilesDir = Join-Path $OutputDir "files"

New-Item -ItemType Directory -Force -Path $FilesDir | Out-Null
Copy-Item -Path $BinaryPath -Destination (Join-Path $FilesDir $BinaryName) -Force

$pubDate = (Get-Date).ToUniversalTime().ToString("yyyy-MM-ddTHH:mm:ssZ")
$url = "$BaseUrl/launcher/files/$BinaryName"

$metadata = @{
    version  = $Version
    notes    = $Notes
    pub_date = $pubDate
    platforms = @{
        $PlatformKey = @{
            signature = $Signature.Trim()
            url       = $url
        }
    }
}

$json = $metadata | ConvertTo-Json -Depth 6

New-Item -ItemType Directory -Force -Path $OutputDir | Out-Null
$latestPath = Join-Path $OutputDir "latest.json"
$platformPath = Join-Path $OutputDir "$PlatformKey.json"

Set-Content -Path $latestPath -Value $json -Encoding ASCII
Set-Content -Path $platformPath -Value $json -Encoding ASCII

Write-Host "Launcher update metadata written:"
Write-Host " - $latestPath"
Write-Host " - $platformPath"
Write-Host "Launcher binary copied to:"
Write-Host " - $(Join-Path $FilesDir $BinaryName)"
