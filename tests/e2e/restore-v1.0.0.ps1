#
# Restore v1.0.0 Test Data (Windows PowerShell)
#
# This script restores the test data and manifest back to v1.0.0 state
# after running update flow tests.
#
# Usage:
#   .\restore-v1.0.0.ps1
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
    # Restore test data
    $sampleClientPath = Join-Path $ProjectRoot "test-data\sample-client"
    $sampleClientBackup = Join-Path $ProjectRoot "test-data\sample-client.bak"

    if (Test-Path $sampleClientBackup) {
        Write-Info "Restoring original test data..."
        Remove-Item -Path $sampleClientPath -Recurse -Force -ErrorAction SilentlyContinue
        Move-Item -Path $sampleClientBackup -Destination $sampleClientPath
        Write-Success "Test data restored"
    }
    else {
        Write-Warning "No backup found at $sampleClientBackup"
    }

    # Restore v1.0.0 manifest
    $manifestPath = Join-Path $ProjectRoot "test-updates\manifest.json"
    $manifestBackup = Join-Path $ProjectRoot "test-updates\manifest-v1.0.0.json.bak"

    if (Test-Path $manifestBackup) {
        Write-Info "Restoring v1.0.0 manifest..."
        Move-Item -Path $manifestBackup -Destination $manifestPath -Force
        Write-Success "Manifest restored to v1.0.0"
    }
    else {
        Write-Warning "No manifest backup found. Re-publishing v1.0.0..."
        cargo run --release -p publish-cli -- publish `
            --source ./test-data/sample-client `
            --output ./test-updates `
            --key ./test-keys/private.key `
            --version 1.0.0
        Write-Success "v1.0.0 published"
    }

    Write-Success "Test environment restored to v1.0.0 state"
    Write-Host ""
    Write-Host "You may need to restart the host-server to serve the restored files."
}
finally {
    Pop-Location
}
