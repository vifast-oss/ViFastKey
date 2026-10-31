param(
    [string]$X64Build = "",
    [string]$X86Build = "",
    [string]$OutDir = "",
    [string]$Version = "0.1.0"
)

$ErrorActionPreference = "Stop"
$repo = Split-Path -Parent $PSScriptRoot
if (-not $X64Build) { $X64Build = Join-Path $repo "build\tsf-x64" }
if (-not $X86Build) { $X86Build = Join-Path $repo "build\tsf-x86" }
if (-not $OutDir) { $OutDir = Join-Path $repo "dist" }

$pkg = Join-Path ([System.IO.Path]::GetTempPath()) ("vifastkey-package-" + [guid]::NewGuid().ToString("N"))
New-Item -ItemType Directory -Force -Path "$pkg\x64", "$pkg\x86", $OutDir | Out-Null
try {
    foreach ($pair in @(@($X64Build, "x64"), @($X86Build, "x86"))) {
        $build = $pair[0]
        $arch = $pair[1]
        $tsf = Join-Path $build "vifastkey_tsf.dll"
        if (-not (Test-Path $tsf)) { $tsf = Join-Path $build "Release\vifastkey_tsf.dll" }
        $engine = Join-Path $build "vifastkey.dll"
        if (-not (Test-Path $engine)) { $engine = Join-Path $build "Release\vifastkey.dll" }
        if (-not (Test-Path $tsf)) { throw "Không thấy vifastkey_tsf.dll cho $arch trong $build" }
        if (-not (Test-Path $engine)) { throw "Không thấy vifastkey.dll cho $arch trong $build" }
        Copy-Item $tsf "$pkg\$arch\vifastkey_tsf.dll" -Force
        Copy-Item $engine "$pkg\$arch\vifastkey.dll" -Force
    }

    $release = Join-Path $repo "rust\target\release"
    Copy-Item (Join-Path $release "vifastkey-replace.exe") $pkg -Force
    Copy-Item (Join-Path $release "vifastkey-settings.exe") $pkg -Force
    Copy-Item (Join-Path $repo "packaging\windows\README.md") "$pkg\README.md" -Force
    Copy-Item (Join-Path $repo "windows\tsf\README.md") "$pkg\DEVELOPER.md" -Force
    Copy-Item (Join-Path $repo "windows\installer\install.ps1") $pkg -Force
    Copy-Item (Join-Path $repo "windows\installer\uninstall.ps1") $pkg -Force
    Copy-Item (Join-Path $repo "windows\tsf\windows.example.json") $pkg -Force
    Copy-Item (Join-Path $repo "LICENSE") $pkg -Force
    Copy-Item (Join-Path $repo "THIRD_PARTY_NOTICES.md") $pkg -Force

    $zip = Join-Path $OutDir "vifastkey-windows-tsf-$Version.zip"
    Remove-Item $zip -Force -ErrorAction SilentlyContinue
    Compress-Archive -Path "$pkg\*" -DestinationPath $zip
    Write-Host "Đã tạo $zip"
    Get-Item $zip | Select-Object Name, Length
} finally {
    Remove-Item $pkg -Recurse -Force -ErrorAction SilentlyContinue
}
