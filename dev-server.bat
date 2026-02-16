@echo off
REM Start the UltimaForge host server for local testing

echo ===================================
echo Starting UltimaForge Host Server
echo ===================================
echo.
echo Update Server: http://localhost:8080
echo Health Check:  http://localhost:8080/health
echo Manifest:      http://localhost:8080/manifest.json
echo.
echo Press Ctrl+C to stop the server
echo.

REM Check if test-updates directory exists
if not exist "test-updates\manifest.json" (
    echo [33mWarning: test-updates\manifest.json not found[0m
    echo Run this first: npm run test:publish
    echo.
    pause
    exit /b 1
)

cargo run -p host-server -- --dir ./test-updates --port 8080
