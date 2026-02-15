#
# UltimaForge E2E Test Runner (Windows PowerShell)
#
# Usage:
#   .\run-e2e-tests.ps1              # Run all E2E tests
#   .\run-e2e-tests.ps1 first-run    # Run first-run installation test
#   .\run-e2e-tests.ps1 update       # Run update flow test
#   .\run-e2e-tests.ps1 launch       # Run launch flow test
#   .\run-e2e-tests.ps1 security     # Run security tests

param(
    [Parameter(Position=0)]
    [ValidateSet("all", "first-run", "install", "update", "launch", "security")]
    [string]$TestType = "all"
)

$ErrorActionPreference = "Stop"

# Configuration
$ScriptDir = Split-Path -Parent $MyInvocation.MyCommand.Path
$ProjectRoot = Split-Path -Parent (Split-Path -Parent $ScriptDir)
$HostPort = if ($env:ULTIMAFORGE_HOST_PORT) { $env:ULTIMAFORGE_HOST_PORT } else { "8080" }
$TestInstallDir = if ($env:ULTIMAFORGE_TEST_INSTALL_DIR) { $env:ULTIMAFORGE_TEST_INSTALL_DIR } else { "$env:TEMP\ultimaforge-test-install" }

# Host server process
$HostServerProcess = $null

# Logging functions
function Write-Info($message) {
    Write-Host "[INFO] $message" -ForegroundColor Blue
}

function Write-Success($message) {
    Write-Host "[SUCCESS] $message" -ForegroundColor Green
}

function Write-Warning($message) {
    Write-Host "[WARNING] $message" -ForegroundColor Yellow
}

function Write-Error($message) {
    Write-Host "[ERROR] $message" -ForegroundColor Red
}

# Check prerequisites
function Check-Prerequisites {
    Write-Info "Checking prerequisites..."

    # Check Cargo
    if (-not (Get-Command cargo -ErrorAction SilentlyContinue)) {
        Write-Error "cargo not found. Please install Rust."
        exit 1
    }

    # Check npm
    if (-not (Get-Command npm -ErrorAction SilentlyContinue)) {
        Write-Error "npm not found. Please install Node.js."
        exit 1
    }

    Write-Success "Prerequisites check passed"
}

# Build tools
function Build-Tools {
    if ($env:SKIP_BUILD) {
        Write-Info "Skipping build (SKIP_BUILD is set)"
        return
    }

    Write-Info "Building tools..."
    Push-Location $ProjectRoot
    try {
        cargo build --release -p host-server -p publish-cli
        Write-Success "Tools built successfully"
    }
    finally {
        Pop-Location
    }
}

# Generate test updates
function Generate-TestUpdates {
    Write-Info "Generating test updates..."
    Push-Location $ProjectRoot
    try {
        # Create test-updates directory if needed
        New-Item -ItemType Directory -Force -Path "test-updates" | Out-Null

        cargo run --release -p publish-cli -- publish `
            --source ./test-data/sample-client `
            --output ./test-updates `
            --key ./test-keys/private.key `
            --version 1.0.0

        Write-Success "Test updates generated"
    }
    finally {
        Pop-Location
    }
}

# Validate test updates
function Validate-TestUpdates {
    Write-Info "Validating test updates..."
    Push-Location $ProjectRoot
    try {
        cargo run --release -p publish-cli -- validate `
            --dir ./test-updates `
            --key ./test-keys/public.key

        Write-Success "Test updates validated"
    }
    finally {
        Pop-Location
    }
}

# Start host server
function Start-HostServer {
    Write-Info "Starting host server on port $HostPort..."

    Push-Location $ProjectRoot
    try {
        # Start server in background
        $script:HostServerProcess = Start-Process -FilePath "cargo" `
            -ArgumentList "run --release -p host-server -- --dir ./test-updates --port $HostPort" `
            -PassThru -WindowStyle Hidden

        # Wait for server to start
        Start-Sleep -Seconds 3

        # Verify server is running
        try {
            $response = Invoke-RestMethod -Uri "http://localhost:$HostPort/health" -Method Get
            if ($response.status -eq "ok") {
                Write-Success "Host server started (PID: $($script:HostServerProcess.Id))"
            }
            else {
                Write-Error "Host server health check failed"
                exit 1
            }
        }
        catch {
            Write-Error "Host server failed to start: $_"
            exit 1
        }
    }
    finally {
        Pop-Location
    }
}

# Stop host server
function Stop-HostServer {
    if ($script:HostServerProcess -and -not $script:HostServerProcess.HasExited) {
        Write-Info "Stopping host server (PID: $($script:HostServerProcess.Id))..."
        Stop-Process -Id $script:HostServerProcess.Id -Force -ErrorAction SilentlyContinue
        Write-Success "Host server stopped"
    }
}

# Prepare test environment
function Prepare-TestEnvironment {
    Write-Info "Preparing test environment..."

    # Create and clear test install directory
    if (Test-Path $TestInstallDir) {
        Remove-Item -Path $TestInstallDir -Recurse -Force
    }
    New-Item -ItemType Directory -Force -Path $TestInstallDir | Out-Null

    Write-Success "Test environment prepared"
}

# Cleanup
function Cleanup {
    Write-Info "Cleaning up..."
    Stop-HostServer

    # Restore test data if backup exists
    $sampleClientPath = Join-Path $ProjectRoot "test-data\sample-client"
    $sampleClientBackup = Join-Path $ProjectRoot "test-data\sample-client.bak"
    if (Test-Path $sampleClientBackup) {
        Remove-Item -Path $sampleClientPath -Recurse -Force -ErrorAction SilentlyContinue
        Move-Item -Path $sampleClientBackup -Destination $sampleClientPath -ErrorAction SilentlyContinue
    }

    # Remove manifest backup
    $manifestBackup = Join-Path $ProjectRoot "test-updates\manifest-v1.0.0.json.bak"
    if (Test-Path $manifestBackup) {
        Remove-Item -Path $manifestBackup -Force -ErrorAction SilentlyContinue
    }

    if (Test-Path $TestInstallDir) {
        Remove-Item -Path $TestInstallDir -Recurse -Force -ErrorAction SilentlyContinue
    }
    Write-Success "Cleanup complete"
}

# First-run installation test
function Test-FirstRunInstallation {
    Write-Info "=== Running First-Run Installation Test ==="

    try {
        Generate-TestUpdates
        Validate-TestUpdates
        Start-HostServer
        Prepare-TestEnvironment

        Write-Info "Test setup complete. Manual verification required."
        Write-Host ""
        Write-Host "============================================="
        Write-Host "MANUAL TEST STEPS:"
        Write-Host "============================================="
        Write-Host ""
        Write-Host "1. Clear launcher configuration:"
        Write-Host "   Delete: $env:APPDATA\ultimaforge\launcher.json"
        Write-Host ""
        Write-Host "2. Launch the Tauri app:"
        Write-Host "   npm run tauri dev"
        Write-Host ""
        Write-Host "3. Follow the installation wizard:"
        Write-Host "   a) Click 'Get Started' on welcome screen"
        Write-Host "   b) Click 'Browse...' and select: $TestInstallDir"
        Write-Host "   c) Accept the Terms of Service"
        Write-Host "   d) Wait for installation to complete"
        Write-Host "   e) Click 'Start Playing'"
        Write-Host ""
        Write-Host "4. Verify installation files:"
        Write-Host "   dir $TestInstallDir"
        Write-Host ""
        Write-Host "5. Expected files:"
        Write-Host "   - client.exe"
        Write-Host "   - art.mul"
        Write-Host "   - map0.mul"
        Write-Host ""
        Write-Host "============================================="
        Write-Host "Host server running at: http://localhost:$HostPort"
        Write-Host "Test install directory: $TestInstallDir"
        Write-Host "============================================="
        Write-Host ""
        Write-Host "Press Enter when test is complete, or Ctrl+C to abort..."
        Read-Host

        # Verify installation
        Write-Info "Verifying installation..."

        $clientExe = Join-Path $TestInstallDir "client.exe"
        $artMul = Join-Path $TestInstallDir "art.mul"
        $map0Mul = Join-Path $TestInstallDir "map0.mul"

        if ((Test-Path $clientExe) -and (Test-Path $artMul) -and (Test-Path $map0Mul)) {
            Write-Success "All expected files found in installation directory"

            # Show file info
            Write-Host ""
            Write-Host "Installed files:"
            Get-ChildItem $TestInstallDir | Format-Table Name, Length

            Write-Success "First-run installation test PASSED"
        }
        else {
            Write-Error "Missing files in installation directory"
            Get-ChildItem $TestInstallDir -ErrorAction SilentlyContinue
            Write-Error "First-run installation test FAILED"
        }
    }
    finally {
        Cleanup
    }
}

# Main
Write-Host "======================================"
Write-Host "UltimaForge E2E Test Runner (Windows)"
Write-Host "======================================"
Write-Host ""

Check-Prerequisites
Build-Tools

# Update flow test
function Test-UpdateFlow {
    Write-Info "=== Running Update Flow Test ==="

    try {
        # Check if first-run installation was completed
        $clientExe = Join-Path $TestInstallDir "client.exe"
        if (-not (Test-Path $clientExe)) {
            Write-Warning "No existing installation found. Running first-run installation first..."
            Test-FirstRunInstallation
        }

        # Backup original test files
        Write-Info "Backing up original test data..."
        $sampleClientPath = Join-Path $ProjectRoot "test-data\sample-client"
        $sampleClientBackup = Join-Path $ProjectRoot "test-data\sample-client.bak"
        if (Test-Path $sampleClientBackup) {
            Remove-Item -Path $sampleClientBackup -Recurse -Force
        }
        Copy-Item -Path $sampleClientPath -Destination $sampleClientBackup -Recurse

        # Modify test files for v1.1.0
        Write-Info "Creating v1.1.0 test files..."
        $artMulPath = Join-Path $sampleClientPath "art.mul"
        Add-Content -Path $artMulPath -Value "`n[v1.1.0] Updated content for testing - $(Get-Date)"

        # Backup v1.0.0 manifest
        $manifestPath = Join-Path $ProjectRoot "test-updates\manifest.json"
        $manifestBackup = Join-Path $ProjectRoot "test-updates\manifest-v1.0.0.json.bak"
        if (Test-Path $manifestPath) {
            Copy-Item -Path $manifestPath -Destination $manifestBackup
        }

        # Publish v1.1.0
        Write-Info "Publishing version 1.1.0..."
        Push-Location $ProjectRoot
        try {
            cargo run --release -p publish-cli -- publish `
                --source ./test-data/sample-client `
                --output ./test-updates `
                --key ./test-keys/private.key `
                --version 1.1.0

            # Validate the new release
            Write-Info "Validating v1.1.0 release..."
            cargo run --release -p publish-cli -- validate `
                --dir ./test-updates `
                --key ./test-keys/public.key
        }
        finally {
            Pop-Location
        }

        # Restart host server with updated files
        Stop-HostServer
        Start-Sleep -Seconds 1
        Start-HostServer

        # Verify manifest version
        Write-Info "Verifying server is serving v1.1.0..."
        try {
            $manifest = Invoke-RestMethod -Uri "http://localhost:$HostPort/manifest.json" -Method Get
            if ($manifest.version -ne "1.1.0") {
                Write-Error "Server not serving v1.1.0 (got: $($manifest.version))"
                Restore-TestData
                return
            }
            Write-Success "Server confirmed serving v1.1.0"
        }
        catch {
            Write-Error "Failed to verify server: $_"
            Restore-TestData
            return
        }

        Write-Info "Test setup complete. Manual verification required."
        Write-Host ""
        Write-Host "============================================="
        Write-Host "MANUAL TEST STEPS - UPDATE FLOW:"
        Write-Host "============================================="
        Write-Host ""
        Write-Host "1. Launch the Tauri app:"
        Write-Host "   npm run tauri dev"
        Write-Host ""
        Write-Host "2. Verify update detection:"
        Write-Host "   - App should show 'Update Available' banner"
        Write-Host "   - Current version: 1.0.0"
        Write-Host "   - Available version: 1.1.0"
        Write-Host "   - Files to update: 1 (only art.mul changed)"
        Write-Host ""
        Write-Host "3. Click 'Update Now' and observe:"
        Write-Host "   - Download progress bar"
        Write-Host "   - File count (1/1)"
        Write-Host "   - Only art.mul should be downloaded (differential)"
        Write-Host "   - Verification step"
        Write-Host "   - Apply step"
        Write-Host ""
        Write-Host "4. After update completes:"
        Write-Host "   - Version should show 1.1.0"
        Write-Host "   - Launch button should be enabled"
        Write-Host ""
        Write-Host "5. Verify files on disk:"
        Write-Host "   Get-Content '$TestInstallDir\art.mul' | Select-Object -Last 1"
        Write-Host "   # Should show: [v1.1.0] Updated content for testing"
        Write-Host ""
        Write-Host "============================================="
        Write-Host "Host server running at: http://localhost:$HostPort"
        Write-Host "Test install directory: $TestInstallDir"
        Write-Host "============================================="
        Write-Host ""
        Write-Host "Press Enter when test is complete, or Ctrl+C to abort..."
        Read-Host

        # Verify update applied
        Write-Info "Verifying update was applied..."

        $installedArtMul = Join-Path $TestInstallDir "art.mul"
        if (Test-Path $installedArtMul) {
            $content = Get-Content -Path $installedArtMul -Raw
            if ($content -match "v1\.1\.0") {
                Write-Success "art.mul contains v1.1.0 content"
            }
            else {
                Write-Error "art.mul does not contain v1.1.0 content"
                Restore-TestData
                return
            }
        }
        else {
            Write-Error "art.mul not found in installation directory"
            Restore-TestData
            return
        }

        # Cleanup test data
        Restore-TestData

        Write-Success "Update flow test PASSED"
    }
    finally {
        Cleanup
    }
}

# Restore original test data
function Restore-TestData {
    Write-Info "Restoring original test data..."
    $sampleClientPath = Join-Path $ProjectRoot "test-data\sample-client"
    $sampleClientBackup = Join-Path $ProjectRoot "test-data\sample-client.bak"

    if (Test-Path $sampleClientBackup) {
        Remove-Item -Path $sampleClientPath -Recurse -Force -ErrorAction SilentlyContinue
        Move-Item -Path $sampleClientBackup -Destination $sampleClientPath
        Write-Success "Test data restored"
    }

    # Restore v1.0.0 manifest
    $manifestBackup = Join-Path $ProjectRoot "test-updates\manifest-v1.0.0.json.bak"
    $manifestPath = Join-Path $ProjectRoot "test-updates\manifest.json"
    if (Test-Path $manifestBackup) {
        Move-Item -Path $manifestBackup -Destination $manifestPath -Force
        Write-Success "Manifest restored to v1.0.0"
    }
}

switch ($TestType) {
    { $_ -in "first-run", "install" } {
        Test-FirstRunInstallation
    }
    "update" {
        Test-UpdateFlow
    }
    "launch" {
        Write-Warning "Launch flow test not yet implemented"
        Write-Info "See: tests/e2e/launch-flow.md"
    }
    "security" {
        Write-Warning "Security tests not yet implemented"
        Write-Info "See: tests/e2e/security-tests.md"
    }
    "all" {
        Test-FirstRunInstallation
        Test-UpdateFlow
        Write-Warning "Launch/Security tests not yet implemented"
    }
}
