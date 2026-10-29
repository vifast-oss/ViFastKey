//! vfk-engine: tầng application của hexagon. Port nguyên văn `vfk_engine/` (bản Python v4).
//!
//! * Port vào: [`ImeSession::process_key`] -> [`Action`].
//! * Port ra: [`ConfigStore`], [`AppContext`], [`SurroundingText`], [`Dictionary`].
//! * [`testing`]: adapter mẫu, ô nhập giả lập (dùng cho test, CLI, và làm khuôn adapter thật).

pub mod ports;
pub mod session;
pub mod testing;

pub use ports::{
    default_app_profiles, resolve_profile, AppContext, AppProfile, Config, ConfigStore, Dictionary,
    OutputMode, Ports, SurroundingText, UnicodeForm, CONFIG_SCHEMA_VERSION,
};
pub use session::{Action, FieldKind, ImeSession, Key, KeyEvent};
