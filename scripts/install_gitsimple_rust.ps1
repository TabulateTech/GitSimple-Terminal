$ErrorActionPreference = "Stop"

$ProductName = "GitSimple-Terminal"
$InstallDir = Join-Path $env:LOCALAPPDATA $ProductName
$LegacyInstallDir = Join-Path $env:LOCALAPPDATA "GitSimple"
$BinDir = Join-Path $InstallDir "bin"
$LegacyBinDir = Join-Path $LegacyInstallDir "bin"
$Exe = Join-Path $BinDir "gitsimple.exe"
$ScriptDir = Split-Path -Parent $MyInvocation.MyCommand.Path
$ProjectDir = Split-Path -Parent $ScriptDir
$CargoToml = Join-Path $ProjectDir "Cargo.toml"
$ReleaseExe = Join-Path $ProjectDir "target\release\gitsimple.exe"

function Normalize-PathEntry([string]$PathEntry) {
    if ([string]::IsNullOrWhiteSpace($PathEntry)) { return $null }
    return $PathEntry.Trim().TrimEnd('\')
}

function Add-PathEntryOnce([string]$OriginalPath, [string]$EntryToAdd) {
    $target = Normalize-PathEntry $EntryToAdd
    $parts = New-Object System.Collections.Generic.List[string]
    $found = $false

    foreach ($part in ($OriginalPath -split ';')) {
        $norm = Normalize-PathEntry $part
        if ([string]::IsNullOrWhiteSpace($norm)) { continue }

        $already = $false
        foreach ($existing in $parts) {
            if ((Normalize-PathEntry $existing) -ieq $norm) {
                $already = $true
                break
            }
        }
        if (-not $already) { $parts.Add($norm) }
        if ($norm -ieq $target) { $found = $true }
    }

    if (-not $found) { $parts.Add($target) }
    return ($parts -join ';')
}

function Remove-PathEntry([string]$OriginalPath, [string]$EntryToRemove) {
    $target = Normalize-PathEntry $EntryToRemove
    $parts = New-Object System.Collections.Generic.List[string]

    foreach ($part in ($OriginalPath -split ';')) {
        $norm = Normalize-PathEntry $part
        if ([string]::IsNullOrWhiteSpace($norm)) { continue }
        if ($norm -ieq $target) { continue }

        $already = $false
        foreach ($existing in $parts) {
            if ((Normalize-PathEntry $existing) -ieq $norm) {
                $already = $true
                break
            }
        }
        if (-not $already) { $parts.Add($norm) }
    }

    return ($parts -join ';')
}

Write-Host "[1/6] Verificando proyecto Rust..."
if (-not (Test-Path -LiteralPath $CargoToml)) {
    throw "No se encontro Cargo.toml en: $ProjectDir"
}

Write-Host "[2/6] Verificando Cargo..."
$cargo = Get-Command cargo -ErrorAction SilentlyContinue
if (-not $cargo) {
    throw "No se encontro cargo en el PATH. Instala Rust con rustup y vuelve a ejecutar este script."
}
cargo --version

Write-Host "[3/6] Limpiando instalacion anterior de GitSimple-Terminal en AppData..."
if (Test-Path -LiteralPath $InstallDir) {
    Remove-Item -LiteralPath $InstallDir -Recurse -Force
}
if (($LegacyInstallDir -ine $InstallDir) -and (Test-Path -LiteralPath $LegacyInstallDir)) {
    Remove-Item -LiteralPath $LegacyInstallDir -Recurse -Force
}
New-Item -ItemType Directory -Force -Path $BinDir | Out-Null

Write-Host "[4/6] Compilando en modo release..."
cargo build --release --manifest-path $CargoToml
if (-not (Test-Path -LiteralPath $ReleaseExe)) {
    throw "No se genero el ejecutable esperado: $ReleaseExe"
}

Write-Host "[5/6] Instalando ejecutable..."
Copy-Item -LiteralPath $ReleaseExe -Destination $Exe -Force

Write-Host "[6/6] Agregando GitSimple-Terminal al PATH de usuario..."
$userPath = [Environment]::GetEnvironmentVariable("Path", "User")
$cleanUserPath = Remove-PathEntry $userPath $LegacyBinDir
$newUserPath = Add-PathEntryOnce $cleanUserPath $BinDir
[Environment]::SetEnvironmentVariable("Path", $newUserPath, "User")
$cleanSessionPath = Remove-PathEntry $env:Path $LegacyBinDir
$env:Path = Add-PathEntryOnce $cleanSessionPath $BinDir

Write-Host ""
Write-Host "Instalacion completada."
Write-Host "Ejecutable instalado en: $Exe"
Write-Host ""
Write-Host "Prueba en esta misma terminal:"
Write-Host "  gitsimple"
Write-Host ""
Write-Host "Para verificar que ruta se esta usando:"
Write-Host "  where.exe gitsimple"
