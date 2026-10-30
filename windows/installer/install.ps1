param(
    [string]$BuildDir = "build/tsf",
    [string]$Configuration = "Release"
)

$ErrorActionPreference = "Stop"

function Get-PeMachine([string]$Path) {
    $bytes = [System.IO.File]::ReadAllBytes($Path)
    if ($bytes.Length -lt 0x40) { return 0 }
    $pe = [BitConverter]::ToInt32($bytes, 0x3C)
    if ($pe -lt 0 -or $pe + 6 -ge $bytes.Length) { return 0 }
    return [BitConverter]::ToUInt16($bytes, $pe + 4)
}

$repo = Split-Path -Parent (Split-Path -Parent $PSScriptRoot)
$root = if (Test-Path (Join-Path $PSScriptRoot "x64")) {
    $PSScriptRoot
} elseif (Test-Path (Join-Path $repo "rust")) {
    $repo
} else {
    $PSScriptRoot
}
$source = if ([System.IO.Path]::IsPathRooted($BuildDir)) { $BuildDir } else { Join-Path $root $BuildDir }
$candidates = @(
    (Join-Path $source "$Configuration\vifastkey_tsf.dll"),
    (Join-Path $source "vifastkey_tsf.dll")
)
$tsf = $candidates | Where-Object { Test-Path $_ } | Select-Object -First 1
if (-not $tsf) {
    throw "Không thấy vifastkey_tsf.dll trong $source. Build trước bằng cmake --build."
}

$engineCandidates = @(
    (Join-Path (Split-Path -Parent $tsf) "vifastkey.dll"),
    (Join-Path $root "rust\target\release\vifastkey.dll")
)
$engine = $engineCandidates | Where-Object { Test-Path $_ } | Select-Object -First 1
if (-not $engine) {
    throw "Không thấy vifastkey.dll. Chạy cargo build --release -p vfk-ffi trước."
}

$machine = Get-PeMachine $tsf
$is32 = $machine -eq 0x014C
$arch = if ($is32) { "x86" } else { "x64" }
if ($is32 -and [Environment]::Is64BitOperatingSystem) {
    $regsvr = Join-Path $env:WINDIR "SysWOW64\regsvr32.exe"
} else {
    $regsvr = Join-Path $env:WINDIR "System32\regsvr32.exe"
}

$dest = Join-Path $env:LOCALAPPDATA "ViFastKey\tsf\$arch"
New-Item -ItemType Directory -Force -Path $dest | Out-Null
Copy-Item $tsf (Join-Path $dest "vifastkey_tsf.dll") -Force
Copy-Item $engine (Join-Path $dest "vifastkey.dll") -Force

$dll = Join-Path $dest "vifastkey_tsf.dll"
$process = Start-Process -FilePath $regsvr -ArgumentList @("/s", "`"$dll`"") -Wait -PassThru
if ($process.ExitCode -ne 0) {
    throw "regsvr32 thất bại (exit $($process.ExitCode)). Chạy PowerShell không elevated."
}

Write-Host "Đã cài ViFastKey TSF ($arch) vào $dest"
Write-Host "Mở Settings > Time & language > Language & region > Vietnamese > Language options"
Write-Host "> Add a keyboard > ViFastKey. Nếu chưa thấy, đăng xuất/đăng nhập lại."
Write-Host "Cấu hình: $env:APPDATA\ViFastKey\config.json và windows.json"
