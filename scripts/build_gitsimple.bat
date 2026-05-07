@echo off
setlocal

set "SCRIPT_DIR=%~dp0"
for %%I in ("%SCRIPT_DIR%..") do set "PROJECT_DIR=%%~fI"
set "EXE=%PROJECT_DIR%\target\release\gitsimple.exe"

cargo build --release --manifest-path "%PROJECT_DIR%\Cargo.toml"
if errorlevel 1 (
    echo [ERROR] No se pudo compilar GitSimple-Terminal.
    exit /b 1
)

echo [OK] %EXE%
