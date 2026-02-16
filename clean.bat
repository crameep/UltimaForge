@echo off
REM Clean build artifacts and generated files

echo ===================================
echo Cleaning Build Artifacts
echo ===================================
echo.

set /p confirm="This will delete node_modules, target, and build files. Continue? (y/N): "
if /i not "%confirm%"=="y" (
    echo Cancelled.
    pause
    exit /b 0
)

echo.
echo Cleaning...

REM Remove node_modules
if exist "node_modules" (
    echo Removing node_modules...
    rmdir /S /Q "node_modules" 2>nul
    echo   [32m✓[0m node_modules
)

REM Remove Rust target directory
if exist "target" (
    echo Removing target...
    rmdir /S /Q "target" 2>nul
    echo   [32m✓[0m target
)

REM Remove src-tauri target directory
if exist "src-tauri\target" (
    echo Removing src-tauri\target...
    rmdir /S /Q "src-tauri\target" 2>nul
    echo   [32m✓[0m src-tauri\target
)

REM Remove generated files
if exist "src-tauri\gen" (
    echo Removing src-tauri\gen...
    rmdir /S /Q "src-tauri\gen" 2>nul
    echo   [32m✓[0m src-tauri\gen
)

REM Remove dist
if exist "dist" (
    echo Removing dist...
    rmdir /S /Q "dist" 2>nul
    echo   [32m✓[0m dist
)

REM Remove lock files
if exist "Cargo.lock" (
    del /Q "Cargo.lock" 2>nul
    echo   [32m✓[0m Cargo.lock
)

if exist "package-lock.json" (
    del /Q "package-lock.json" 2>nul
    echo   [32m✓[0m package-lock.json
)

echo.
echo ===================================
echo [32mCleaning complete![0m
echo ===================================
echo.
echo To rebuild, run:
echo   npm install
echo   npm run tauri dev
echo.
pause
