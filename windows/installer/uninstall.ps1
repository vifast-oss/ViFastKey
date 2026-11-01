param()

$ErrorActionPreference = "Continue"
$dest = Join-Path $env:LOCALAPPDATA "ViFastKey"
$runKey = "HKCU:\Software\Microsoft\Windows\CurrentVersion\Run"
$startMenu = Join-Path $env:APPDATA "Microsoft\Windows\Start Menu\Programs"

# Dừng Settings + hook (cùng một image, taskkill theo tên là đủ khi uninstall).
Get-Process vifastkey-settings, vifastkey-replace, vfk-windows -ErrorAction SilentlyContinue |
    Stop-Process -Force
Start-Sleep -Milliseconds 250
Remove-ItemProperty -Path $runKey -Name "ViFastKey" -ErrorAction SilentlyContinue
Remove-ItemProperty -Path $runKey -Name "ViFastKeyHook" -ErrorAction SilentlyContinue

# Gỡ TSF legacy.
foreach ($arch in @("x64", "x86")) {
    $tsfDir = Join-Path $dest "tsf\$arch"
    $dll = Join-Path $tsfDir "vifastkey_tsf.dll"
    if (Test-Path $dll) {
        $regsvr = if ($arch -eq "x86" -and [Environment]::Is64BitOperatingSystem) {
            Join-Path $env:WINDIR "SysWOW64\regsvr32.exe"
        } else {
            Join-Path $env:WINDIR "System32\regsvr32.exe"
        }
        $p = Start-Process -FilePath $regsvr -ArgumentList @("/s", "/u", "`"$dll`"") -Wait -PassThru
        if ($p.ExitCode -ne 0) { Write-Warning "regsvr32 /u $arch trả về $($p.ExitCode)" }
    }
    Remove-Item $tsfDir -Recurse -Force -ErrorAction SilentlyContinue
}

Remove-Item (Join-Path $dest "vifastkey-settings.exe") -Force -ErrorAction SilentlyContinue
Remove-Item (Join-Path $dest "vifastkey-replace.exe") -Force -ErrorAction SilentlyContinue
Remove-Item (Join-Path $dest "vfk-windows.exe") -Force -ErrorAction SilentlyContinue
Remove-Item (Join-Path $dest "vifastkey_tsf.dll") -Force -ErrorAction SilentlyContinue
Remove-Item (Join-Path $env:APPDATA "ViFastKey\hook-state.json") -Force -ErrorAction SilentlyContinue
Remove-Item (Join-Path $env:APPDATA "ViFastKey\windows.json") -Force -ErrorAction SilentlyContinue

$link = Join-Path $startMenu "ViFastKey Settings.lnk"
if (Test-Path $link) { Remove-Item $link -Force }

Write-Host "Đã gỡ ViFastKey hook/TSF legacy. Giữ config, macro và settings-ui.json."
Write-Host "Nếu muốn xoá sạch: Remove-Item -Recurse $env:APPDATA\ViFastKey"
