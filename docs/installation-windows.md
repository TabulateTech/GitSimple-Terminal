# Instalacion de GitSimple-Terminal en Rust

Este paquete instala la version Rust de `gitsimple` como comando global de usuario en Windows.

## Instalar

Desde PowerShell, en la raiz del repositorio:

```powershell
powershell -ExecutionPolicy Bypass -File .\scripts\install_gitsimple_rust.ps1
```

O doble clic / ejecutar:

```powershell
.\scripts\install_gitsimple_rust.bat
```

## Verificar

```powershell
where.exe gitsimple
gitsimple
```

La ruta esperada es algo parecido a:

```text
C:\Users\TU_USUARIO\AppData\Local\GitSimple-Terminal\bin\gitsimple.exe
```
