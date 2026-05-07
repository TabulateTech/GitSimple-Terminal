@echo off
setlocal EnableExtensions
powershell -NoProfile -ExecutionPolicy Bypass -File "%~dp0install_gitsimple_rust.ps1"
if errorlevel 1 (
    echo.
    echo [ERROR] La instalacion fallo.
    pause
    exit /b 1
)
echo.
echo Listo. Si PowerShell no reconoce gitsimple, cierra y abre la terminal.
pause
