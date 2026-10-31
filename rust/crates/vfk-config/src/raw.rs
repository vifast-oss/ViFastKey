//! Đọc file config thô và giữ nguyên các key chưa biết.

use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use serde_json::{Map, Value};

use crate::error::Error;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Fingerprint {
    pub len: u64,
    pub hash: String,
}

impl Fingerprint {
    pub fn of(bytes: &[u8]) -> Self {
        Fingerprint { len: bytes.len() as u64, hash: blake3::hash(bytes).to_hex().to_string() }
    }
}

#[derive(Clone, Debug)]
pub struct RawConfig {
    /// Toàn bộ object trong file. Rỗng nếu file chưa tồn tại.
    pub value: Map<String, Value>,
    pub fingerprint: Option<Fingerprint>,
    pub exists: bool,
}

impl RawConfig {
    pub fn missing() -> Self {
        RawConfig { value: Map::new(), fingerprint: None, exists: false }
    }

    pub fn from_value(value: Map<String, Value>, bytes: &[u8]) -> Self {
        RawConfig { value, fingerprint: Some(Fingerprint::of(bytes)), exists: true }
    }

    pub fn get(&self, key: &str) -> Option<&Value> {
        self.value.get(key)
    }

    /// Đọc lại từ đĩa để so sánh với baseline. Không tạo side effect.
    pub fn read(path: &Path) -> Result<Self, Error> {
        read_raw(path)
    }
}

pub fn read_raw(path: &Path) -> Result<RawConfig, Error> {
    let bytes = match fs::read(path) {
        Ok(bytes) => bytes,
        Err(err) if err.kind() == io::ErrorKind::NotFound => return Ok(RawConfig::missing()),
        Err(source) => return Err(Error::Io { path: path.to_path_buf(), source }),
    };

    let parsed: Value = serde_json::from_slice(strip_bom(&bytes))
        .map_err(|err| Error::Json { message: format_json_error(&err) })?;
    let Value::Object(value) = parsed else {
        return Err(Error::NotObject);
    };

    Ok(RawConfig::from_value(value, &bytes))
}

/// Windows PowerShell 5.1 và một số editor ghi UTF-8 BOM; JSON parser không nhận.
fn strip_bom(bytes: &[u8]) -> &[u8] {
    bytes.strip_prefix(&[0xEF, 0xBB, 0xBF]).unwrap_or(bytes)
}

/// Đọc bytes + parse lỗi để có thể sao lưu file hỏng trước khi báo lỗi.
pub fn read_bytes(path: &Path) -> Result<Option<Vec<u8>>, Error> {
    match fs::read(path) {
        Ok(bytes) => Ok(Some(bytes)),
        Err(err) if err.kind() == io::ErrorKind::NotFound => Ok(None),
        Err(source) => Err(Error::Io { path: path.to_path_buf(), source }),
    }
}

pub fn parse_object(bytes: &[u8]) -> Result<Map<String, Value>, Error> {
    let parsed: Value = serde_json::from_slice(strip_bom(bytes))
        .map_err(|err| Error::Json { message: format_json_error(&err) })?;
    match parsed {
        Value::Object(map) => Ok(map),
        _ => Err(Error::NotObject),
    }
}

fn format_json_error(err: &serde_json::Error) -> String {
    format!("{} (dòng {}, cột {})", err, err.line(), err.column())
}

/// File config hỏng được sao lưu cạnh file gốc, không bao giờ ghi đè lên bản cũ.
pub fn backup_corrupt(path: &Path, bytes: &[u8]) -> Option<PathBuf> {
    let parent = path.parent()?;
    let stamp =
        std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0);
    let mut candidate = parent.join(format!("config.bad.{stamp}.json"));
    let mut counter = 1u32;
    while candidate.exists() {
        candidate = parent.join(format!("config.bad.{stamp}.{counter}.json"));
        counter += 1;
    }
    fs::write(&candidate, bytes).ok()?;
    Some(candidate)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fingerprint_changes_with_content() {
        let a = Fingerprint::of(b"{\"a\":1}");
        let b = Fingerprint::of(b"{\"a\":2}");
        assert_ne!(a, b);
        assert_eq!(a, Fingerprint::of(b"{\"a\":1}"));
    }

    #[test]
    fn accepts_utf8_bom_from_windows_editors() {
        let mut bytes = b"\xEF\xBB\xBF{\"input_method\":\"telex\"}".to_vec();
        let map = parse_object(&bytes).unwrap();
        assert_eq!(map["input_method"], "telex");

        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("config.json");
        bytes.extend_from_slice(b"\n");
        fs::write(&path, &bytes).unwrap();
        let raw = RawConfig::read(&path).unwrap();
        assert_eq!(raw.value["input_method"], "telex");
    }
}
