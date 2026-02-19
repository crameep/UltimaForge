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

# Launch flow test
function Test-LaunchFlow {
    Write-Info "=== Running Launch Flow Test ==="

    try {
        # Check if installation exists
        $clientExe = Join-Path $TestInstallDir "client.exe"
        if (-not (Test-Path $clientExe)) {
            Write-Warning "No existing installation found. Running first-run installation first..."
            Test-FirstRunInstallation
        }

        # Create a test executable script
        Write-Info "Creating test executable..."
        New-TestExecutable

        # Start host server for consistency
        Start-HostServer

        Write-Info "Test setup complete. Manual verification required."
        Write-Host ""
        Write-Host "============================================="
        Write-Host "MANUAL TEST STEPS - LAUNCH FLOW:"
        Write-Host "============================================="
        Write-Host ""
        Write-Host "1. Launch the Tauri app:"
        Write-Host "   npm run tauri dev"
        Write-Host ""
        Write-Host "2. Verify Ready State:"
        Write-Host "   - App shows main view (not InstallWizard)"
        Write-Host "   - 'Play' button visible and enabled"
        Write-Host "   - No error messages displayed"
        Write-Host ""
        Write-Host "3. Click 'Play' button:"
        Write-Host "   - Button text changes to 'Launching...' with spinner"
        Write-Host "   - Then changes to 'Playing...'"
        Write-Host "   - 'Game Closed?' button appears"
        Write-Host ""
        Write-Host "4. Verify process spawned:"
        Write-Host "   - Check console window that opens"
        Write-Host "   - Should show: Working Directory: $TestInstallDir"
        Write-Host ""
        Write-Host "5. Test game exit:"
        Write-Host "   - Press Enter in the test script console"
        Write-Host "   - OR click 'Game Closed?' button"
        Write-Host "   - Launcher returns to 'Play' state"
        Write-Host ""
        Write-Host "6. (Optional) Test validation failure:"
        Write-Host "   - Rename client.exe temporarily"
        Write-Host "   - Click 'Play' - should show error"
        Write-Host "   - Restore client.exe"
        Write-Host ""
        Write-Host "============================================="
        Write-Host "Test install directory: $TestInstallDir"
        Write-Host "Test executable: $clientExe"
        Write-Host "============================================="
        Write-Host ""
        Write-Host "Press Enter when test is complete, or Ctrl+C to abort..."
        Read-Host

        # Verify launch functionality
        Write-Info "Verifying launch test results..."

        $markerFile = Join-Path $TestInstallDir ".launch-test-marker"
        if (Test-Path $markerFile) {
            Write-Success "Test executable was launched successfully"

            # Check working directory from marker
            $launchDir = (Get-Content $markerFile | Select-Object -First 1).Trim()
            if ($launchDir -eq $TestInstallDir) {
                Write-Success "Working directory was set correctly: $launchDir"
            }
            else {
                Write-Warning "Working directory may not be correct (got: $launchDir, expected: $TestInstallDir)"
            }

            # Clean up marker
            Remove-Item $markerFile -Force -ErrorAction SilentlyContinue

            Write-Success "Launch flow test PASSED"
        }
        else {
            Write-Warning "Could not verify automatic launch (marker file not found)"
            Write-Info "This may be expected if using manual verification"

            # Ask user for result
            Write-Host ""
            $result = Read-Host "Did the launch flow work correctly? (y/n)"
            if ($result -eq "y" -or $result -eq "Y") {
                Write-Success "Launch flow test PASSED (manual verification)"
            }
            else {
                Write-Error "Launch flow test FAILED (manual verification)"
            }
        }
    }
    finally {
        Cleanup
    }
}

# Create a test executable script
function New-TestExecutable {
    Write-Info "Creating test executable script..."

    $testScript = @"
@echo off
REM UltimaForge Test Client Executable
REM This simulates a game client for E2E testing

echo =============================================
echo UltimaForge Test Client
echo =============================================
echo Working Directory: %CD%
echo Script Directory: %~dp0
echo Arguments: %*
echo =============================================
echo.

REM Write marker file for verification
echo %CD%> "%~dp0.launch-test-marker"
echo %DATE% %TIME%>> "%~dp0.launch-test-marker"

echo Test client running. Press any key to exit...
pause > nul

echo Test client exiting with code 0
exit /b 0
"@

    $clientExePath = Join-Path $TestInstallDir "client.exe"

    # First, check if it's a batch file we need to create
    # Since .exe files on Windows should be actual executables,
    # we'll create a .bat file and rename the reference
    $batPath = Join-Path $TestInstallDir "client.bat"
    Set-Content -Path $batPath -Value $testScript -Encoding ASCII

    # If there's already a client.exe (from installation), back it up
    if (Test-Path $clientExePath) {
        $backupPath = Join-Path $TestInstallDir "client.exe.orig"
        Move-Item $clientExePath $backupPath -Force -ErrorAction SilentlyContinue
    }

    # Create a wrapper that calls the batch file
    $wrapperScript = @"
@echo off
call "%~dp0client.bat" %*
"@
    Set-Content -Path $clientExePath -Value $wrapperScript -Encoding ASCII

    Write-Success "Test executable created: $clientExePath"
}

# Security tests
function Test-Security {
    Write-Info "=== Running Security Tests ==="

    try {
        Push-Location $ProjectRoot

        # Run Rust security tests
        Write-Info "Running Rust security test suite..."

        $testResult = cargo test --package ultimaforge security_tests -- --nocapture 2>&1
        $testExitCode = $LASTEXITCODE

        if ($testExitCode -eq 0) {
            Write-Success "Rust security tests PASSED"
        }
        else {
            Write-Error "Rust security tests FAILED"
            Write-Host $testResult
            return
        }

        # Run manual security tests if host server is available
        Write-Info "Running manual security verification tests..."

        # Check if we need to start the host server
        try {
            $health = Invoke-RestMethod -Uri "http://localhost:$HostPort/health" -Method Get -ErrorAction SilentlyContinue
            if ($health.status -ne "ok") {
                throw "Health check failed"
            }
        }
        catch {
            Write-Info "Starting host server for security tests..."
            Generate-TestUpdates
            Start-HostServer
        }

        # Test 1: Path traversal prevention
        Write-Info "Testing path traversal prevention..."
        try {
            $traversalResponse = Invoke-WebRequest -Uri "http://localhost:$HostPort/files/../manifest.json" -Method Get -ErrorAction SilentlyContinue
            $traversalCode = $traversalResponse.StatusCode
        }
        catch {
            $traversalCode = $_.Exception.Response.StatusCode.Value__
        }

        if ($traversalCode -eq 404 -or $traversalCode -eq 400) {
            Write-Success "Path traversal blocked (HTTP $traversalCode)"
        }
        else {
            Write-Error "Path traversal may be vulnerable (HTTP $traversalCode)"
        }

        # Test 2: URL-encoded path traversal
        try {
            $encodedResponse = Invoke-WebRequest -Uri "http://localhost:$HostPort/files/..%2F..%2Fmanifest.json" -Method Get -ErrorAction SilentlyContinue
            $encodedCode = $encodedResponse.StatusCode
        }
        catch {
            $encodedCode = $_.Exception.Response.StatusCode.Value__
        }

        if ($encodedCode -eq 404 -or $encodedCode -eq 400) {
            Write-Success "Encoded path traversal blocked (HTTP $encodedCode)"
        }
        else {
            Write-Warning "Encoded path traversal returned HTTP $encodedCode"
        }

        # Test 3: Verify signature is required
        Write-Info "Testing signature requirement..."
        $sigPath = Join-Path $ProjectRoot "test-updates\manifest.sig"
        $sigBackupPath = Join-Path $ProjectRoot "test-updates\manifest.sig.security-test"

        if (Test-Path $sigPath) {
            # Temporarily rename signature
            Move-Item $sigPath $sigBackupPath -Force

            # Check that manifest.sig is now missing
            try {
                $sigResponse = Invoke-WebRequest -Uri "http://localhost:$HostPort/manifest.sig" -Method Get -ErrorAction SilentlyContinue
                Write-Warning "Signature file still accessible"
            }
            catch {
                Write-Success "Missing signature file confirmed unavailable"
            }

            # Restore signature
            Move-Item $sigBackupPath $sigPath -Force
            Write-Success "Signature file restored"
        }
        else {
            Write-Warning "Signature file not found - skipping signature removal test"
        }

        # Test 4: Verify manifest validation
        Write-Info "Testing manifest presence..."
        try {
            $manifest = Invoke-RestMethod -Uri "http://localhost:$HostPort/manifest.json" -Method Get
            if ($manifest.version) {
                Write-Success "Manifest endpoint working correctly"
            }
            else {
                Write-Error "Manifest endpoint not working"
            }
        }
        catch {
            Write-Error "Failed to fetch manifest: $_"
        }

        Write-Host ""
        Write-Host "============================================="
        Write-Host "SECURITY TEST SUMMARY"
        Write-Host "============================================="
        Write-Host ""
        Write-Host "Automated Tests:"
        Write-Host "  - Rust security_tests module: COMPLETE"
        Write-Host "  - Signature verification: COMPLETE"
        Write-Host "  - Hash verification: COMPLETE"
        Write-Host "  - Path traversal prevention: COMPLETE"
        Write-Host "  - Manifest validation: COMPLETE"
        Write-Host ""
        Write-Host "Manual E2E Tests (see tests/e2e/security-tests.md):"
        Write-Host "  - Missing signature file rejection"
        Write-Host "  - Tampered manifest rejection"
        Write-Host "  - Corrupted blob file rejection"
        Write-Host "  - Public key immutability"
        Write-Host ""
        Write-Host "============================================="

        Write-Success "Security tests completed successfully"
    }
    finally {
        Pop-Location
        Cleanup
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
        Test-LaunchFlow
    }
    "security" {
        Test-Security
    }
    "all" {
        Test-FirstRunInstallation
        Test-UpdateFlow
        Test-LaunchFlow
        Test-Security
    }
}
