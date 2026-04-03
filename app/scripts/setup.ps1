# UltimaForge Windows Setup Script
# Installs Rust, Node.js, VS Build Tools, and Tauri CLI
#
# Usage:
#   .\scripts\setup.ps1                    # Interactive mode
#   .\scripts\setup.ps1 -SkipPrompts       # Non-interactive (CI mode)
#   .\scripts\setup.ps1 -UseScoop          # Use Scoop instead of winget (no admin required)

param(
    [switch]$SkipPrompts,
    [switch]$UseScoop,
    [switch]$Help
)

# Version requirements
$RUST_MIN_VERSION = "1.77.2"
$NODE_MIN_VERSION = "18.0.0"
$NPM_MIN_VERSION = "8.0.0"

# Colors for output
$script:Colors = @{
    Green  = "Green"
    Yellow = "Yellow"
    Red    = "Red"
    Cyan   = "Cyan"
    White  = "White"
}

function Show-Help {
    Write-Host @"
UltimaForge Windows Setup Script

USAGE:
    .\scripts\setup.ps1 [OPTIONS]

OPTIONS:
    -SkipPrompts    Run without user prompts (for CI/automation)
    -UseScoop       Use Scoop package manager instead of winget (no admin required)
    -Help           Show this help message

DESCRIPTION:
    This script installs all dependencies required to build UltimaForge:
    - Git (for updates and version control)
    - Rust (via rustup)
    - Node.js LTS
    - Visual Studio Build Tools 2022
    - Tauri CLI (via npm)

EXAMPLES:
    # Interactive installation with winget (requires admin for VS Build Tools)
    .\scripts\setup.ps1

    # Non-interactive installation for CI
    .\scripts\setup.ps1 -SkipPrompts

    # Install without admin rights using Scoop
    .\scripts\setup.ps1 -UseScoop

"@
}

function Write-Status {
    param(
        [string]$Message,
        [string]$Type = "Info"
    )

    switch ($Type) {
        "Success" { Write-Host "[OK] $Message" -ForegroundColor $Colors.Green }
        "Warning" { Write-Host "[WARN] $Message" -ForegroundColor $Colors.Yellow }
        "Error"   { Write-Host "[ERROR] $Message" -ForegroundColor $Colors.Red }
        "Info"    { Write-Host "[INFO] $Message" -ForegroundColor $Colors.Cyan }
        "Step"    { Write-Host "`n>>> $Message" -ForegroundColor $Colors.White }
    }
}

function Test-Command {
    param([string]$CmdName)
    return [bool](Get-Command -Name $CmdName -ErrorAction SilentlyContinue)
}

function Test-AdminRights {
    $identity = [Security.Principal.WindowsIdentity]::GetCurrent()
    $principal = New-Object Security.Principal.WindowsPrincipal($identity)
    return $principal.IsInRole([Security.Principal.WindowsBuiltInRole]::Administrator)
}

function Compare-Version {
    param(
        [string]$Actual,
        [string]$Required
    )

    # Clean version strings
    $Actual = $Actual -replace '^v', '' -replace '[^0-9.].*$', ''
    $Required = $Required -replace '^v', ''

    try {
        $actualParts = $Actual.Split('.') | ForEach-Object { [int]$_ }
        $requiredParts = $Required.Split('.') | ForEach-Object { [int]$_ }

        for ($i = 0; $i -lt 3; $i++) {
            $a = if ($i -lt $actualParts.Count) { $actualParts[$i] } else { 0 }
            $r = if ($i -lt $requiredParts.Count) { $requiredParts[$i] } else { 0 }

            if ($a -gt $r) { return $true }
            if ($a -lt $r) { return $false }
        }
        return $true  # Equal versions
    }
    catch {
        return $false
    }
}

function Get-UserConfirmation {
    param([string]$Message)

    if ($SkipPrompts) {
        return $true
    }

    $response = Read-Host "$Message (Y/n)"
    return ($response -eq '' -or $response -match '^[Yy]')
}

function Install-Scoop {
    if (Test-Command "scoop") {
        Write-Status "Scoop already installed" -Type "Success"
        return $true
    }

    Write-Status "Installing Scoop package manager..." -Type "Info"

    # Set execution policy for current user if not already permissive enough.
    # Ignore errors - setup.bat already launches with -ExecutionPolicy Bypass
    # so the process policy is fine regardless.
    try {
        Set-ExecutionPolicy -ExecutionPolicy RemoteSigned -Scope CurrentUser -Force -ErrorAction Ignore
    }
    catch {
        # Ignore - the process already runs under Bypass
    }

    try {
        # Install Scoop
        Invoke-RestMethod -Uri https://get.scoop.sh | Invoke-Expression

        # Refresh PATH
        $env:PATH = [System.Environment]::GetEnvironmentVariable("PATH", "User") + ";" + $env:PATH

        if (Test-Command "scoop") {
            Write-Status "Scoop installed successfully" -Type "Success"
            return $true
        }
    }
    catch {
        Write-Status "Failed to install Scoop: $_" -Type "Error"
    }

    return $false
}

function Install-Git {
    Write-Status "Checking Git installation..." -Type "Step"

    if (Test-Command "git") {
        $version = (git --version) -replace 'git version\s*', ''
        Write-Status "Git $version already installed" -Type "Success"
        return $true
    }

    Write-Status "Installing Git..." -Type "Info"

    if ($UseScoop) {
        if (-not (Test-Command "scoop")) {
            if (-not (Install-Scoop)) { return $false }
        }
        try {
            scoop install git
            $env:PATH = [System.Environment]::GetEnvironmentVariable("PATH", "User") + ";" + $env:PATH
            if (Test-Command "git") {
                Write-Status "Git installed successfully via Scoop" -Type "Success"
                return $true
            }
        }
        catch {
            Write-Status "Failed to install Git via Scoop: $_" -Type "Error"
            return $false
        }
    }
    else {
        if (-not (Test-Command "winget")) {
            Write-Status "winget not found. Please install Git manually from https://git-scm.com" -Type "Error"
            return $false
        }

        try {
            Write-Status "Installing via winget..." -Type "Info"
            winget install Git.Git --accept-source-agreements --accept-package-agreements

            # Refresh PATH
            $env:PATH = [System.Environment]::GetEnvironmentVariable("PATH", "Machine") + ";" + [System.Environment]::GetEnvironmentVariable("PATH", "User") + ";" + $env:PATH

            if (Test-Command "git") {
                $version = (git --version) -replace 'git version\s*', ''
                Write-Status "Git $version installed successfully" -Type "Success"
                return $true
            }
            else {
                Write-Status "Git installed but not found in PATH. Please restart your terminal." -Type "Warning"
                return $true
            }
        }
        catch {
            Write-Status "Failed to install Git: $_" -Type "Error"
            Write-Status "Install manually from: https://git-scm.com" -Type "Info"
            return $false
        }
    }

    return $false
}

function Install-Rust {
    Write-Status "Checking Rust installation..." -Type "Step"

    if (Test-Command "rustc") {
        $version = (rustc --version) -replace 'rustc\s+', '' -replace '\s.*', ''

        # On ARM64, ensure we're using the x64 toolchain for VS Build Tools compatibility
        if ($env:PROCESSOR_ARCHITECTURE -eq "ARM64" -and (rustc --version 2>$null) -match "aarch64") {
            Write-Status "ARM64 detected with aarch64 toolchain — switching to x64 for build compatibility" -Type "Info"
            & rustup default stable-x86_64-pc-windows-msvc 2>$null
            $version = (rustc --version) -replace 'rustc\s+', '' -replace '\s.*', ''
        }

        if (Compare-Version -Actual $version -Required $RUST_MIN_VERSION) {
            Write-Status "Rust $version already installed (>= $RUST_MIN_VERSION required)" -Type "Success"
            return $true
        }
        else {
            Write-Status "Rust $version installed but $RUST_MIN_VERSION or newer required" -Type "Warning"
            if (-not (Get-UserConfirmation "Update Rust?")) {
                return $false
            }

            Write-Status "Updating Rust..." -Type "Info"
            & rustup update stable
            return $?
        }
    }

    Write-Status "Installing Rust..." -Type "Info"

    try {
        $installerPath = "$env:TEMP\rustup-init.exe"

        # Download rustup-init
        Write-Status "Downloading rustup-init.exe..." -Type "Info"
        [Net.ServicePointManager]::SecurityProtocol = [Net.SecurityProtocolType]::Tls12
        Invoke-WebRequest -Uri "https://win.rustup.rs/x86_64" -OutFile $installerPath -UseBasicParsing

        # Run installer
        Write-Status "Running Rust installer..." -Type "Info"
        $installArgs = @("-y", "--default-toolchain", "stable")
        $process = Start-Process -FilePath $installerPath -ArgumentList $installArgs -Wait -PassThru -NoNewWindow

        if ($process.ExitCode -ne 0) {
            Write-Status "Rust installation failed with exit code $($process.ExitCode)" -Type "Error"
            return $false
        }

        # Add cargo to PATH for this session
        $cargoPath = "$env:USERPROFILE\.cargo\bin"
        if (Test-Path $cargoPath) {
            $env:PATH = "$cargoPath;$env:PATH"
        }

        # On ARM64 Windows, default to x64 toolchain for VS Build Tools compatibility.
        # x64 binaries run fine on ARM64 via emulation, and most VS installs only
        # include x64 MSVC tools.
        if ($env:PROCESSOR_ARCHITECTURE -eq "ARM64") {
            Write-Status "ARM64 detected — setting Rust to use x64 toolchain for build compatibility" -Type "Info"
            & rustup default stable-x86_64-pc-windows-msvc 2>$null
        }

        # Verify installation
        if (Test-Command "rustc") {
            $version = (rustc --version) -replace 'rustc\s+', '' -replace '\s.*', ''
            Write-Status "Rust $version installed successfully" -Type "Success"
            return $true
        }
        else {
            Write-Status "Rust installed but not found in PATH. Please restart your terminal." -Type "Warning"
            return $true
        }
    }
    catch {
        Write-Status "Failed to install Rust: $_" -Type "Error"
        Write-Status "Try manual installation: https://rustup.rs" -Type "Info"
        return $false
    }
    finally {
        # Cleanup
        if (Test-Path "$env:TEMP\rustup-init.exe") {
            Remove-Item "$env:TEMP\rustup-init.exe" -Force -ErrorAction SilentlyContinue
        }
    }
}

function Install-NodeJS {
    Write-Status "Checking Node.js installation..." -Type "Step"

    if (Test-Command "node") {
        $version = (node --version) -replace '^v', ''
        if (Compare-Version -Actual $version -Required $NODE_MIN_VERSION) {
            Write-Status "Node.js v$version already installed (>= v$NODE_MIN_VERSION required)" -Type "Success"
            return $true
        }
        else {
            Write-Status "Node.js v$version installed but v$NODE_MIN_VERSION or newer required" -Type "Warning"
        }
    }

    Write-Status "Installing Node.js LTS..." -Type "Info"

    if ($UseScoop) {
        # Install via Scoop
        if (-not (Test-Command "scoop")) {
            if (-not (Install-Scoop)) {
                return $false
            }
        }

        try {
            scoop install nodejs-lts

            # Refresh PATH
            $env:PATH = [System.Environment]::GetEnvironmentVariable("PATH", "User") + ";" + $env:PATH

            if (Test-Command "node") {
                $version = (node --version)
                Write-Status "Node.js $version installed successfully via Scoop" -Type "Success"
                return $true
            }
        }
        catch {
            Write-Status "Failed to install Node.js via Scoop: $_" -Type "Error"
            return $false
        }
    }
    else {
        # Install via winget
        if (-not (Test-Command "winget")) {
            Write-Status "winget not found. Please install App Installer from Microsoft Store or use -UseScoop flag" -Type "Error"
            return $false
        }

        try {
            Write-Status "Installing via winget..." -Type "Info"
            $result = winget install OpenJS.NodeJS.LTS --accept-source-agreements --accept-package-agreements

            # Refresh PATH
            $env:PATH = [System.Environment]::GetEnvironmentVariable("PATH", "Machine") + ";" + [System.Environment]::GetEnvironmentVariable("PATH", "User") + ";" + $env:PATH

            if (Test-Command "node") {
                $version = (node --version)
                Write-Status "Node.js $version installed successfully via winget" -Type "Success"
                return $true
            }
            else {
                Write-Status "Node.js installed but not found in PATH. Please restart your terminal." -Type "Warning"
                return $true
            }
        }
        catch {
            Write-Status "Failed to install Node.js via winget: $_" -Type "Error"
            Write-Status "Try: winget install OpenJS.NodeJS.LTS" -Type "Info"
            return $false
        }
    }

    return $false
}

function Install-VSBuildTools {
    Write-Status "Checking Visual Studio Build Tools..." -Type "Step"

    # Check for existing VS installation
    $vsWhere = "${env:ProgramFiles(x86)}\Microsoft Visual Studio\Installer\vswhere.exe"
    if (Test-Path $vsWhere) {
        $vsPath = & $vsWhere -latest -products * -requires Microsoft.VisualStudio.Component.VC.Tools.x86.x64 -property installationPath 2>$null
        if ($vsPath) {
            Write-Status "Visual Studio Build Tools already installed at $vsPath" -Type "Success"
            return $true
        }
    }

    # Check for standalone Build Tools
    $buildToolsPath = "${env:ProgramFiles(x86)}\Microsoft Visual Studio\2022\BuildTools"
    if (Test-Path $buildToolsPath) {
        Write-Status "Visual Studio Build Tools 2022 already installed" -Type "Success"
        return $true
    }

    # VS Build Tools requires admin rights
    if (-not (Test-AdminRights)) {
        if ($UseScoop) {
            Write-Status "VS Build Tools require admin rights even with Scoop" -Type "Warning"
            Write-Status "Please run this script as Administrator or install manually" -Type "Info"
            Write-Status "Download: https://visualstudio.microsoft.com/visual-cpp-build-tools/" -Type "Info"

            if (-not (Get-UserConfirmation "Continue without VS Build Tools?")) {
                return $false
            }
            return $true  # Continue but warn
        }
        else {
            Write-Status "VS Build Tools require administrator rights" -Type "Error"
            Write-Status "Please run this script as Administrator or use manual installation" -Type "Info"
            return $false
        }
    }

    Write-Status "Installing Visual Studio Build Tools 2022..." -Type "Info"
    Write-Status "This may take 10-20 minutes and requires 5-10GB of disk space" -Type "Warning"

    if (-not (Get-UserConfirmation "Proceed with VS Build Tools installation?")) {
        return $false
    }

    if (Test-Command "winget") {
        try {
            Write-Status "Installing via winget (this will take a while)..." -Type "Info"

            # Install with required components for Rust/Tauri
            winget install Microsoft.VisualStudio.2022.BuildTools --accept-source-agreements --accept-package-agreements --override "--quiet --wait --add Microsoft.VisualStudio.Workload.VCTools --includeRecommended"

            Write-Status "Visual Studio Build Tools 2022 installed successfully" -Type "Success"
            return $true
        }
        catch {
            Write-Status "Failed to install via winget: $_" -Type "Error"
        }
    }

    # Fallback: download and install directly
    Write-Status "Attempting direct download installation..." -Type "Info"

    try {
        $installerUrl = "https://aka.ms/vs/17/release/vs_buildtools.exe"
        $installerPath = "$env:TEMP\vs_buildtools.exe"

        Write-Status "Downloading VS Build Tools installer..." -Type "Info"
        [Net.ServicePointManager]::SecurityProtocol = [Net.SecurityProtocolType]::Tls12
        Invoke-WebRequest -Uri $installerUrl -OutFile $installerPath -UseBasicParsing

        Write-Status "Running installer (this will take a while)..." -Type "Info"
        $installArgs = @(
            "--quiet",
            "--wait",
            "--norestart",
            "--add", "Microsoft.VisualStudio.Workload.VCTools",
            "--includeRecommended"
        )

        $process = Start-Process -FilePath $installerPath -ArgumentList $installArgs -Wait -PassThru

        if ($process.ExitCode -eq 0 -or $process.ExitCode -eq 3010) {
            Write-Status "Visual Studio Build Tools installed successfully" -Type "Success"
            if ($process.ExitCode -eq 3010) {
                Write-Status "A system restart may be required" -Type "Warning"
            }
            return $true
        }
        else {
            Write-Status "Installation completed with exit code $($process.ExitCode)" -Type "Warning"
            return $true  # May still work
        }
    }
    catch {
        Write-Status "Failed to install VS Build Tools: $_" -Type "Error"
        Write-Status "Please install manually from: https://visualstudio.microsoft.com/visual-cpp-build-tools/" -Type "Info"
        return $false
    }
    finally {
        if (Test-Path "$env:TEMP\vs_buildtools.exe") {
            Remove-Item "$env:TEMP\vs_buildtools.exe" -Force -ErrorAction SilentlyContinue
        }
    }
}

function Install-Rsync {
    Write-Status "Checking rsync installation..." -Type "Step"

    if (Test-Command "rsync") {
        $version = (rsync --version 2>$null | Select-Object -First 1) -replace '^rsync\s+version\s+', '' -replace '\s.*', ''
        Write-Status "rsync $version already installed" -Type "Success"
        return $true
    }

    Write-Status "rsync not found. Installing for efficient VPS deploys..." -Type "Info"
    Write-Status "(Without rsync, deploy falls back to scp which re-uploads all files each time)" -Type "Warning"

    # Install via Scoop (no admin required). Install Scoop first if needed.
    if (-not (Test-Command "scoop")) {
        Write-Status "Installing Scoop to get rsync (no admin required)..." -Type "Info"
        Install-Scoop | Out-Null
    }

    if (Test-Command "scoop") {
        try {
            # Add extras bucket (has cwrsync) and update
            Write-Status "Adding Scoop extras bucket..." -Type "Info"
            scoop bucket add extras 2>&1 | Out-Host
            scoop update 2>&1 | Out-Host

            # Try package names that are known to provide rsync on Windows via Scoop
            $scoopPackages = @("rsync", "cwrsync")
            foreach ($pkg in $scoopPackages) {
                Write-Status "Trying: scoop install $pkg ..." -Type "Info"
                scoop install $pkg 2>&1 | Out-Host

                # Refresh PATH so newly installed shims are visible
                $env:PATH = [System.Environment]::GetEnvironmentVariable("PATH", "User") + ";" + $env:PATH
                $scoopShims = Join-Path $env:USERPROFILE "scoop\shims"
                if (Test-Path $scoopShims) { $env:PATH = "$scoopShims;$env:PATH" }

                if ((Test-Command "rsync") -or (Test-Path (Join-Path $scoopShims "rsync.exe"))) {
                    Write-Status "rsync installed successfully via Scoop ($pkg)" -Type "Success"
                    return $true
                }
            }
        }
        catch {
            Write-Status "Scoop rsync install failed: $_" -Type "Warning"
        }
    }

    # Fallback: check if WSL has rsync (very likely on dev machines)
    try {
        $wslRsync = wsl which rsync 2>$null
        if ($wslRsync -and $wslRsync.Trim() -ne "") {
            Write-Status "rsync found in WSL at $($wslRsync.Trim()) - deploy will use 'wsl rsync'" -Type "Success"
            return $true
        }
    }
    catch { }

    Write-Status "Could not install rsync automatically." -Type "Warning"
    Write-Status "Deploy will use scp (works but uploads all files each time)." -Type "Warning"
    Write-Status "Options to fix:" -Type "Info"
    Write-Status "  1. In WSL: sudo apt install rsync" -Type "Info"
    Write-Status "  2. In PowerShell: scoop bucket add extras; scoop install cwrsync" -Type "Info"
    return $false  # Not installed - summary will show as optional/warning
}

function Install-TauriCLI {
    Write-Status "Checking Tauri CLI installation..." -Type "Step"

    if (-not (Test-Command "npm")) {
        Write-Status "npm not found. Please install Node.js first." -Type "Error"
        return $false
    }

    # Check for Tauri CLI
    $tauriVersion = $null
    try {
        $tauriVersion = (npm list -g @tauri-apps/cli 2>$null | Select-String "@tauri-apps/cli@(\d+\.\d+\.\d+)").Matches.Groups[1].Value
    }
    catch {
        # Not installed or error checking
    }

    if ($tauriVersion -and (Compare-Version -Actual $tauriVersion -Required "2.0.0")) {
        Write-Status "Tauri CLI v$tauriVersion already installed globally" -Type "Success"
        return $true
    }

    # Also check local installation
    try {
        $localTauri = npm list @tauri-apps/cli 2>$null | Select-String "@tauri-apps/cli@(\d+\.\d+\.\d+)"
        if ($localTauri) {
            $localVersion = $localTauri.Matches.Groups[1].Value
            Write-Status "Tauri CLI v$localVersion found in project dependencies" -Type "Success"
            return $true
        }
    }
    catch {
        # Not in local dependencies
    }

    Write-Status "Installing Tauri CLI globally..." -Type "Info"

    if (-not (Get-UserConfirmation "Install @tauri-apps/cli globally via npm?")) {
        Write-Status "Skipping Tauri CLI global installation" -Type "Warning"
        Write-Status "You can install it locally with: npm install @tauri-apps/cli" -Type "Info"
        return $true
    }

    try {
        npm install -g @tauri-apps/cli

        # Verify installation
        $newVersion = (npm list -g @tauri-apps/cli 2>$null | Select-String "@tauri-apps/cli@(\d+\.\d+\.\d+)").Matches.Groups[1].Value
        if ($newVersion) {
            Write-Status "Tauri CLI v$newVersion installed successfully" -Type "Success"
            return $true
        }
    }
    catch {
        Write-Status "Failed to install Tauri CLI: $_" -Type "Error"
    }

    Write-Status "Tauri CLI installation may have failed. Try: npm install -g @tauri-apps/cli" -Type "Warning"
    return $false
}

function Show-Summary {
    param(
        [hashtable]$Results
    )

    Write-Host "`n" -NoNewline
    Write-Host "==========================================================" -ForegroundColor $Colors.Cyan
    Write-Host "                  Installation Summary                   " -ForegroundColor $Colors.Cyan
    Write-Host "==========================================================" -ForegroundColor $Colors.Cyan
    Write-Host ""

    $allSuccess = $true

    foreach ($component in @("Git", "Rust", "Node.js", "VS Build Tools", "Tauri CLI")) {
        $status = $Results[$component]
        if ($status) {
            Write-Host "  [+] $component" -ForegroundColor $Colors.Green
        }
        else {
            Write-Host "  [x] $component" -ForegroundColor $Colors.Red
            $allSuccess = $false
        }
    }

    # rsync is optional - show status but don't fail
    # Use -eq $true (not just truthy) to avoid false-positive from captured stdout arrays
    if ($Results["rsync"] -eq $true) {
        Write-Host "  [+] rsync (efficient VPS deploy)" -ForegroundColor $Colors.Green
    }
    else {
        Write-Host "  [-] rsync (optional - deploy falls back to scp)" -ForegroundColor $Colors.Yellow
    }

    Write-Host ""
    Write-Host "==========================================================" -ForegroundColor $Colors.Cyan

    if ($allSuccess) {
        Write-Host "`nAll dependencies installed successfully!" -ForegroundColor $Colors.Green
        # Detect if launched from ultimaforge.bat (it sets INTERACTIVE=1)
        if ($env:INTERACTIVE -eq "1" -or $env:ULTIMAFORGE_MENU -eq "1") {
            Write-Host "`nReturn to the menu to continue setup." -ForegroundColor $Colors.White
        } else {
            Write-Host "`nNext steps:" -ForegroundColor $Colors.White
            Write-Host "  1. Open a new terminal (to refresh PATH)" -ForegroundColor $Colors.White
            Write-Host "  2. Run: npm install" -ForegroundColor $Colors.White
            Write-Host "  3. Run: npm run tauri dev" -ForegroundColor $Colors.White
        }
        Write-Host ""
        return 0
    }
    else {
        Write-Host "`nSome dependencies failed to install." -ForegroundColor $Colors.Yellow
        Write-Host "Please check the errors above and try again." -ForegroundColor $Colors.Yellow
        Write-Host ""
        return 1
    }
}

# Main execution
function Main {
    if ($Help) {
        Show-Help
        exit 0
    }

    Write-Host ""
    Write-Host "============================================================" -ForegroundColor $Colors.Cyan
    Write-Host "|           UltimaForge Setup Script (Windows)             |" -ForegroundColor $Colors.Cyan
    Write-Host "============================================================" -ForegroundColor $Colors.Cyan
    Write-Host ""

    # Detect CI environment
    if ($env:CI -or $env:GITHUB_ACTIONS -or $env:TF_BUILD) {
        $script:SkipPrompts = $true
        Write-Status "CI environment detected, running non-interactively" -Type "Info"
    }

    if ($UseScoop) {
        Write-Status "Using Scoop package manager (no admin required)" -Type "Info"
    }
    elseif (-not (Test-AdminRights)) {
        Write-Status "Running without administrator rights" -Type "Warning"
        Write-Status "VS Build Tools installation may require admin rights" -Type "Warning"
        Write-Status "Use -UseScoop flag for completely non-admin installation" -Type "Info"
        Write-Host ""
    }

    # Track results
    $results = @{
        "Git" = $false
        "Rust" = $false
        "Node.js" = $false
        "VS Build Tools" = $false
        "Tauri CLI" = $false
        "rsync" = $false
    }

    # Install dependencies in order (Git first — needed for updates)
    $results["Git"] = Install-Git
    $results["Rust"] = Install-Rust
    $results["Node.js"] = Install-NodeJS
    $results["VS Build Tools"] = Install-VSBuildTools
    $results["Tauri CLI"] = Install-TauriCLI
    $results["rsync"] = Install-Rsync

    # Show summary and exit
    $exitCode = Show-Summary -Results $results

    # Pause in interactive mode so users can see output before window closes
    if (-not $SkipPrompts) {
        Write-Host ""
        Read-Host "Press Enter to close"
    }

    exit $exitCode
}

# Run main function
Main
