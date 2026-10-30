//! Ghi config atomic: temp cùng thư mục -> fsync -> backup -> rename.

#[cfg(unix)]
use std::fs::File;
use std::fs::{self, OpenOptions};
use std::io::{self, Write};
use std::path::{Path, PathBuf};

use serde_json::{Map, Value};

use crate::error::Error;

#[derive(Clone, Debug)]
pub struct SaveOutcome {
    pub path: PathBuf,
    pub backup: Option<PathBuf>,
    pub bytes: usize,
}

pub fn atomic_write(path: &Path, value: &Map<String, Value>) -> Result<SaveOutcome, Error> {
    if let Some(parent) = path.parent().filter(|p| !p.as_os_str().is_empty()) {
        fs::create_dir_all(parent).map_err(|source| Error::Write { path: parent.to_path_buf(), source })?;
    }

    let mut bytes = serde_json::to_vec_pretty(&Value::Object(value.clone()))
        .map_err(|err| Error::Invalid { message: err.to_string() })?;
    bytes.push(b'\n');

    let tmp = temp_path(path);
    {
        let mut file = OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .open(&tmp)
            .map_err(|source| Error::Write { path: tmp.clone(), source })?;
        file.write_all(&bytes).map_err(|source| Error::Write { path: tmp.clone(), source })?;
        file.sync_all().map_err(|source| Error::Write { path: tmp.clone(), source })?;
    }

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let permissions = fs::metadata(path)
            .map(|metadata| metadata.permissions())
            .unwrap_or_else(|_| std::fs::Permissions::from_mode(0o600));
        let _ = fs::set_permissions(&tmp, permissions);
    }

    let backup = if path.exists() { Some(backup_file(path)?) } else { None };

    fs::rename(&tmp, path).map_err(|source| {
        let _ = fs::remove_file(&tmp);
        Error::Write { path: path.to_path_buf(), source }
    })?;

    #[cfg(unix)]
    if let Some(parent) = path.parent() {
        // fsync directory để rename sống sót sau crash trên filesystem hỗ trợ.
        if let Ok(dir) = File::open(parent) {
            let _ = dir.sync_all();
        }
    }

    Ok(SaveOutcome { path: path.to_path_buf(), backup, bytes: bytes.len() })
}

/// Backup `config.json` -> `config.json.bak`; trả path backup.
pub fn backup_file(path: &Path) -> Result<PathBuf, Error> {
    let backup = backup_path(path);
    if backup.exists() {
        fs::remove_file(&backup).map_err(|source| Error::Write { path: backup.clone(), source })?;
    }
    fs::copy(path, &backup).map_err(|source| Error::Write { path: backup.clone(), source })?;
    Ok(backup)
}

pub fn backup_path(path: &Path) -> PathBuf {
    path.with_extension("json.bak")
}

fn temp_path(path: &Path) -> PathBuf {
    let pid = std::process::id();
    let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("config.json");
    path.with_file_name(format!(".{name}.tmp.{pid}"))
}

pub fn read_optional(path: &Path) -> Result<Option<Vec<u8>>, Error> {
    match fs::read(path) {
        Ok(bytes) => Ok(Some(bytes)),
        Err(err) if err.kind() == io::ErrorKind::NotFound => Ok(None),
        Err(source) => Err(Error::Io { path: path.to_path_buf(), source }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn atomic_write_keeps_backup_and_unknown_keys() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("config.json");
        fs::write(&path, "{\"old\":true}").unwrap();

        let mut map = Map::new();
        map.insert("new".into(), Value::Bool(true));
        let outcome = atomic_write(&path, &map).unwrap();

        assert!(outcome.backup.is_some());
        let saved = fs::read_to_string(&path).unwrap();
        assert!(saved.contains("\"new\": true"));
        let backup = fs::read_to_string(backup_path(&path)).unwrap();
        assert!(backup.contains("\"old\""));
    }
}
