param(
    [string]$OutDir = "",
    [string]$Version = "0.1.2"
)

$ErrorActionPreference = "Stop"
$repo = Split-Path -Parent $PSScriptRoot
if (-not $OutDir) { $OutDir = Join-Path $repo "dist" }

$pkg = Join-Path ([System.IO.Path]::GetTempPath()) ("vifastkey-package-" + [guid]::NewGuid().ToString("N"))
New-Item -ItemType Directory -Force -Path $pkg, $OutDir | Out-Null
try {
    $exe = Join-Path $repo "rust\target\release\vifastkey-settings.exe"
    if (-not (Test-Path $exe)) { throw "Không thấy $exe; build trước bằng cargo build --release -p vifastkey-settings." }
    Copy-Item $exe (Join-Path $pkg "vifastkey-settings.exe") -Force
    Copy-Item (Join-Path $repo "windows\installer\install.ps1") $pkg -Force
    Copy-Item (Join-Path $repo "windows\installer\uninstall.ps1") $pkg -Force
    Copy-Item (Join-Path $repo "packaging\windows\README.md") (Join-Path $pkg "README.md") -Force
    Copy-Item (Join-Path $repo "docs\INSTALL.md") (Join-Path $pkg "INSTALL.md") -Force
    Copy-Item (Join-Path $repo "LICENSE") $pkg -Force
    Copy-Item (Join-Path $repo "THIRD_PARTY_NOTICES.md") $pkg -Force

    $zip = Join-Path $OutDir "vifastkey-windows-hook-$Version.zip"
    Remove-Item $zip -Force -ErrorAction SilentlyContinue
    Compress-Archive -Path "$pkg\*" -DestinationPath $zip
    Write-Host "Đã tạo $zip"
    Get-Item $zip | Select-Object Name, Length
} finally {
    Remove-Item $pkg -Recurse -Force -ErrorAction SilentlyContinue
}
