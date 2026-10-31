fn main() {
    #[cfg(windows)]
    {
        let root = std::path::PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").unwrap());
        let icon = root.join("../../../resources/brand/favicon.ico");
        println!("cargo:rerun-if-changed={}", icon.display());
        let mut res = winres::WindowsResource::new();
        res.set_icon(icon.to_str().unwrap());
        res.set("ProductName", "ViFastKey");
        res.set("FileDescription", "ViFastKey CLI");
        res.set("LegalCopyright", "MIT");
        res.compile().expect("không nhúng được icon Windows");
    }
}
