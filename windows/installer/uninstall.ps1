$ErrorActionPreference = "Stop"

function Get-PeMachine([string]$Path) {
    $bytes = [System.IO.File]::ReadAllBytes($Path)
    if ($bytes.Length -lt 0x40) { return 0 }
    $pe = [BitConverter]::ToInt32($bytes, 0x3C)
    if ($pe -lt 0 -or $pe + 6 -ge $bytes.Length) { return 0 }
    return [BitConverter]::ToUInt16($bytes, $pe + 4)
}

foreach ($arch in @("x64", "x86")) {
    $dest = Join-Path $env:LOCALAPPDATA "ViFastKey\tsf\$arch"
    $dll = Join-Path $dest "vifastkey_tsf.dll"
    if (-not (Test-Path $dll)) { continue }

    $is32 = (Get-PeMachine $dll) -eq 0x014C
    if ($is32 -and [Environment]::Is64BitOperatingSystem) {
        $regsvr = Join-Path $env:WINDIR "SysWOW64\regsvr32.exe"
    } else {
        $regsvr = Join-Path $env:WINDIR "System32\regsvr32.exe"
    }
    $process = Start-Process -FilePath $regsvr -ArgumentList @("/s", "/u", "`"$dll`"") -Wait -PassThru
    if ($process.ExitCode -ne 0) {
        Write-Warning "regsvr32 /u ($arch) trả về exit $($process.ExitCode)"
    }
    Remove-Item $dest -Recurse -Force
}

$settingsExe = Join-Path $env:LOCALAPPDATA "ViFastKey\vifastkey-settings.exe"
if (Test-Path $settingsExe) {
    Remove-Item $settingsExe -Force
}

$shortcut = Join-Path $env:APPDATA "Microsoft\Windows\Start Menu\Programs\ViFastKey Settings.lnk"
if (Test-Path $shortcut) {
    Remove-Item $shortcut -Force
}

Write-Host "Đã gỡ ViFastKey TSF và UI cấu hình (giữ config người dùng)."
