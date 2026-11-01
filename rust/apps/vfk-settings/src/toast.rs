//! Toast/OSD nhẹ. Trên Windows dùng balloon của notification area, không chiếm focus.
//! Trên Linux/macOS, UI vẫn có toast nội bộ; module này là no-op.

#[cfg(windows)]
pub fn show(title: &str, body: &str) {
    use std::mem::zeroed;
    use windows_sys::Win32::UI::Shell::{
        Shell_NotifyIconW, NIF_ICON, NIF_INFO, NIF_TIP, NIIF_NOSOUND, NIM_ADD, NIM_DELETE, NIM_MODIFY,
        NOTIFYICONDATAW,
    };
    use windows_sys::Win32::UI::WindowsAndMessaging::{LoadIconW, IDI_APPLICATION};

    fn copy_utf16<const N: usize>(dst: &mut [u16; N], text: &str) {
        let units: Vec<u16> = text.encode_utf16().take(N - 1).collect();
        dst[..units.len()].copy_from_slice(&units);
        dst[units.len()] = 0;
    }

    let mut data: NOTIFYICONDATAW = unsafe { zeroed() };
    data.cbSize = std::mem::size_of::<NOTIFYICONDATAW>() as u32;
    data.hWnd = std::ptr::null_mut();
    data.uID = 0x5646; // 'VF'
    data.uFlags = NIF_ICON | NIF_TIP | NIF_INFO;
    data.hIcon = unsafe { LoadIconW(std::ptr::null_mut(), IDI_APPLICATION) };
    copy_utf16(&mut data.szTip, "ViFastKey");
    copy_utf16(&mut data.szInfoTitle, title);
    copy_utf16(&mut data.szInfo, body);
    data.dwInfoFlags = NIIF_NOSOUND;
    unsafe {
        Shell_NotifyIconW(NIM_ADD, &data);
        Shell_NotifyIconW(NIM_MODIFY, &data);
    }
    // Dọn icon tạm sau khi Windows đã nhận balloon.
    std::thread::spawn(move || {
        std::thread::sleep(std::time::Duration::from_secs(4));
        let mut cleanup: NOTIFYICONDATAW = unsafe { zeroed() };
        cleanup.cbSize = std::mem::size_of::<NOTIFYICONDATAW>() as u32;
        cleanup.hWnd = std::ptr::null_mut();
        cleanup.uID = 0x5646;
        unsafe {
            Shell_NotifyIconW(NIM_DELETE, &cleanup);
        }
    });
}

#[cfg(not(windows))]
pub fn show(_title: &str, _body: &str) {}
