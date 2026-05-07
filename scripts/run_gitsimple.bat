@echo off
setlocal

REM Ejecuta GitSimple-Terminal desde esta carpeta sin instalarlo globalmente.
set "SCRIPT_DIR=%~dp0"
for %%I in ("%SCRIPT_DIR%..") do set "PROJECT_DIR=%%~fI"
set "EXE=%PROJECT_DIR%\target\release\gitsimple.exe"

if not exist "%EXE%" (
    echo [INFO] Compilando GitSimple-Terminal...
    cargo build --release --manifest-path "%PROJECT_DIR%\Cargo.toml"
)

if not exist "%EXE%" (
    echo.
    echo [ERROR] No se pudo crear gitsimple.exe.
    echo Instala Rust/Cargo o ejecuta scripts\install_gitsimple_rust.bat desde una terminal con cargo en PATH.
    pause
    exit /b 1
)

"%EXE%" %*
