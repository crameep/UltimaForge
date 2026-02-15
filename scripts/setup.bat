@echo off
REM UltimaForge Setup Launcher
REM Double-click this file to run the setup script without execution policy issues
REM
REM Usage:
REM   Double-click setup.bat          - Interactive mode
REM   setup.bat -UseScoop             - Use Scoop (no admin required)
REM   setup.bat -SkipPrompts          - Non-interactive (CI mode)
REM   setup.bat -Help                 - Show help

REM Get the directory where this batch file is located
set "SCRIPT_DIR=%~dp0"

REM Run PowerShell with ExecutionPolicy Bypass and pass all arguments
PowerShell -ExecutionPolicy Bypass -File "%SCRIPT_DIR%setup.ps1" %*
