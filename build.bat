@echo off
REM Build UltimaForge launcher for production

echo ===================================
echo Building UltimaForge Launcher
echo ===================================
echo.

REM Sync branding assets
echo [1/4] Syncing branding assets...
call sync-branding.bat

REM Install dependencies
echo.
echo [2/4] Installing dependencies...
if not exist "node_modules" (
    call npm install
    if errorlevel 1 (
        echo [31mFailed to install dependencies[0m
        pause
        exit /b 1
    )
) else (
    echo   [32m✓[0m Dependencies already installed
)

REM Build frontend
echo.
echo [3/4] Building frontend...
call npm run build
if errorlevel 1 (
    echo [31mFrontend build failed[0m
    pause
    exit /b 1
)
echo   [32m✓[0m Frontend built

REM Build Tauri app
echo.
echo [4/4] Building Tauri application...
echo   This may take several minutes...
call npm run tauri build
if errorlevel 1 (
    echo [31mTauri build failed[0m
    pause
    exit /b 1
)

echo.
echo ===================================
echo [32mBuild Complete![0m
echo ===================================
echo.
echo Your launcher is ready at:
echo   src-tauri\target\release\
echo.
echo Files created:
dir /B "src-tauri\target\release\*.exe" 2>nul
dir /B "src-tauri\target\release\bundle\msi\*.msi" 2>nul
echo.
pause
