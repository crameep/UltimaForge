@echo off
REM Start both server and launcher in separate windows

echo ===================================
echo UltimaForge Development Launcher
echo ===================================
echo.

REM Sync branding first
echo Syncing branding assets...
call sync-branding.bat

REM Check if test manifest exists
if not exist "test-updates\manifest.json" (
    echo.
    echo [33mGenerating test manifest...[0m
    cargo run -p publish-cli -- publish --source ./test-data/sample-client --output ./test-updates --key ./test-keys/private.key --version 1.0.0
    if errorlevel 1 (
        echo [31mFailed to generate test manifest[0m
        pause
        exit /b 1
    )
    echo [32m✓ Test manifest generated[0m
)

echo.
echo ===================================
echo Starting Development Environment
echo ===================================
echo.
echo This will open 2 windows:
echo   1. Host Server (port 8080)
echo   2. Tauri Launcher (dev mode)
echo.
echo Press any key to continue...
pause >nul

REM Start server in new window
echo Starting host server...
start "UltimaForge Server" cmd /k "dev-server.bat"

REM Wait a moment for server to start
timeout /t 2 >nul

REM Start launcher in new window
echo Starting launcher...
start "UltimaForge Launcher" cmd /k "dev-launcher.bat"

echo.
echo ===================================
echo [32mDevelopment environment started![0m
echo ===================================
echo.
echo Two windows opened:
echo   - Host Server (http://localhost:8080)
echo   - Tauri Launcher
echo.
echo Close this window or press any key to exit...
pause >nul
