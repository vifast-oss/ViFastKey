param()

$ErrorActionPreference = "Stop"
$here = Split-Path -Parent $MyInvocation.MyCommand.Path
$dest = Join-Path $env:LOCALAPPDATA "ViFastKey"
$startMenu = Join-Path $env:APPDATA "Microsoft\Windows\Start Menu\Programs"
$runKey = "HKCU:\Software\Microsoft\Windows\CurrentVersion\Run"

New-Item -ItemType Directory -Force -Path $dest | Out-Null

# Dọn process bản cũ trước khi ghi đè binary (Settings và hook dùng cùng tên file).
Get-Process vifastkey-settings, vifastkey-replace, vfk-windows -ErrorAction SilentlyContinue |
    Stop-Process -Force
Start-Sleep -Milliseconds 250

# Gỡ TSF legacy nếu máy đã cài bản cũ. Từ bản này Windows chỉ dùng hook.
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

$exeSrc = Join-Path $here "vifastkey-settings.exe"
if (-not (Test-Path $exeSrc)) { throw "Không thấy vifastkey-settings.exe cạnh install.ps1." }
$installed = Join-Path $dest "vifastkey-settings.exe"
Copy-Item $exeSrc $installed -Force

# Hook-only: Run key mở Settings ẩn vào tray; Settings tự chạy hook --hook.
Remove-ItemProperty -Path $runKey -Name "ViFastKeyHook" -ErrorAction SilentlyContinue
New-Item -Path $runKey -Force | Out-Null
Set-ItemProperty -Path $runKey -Name "ViFastKey" -Value "`"$installed`" --hidden"

# Dọn binary hook/TSF cũ để không còn process hoặc autostart cũ.
Remove-Item (Join-Path $dest "vifastkey-replace.exe") -Force -ErrorAction SilentlyContinue
Remove-Item (Join-Path $dest "vfk-windows.exe") -Force -ErrorAction SilentlyContinue
Remove-Item (Join-Path $dest "vifastkey_tsf.dll") -Force -ErrorAction SilentlyContinue

$configDir = Join-Path $env:APPDATA "ViFastKey"
New-Item -ItemType Directory -Force -Path $configDir | Out-Null
$windowsJson = Join-Path $configDir "windows.json"
Set-Content -Path $windowsJson -Value '{"backend":"hook"}' -Encoding ASCII

# Shortcut mở UI.
New-Item -ItemType Directory -Force -Path $startMenu | Out-Null
$shell = New-Object -ComObject WScript.Shell
$link = $shell.CreateShortcut((Join-Path $startMenu "ViFastKey Settings.lnk"))
$link.TargetPath = $installed
$link.Arguments = "--tab apps"
$link.Save()

if (-not (Get-Process vifastkey-settings -ErrorAction SilentlyContinue)) {
    Start-Process -FilePath $installed -ArgumentList "--hidden" -WindowStyle Hidden
}

Write-Host "Đã cài ViFastKey hook-only: $installed"
Write-Host "Tự khởi động: HKCU Run\ViFastKey (Settings --hidden)"
Write-Host "Hook nền: $installed --hook"
Write-Host ""
try {
    $hasEnUs = (Get-WinUserLanguageList -ErrorAction Stop | ForEach-Object { $_.InputMethodTips }) -match '^0409:'
    if (-not $hasEnUs) {
        Write-Warning "Chưa thấy English (US) trong input source. Hãy thêm English (United States) và chọn ENG; nếu không ViFastKey sẽ nhường cho VI/JP."
    }
} catch {}
Write-Host "QUAN TRỌNG: chọn English (US) trong Windows input source."
Write-Host "Nếu chọn Vietnamese/Japanese, ViFastKey sẽ nhường cho bộ gõ Windows."
Write-Host "Mở Settings: Start Menu > ViFastKey Settings, hoặc Ctrl+Shift+,"
Write-Host "Cấu hình: $env:APPDATA\ViFastKey\config.json"
Write-Host "Chưa ký Authenticode; SmartScreen có thể cảnh báo."
