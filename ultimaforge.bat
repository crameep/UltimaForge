@echo off
setlocal enabledelayedexpansion

REM UltimaForge Server Owner Tools
REM This batch file handles all server owner tasks

REM Initialize MSVC environment (link.exe, cl.exe) if not already set
if not defined VCINSTALLDIR call :INIT_MSVC

REM Force x64 Rust toolchain via env var (works immediately, no restart needed).
REM On ARM64 machines the aarch64 host toolchain can't use the x64 VS linker.
REM RUSTUP_TOOLCHAIN overrides the default toolchain for all cargo/rustc calls.
REM On x64 machines this is already the default toolchain so it's a no-op.
where rustup >nul 2>nul
if not errorlevel 1 (
    set "RUSTUP_TOOLCHAIN=stable-x86_64-pc-windows-msvc"
    REM Only install the toolchain on ARM64 where it's not the default
    if /i "%PROCESSOR_ARCHITECTURE%"=="ARM64" (
        rustup toolchain install stable-x86_64-pc-windows-msvc >nul 2>nul
    )
)

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
REM New main menu mappings (server owner steps 1-7)
if /i "%choice%"=="1" goto INSTALL_PREREQS
if /i "%choice%"=="2" goto SERVER_OWNER_WIZARD
if /i "%choice%"=="3" goto GENERATE_ICONS
if /i "%choice%"=="4" goto BUILD
if /i "%choice%"=="5" goto SETUP_VPS
if /i "%choice%"=="6" goto PUBLISH_ALL
if /i "%choice%"=="7" goto DEPLOY_VPS
if /i "%choice%"=="8" goto UPDATE_SOURCE
if /i "%choice%"=="D" goto DEV_MENU
if /i "%choice%"=="X" goto END

REM Legacy letter mappings (backward compatibility)
if /i "%choice%"=="A" goto PUBLISH_UPDATE
if /i "%choice%"=="B" goto GENERATE_ICONS
if /i "%choice%"=="C" goto PUBLISH_LAUNCHER_UPDATE
if /i "%choice%"=="E" goto PUBLISH_ALL
if /i "%choice%"=="F" goto DEV_ALL
if /i "%choice%"=="G" goto PUBLISH_LAUNCHER_ONLY
if /i "%choice%"=="H" goto SETUP_VPS
if /i "%choice%"=="I" goto DEPLOY_VPS

echo Invalid choice: %choice%
exit /b 1

REM ============================================================================
REM MAIN MENU
REM ============================================================================
:MENU
cd /d "%~dp0"
cls
call :CHECK_STATUS

if "%PREREQS_OK%"=="1" (set "S1=[DONE]") else (set "S1=[...]")
if "%BRANDING_OK%"=="1" (set "S2=[DONE]") else (set "S2=[...]")
if "%ICONS_OK%"=="1" (set "S3=[DONE]") else (set "S3=[...]")
if "%BUILD_OK%"=="1" (set "S4=[DONE]") else (set "S4=[...]")
if "%VPS_OK%"=="1" (set "S5=[DONE]") else (set "S5=[...]")

echo.
echo ========================================
echo    UltimaForge - Server Owner Tools
echo ========================================
echo.
echo   FIRST TIME SETUP
echo.
echo   [1] Install Prerequisites          %S1%
echo   [2] Configure Branding ^& Keys      %S2%
echo   [3] Generate App Icons             %S3%
echo   [4] Build Launcher                 %S4%
echo   [5] Setup VPS (optional)           %S5%
echo.
echo   ONGOING
echo.
echo   [6] Publish Game Update
echo   [7] Deploy to VPS
echo   [8] Update Launcher Source
echo.
echo   [D] Developer Tools
echo   [X] Exit
echo.
echo ========================================
echo.
set /p choice="Enter your choice: "

if /i "%choice%"=="1" goto INSTALL_PREREQS
if /i "%choice%"=="2" goto SERVER_OWNER_WIZARD
if /i "%choice%"=="3" goto GENERATE_ICONS
if /i "%choice%"=="4" goto BUILD
if /i "%choice%"=="5" goto SETUP_VPS
if /i "%choice%"=="6" goto PUBLISH_CHOICE
if /i "%choice%"=="7" goto DEPLOY_VPS
if /i "%choice%"=="8" goto UPDATE_SOURCE
if /i "%choice%"=="D" goto DEV_MENU
if /i "%choice%"=="X" goto END

echo Invalid choice. Please try again.
timeout /t 2 >nul
goto MENU

REM ============================================================================
REM CHECK STATUS - Sets status variables for menu display
REM ============================================================================
:CHECK_STATUS
set "PREREQS_OK=0"
set "BRANDING_OK=0"
set "ICONS_OK=0"
set "BUILD_OK=0"
set "VPS_OK=0"

REM Re-init MSVC environment only if not already done (vcvarsall is slow)
if not defined VCINSTALLDIR (
    where link.exe >nul 2>nul
    if errorlevel 1 call :INIT_MSVC
)

REM Also refresh PATH for newly installed tools (node, cargo, git)
REM Use a single powershell call to avoid double cold-start penalty
set "PATH=%USERPROFILE%\.cargo\bin;%PATH%"
for /f "usebackq tokens=*" %%p in (`powershell -NoProfile -Command "[Environment]::GetEnvironmentVariable('PATH','User') + ';' + [Environment]::GetEnvironmentVariable('PATH','Machine')"`) do set "PATH=%%p;%PATH%"

set "PREREQS_OK=1"
where git >nul 2>nul
if errorlevel 1 set "PREREQS_OK=0"
where node >nul 2>nul
if errorlevel 1 set "PREREQS_OK=0"
where cargo >nul 2>nul
if errorlevel 1 set "PREREQS_OK=0"
REM Check for VS Build Tools via VCINSTALLDIR (set by vcvarsall) or vswhere
if not defined VCINSTALLDIR (
    if not exist "%ProgramFiles(x86)%\Microsoft Visual Studio\Installer\vswhere.exe" (
        if not exist "%ProgramFiles%\Microsoft Visual Studio\2022\BuildTools" set "PREREQS_OK=0"
    )
)

REM Check for branding + keys in both legacy (keys\) and new (server-data\keys\) locations
if exist "%~dp0branding\brand.json" (
    if exist "%~dp0keys\private.key" set "BRANDING_OK=1"
    if exist "%~dp0server-data\keys\private.key" set "BRANDING_OK=1"
)

if exist "app\src-tauri\icons\icon.ico" set "ICONS_OK=1"

REM Check for built launcher (exe in target\release or NSIS installer in bundle)
dir /b "app\src-tauri\target\release\*.exe" >nul 2>nul
if not errorlevel 1 set "BUILD_OK=1"
dir /b "app\src-tauri\target\release\bundle\nsis\*.exe" >nul 2>nul
if not errorlevel 1 set "BUILD_OK=1"
dir /b "target\release\*.exe" >nul 2>nul
if not errorlevel 1 set "BUILD_OK=1"

if exist "server-data\deploy.json" set "VPS_OK=1"

exit /b 0

REM ============================================================================
REM DEVELOPER TOOLS SUBMENU
REM ============================================================================
:DEV_MENU
cls
echo.
echo ========================================
echo    Developer Tools
echo ========================================
echo.
echo   [1] Quick Start (Sync + Server + Launcher)
echo   [2] Sync Branding Assets
echo   [3] Install npm Dependencies
echo   [4] Generate Test Manifest (v1.0.0)
echo   [5] Start Test Server
echo   [6] Start Launcher (Dev Mode)
echo   [7] Dev All-in-One
echo   [8] Clean Everything
echo   [9] Run All Tests
echo   [A] Publish New Test Version
echo   [B] Publish Launcher Update Metadata
echo   [C] Build Production (manual)
echo   [D] Undo Last Source Update
echo.
echo   [M] Back to Main Menu
echo.
echo ========================================
echo.
set /p dev_choice="Enter your choice: "

if /i "%dev_choice%"=="1" goto QUICK_START
if /i "%dev_choice%"=="2" goto SYNC_BRANDING
if /i "%dev_choice%"=="3" goto NPM_INSTALL
if /i "%dev_choice%"=="4" goto GEN_MANIFEST
if /i "%dev_choice%"=="5" goto START_SERVER
if /i "%dev_choice%"=="6" goto START_LAUNCHER
if /i "%dev_choice%"=="7" goto DEV_ALL
if /i "%dev_choice%"=="8" goto CLEAN
if /i "%dev_choice%"=="9" goto TEST
if /i "%dev_choice%"=="A" goto PUBLISH_UPDATE
if /i "%dev_choice%"=="B" goto PUBLISH_LAUNCHER_UPDATE
if /i "%dev_choice%"=="C" goto BUILD
if /i "%dev_choice%"=="D" goto UNDO_UPDATE
if /i "%dev_choice%"=="M" goto MENU

echo Invalid choice. Please try again.
timeout /t 2 >nul
goto DEV_MENU

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
echo   - Git (for launcher updates)
echo   - Rust (via rustup)
echo   - Node.js LTS
echo   - Visual Studio Build Tools
echo   - Tauri CLI
echo   - rsync (optional, for efficient VPS deploys)
echo.
echo Note: VS Build Tools installation requires administrator rights.
echo.

REM Check if running as admin
net session >nul 2>nul
if errorlevel 1 (
    echo You are NOT running as administrator.
    echo.
    set /p elevate="Relaunch as administrator? (Y/n): "
    if /i not "!elevate!"=="n" (
        echo Relaunching as administrator...
        echo This window will close. Continue in the new admin window.
        powershell -Command "Start-Process -FilePath '%~f0' -Verb RunAs"
        exit /b 0
    )
    echo.
    echo Continuing without admin. VS Build Tools may fail to install.
    echo.
)

echo Press any key to start installation...
pause >nul

REM Run setup from repo root using absolute path
set "ULTIMAFORGE_MENU=1"
call "%~dp0app\scripts\setup.bat"
set "ULTIMAFORGE_MENU="

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

cd /d "%~dp0app"
call npm install
set RESULT=%errorlevel%
cd /d "%~dp0"

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

if exist "%~dp0app\node_modules" (
    rmdir /s /q "%~dp0app\node_modules"
)
if exist "%~dp0app\package-lock.json" (
    del /f /q "%~dp0app\package-lock.json"
)

cd /d "%~dp0app"
call npm install
set RESULT=%errorlevel%
cd /d "%~dp0"

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

cd /d "%~dp0app"
npm run tauri dev
cd /d "%~dp0"

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
cd /d "%~dp0app"
node sync-branding-config.js
set RESULT=%errorlevel%
cd /d "%~dp0"

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
cd /d "%~dp0app"
node -e "require('@rollup/rollup-win32-x64-msvc')" >nul 2>nul
set ROLLUP_OK=%errorlevel%
node -e "require('@tauri-apps/cli-win32-x64-msvc')" >nul 2>nul
set TAURI_OK=%errorlevel%
cd /d "%~dp0"

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
cd /d "%~dp0app"
call npm run build
set RESULT=%errorlevel%
cd /d "%~dp0"

if %RESULT% neq 0 (
    echo ERROR: Frontend build failed
    goto ERROR_EXIT
)

echo.
echo [5/5] Building Tauri application...
echo This will take several minutes...
cd /d "%~dp0app"
REM Look for Tauri updater keys in both legacy and new locations
set "UPDATER_KEY_DIR="
if exist "..\keys\tauri-updater\tauri.key" set "UPDATER_KEY_DIR=..\keys\tauri-updater"
if exist "..\server-data\keys\tauri-updater\tauri.key" set "UPDATER_KEY_DIR=..\server-data\keys\tauri-updater"
if defined UPDATER_KEY_DIR (
    echo Loading Tauri updater signing key...
    for /f "delims=" %%k in ('node scripts\print-signing-key.js "!UPDATER_KEY_DIR!\tauri.key"') do set "TAURI_SIGNING_PRIVATE_KEY=%%k"
    if exist "!UPDATER_KEY_DIR!\password.txt" (
        for /f "usebackq delims=" %%p in ("!UPDATER_KEY_DIR!\password.txt") do set "TAURI_SIGNING_PRIVATE_KEY_PASSWORD=%%p"
    )
)
call npm run tauri build
set RESULT=%errorlevel%
cd /d "%~dp0"

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
node "%~dp0app\scripts\server-owner-wizard.js"
echo.
echo ========================================
echo    Launcher Updater Key Setup
echo ========================================
echo.
echo This will generate or configure the Tauri updater keypair
echo and embed the public key into the launcher config.
echo.
node "%~dp0app\scripts\configure-launcher-updater.js"

echo.
echo Press any key to return to menu...
pause >nul
goto MENU

REM ============================================================================
REM PUBLISH CHOICE - Sub-choice for game update publishing
REM ============================================================================
:PUBLISH_CHOICE
cls
echo.
echo ========================================
echo    Publish Game Update
echo ========================================
echo.
echo   Publish what?
echo   [1] Full (game + launcher) - default
echo   [2] Game only (fast, skips launcher build)
echo   [3] Launcher only (fast, skips game files)
echo.
echo   [M] Back to Main Menu
echo.
set "pub_choice="
set /p pub_choice="Enter choice (or Enter for Full): "

if "%pub_choice%"=="" goto PUBLISH_ALL
if "%pub_choice%"=="1" goto PUBLISH_ALL
if "%pub_choice%"=="2" goto PUBLISH_GAME_ONLY
if "%pub_choice%"=="3" goto PUBLISH_LAUNCHER_ONLY
if /i "%pub_choice%"=="M" goto MENU

echo Invalid choice. Using Full publish...
goto PUBLISH_ALL

REM ============================================================================
REM PUBLISH GAME ONLY (FAST)
REM ============================================================================
:PUBLISH_GAME_ONLY
cls
echo.
echo ========================================
echo    Publish Game Only (Fast)
echo ========================================
echo.
echo This will publish game file updates only.
echo (Skips launcher build and launcher metadata.)
echo.
node "%~dp0app\scripts\publish-all.js" --game-only true --auto-bump patch --auto-fix-deps true

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
node "%~dp0app\scripts\publish-all.js" --auto-bump patch --auto-fix-deps true

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
node "%~dp0app\scripts\publish-all.js" --launcher-only true --auto-bump patch --auto-fix-deps true

echo.
echo Press any key to return to menu...
pause >nul
goto MENU

REM ============================================================================
REM UNDO LAST SOURCE UPDATE
REM ============================================================================
:UNDO_UPDATE
cls
echo.
echo ========================================
echo    Undo Last Source Update
echo ========================================
echo.
echo This will revert the last upstream merge, restoring your launcher
echo source to the state before the update.
echo.

REM Check if the last commit is a merge from upstream
git log -1 --format="%%s" 2>nul | findstr /i "merge upstream" >nul
if errorlevel 1 (
    echo The last commit doesn't appear to be an upstream update.
    echo.
    echo Last 5 commits:
    git log --oneline -5 2>nul
    echo.
    echo This option only undoes updates applied via option [8].
    echo.
    echo Press any key to return to menu...
    pause >nul
    goto DEV_MENU
)

echo Last update commit:
git log -1 --format="  %%h %%s (%%ar)" 2>nul
echo.

set /p do_undo="Undo this update? (y/N): "
if /i not "%do_undo%"=="y" (
    echo Cancelled.
    echo.
    echo Press any key to return to menu...
    pause >nul
    goto DEV_MENU
)

echo.
echo Reverting...
git reset --hard HEAD~1
if errorlevel 1 (
    echo ERROR: Failed to revert. You may need to resolve this manually.
) else (
    echo.
    echo Update reverted successfully.
    echo You are now back to:
    git log -1 --format="  %%h %%s" 2>nul
)

echo.
echo Press any key to return to menu...
pause >nul
goto DEV_MENU

REM ============================================================================
REM UPDATE LAUNCHER SOURCE FROM UPSTREAM
REM ============================================================================
:UPDATE_SOURCE
cd /d "%~dp0"
cls
echo.
echo ========================================
echo    Update Launcher Source
echo ========================================
echo.
echo This pulls the latest launcher code from the official UltimaForge
echo repository. Your branding, keys, and game files are preserved.
echo.

REM Check if this is a working git repo. If not, initialize it.
REM Also handles broken .git from a previous failed attempt.
git rev-parse HEAD >nul 2>nul
if errorlevel 1 (
    if exist ".git" rmdir /s /q ".git" >nul 2>nul
    call :INIT_GIT_REPO
)

REM Add or update upstream remote
git remote get-url upstream >nul 2>nul
if errorlevel 1 (
    echo Adding upstream remote...
    git remote add upstream https://github.com/crameep/UltimaForge.git
)

REM Also set origin if missing (zip downloads won't have it)
git remote get-url origin >nul 2>nul
if errorlevel 1 (
    git remote add origin https://github.com/crameep/UltimaForge.git
)

echo Fetching latest from upstream...
echo.
git fetch upstream
if errorlevel 1 (
    echo ERROR: Failed to fetch from upstream. Check your internet connection.
    echo.
    echo Press any key to return to menu...
    pause >nul
    goto MENU
)

REM Check if histories are related (zip download vs git clone)
git merge-base HEAD upstream/main >nul 2>nul
if errorlevel 1 goto FIRST_TIME_UPDATE

REM Check if there's anything to merge
git log --oneline HEAD..upstream/main 2>nul | findstr /r "." >nul
if errorlevel 1 (
    echo Already up to date! No new changes from upstream.
    echo.
    echo Press any key to return to menu...
    pause >nul
    goto MENU
)

REM Show changelog
echo ----------------------------------------
echo   What's New
echo ----------------------------------------
echo.
git log --format="  %%h %%s" HEAD..upstream/main 2>nul
echo.
echo ----------------------------------------
echo.
echo Changed files:
git diff --stat HEAD..upstream/main 2>nul
echo.

:UPDATE_CONFIRM
set /p do_update="Apply these updates? (Y/n): "
if /i "%do_update%"=="n" (
    echo Update cancelled.
    echo.
    echo Press any key to return to menu...
    pause >nul
    goto MENU
)

REM Stash any local uncommitted changes (branding edits etc)
git stash push -m "pre-update-stash" --include-untracked >nul 2>nul
set STASHED=%errorlevel%

REM Record current batch file hash to detect self-update
if not exist "_update_backup" mkdir "_update_backup"
certutil -hashfile ultimaforge.bat SHA256 2>nul | findstr /v "hash" > "_update_backup\bat_hash_before.txt"

REM Merge upstream (branding, keys, server-data, updates are gitignored — safe)
echo.
echo Merging upstream changes...
git merge upstream/main --allow-unrelated-histories -m "chore: merge upstream launcher updates"
set MERGE_RESULT=%errorlevel%

if %MERGE_RESULT% neq 0 (
    echo.
    echo Merge conflict detected. Aborting...
    git merge --abort >nul 2>nul
    rmdir /s /q "_update_backup" 2>nul
    if %STASHED% equ 0 git stash pop >nul 2>nul
    echo.
    echo Update failed due to conflicts. Your files are unchanged.
    echo Please report this issue so we can fix it upstream.
    echo.
    echo Press any key to return to menu...
    pause >nul
    goto MENU
)

REM Pop stash to restore any uncommitted local changes
if %STASHED% equ 0 (
    git stash pop >nul 2>nul
)

REM Reinstall deps if package.json or Cargo.toml changed
echo.
echo Checking if dependencies need updating...
git diff HEAD~1 --name-only 2>nul | findstr /i "package.json Cargo.toml" >nul
if not errorlevel 1 (
    echo Dependencies changed. Reinstalling...
    call :NPM_CLEAN_INSTALL_FUNCTION
)

REM Check if the batch file itself was updated
certutil -hashfile ultimaforge.bat SHA256 2>nul | findstr /v "hash" > "_update_backup\bat_hash_after.txt"
fc /b "_update_backup\bat_hash_before.txt" "_update_backup\bat_hash_after.txt" >nul 2>nul
set BAT_CHANGED=%errorlevel%

REM Clean up backup
rmdir /s /q "_update_backup" 2>nul

echo.
echo ========================================
echo    Update Complete!
echo ========================================
echo.
echo Your launcher source has been updated.
echo Branding, keys, and server config are untouched (not tracked in git).

if %BAT_CHANGED% neq 0 (
    echo.
    echo NOTE: This tools menu was also updated.
    echo Please close and re-open ultimaforge.bat to use the new version.
)

echo.
echo Next: Rebuild your launcher with option [4]
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
node "%~dp0app\scripts\setup-vps.js"

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

node "%~dp0app\scripts\deploy.js"

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
node "%~dp0app\scripts\dev-all-in-one.js"

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
cd /d "%~dp0app"
powershell -ExecutionPolicy Bypass -File "generate-ico.ps1"
set RESULT=%errorlevel%
cd /d "%~dp0"

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
cd /d "%~dp0app"
powershell -ExecutionPolicy Bypass -File "generate-installer-images.ps1"
cd /d "%~dp0"

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
echo Next: Build launcher (step 4) to create installer
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
cd /d "%~dp0app"
call npm test
set NPM_RESULT=%errorlevel%
cd /d "%~dp0"

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
REM FIRST TIME UPDATE (zip download — reset to upstream)
REM ============================================================================
:FIRST_TIME_UPDATE
cd /d "%~dp0"
echo This is your first update from the official repository.
echo Your launcher source will be synced to the latest version.
echo Branding, keys, and game files will not be affected.
echo.
set /p do_first_update="Apply update? (Y/n): "
if /i "%do_first_update%"=="n" (
    echo Update cancelled.
    echo.
    echo Press any key to return to menu...
    pause >nul
    goto MENU
)
echo.
echo Backing up your files...
if not exist "_update_backup" mkdir "_update_backup"
if exist "%~dp0branding\brand.json" copy /Y "%~dp0branding\brand.json" "_update_backup\brand.json" >nul 2>nul
if exist "%~dp0keys" xcopy /E /I /Y "%~dp0keys" "_update_backup\keys" >nul 2>nul
if exist "%~dp0server-data" xcopy /E /I /Y "%~dp0server-data" "_update_backup\server-data" >nul 2>nul
if exist "%~dp0.publish-all-cache.json" copy /Y "%~dp0.publish-all-cache.json" "_update_backup\" >nul 2>nul
if exist "%~dp0updates" xcopy /E /I /Y "%~dp0updates" "_update_backup\updates" >nul 2>nul

echo Syncing to upstream...
git reset --hard upstream/main
echo.
echo Restoring your files...
if exist "_update_backup\brand.json" (
    if not exist "%~dp0branding" mkdir "%~dp0branding"
    copy /Y "_update_backup\brand.json" "%~dp0branding\brand.json" >nul 2>nul
)
if exist "_update_backup\keys" xcopy /E /I /Y "_update_backup\keys" "%~dp0keys" >nul 2>nul
if exist "_update_backup\server-data" xcopy /E /I /Y "_update_backup\server-data" "%~dp0server-data" >nul 2>nul
if exist "_update_backup\.publish-all-cache.json" copy /Y "_update_backup\.publish-all-cache.json" "%~dp0" >nul 2>nul
if exist "_update_backup\updates" xcopy /E /I /Y "_update_backup\updates" "%~dp0updates" >nul 2>nul
rmdir /s /q "_update_backup" 2>nul

echo.
echo ========================================
echo    Update Complete!
echo ========================================
echo.
echo Your launcher source has been synced to the latest version.
echo Branding, keys, and server config are untouched.
echo.
echo NOTE: This tools menu was updated. Please close and re-open ultimaforge.bat.
echo.
echo Next: Run option [2] to configure branding, then [4] to build.
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

REM ============================================================================
REM INIT MSVC (find and call vcvarsall.bat for link.exe)
REM ============================================================================
:INIT_MSVC
set "VCVARSALL="
for %%v in (
    "%ProgramFiles%\Microsoft Visual Studio\2022\BuildTools\VC\Auxiliary\Build\vcvarsall.bat"
    "%ProgramFiles%\Microsoft Visual Studio\2022\Community\VC\Auxiliary\Build\vcvarsall.bat"
    "%ProgramFiles%\Microsoft Visual Studio\2022\Professional\VC\Auxiliary\Build\vcvarsall.bat"
    "%ProgramFiles%\Microsoft Visual Studio\2022\Enterprise\VC\Auxiliary\Build\vcvarsall.bat"
    "%ProgramFiles(x86)%\Microsoft Visual Studio\2022\BuildTools\VC\Auxiliary\Build\vcvarsall.bat"
    "%ProgramFiles(x86)%\Microsoft Visual Studio\2019\BuildTools\VC\Auxiliary\Build\vcvarsall.bat"
    "%ProgramFiles(x86)%\Microsoft Visual Studio\2019\Community\VC\Auxiliary\Build\vcvarsall.bat"
) do (
    if exist %%v set "VCVARSALL=%%~v"
)
if defined VCVARSALL (
    REM Always use x64 - we force Rust to target x64 even on ARM64 machines
    call "!VCVARSALL!" x64 >nul 2>nul
)
exit /b 0

REM ============================================================================
REM INIT GIT REPO (called when downloaded as zip, no .git directory)
REM ============================================================================
:INIT_GIT_REPO
cd /d "%~dp0"
echo This doesn't appear to be a git repository.
echo Initializing git so updates can be tracked...
echo.
git init
git config user.email "server-owner@ultimaforge.local"
git config user.name "UltimaForge Server Owner"
REM Write gitignore before adding files to exclude builds, deps, and user data
echo /target/> .gitignore
echo /app/target/>> .gitignore
echo /app/src-tauri/target/>> .gitignore
echo /app/node_modules/>> .gitignore
echo /app/dist/>> .gitignore
echo /keys/>> .gitignore
echo /server-data/>> .gitignore
echo /updates/>> .gitignore
echo /branding/brand.json>> .gitignore
echo .publish-all-cache.json>> .gitignore
git add -A
git commit -m "Initial commit from downloaded zip"
if errorlevel 1 (
    echo ERROR: Failed to initialize git repository.
    echo.
    echo Press any key to return to menu...
    pause >nul
    goto MENU
)
echo.
echo Git repository initialized.
echo.
exit /b 0
