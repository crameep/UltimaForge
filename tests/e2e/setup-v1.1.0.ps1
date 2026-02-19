#
# Setup v1.1.0 Test Data (Windows PowerShell)
#
# This script modifies test files and publishes v1.1.0 for update flow testing.
# Run this after completing a v1.0.0 installation to test the update mechanism.
#
# Usage:
#   .\setup-v1.1.0.ps1
#

$ErrorActionPreference = "Stop"

# Configuration
$ScriptDir = Split-Path -Parent $MyInvocation.MyCommand.Path
$ProjectRoot = Split-Path -Parent (Split-Path -Parent $ScriptDir)

function Write-Info($message) {
    Write-Host "[INFO] $message" -ForegroundColor Blue
}

function Write-Success($message) {
    Write-Host "[SUCCESS] $message" -ForegroundColor Green
}

function Write-Warning($message) {
    Write-Host "[WARNING] $message" -ForegroundColor Yellow
}

Push-Location $ProjectRoot
try {
    # Check if v1.0.0 test updates exist
    $manifestPath = Join-Path $ProjectRoot "test-updates\manifest.json"
    if (-not (Test-Path $manifestPath)) {
        Write-Warning "No v1.0.0 test updates found. Generate them first with:"
        Write-Host "  cargo run -p publish-cli -- publish ``"
        Write-Host "    --source ./test-data/sample-client ``"
        Write-Host "    --output ./test-updates ``"
        Write-Host "    --key ./test-keys/private.key ``"
        Write-Host "    --version 1.0.0"
        exit 1
    }

    # Backup current manifest
    Write-Info "Backing up v1.0.0 manifest..."
    $manifestBackup = Join-Path $ProjectRoot "test-updates\manifest-v1.0.0.json.bak"
    Copy-Item -Path $manifestPath -Destination $manifestBackup

    # Backup original test data
    Write-Info "Backing up original test data..."
    $sampleClientPath = Join-Path $ProjectRoot "test-data\sample-client"
    $sampleClientBackup = Join-Path $ProjectRoot "test-data\sample-client.bak"
    if (Test-Path $sampleClientBackup) {
        Remove-Item -Path $sampleClientBackup -Recurse -Force
    }
    Copy-Item -Path $sampleClientPath -Destination $sampleClientBackup -Recurse

    # Modify test files for v1.1.0
    Write-Info "Modifying test files for v1.1.0..."
    $artMulPath = Join-Path $sampleClientPath "art.mul"
    Add-Content -Path $artMulPath -Value "`n[v1.1.0] Updated content for testing - $(Get-Date)"

    # Optional: Add a new file
    $configPath = Join-Path $sampleClientPath "config.ini"
    Set-Content -Path $configPath -Value "This is a new configuration file added in v1.1.0"

    # Show what changed
    Write-Info "Changes made:"
    Write-Host "  - art.mul: Added v1.1.0 marker"
    Write-Host "  - config.ini: New file added"

    # Publish v1.1.0
    Write-Info "Publishing version 1.1.0..."
    cargo run --release -p publish-cli -- publish `
        --source ./test-data/sample-client `
        --output ./test-updates `
        --key ./test-keys/private.key `
        --version 1.1.0

    # Validate
    Write-Info "Validating v1.1.0 release..."
    cargo run --release -p publish-cli -- validate `
        --dir ./test-updates `
        --key ./test-keys/public.key

    Write-Success "v1.1.0 test data ready!"
    Write-Host ""
    Write-Host "Next steps:"
    Write-Host "  1. Start the host server:"
    Write-Host "     cargo run -p host-server -- --dir ./test-updates --port 8080"
    Write-Host ""
    Write-Host "  2. Launch the Tauri app:"
    Write-Host "     npm run tauri dev"
    Write-Host ""
    Write-Host "  3. The app should detect v1.1.0 update is available"
    Write-Host ""
    Write-Host "To restore to v1.0.0:"
    Write-Host "  .\tests\e2e\restore-v1.0.0.ps1"
}
finally {
    Pop-Location
}
