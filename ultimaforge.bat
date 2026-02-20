@echo off
setlocal enabledelayedexpansion

REM UltimaForge All-in-One Development Tool
REM This batch file handles all development tasks

REM Check for command-line argument (non-interactive mode)
if not "%~1"=="" (
    set "choice=%~1"
    set "INTERACTIVE=0"
    goto DISPATCH
)

set "INTERACTIVE=1"
goto MENU

REM ============================================================================
REM DISPATCH - Non-interactive mode routing
REM ============================================================================
:DISPATCH
if /i "%choice%"=="0" goto INSTALL_PREREQS
if /i "%choice%"=="1" goto QUICK_START
if /i "%choice%"=="2" goto SYNC_BRANDING
if /i "%choice%"=="3" goto NPM_INSTALL
if /i "%choice%"=="4" goto GEN_MANIFEST
if /i "%choice%"=="5" goto START_SERVER
if /i "%choice%"=="6" goto START_LAUNCHER
if /i "%choice%"=="7" goto BUILD
if /i "%choice%"=="8" goto CLEAN
if /i "%choice%"=="9" goto TEST
if /i "%choice%"=="A" goto PUBLISH_UPDATE
if /i "%choice%"=="B" goto GENERATE_ICONS
if /i "%choice%"=="C" goto PUBLISH_LAUNCHER_UPDATE
if /i "%choice%"=="D" goto SERVER_OWNER_WIZARD
if /i "%choice%"=="E" goto PUBLISH_ALL
if /i "%choice%"=="F" goto DEV_ALL
if /i "%choice%"=="G" goto PUBLISH_LAUNCHER_ONLY
if /i "%choice%"=="H" goto SETUP_VPS
if /i "%choice%"=="I" goto DEPLOY_VPS
if /i "%choice%"=="X" goto END

echo Invalid choice: %choice%
exit /b 1

:MENU
cls
echo.
echo ========================================
echo    UltimaForge Development Tool
echo ========================================
echo.
echo What would you like to do?
echo.
echo  [0] Install Prerequisites (first-time setup)
echo  [1] Quick Start (Sync + Server + Launcher)
echo  [2] Sync Branding Only
echo  [3] Install Dependencies (npm install)
echo  [4] Generate Test Manifest (v1.0.0)
echo  [5] Start Test Server Only
echo  [6] Start Launcher Only
echo  [7] Build Production
echo  [8] Clean Everything
echo  [9] Run All Tests
echo  [A] Publish New Test Version (for update testing)
echo  [B] Generate App Icons from Branding
echo  [C] Publish Launcher Update Metadata
echo  [D] Server Owner Wizard (branding + keys)
echo  [E] Publish All (game + launcher)
echo  [F] Dev All-in-One (server + launcher)
echo  [G] Publish Launcher Only (fast)
echo  [H] Setup VPS (first-time)
echo  [I] Deploy to VPS
echo  [X] Exit
echo.
echo ========================================
echo.
set /p choice="Enter your choice (0-9, A-I, X): "

if /i "%choice%"=="0" goto INSTALL_PREREQS
if /i "%choice%"=="1" goto QUICK_START
if /i "%choice%"=="2" goto SYNC_BRANDING
if /i "%choice%"=="3" goto NPM_INSTALL
if /i "%choice%"=="4" goto GEN_MANIFEST
if /i "%choice%"=="5" goto START_SERVER
if /i "%choice%"=="6" goto START_LAUNCHER
if /i "%choice%"=="7" goto BUILD
if /i "%choice%"=="8" goto CLEAN
if /i "%choice%"=="9" goto TEST
if /i "%choice%"=="A" goto PUBLISH_UPDATE
if /i "%choice%"=="B" goto GENERATE_ICONS
if /i "%choice%"=="C" goto PUBLISH_LAUNCHER_UPDATE
if /i "%choice%"=="D" goto SERVER_OWNER_WIZARD
if /i "%choice%"=="E" goto PUBLISH_ALL
if /i "%choice%"=="F" goto DEV_ALL
if /i "%choice%"=="G" goto PUBLISH_LAUNCHER_ONLY
if /i "%choice%"=="H" goto SETUP_VPS
if /i "%choice%"=="I" goto DEPLOY_VPS
if /i "%choice%"=="X" goto END

echo Invalid choice. Please try again.
timeout /t 2 >nul
goto MENU

REM ============================================================================
REM INSTALL PREREQUISITES
REM ============================================================================
:INSTALL_PREREQS
cls
echo.
echo ========================================
echo    Install Prerequisites
echo ========================================
echo.
echo This will install:
echo   - Rust (via rustup)
echo   - Node.js LTS
echo   - Visual Studio Build Tools
echo   - Tauri CLI
echo   - rsync (optional, for efficient VPS deploys)
echo.
echo Press any key to start installation...
pause >nul

cd app
call scripts\setup.bat
cd ..

echo.
echo Press any key to return to menu...
pause >nul
goto MENU

REM ============================================================================
REM QUICK START - Everything you need
REM ============================================================================
:QUICK_START
cls
echo.
echo ========================================
echo    Quick Start - Full Setup
echo ========================================
echo.

echo [Step 1/5] Syncing branding assets...
call :SYNC_BRANDING_FUNCTION
if errorlevel 1 goto ERROR_EXIT

echo.
echo [Step 2/5] Checking npm dependencies...
if not exist "app\node_modules" (
    echo npm dependencies not found. Installing...
    call :NPM_INSTALL_FUNCTION
    if errorlevel 1 goto ERROR_EXIT
) else (
    echo Dependencies already installed - OK
)

echo.
echo [Step 3/5] Checking test manifest...
if not exist "app\test-updates\manifest.json" (
    echo Test manifest not found. Generating...
    call :GEN_MANIFEST_FUNCTION
    if errorlevel 1 goto ERROR_EXIT
) else (
    echo Test manifest exists - OK
)

echo.
echo [Step 4/5] Starting test server in new window...
start "UltimaForge Server" cmd /k "echo Starting server... && cargo run -p host-server -- --dir ./app/test-updates --port 8080"

echo Waiting for server to start...
timeout /t 3 >nul

echo.
echo [Step 5/5] Starting launcher in new window...
start "UltimaForge Launcher" cmd /k "cd app && echo Starting launcher... && npm run tauri dev"

echo.
echo ========================================
echo  Development Environment Started!
echo ========================================
echo.
echo Two windows opened:
echo   - Test Server (http://localhost:8080)
echo   - Tauri Launcher (dev mode)
echo.
echo Press any key to return to menu...
pause >nul
goto MENU

REM ============================================================================
REM SYNC BRANDING
REM ============================================================================
:SYNC_BRANDING
call :SYNC_BRANDING_FUNCTION
echo.
echo Press any key to return to menu...
pause >nul
goto MENU

:SYNC_BRANDING_FUNCTION
echo.
echo ========================================
echo    Syncing Branding Assets
echo ========================================
echo.

if not exist "app\public" mkdir "app\public"
if not exist "app\public\branding" mkdir "app\public\branding"

set COPIED=0

if exist "branding\hero-bg.png" (
    copy /Y "branding\hero-bg.png" "app\public\branding\hero-bg.png" >nul 2>&1
    if !errorlevel! equ 0 (
        echo [OK] hero-bg.png
        set /a COPIED+=1
    )
)

if exist "branding\sidebar-logo.png" (
    copy /Y "branding\sidebar-logo.png" "app\public\branding\sidebar-logo.png" >nul 2>&1
    if !errorlevel! equ 0 (
        echo [OK] sidebar-logo.png
        set /a COPIED+=1
    )
)

if exist "branding\sidebar-texture.png" (
    copy /Y "branding\sidebar-texture.png" "app\public\branding\sidebar-texture.png" >nul 2>&1
    if !errorlevel! equ 0 (
        echo [OK] sidebar-texture.png
        set /a COPIED+=1
    )
)

if exist "branding\brand.json" (
    copy /Y "branding\brand.json" "app\public\branding\brand.json" >nul 2>&1
    if !errorlevel! equ 0 (
        echo [OK] brand.json
        set /a COPIED+=1
    )
)

for %%f in (branding\*.png branding\*.jpg) do (
    if not "%%~nxf"=="hero-bg.png" if not "%%~nxf"=="sidebar-logo.png" if not "%%~nxf"=="sidebar-texture.png" (
        copy /Y "%%f" "app\public\branding\%%~nxf" >nul 2>&1
        if !errorlevel! equ 0 (
            echo [OK] %%~nxf
            set /a COPIED+=1
        )
    )
)

echo.
echo Synced !COPIED! file(s) to app\public\branding\
exit /b 0

REM ============================================================================
REM NPM INSTALL
REM ============================================================================
:NPM_INSTALL
call :NPM_INSTALL_FUNCTION
if errorlevel 1 goto ERROR_EXIT
echo.
echo Press any key to return to menu...
pause >nul
goto MENU

:NPM_INSTALL_FUNCTION
echo.
echo ========================================
echo    Installing npm Dependencies
echo ========================================
echo.

cd app
call npm install
set RESULT=%errorlevel%
cd ..

if %RESULT% neq 0 (
    echo.
    echo ERROR: npm install failed
    exit /b 1
)

echo.
echo Dependencies installed successfully!
exit /b 0

REM ============================================================================
REM NPM CLEAN INSTALL (optional-deps fix)
REM ============================================================================
:NPM_CLEAN_INSTALL_FUNCTION
echo.
echo ========================================
echo    Repairing npm Dependencies
echo ========================================
echo.

if exist "app\node_modules" (
    rmdir /s /q "app\node_modules"
)
if exist "app\package-lock.json" (
    del /f /q "app\package-lock.json"
)

cd app
call npm install
set RESULT=%errorlevel%
cd ..

if %RESULT% neq 0 (
    echo.
    echo ERROR: npm install failed
    exit /b 1
)

echo.
echo Dependencies repaired successfully!
exit /b 0

REM ============================================================================
REM GENERATE TEST MANIFEST
REM ============================================================================
:GEN_MANIFEST
call :GEN_MANIFEST_FUNCTION
if errorlevel 1 goto ERROR_EXIT
echo.
echo Press any key to return to menu...
pause >nul
goto MENU

:GEN_MANIFEST_FUNCTION
echo.
echo ========================================
echo    Generating Test Manifest
echo ========================================
echo.

if not exist "app\test-updates" mkdir "app\test-updates"

cargo run -p publish-cli -- publish --source ./app/test-data/sample-client --output ./app/test-updates --key ./app/test-keys/private.key --version 1.0.0
set RESULT=%errorlevel%

if %RESULT% neq 0 (
    echo.
    echo ERROR: Failed to generate manifest
    exit /b 1
)

echo.
echo Test manifest generated successfully!
exit /b 0

REM ============================================================================
REM START SERVER
REM ============================================================================
:START_SERVER
cls
echo.
echo ========================================
echo    Starting Test Server
echo ========================================
echo.

if not exist "app\test-updates\manifest.json" (
    echo ERROR: Test manifest not found!
    echo Run option [4] to generate it first.
    echo.
    echo Press any key to return to menu...
    pause >nul
    goto MENU
)

echo Server will start at: http://localhost:8080
echo.
echo Press Ctrl+C to stop the server
echo.

cargo run -p host-server -- --dir ./app/test-updates --port 8080

echo.
echo Server stopped.
echo.
echo Press any key to return to menu...
pause >nul
goto MENU

REM ============================================================================
REM START LAUNCHER
REM ============================================================================
:START_LAUNCHER
cls
echo.
echo ========================================
echo    Starting Launcher (Dev Mode)
echo ========================================
echo.

if not exist "app\node_modules" (
    echo WARNING: node_modules not found
    echo Installing dependencies first...
    call :NPM_INSTALL_FUNCTION
    if errorlevel 1 goto ERROR_EXIT
)

echo Syncing branding...
call :SYNC_BRANDING_FUNCTION

echo.
echo Starting Tauri dev server...
echo.

cd app
npm run tauri dev
cd ..

echo.
echo Launcher stopped.
echo.
echo Press any key to return to menu...
pause >nul
goto MENU

REM ============================================================================
REM BUILD PRODUCTION
REM ============================================================================
:BUILD
cls
echo.
echo ========================================
echo    Building Production Launcher
echo ========================================
echo.

echo [1/5] Syncing branding to config...
cd app
node sync-branding-config.js
set RESULT=%errorlevel%
cd ..

if %RESULT% neq 0 (
    echo ERROR: Failed to sync branding config
    goto ERROR_EXIT
)

echo.
echo [2/5] Syncing branding assets...
call :SYNC_BRANDING_FUNCTION

echo.
echo [3/5] Installing dependencies...
if not exist "app\node_modules" (
    call :NPM_INSTALL_FUNCTION
    if errorlevel 1 goto ERROR_EXIT
) else (
    echo Dependencies OK
)

REM Auto-fix optional dependency issues (Rollup/Tauri native bindings)
cd app
node -e "require('@rollup/rollup-win32-x64-msvc')" >nul 2>nul
set ROLLUP_OK=%errorlevel%
node -e "require('@tauri-apps/cli-win32-x64-msvc')" >nul 2>nul
set TAURI_OK=%errorlevel%
cd ..

if not "%ROLLUP_OK%"=="0" (
    echo.
    echo Detected missing Rollup native binding. Repairing dependencies...
    call :NPM_CLEAN_INSTALL_FUNCTION
    if errorlevel 1 goto ERROR_EXIT
)
if not "%TAURI_OK%"=="0" (
    echo.
    echo Detected missing Tauri native binding. Repairing dependencies...
    call :NPM_CLEAN_INSTALL_FUNCTION
    if errorlevel 1 goto ERROR_EXIT
)

echo.
echo [4/5] Building frontend...
cd app
call npm run build
set RESULT=%errorlevel%
cd ..

if %RESULT% neq 0 (
    echo ERROR: Frontend build failed
    goto ERROR_EXIT
)

echo.
echo [5/5] Building Tauri application...
echo This will take several minutes...
cd app
if exist "..\\keys\\tauri-updater\\tauri.key" (
    echo Using Tauri updater signing key from keys\\tauri-updater\\tauri.key
    set "TAURI_SIGNING_PRIVATE_KEY_PATH=..\\keys\\tauri-updater\\tauri.key"
    if exist "..\\keys\\tauri-updater\\password.txt" (
        for /f "usebackq delims=" %%p in ("..\\keys\\tauri-updater\\password.txt") do set "TAURI_SIGNING_PRIVATE_KEY_PASSWORD=%%p"
    )
)
call npm run tauri build
set RESULT=%errorlevel%
cd ..

if %RESULT% neq 0 (
    echo ERROR: Tauri build failed
    goto ERROR_EXIT
)

echo.
echo ========================================
echo    Build Complete!
echo ========================================
echo.
echo Your launcher is at:
echo   app\src-tauri\target\release\ultimaforge.exe
echo.
dir /B "app\src-tauri\target\release\*.exe" 2>nul
echo.
echo Press any key to return to menu...
pause >nul
goto MENU

REM ============================================================================
REM CLEAN
REM ============================================================================
:CLEAN
cls
echo.
echo ========================================
echo    Clean Build Artifacts
echo ========================================
echo.
echo This will delete:
echo   - app\node_modules
echo   - app\target
echo   - app\dist
echo   - app\Cargo.lock
echo   - app\package-lock.json
echo.
set /p confirm="Are you sure? (y/N): "

if /i not "%confirm%"=="y" (
    echo Cancelled.
    echo.
    echo Press any key to return to menu...
    pause >nul
    goto MENU
)

echo.
echo Cleaning...

if exist "app\node_modules" (
    echo Removing app\node_modules...
    rmdir /S /Q "app\node_modules" 2>nul
)

if exist "app\target" (
    echo Removing app\target...
    rmdir /S /Q "app\target" 2>nul
)

if exist "app\src-tauri\target" (
    echo Removing app\src-tauri\target...
    rmdir /S /Q "app\src-tauri\target" 2>nul
)

if exist "app\dist" (
    echo Removing app\dist...
    rmdir /S /Q "app\dist" 2>nul
)

if exist "app\Cargo.lock" del /Q "app\Cargo.lock" 2>nul
if exist "app\package-lock.json" del /Q "app\package-lock.json" 2>nul

echo.
echo Clean complete!
echo.
echo Press any key to return to menu...
pause >nul
goto MENU

REM ============================================================================
REM PUBLISH NEW TEST VERSION
REM ============================================================================
:PUBLISH_UPDATE
cls
echo.
echo ========================================
echo    Publish New Test Version
echo ========================================
echo.
echo This will create a new version for testing updates.
echo.
set /p new_version="Enter new version (e.g., 1.0.1): "

if "%new_version%"=="" (
    echo Error: Version cannot be empty
    echo.
    echo Press any key to return to menu...
    pause >nul
    goto MENU
)

echo.
echo Creating test file changes...

REM Create a timestamp file to simulate a change
if not exist "app\test-data\sample-client" mkdir "app\test-data\sample-client"
echo Build Date: %DATE% %TIME% > app\test-data\sample-client\build-info.txt
echo Version: %new_version% >> app\test-data\sample-client\build-info.txt

echo.
echo Publishing version %new_version%...
echo.

cargo run -p publish-cli -- publish --source ./app/test-data/sample-client --output ./app/test-updates --key ./app/test-keys/private.key --version %new_version%
set RESULT=%errorlevel%

if %RESULT% neq 0 (
    echo.
    echo ERROR: Failed to publish new version
    echo.
    echo Press any key to return to menu...
    pause >nul
    goto MENU
)

echo.
echo ========================================
echo  New Version Published!
echo ========================================
echo.
echo Version %new_version% is now available on the test server.
echo.
echo Next steps:
echo   1. Make sure the test server is running (option 5)
echo   2. Start the launcher (option 6)
echo   3. The launcher should detect the update
echo.
echo Press any key to return to menu...
pause >nul
goto MENU

REM ============================================================================
REM PUBLISH LAUNCHER UPDATE METADATA
REM ============================================================================
:PUBLISH_LAUNCHER_UPDATE
cls
echo.
echo ========================================
echo    Publish Launcher Update Metadata
echo ========================================
echo.
echo This will generate Tauri updater metadata and copy the launcher binary
echo to app\test-updates\launcher for the built-in host server.
echo.
echo You will need a valid Tauri updater signature string.
echo.
powershell -ExecutionPolicy Bypass -File app\scripts\publish-launcher-update.ps1 -OutputDir app\test-updates\launcher -BaseUrl http://localhost:8080

echo.
echo Press any key to return to menu...
pause >nul
goto MENU

REM ============================================================================
REM SERVER OWNER WIZARD
REM ============================================================================
:SERVER_OWNER_WIZARD
cls
echo.
echo ========================================
echo    Server Owner Wizard
echo ========================================
echo.
echo This will guide you through branding setup and key generation.
echo.
node app\scripts\server-owner-wizard.js
echo.
echo ========================================
echo    Launcher Updater Key Setup
echo ========================================
echo.
echo This will generate or configure the Tauri updater keypair
echo and embed the public key into the launcher config.
echo.
node app\scripts\configure-launcher-updater.js

echo.
echo Press any key to return to menu...
pause >nul
goto MENU

REM ============================================================================
REM PUBLISH ALL (GAME + LAUNCHER)
REM ============================================================================
:PUBLISH_ALL
cls
echo.
echo ========================================
echo    Publish All (Game + Launcher)
echo ========================================
echo.
echo This will publish game updates and launcher update metadata.
echo.
node app\scripts\publish-all.js

echo.
echo Press any key to return to menu...
pause >nul
goto MENU

REM ============================================================================
REM PUBLISH LAUNCHER ONLY (FAST)
REM ============================================================================
:PUBLISH_LAUNCHER_ONLY
cls
echo.
echo ========================================
echo    Publish Launcher Only (Fast)
echo ========================================
echo.
echo This will build and publish launcher updates only.
echo (Skips game update manifest/blob generation.)
echo.
node app\scripts\publish-all.js --launcher-only true

echo.
echo Press any key to return to menu...
pause >nul
goto MENU

REM ============================================================================
REM SETUP VPS (FIRST-TIME)
REM ============================================================================
:SETUP_VPS
cls
echo.
echo ========================================
echo    Setup VPS (First-Time)
echo ========================================
echo.
echo This will guide you through setting up a VPS to host game updates.
echo You will need a VPS (e.g. Digital Ocean) and a domain name pointed at it.
echo.
node app\scripts\setup-vps.js

echo.
echo Press any key to return to menu...
pause >nul
goto MENU

REM ============================================================================
REM DEPLOY TO VPS
REM ============================================================================
:DEPLOY_VPS
cls
echo.
echo ========================================
echo    Deploy to VPS
echo ========================================
echo.
echo This will sync your published files to your VPS.
echo Run Option E first to publish, then Option I to deploy.
echo.

REM Refresh user PATH so rsync installed by Scoop/cwrsync is visible
for /f "usebackq tokens=*" %%p in (`powershell -NoProfile -Command "[Environment]::GetEnvironmentVariable('PATH','User')"`) do set "PATH=%%p;%PATH%"

REM Also add common rsync install locations directly
if exist "%USERPROFILE%\scoop\shims\rsync.exe" set "PATH=%USERPROFILE%\scoop\shims;%PATH%"
for /d %%d in ("%ProgramFiles%\cwRsync*") do (
    if exist "%%d\bin\rsync.exe" set "PATH=%%d\bin;%PATH%"
)
for /d %%d in ("%ProgramFiles(x86)%\cwRsync*") do (
    if exist "%%d\bin\rsync.exe" set "PATH=%%d\bin;%PATH%"
)

node app\scripts\deploy.js

echo.
echo Press any key to return to menu...
pause >nul
goto MENU

REM ============================================================================
REM DEV ALL-IN-ONE
REM ============================================================================
:DEV_ALL
cls
echo.
echo ========================================
echo    Dev All-in-One
echo ========================================
echo.
echo This will start the host server and launcher in a single terminal.
echo.
node app\scripts\dev-all-in-one.js

echo.
echo Press any key to return to menu...
pause >nul
goto MENU

REM ============================================================================
REM GENERATE APP ICONS FROM BRANDING
REM ============================================================================
:GENERATE_ICONS
cls
echo.
echo ========================================
echo    Generate App Icons from Branding
echo ========================================
echo.

if not exist "branding\sidebar-logo.png" (
    echo ERROR: branding\sidebar-logo.png not found!
    echo.
    echo Please ensure your logo exists at:
    echo   branding\sidebar-logo.png
    echo.
    echo Requirements:
    echo   - Square PNG (1024x1024 recommended)
    echo   - Transparent background (RGBA)
    echo   - Clear at small sizes
    echo.
    echo Press any key to return to menu...
    pause >nul
    goto MENU
)

echo Generating app icons from branding\sidebar-logo.png...
echo.

REM Ensure icons directory exists
if not exist "app\src-tauri\icons" mkdir "app\src-tauri\icons"

REM Generate PNG icons
powershell -ExecutionPolicy Bypass -Command "Add-Type -AssemblyName System.Drawing; $source = 'branding/sidebar-logo.png'; $outputDir = 'app/src-tauri/icons'; $img = [System.Drawing.Image]::FromFile((Resolve-Path $source)); function Resize-Image($size, $filename) { $newImg = New-Object System.Drawing.Bitmap($size, $size); $graphics = [System.Drawing.Graphics]::FromImage($newImg); $graphics.InterpolationMode = [System.Drawing.Drawing2D.InterpolationMode]::HighQualityBicubic; $graphics.SmoothingMode = [System.Drawing.Drawing2D.SmoothingMode]::HighQuality; $graphics.DrawImage($img, 0, 0, $size, $size); $newImg.Save(\"$outputDir/$filename\", [System.Drawing.Imaging.ImageFormat]::Png); $graphics.Dispose(); $newImg.Dispose(); Write-Host \"Created $filename (${size}x${size})\" }; Resize-Image 32 '32x32.png'; Resize-Image 128 '128x128.png'; Resize-Image 256 '128x128@2x.png'; Resize-Image 256 'icon.png'; $img.Dispose(); Write-Host ''"

REM Generate proper multi-resolution .ico file
echo.
cd app
powershell -ExecutionPolicy Bypass -File "generate-ico.ps1"
set RESULT=%errorlevel%
cd ..

if %RESULT% neq 0 (
    echo.
    echo ERROR: Failed to generate icons
    echo.
    echo Press any key to return to menu...
    pause >nul
    goto MENU
)

REM Generate installer branding images
echo.
cd app
powershell -ExecutionPolicy Bypass -File "generate-installer-images.ps1"
cd ..

if errorlevel 1 (
    echo.
    echo WARNING: Installer images may have issues, but icons generated OK
)

echo.
echo ========================================
echo  Icons Generated Successfully!
echo ========================================
echo.
echo Generated in app\src-tauri\icons\:
echo   - 32x32.png (taskbar, small icons)
echo   - 128x128.png (standard size)
echo   - 128x128@2x.png (retina displays)
echo   - icon.png (256x256 main icon)
echo   - icon.ico (Windows multi-resolution)
echo.
echo Generated in app\src-tauri\installer-assets\:
echo   - nsis-header.bmp (installer header)
echo   - nsis-sidebar.bmp (installer wizard sidebar)
echo.
echo Your branding will appear in:
echo   - Application window
echo   - Taskbar
echo   - Desktop shortcut
echo   - Add/Remove Programs
echo   - NSIS Installer screens
echo.
echo Next: Build production (option 7) to create installer
echo.
echo Press any key to return to menu...
pause >nul
goto MENU

REM ============================================================================
REM RUN TESTS
REM ============================================================================
:TEST
cls
echo.
echo ========================================
echo    Running Tests
echo ========================================
echo.

set TEST_FAILURES=0

echo [Step 1/2] Running Rust tests...
echo.
cargo test
set RUST_RESULT=%errorlevel%

if %RUST_RESULT% neq 0 (
    echo.
    echo [FAIL] Rust tests failed with exit code %RUST_RESULT%
    set /a TEST_FAILURES+=1
    goto TEST_SUMMARY
)

echo.
echo [PASS] Rust tests passed
echo.

echo [Step 2/2] Running npm tests...
echo.
cd app
call npm test
set NPM_RESULT=%errorlevel%
cd ..

if %NPM_RESULT% neq 0 (
    echo.
    echo [FAIL] npm tests failed with exit code %NPM_RESULT%
    set /a TEST_FAILURES+=1
    goto TEST_SUMMARY
)

echo.
echo [PASS] npm tests passed

:TEST_SUMMARY
echo.
echo ========================================
if %TEST_FAILURES% equ 0 (
    echo    All Tests Passed!
    echo    Tests complete
) else (
    echo    Tests Failed! (%TEST_FAILURES% failure^(s^)^)
)
echo ========================================
echo.

REM Non-interactive mode: exit with proper error code
if "%INTERACTIVE%"=="0" (
    if %TEST_FAILURES% neq 0 (
        exit /b 1
    )
    exit /b 0
)

echo Press any key to return to menu...
pause >nul
goto MENU

REM ============================================================================
REM ERROR HANDLING
REM ============================================================================
:ERROR_EXIT
echo.
echo ========================================
echo    An error occurred!
echo ========================================
echo.
echo Press any key to return to menu...
pause >nul
goto MENU

REM ============================================================================
REM EXIT
REM ============================================================================
:END
echo.
echo Goodbye!
timeout /t 1 >nul
exit /b 0
