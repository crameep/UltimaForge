@echo off
REM Sync branding assets from branding/ to public/branding/

echo ===================================
echo Syncing Branding Assets
echo ===================================
echo.

REM Create public/branding directory if it doesn't exist
if not exist "public\branding" (
    echo Creating public\branding directory...
    mkdir "public\branding"
)

REM Copy branding images
if exist "branding\hero-bg.png" (
    echo Copying hero-bg.png...
    copy /Y "branding\hero-bg.png" "public\branding\hero-bg.png" >nul
    echo   [32m✓[0m hero-bg.png
) else (
    echo   [33m! hero-bg.png not found[0m
)

if exist "branding\sidebar-logo.png" (
    echo Copying sidebar-logo.png...
    copy /Y "branding\sidebar-logo.png" "public\branding\sidebar-logo.png" >nul
    echo   [32m✓[0m sidebar-logo.png
) else (
    echo   [33m! sidebar-logo.png not found[0m
)

REM Copy any other PNG/JPG files from branding folder
echo.
echo Copying any additional images...
for %%f in (branding\*.png branding\*.jpg branding\*.jpeg) do (
    if not "%%~nxf"=="hero-bg.png" if not "%%~nxf"=="sidebar-logo.png" (
        copy /Y "%%f" "public\branding\%%~nxf" >nul
        echo   [32m✓[0m %%~nxf
    )
)

echo.
echo ===================================
echo [32mBranding assets synced![0m
echo ===================================
echo.
echo Files synced to: public\branding\
echo.
