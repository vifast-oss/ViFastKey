use std::io;
use std::path::PathBuf;

use thiserror::Error;

/// Lỗi đọc/ghi/validate config. Message hướng tới người dùng, có đường dẫn khi cần.
#[derive(Debug, Error)]
pub enum Error {
    #[error("Không đọc được {path}: {source}")]
    Io {
        path: PathBuf,
        #[source]
        source: io::Error,
    },

    #[error("Không ghi được {path}: {source}")]
    Write {
        path: PathBuf,
        #[source]
        source: io::Error,
    },

    #[error("JSON không hợp lệ: {message}")]
    Json { message: String },

    #[error("Config phải là một JSON object")]
    NotObject,

    #[error("Config không khớp schema: {message}")]
    Invalid { message: String },

    #[error("File config bị hỏng: {message}")]
    Corrupt { message: String, backup: Option<PathBuf> },

    #[error("Xung đột cấu hình ở: {fields:?}")]
    Conflict { fields: Vec<String> },
}
