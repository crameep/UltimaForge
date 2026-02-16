@echo off
REM Start the UltimaForge launcher in development mode

echo ===================================
echo Starting UltimaForge Launcher (Dev)
echo ===================================
echo.

REM Check if node_modules exists
if not exist "node_modules" (
    echo [33mnode_modules not found. Installing dependencies...[0m
    call npm install
    if errorlevel 1 (
        echo [31mFailed to install dependencies[0m
        pause
        exit /b 1
    )
)

REM Sync branding assets
echo Syncing branding assets...
call sync-branding.bat
echo.

REM Check if server is running
echo Checking if server is running...
curl -s http://localhost:8080/health >nul 2>&1
if errorlevel 1 (
    echo.
    echo [33mWarning: Server not detected at http://localhost:8080[0m
    echo Make sure to run dev-server.bat in another terminal!
    echo.
    timeout /t 3 >nul
)

echo Starting Tauri dev server...
echo.
npm run tauri dev
