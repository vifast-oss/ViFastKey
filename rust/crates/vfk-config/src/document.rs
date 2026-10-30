use std::collections::{BTreeMap, HashMap};
use std::path::{Path, PathBuf};

use serde_json::{json, Map, Value};
use vfk_engine::{default_app_profiles, AppProfile, Config, CONFIG_SCHEMA_VERSION};

use crate::draft::Draft;
use crate::error::Error;
use crate::meta::{DirtySet, FieldId};
use crate::raw::{self, Fingerprint, RawConfig};
use crate::store::{self, SaveOutcome};
use crate::validate::{self, Issue};

/// Tài liệu config đang mở: baseline trên đĩa + bản nháp + preset.
#[derive(Clone, Debug)]
pub struct ConfigDocument {
    path: PathBuf,
    baseline: RawConfig,
    baseline_config: Config,
    draft: Draft,
    presets: HashMap<String, AppProfile>,
    force_write: bool,
    clear_on_save: bool,
    allow_corrupt_overwrite: bool,
    /// Import thay thế toàn bộ document; giữ cả key lạ của file import.
    replace_with: Option<Map<String, Value>>,
}

impl ConfigDocument {
    pub fn load(path: impl Into<PathBuf>) -> Result<Self, Error> {
        let path = path.into();
        let bytes = store::read_optional(&path)?;

        let (baseline, baseline_config) = match bytes {
            None => (RawConfig::missing(), Config::default()),
            Some(bytes) => {
                let map = match raw::parse_object(&bytes) {
                    Ok(map) => map,
                    Err(err) => {
                        let message = err.to_string();
                        let backup = raw::backup_corrupt(&path, &bytes);
                        return Err(Error::Corrupt { message, backup });
                    }
                };
                let config = match serde_json::from_value::<Config>(Value::Object(map.clone())) {
                    Ok(config) => config,
                    Err(err) => {
                        let backup = raw::backup_corrupt(&path, &bytes);
                        return Err(Error::Invalid {
                            message: format!(
                                "{err} (đã sao lưu thành {})",
                                backup
                                    .as_ref()
                                    .map(|p| p.display().to_string())
                                    .unwrap_or_else(|| "không sao lưu được".into())
                            ),
                        });
                    }
                };
                (RawConfig::from_value(map, &bytes), config)
            }
        };

        let mut baseline_config = baseline_config;
        baseline_config.app_profiles.clear();
        let draft = Draft::from_config(baseline_config.clone(), &baseline.value);

        Ok(ConfigDocument {
            path,
            baseline,
            baseline_config,
            draft,
            presets: default_app_profiles(),
            force_write: false,
            clear_on_save: false,
            allow_corrupt_overwrite: false,
            replace_with: None,
        })
    }

    /// Tạo document mới (chưa ghi file) từ default. Dùng khi file hỏng và người
    /// dùng chọn “Dùng mặc định”, hoặc cho `--self-test`.
    pub fn new_default(path: impl Into<PathBuf>) -> Self {
        let mut baseline_config = Config::default();
        baseline_config.app_profiles.clear();
        ConfigDocument {
            path: path.into(),
            baseline: RawConfig::missing(),
            baseline_config: baseline_config.clone(),
            draft: Draft::from_config(baseline_config, &Map::new()),
            presets: default_app_profiles(),
            force_write: false,
            clear_on_save: false,
            allow_corrupt_overwrite: true,
            replace_with: None,
        }
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn exists(&self) -> bool {
        self.baseline.exists
    }

    pub fn fingerprint(&self) -> Option<&Fingerprint> {
        self.baseline.fingerprint.as_ref()
    }

    pub fn draft(&self) -> &Draft {
        &self.draft
    }

    pub fn draft_mut(&mut self) -> &mut Draft {
        &mut self.draft
    }

    pub fn config(&self) -> &Config {
        &self.draft.config
    }

    pub fn presets(&self) -> &HashMap<String, AppProfile> {
        &self.presets
    }

    pub fn profiles(&self) -> &BTreeMap<String, AppProfile> {
        &self.draft.app_profiles
    }

    pub fn preset(&self, app: &str) -> Option<&AppProfile> {
        self.presets.get(app)
    }

    pub fn profile(&self, app: &str) -> Option<&AppProfile> {
        self.draft.app_profiles.get(app)
    }

    pub fn effective_config(&self) -> Config {
        self.draft.effective_config(&self.presets)
    }

    pub fn dirty(&self) -> &DirtySet {
        &self.draft.dirty
    }

    pub fn is_dirty(&self) -> bool {
        self.draft.is_dirty() || self.force_write || self.replace_with.is_some()
    }

    pub fn mark(&mut self, field: FieldId) {
        self.draft.mark(field);
    }

    pub fn mark_many<I: IntoIterator<Item = FieldId>>(&mut self, fields: I) {
        self.draft.mark_many(fields);
    }

    pub fn validate(&self) -> Vec<Issue> {
        validate::validate(&self.draft, &self.presets)
    }

    pub fn app_ids(&self) -> Vec<String> {
        let mut ids = BTreeMap::<String, ()>::new();
        for app in self.presets.keys() {
            ids.insert(app.clone(), ());
        }
        for app in self.draft.app_profiles.keys() {
            ids.insert(app.clone(), ());
        }
        for app in &self.draft.config.excluded_apps {
            ids.insert(app.clone(), ());
        }
        ids.into_keys().collect()
    }

    pub fn set_profile(&mut self, app: impl Into<String>, profile: AppProfile) {
        let app = app.into();
        if self.draft.app_profiles.get(&app) == Some(&profile) {
            return;
        }
        self.draft.app_profiles.insert(app, profile);
        self.draft.mark(FieldId::AppProfiles);
    }

    pub fn remove_profile(&mut self, app: &str) {
        if self.draft.app_profiles.remove(app).is_some() {
            self.draft.mark(FieldId::AppProfiles);
        }
    }

    pub fn set_excluded_apps(&mut self, apps: Vec<String>) {
        if self.draft.config.excluded_apps == apps {
            return;
        }
        self.draft.config.excluded_apps = apps;
        self.draft.mark(FieldId::ExcludedApps);
    }

    pub fn set_macros(&mut self, macros: BTreeMap<String, String>) {
        let macros: HashMap<String, String> = macros.into_iter().collect();
        if self.draft.config.macros == macros {
            return;
        }
        self.draft.config.macros = macros;
        self.draft.mark(FieldId::Macros);
    }

    pub fn clear_app_mode(&mut self, app: &str) {
        if self.draft.config.app_modes.remove(app).is_some() {
            self.draft.mark(FieldId::AppModes);
        }
    }

    /// Reset về default tối thiểu: lần lưu kế tiếp ghi `{ "schema_version": 1 }`.
    pub fn reset(&mut self) {
        self.draft.reset();
        self.draft.mark(FieldId::SchemaVersion);
        self.force_write = true;
        self.clear_on_save = true;
        self.allow_corrupt_overwrite = true;
    }

    /// Nạp file JSON để thay thế toàn bộ bản nháp. Không ghi file ngay; người dùng
    /// vẫn phải bấm Lưu (và file cũ được backup ở bước ghi).
    pub fn import_json(&mut self, text: &str) -> Result<(), Error> {
        let map = raw::parse_object(text.as_bytes())?;
        let config: Config = serde_json::from_value(Value::Object(map.clone()))
            .map_err(|err| Error::Invalid { message: err.to_string() })?;
        let mut draft_config = config;
        draft_config.app_profiles.clear();
        self.draft = Draft::from_config(draft_config, &map);
        self.replace_with = Some(map);
        self.force_write = true;
        self.clear_on_save = false;
        self.allow_corrupt_overwrite = true;
        Ok(())
    }

    pub fn has_external_change(&self) -> bool {
        match RawConfig::read(&self.path) {
            Ok(raw) => raw.fingerprint != self.baseline.fingerprint,
            Err(_) => true,
        }
    }

    pub fn external_fingerprint(&self) -> Result<Option<Fingerprint>, Error> {
        Ok(RawConfig::read(&self.path)?.fingerprint)
    }

    pub fn reload(&mut self) -> Result<(), Error> {
        *self = ConfigDocument::load(self.path.clone())?;
        Ok(())
    }

    /// JSON đầy đủ để hiển thị/copy; không dùng để ghi file.
    pub fn pretty_effective_json(&self) -> String {
        serde_json::to_string_pretty(&self.effective_config()).unwrap_or_else(|_| "{}".to_string())
    }

    /// Lưu các field đã sửa. Trả `None` nếu không có gì thay đổi.
    pub fn save(&mut self) -> Result<Option<SaveOutcome>, Error> {
        if !self.is_dirty() && !self.force_write && self.baseline.exists {
            return Ok(None);
        }

        let current = match RawConfig::read(&self.path) {
            Ok(current) => current,
            Err(err)
                if self.allow_corrupt_overwrite && matches!(err, Error::Json { .. } | Error::NotObject) =>
            {
                if let Ok(Some(bytes)) = raw::read_bytes(&self.path) {
                    let _ = raw::backup_corrupt(&self.path, &bytes);
                }
                RawConfig::missing()
            }
            Err(err) => return Err(err),
        };
        let replacing = self.replace_with.is_some();
        let mut out = if let Some(replace) = &self.replace_with {
            replace.clone()
        } else if self.clear_on_save {
            Map::new()
        } else {
            current.value.clone()
        };
        let base_typed = serde_json::to_value(&self.baseline_config)
            .map_err(|err| Error::Invalid { message: err.to_string() })?;
        let mine_typed = serde_json::to_value(&self.draft.config)
            .map_err(|err| Error::Invalid { message: err.to_string() })?;

        let mut conflicts = Vec::new();

        for field in &self.draft.dirty {
            if field.is_map() {
                if replacing {
                    let mine = self.map_for(*field);
                    if mine.is_empty() {
                        out.remove(field.as_key());
                    } else {
                        out.insert(field.as_key().to_string(), Value::Object(mine));
                    }
                } else {
                    self.merge_map_field(*field, &current, &mut out, &mut conflicts);
                }
                continue;
            }

            let key = field.as_key();
            let mine = mine_typed.get(key).cloned();
            let base = base_typed.get(key).cloned();
            let Some(mine) = mine else { continue };
            if replacing {
                out.insert(key.to_string(), mine);
                continue;
            }
            if Some(&mine) == base.as_ref() {
                continue;
            }

            let current_value = out.get(key).cloned().or_else(|| base.clone());
            if current_value != base && current_value.as_ref() != Some(&mine) {
                conflicts.push(key.to_string());
                continue;
            }
            out.insert(key.to_string(), mine);
        }

        if self.force_write || out.get("schema_version").is_none() {
            let version = self.draft.config.schema_version.max(CONFIG_SCHEMA_VERSION);
            out.insert("schema_version".into(), json!(version));
        }

        if !conflicts.is_empty() {
            conflicts.sort();
            conflicts.dedup();
            return Err(Error::Conflict { fields: conflicts });
        }

        // Validate toàn bộ object trước khi ghi, kể cả key lạ bị bỏ qua.
        serde_json::from_value::<Config>(Value::Object(out.clone()))
            .map_err(|err| Error::Invalid { message: err.to_string() })?;

        let outcome = store::atomic_write(&self.path, &out)?;

        // Đọc lại để baseline/fingerprint khớp chính xác bytes vừa ghi.
        let saved = RawConfig::read(&self.path)?;
        let mut saved_config = serde_json::from_value::<Config>(Value::Object(saved.value.clone()))
            .map_err(|err| Error::Invalid { message: err.to_string() })?;
        saved_config.app_profiles.clear();
        self.baseline = saved;
        self.baseline_config = saved_config;
        self.draft.clear_dirty();
        self.force_write = false;
        self.clear_on_save = false;
        self.allow_corrupt_overwrite = false;
        self.replace_with = None;

        Ok(Some(outcome))
    }

    fn map_for(&self, field: FieldId) -> Map<String, Value> {
        match field {
            FieldId::Macros => {
                self.draft.config.macros.iter().map(|(k, v)| (k.clone(), Value::String(v.clone()))).collect()
            }
            FieldId::AppModes => {
                self.draft.config.app_modes.iter().map(|(k, v)| (k.clone(), Value::Bool(*v))).collect()
            }
            FieldId::AppProfiles => self
                .draft
                .app_profiles
                .iter()
                .filter_map(|(k, v)| serde_json::to_value(v).ok().map(|value| (k.clone(), value)))
                .collect(),
            _ => Map::new(),
        }
    }

    fn merge_map_field(
        &self,
        field: FieldId,
        current: &RawConfig,
        out: &mut Map<String, Value>,
        conflicts: &mut Vec<String>,
    ) {
        let key = field.as_key();
        let base_map = self.baseline.get(key).and_then(Value::as_object).cloned().unwrap_or_default();
        let current_map = current.get(key).and_then(Value::as_object).cloned().unwrap_or_default();

        let mine_map = self.map_for(field);
        if mine_map.is_empty() && base_map.is_empty() && current_map.is_empty() {
            return;
        }

        let mut merged = Map::new();
        let mut changed = false;
        let mut keys = BTreeMap::<String, ()>::new();
        for key in base_map.keys().chain(current_map.keys()).chain(mine_map.keys()) {
            keys.insert(key.clone(), ());
        }

        for key in keys.into_keys() {
            let base = base_map.get(&key);
            let current_value = current_map.get(&key);
            let mine = mine_map.get(&key);

            let chosen = if mine == base {
                current_value.cloned()
            } else if current_value == base {
                changed = true;
                mine.cloned()
            } else if current_value == mine {
                current_value.cloned()
            } else {
                conflicts.push(format!("{}.{}", field.as_key(), key));
                current_value.cloned()
            };

            if let Some(value) = chosen {
                merged.insert(key, value);
            }
        }

        if changed || current_map != merged {
            if merged.is_empty() {
                out.remove(key);
            } else {
                out.insert(key.to_string(), Value::Object(merged));
            }
        }
    }
}
