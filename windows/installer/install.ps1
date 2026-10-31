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
try {
    Copy-Item $tsf (Join-Path $dest "vifastkey_tsf.dll") -Force -ErrorAction Stop
    Copy-Item $engine (Join-Path $dest "vifastkey.dll") -Force -ErrorAction Stop
} catch {
    $dllPath = Join-Path $dest "vifastkey_tsf.dll"
    $enginePath = Join-Path $dest "vifastkey.dll"
    if (-not (Test-Path $dllPath) -or -not (Test-Path $enginePath)) {
        throw "Không ghi được DLL TSF ($arch): $($_.Exception.Message)"
    }
    Write-Warning "TSF DLL đang được app sử dụng; giữ bản hiện có. Đăng xuất/khởi động lại để nạp bản mới ($arch)."
}

$dll = Join-Path $dest "vifastkey_tsf.dll"
$process = Start-Process -FilePath $regsvr -ArgumentList @("/s", "`"$dll`"") -Wait -PassThru
if ($process.ExitCode -ne 0) {
    throw "regsvr32 thất bại (exit $($process.ExitCode)). Chạy PowerShell không elevated."
}

Write-Host "Đã cài ViFastKey TSF ($arch) vào $dest"

# Hook Replace: chỉ chạy khi người dùng chọn Replace; installer chỉ copy binary.
# Tên mới: vifastkey-replace.exe; vfk-windows.exe là tên cũ, chỉ dùng để nâng cấp.
$hookCandidates = @(
    (Join-Path $PSScriptRoot "vifastkey-replace.exe"),
    (Join-Path $root "vifastkey-replace.exe"),
    (Join-Path $root "rust\target\release\vifastkey-replace.exe"),
    (Join-Path $PSScriptRoot "vfk-windows.exe"),
    (Join-Path $root "rust\target\release\vfk-windows.exe")
)
$hook = $hookCandidates | Where-Object { Test-Path $_ } | Select-Object -First 1
if ($hook) {
    $tools = Join-Path $env:LOCALAPPDATA "ViFastKey"
    New-Item -ItemType Directory -Force -Path $tools | Out-Null
    $installedHook = Join-Path $tools "vifastkey-replace.exe"
    Copy-Item $hook $installedHook -Force
    # Dọn bản cũ để không còn process/Run key trỏ vfk-windows.exe.
    Get-Process vfk-windows -ErrorAction SilentlyContinue | Stop-Process -Force
    Remove-Item (Join-Path $tools "vfk-windows.exe") -Force -ErrorAction SilentlyContinue
    Write-Host "Đã cài hook Replace: $installedHook (bật khi chọn Replace trong cài đặt)"
} else {
    Write-Warning "Không thấy vifastkey-replace.exe; chế độ Replace sẽ chưa chạy được."
}

# Mặc định Preedit (TSF), không ghi đè lựa chọn đã có.
$configDir = Join-Path $env:APPDATA "ViFastKey"
New-Item -ItemType Directory -Force -Path $configDir | Out-Null
$windowsJson = Join-Path $configDir "windows.json"
if (-not (Test-Path $windowsJson)) {
    Set-Content -Path $windowsJson -Value '{"backend":"tsf"}' -Encoding ASCII
}

# Cài UI cấu hình nếu có trong package/build; shortcut mở tab Ứng dụng.
$settingsCandidates = @(
    (Join-Path $PSScriptRoot "vifastkey-settings.exe"),
    (Join-Path $root "vifastkey-settings.exe"),
    (Join-Path $root "rust\target\release\vifastkey-settings.exe")
)
$settings = $settingsCandidates | Where-Object { Test-Path $_ } | Select-Object -First 1
if ($settings) {
    $tools = Join-Path $env:LOCALAPPDATA "ViFastKey"
    New-Item -ItemType Directory -Force -Path $tools | Out-Null
    $installedSettings = Join-Path $tools "vifastkey-settings.exe"
    Copy-Item $settings $installedSettings -Force

    $startMenu = Join-Path $env:APPDATA "Microsoft\Windows\Start Menu\Programs"
    New-Item -ItemType Directory -Force -Path $startMenu | Out-Null
    $shell = New-Object -ComObject WScript.Shell
    $link = $shell.CreateShortcut((Join-Path $startMenu "ViFastKey Settings.lnk"))
    $link.TargetPath = $installedSettings
    $link.Arguments = "--tab apps"
    $link.Save()
    Write-Host "Đã cài UI cấu hình: $installedSettings"
}

Write-Host "Mở Settings > Time & language > Language & region > Vietnamese > Language options"
Write-Host "> Add a keyboard > ViFastKey. Nếu chưa thấy, đăng xuất/đăng nhập lại."
Write-Host "Cấu hình: $env:APPDATA\ViFastKey\config.json và windows.json"
Write-Host "Mở UI: Ctrl+Shift+, hoặc Start Menu > ViFastKey Settings."
