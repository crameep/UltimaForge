@echo off
setlocal enabledelayedexpansion

REM UltimaForge All-in-One Development Tool
REM This batch file handles all development tasks

:MENU
cls
echo.
echo ========================================
echo    UltimaForge Development Tool
echo ========================================
echo.
echo What would you like to do?
echo.
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
echo  [0] Exit
echo.
echo ========================================
echo.
set /p choice="Enter your choice (0-9, A): "

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
if /i "%choice%"=="0" goto END

echo Invalid choice. Please try again.
timeout /t 2 >nul
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
if not exist "node_modules" (
    echo npm dependencies not found. Installing...
    call :NPM_INSTALL_FUNCTION
    if errorlevel 1 goto ERROR_EXIT
) else (
    echo Dependencies already installed - OK
)

echo.
echo [Step 3/5] Checking test manifest...
if not exist "test-updates\manifest.json" (
    echo Test manifest not found. Generating...
    call :GEN_MANIFEST_FUNCTION
    if errorlevel 1 goto ERROR_EXIT
) else (
    echo Test manifest exists - OK
)

echo.
echo [Step 4/5] Starting test server in new window...
start "UltimaForge Server" cmd /k "echo Starting server... && cargo run -p host-server -- --dir ./test-updates --port 8080"

echo Waiting for server to start...
timeout /t 3 >nul

echo.
echo [Step 5/5] Starting launcher in new window...
start "UltimaForge Launcher" cmd /k "echo Starting launcher... && npm run tauri dev"

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

if not exist "public" mkdir "public"
if not exist "public\branding" mkdir "public\branding"

set COPIED=0

if exist "branding\hero-bg.png" (
    copy /Y "branding\hero-bg.png" "public\branding\hero-bg.png" >nul 2>&1
    if !errorlevel! equ 0 (
        echo [OK] hero-bg.png
        set /a COPIED+=1
    )
)

if exist "branding\sidebar-logo.png" (
    copy /Y "branding\sidebar-logo.png" "public\branding\sidebar-logo.png" >nul 2>&1
    if !errorlevel! equ 0 (
        echo [OK] sidebar-logo.png
        set /a COPIED+=1
    )
)

for %%f in (branding\*.png branding\*.jpg) do (
    if not "%%~nxf"=="hero-bg.png" if not "%%~nxf"=="sidebar-logo.png" (
        copy /Y "%%f" "public\branding\%%~nxf" >nul 2>&1
        if !errorlevel! equ 0 (
            echo [OK] %%~nxf
            set /a COPIED+=1
        )
    )
)

echo.
echo Synced !COPIED! file(s) to public\branding\
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

call npm install
if errorlevel 1 (
    echo.
    echo ERROR: npm install failed
    exit /b 1
)

echo.
echo Dependencies installed successfully!
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

if not exist "test-updates" mkdir "test-updates"

cargo run -p publish-cli -- publish --source ./test-data/sample-client --output ./test-updates --key ./test-keys/private.key --version 1.0.0
if errorlevel 1 (
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

if not exist "test-updates\manifest.json" (
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

cargo run -p host-server -- --dir ./test-updates --port 8080

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

if not exist "node_modules" (
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

npm run tauri dev

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

echo [1/4] Syncing branding...
call :SYNC_BRANDING_FUNCTION

echo.
echo [2/4] Installing dependencies...
if not exist "node_modules" (
    call :NPM_INSTALL_FUNCTION
    if errorlevel 1 goto ERROR_EXIT
) else (
    echo Dependencies OK
)

echo.
echo [3/4] Building frontend...
call npm run build
if errorlevel 1 (
    echo ERROR: Frontend build failed
    goto ERROR_EXIT
)

echo.
echo [4/4] Building Tauri application...
echo This will take several minutes...
call npm run tauri build
if errorlevel 1 (
    echo ERROR: Tauri build failed
    goto ERROR_EXIT
)

echo.
echo ========================================
echo    Build Complete!
echo ========================================
echo.
echo Your launcher is at:
echo   src-tauri\target\release\ultimaforge.exe
echo.
dir /B "src-tauri\target\release\*.exe" 2>nul
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
echo   - node_modules
echo   - target
echo   - dist
echo   - Cargo.lock
echo   - package-lock.json
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

if exist "node_modules" (
    echo Removing node_modules...
    rmdir /S /Q "node_modules" 2>nul
)

if exist "target" (
    echo Removing target...
    rmdir /S /Q "target" 2>nul
)

if exist "src-tauri\target" (
    echo Removing src-tauri\target...
    rmdir /S /Q "src-tauri\target" 2>nul
)

if exist "dist" (
    echo Removing dist...
    rmdir /S /Q "dist" 2>nul
)

if exist "Cargo.lock" del /Q "Cargo.lock" 2>nul
if exist "package-lock.json" del /Q "package-lock.json" 2>nul

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
echo Build Date: %DATE% %TIME% > test-data\sample-client\build-info.txt
echo Version: %new_version% >> test-data\sample-client\build-info.txt

echo.
echo Publishing version %new_version%...
echo.

cargo run -p publish-cli -- publish --source ./test-data/sample-client --output ./test-updates --key ./test-keys/private.key --version %new_version%

if errorlevel 1 (
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
REM RUN TESTS
REM ============================================================================
:TEST
cls
echo.
echo ========================================
echo    Running Tests
echo ========================================
echo.

echo Running Rust tests...
cargo test

echo.
echo Running npm tests...
npm test

echo.
echo Tests complete!
echo.
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
