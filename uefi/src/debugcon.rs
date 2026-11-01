//! Xuất debug console QEMU cho bản self-test (chỉ x86_64).

#[cfg(all(feature = "selftest", target_arch = "x86_64"))]
pub fn write(text: &str) {
    use core::arch::asm;

    for byte in text.bytes() {
        // SAFETY: port 0xE9 is the conventional QEMU debug-console sink.
        unsafe {
            asm!(
                "out dx, al",
                in("dx") 0xE9u16,
                in("al") byte,
                options(nomem, nostack, preserves_flags)
            );
        }
    }
}

#[cfg(all(feature = "selftest", target_arch = "x86_64"))]
pub fn line(text: &str) {
    write(text);
    write("\n");
}
