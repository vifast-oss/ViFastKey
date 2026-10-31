//! Cấu hình ViFastKey dùng chung cho UI desktop và adapter.
//!
//! Crate này không phụ thuộc egui. Nó chịu trách nhiệm:
//! - đường dẫn config theo nền tảng;
//! - giữ nguyên key lạ khi đọc/ghi;
//! - merge theo từng key cho `macros`/`app_profiles`/`app_modes`;
//! - validate cross-field;
//! - ghi atomic + backup.

pub mod diff;
pub mod document;
pub mod draft;
pub mod error;
pub mod meta;
pub mod paths;
pub mod raw;
pub mod store;
pub mod validate;

pub use diff::{diff_json, Change};
pub use document::{ConfigDocument, ImportPreview, SettingsGroup};
pub use draft::Draft;
pub use error::Error;
pub use meta::{DirtySet, FieldId};
pub use raw::{Fingerprint, RawConfig};
pub use store::SaveOutcome;
pub use validate::{validate, validate_macro_rows, Issue, Severity, METHODS};
pub use vfk_core::CHARSETS;
pub use vi_syllable::ToneStyle;
