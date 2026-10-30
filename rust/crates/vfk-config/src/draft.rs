use std::collections::{BTreeMap, HashMap};

use serde_json::{Map, Value};
use vfk_engine::{AppProfile, Config};

use crate::meta::{DirtySet, FieldId};

/// Bản nháp người dùng đang sửa.
///
/// `config` giữ các field scalar + `macros`/`excluded_apps`/`app_modes` đọc từ file.
/// `app_profiles` được tách riêng vì `Config::default()` có preset: nếu dùng
/// `config.app_profiles` thì UI sẽ vô tình ghi toàn bộ preset vào file người dùng.
#[derive(Clone, Debug)]
pub struct Draft {
    pub config: Config,
    pub app_profiles: BTreeMap<String, AppProfile>,
    pub dirty: DirtySet,
}

impl Draft {
    pub fn from_config(mut config: Config, raw: &Map<String, Value>) -> Self {
        let app_profiles = raw
            .get("app_profiles")
            .and_then(Value::as_object)
            .map(|map| {
                map.iter()
                    .filter_map(|(key, value)| {
                        serde_json::from_value::<AppProfile>(value.clone())
                            .ok()
                            .map(|profile| (key.clone(), profile))
                    })
                    .collect()
            })
            .unwrap_or_default();

        // Không dùng preset mặc định trong bản nháp: preset được merge khi preview.
        config.app_profiles.clear();

        Draft { config, app_profiles, dirty: DirtySet::new() }
    }

    pub fn default_with_raw(raw: &Map<String, Value>) -> Self {
        Self::from_config(Config::default(), raw)
    }

    pub fn mark(&mut self, field: FieldId) {
        self.dirty.insert(field);
    }

    pub fn mark_many<I: IntoIterator<Item = FieldId>>(&mut self, fields: I) {
        self.dirty.extend(fields);
    }

    pub fn is_dirty(&self) -> bool {
        !self.dirty.is_empty()
    }

    pub fn clear_dirty(&mut self) {
        self.dirty.clear();
    }

    /// Config hiệu lực để preview/validate: preset + user override.
    pub fn effective_config(&self, presets: &HashMap<String, AppProfile>) -> Config {
        let mut config = self.config.clone();
        config.app_profiles = presets.clone();
        for (app, profile) in &self.app_profiles {
            config.app_profiles.insert(app.clone(), profile.clone());
        }
        config
    }

    /// Dùng khi reset: bỏ toàn bộ override, quay về default.
    pub fn reset(&mut self) {
        self.config = Config::default();
        self.config.app_profiles.clear();
        self.app_profiles.clear();
        self.dirty.clear();
    }
}
