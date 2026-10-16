pub mod models;

use std::fs::{create_dir_all, File, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};

pub use models::{
    create_default_bands, default_factory_presets, AppConfig, BassBoostConfig, BassBoostMode,
    EqBand, EqualizerPreset, FilterType, DEFAULT_BAND_FREQUENCIES, FACTORY_PRESET_IDS,
};

/// Storage subsystem error types.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StorageError {
    IoError(String),
    SerializationError(String),
    FactoryPresetProtected(String),
    PresetNotFound(String),
    InvalidPresetData(String),
    AppDataNotFound,
}

impl std::fmt::Display for StorageError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            StorageError::IoError(msg) => write!(f, "Storage I/O error: {}", msg),
            StorageError::SerializationError(msg) => {
                write!(f, "Storage serialization error: {}", msg)
            }
            StorageError::FactoryPresetProtected(id) => {
                write!(f, "Factory preset '{}' is protected and cannot be modified or deleted", id)
            }
            StorageError::PresetNotFound(id) => write!(f, "Equalizer preset '{}' not found", id),
            StorageError::InvalidPresetData(msg) => write!(f, "Invalid preset data: {}", msg),
            StorageError::AppDataNotFound => write!(f, "OS AppData directory could not be located"),
        }
    }
}

impl std::error::Error for StorageError {}

/// Dependency-free ISO-8601 UTC timestamp generator.
fn current_iso_timestamp() -> String {
    use std::time::SystemTime;
    let duration = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap_or_default();
    let total_secs = duration.as_secs();
    let second = total_secs % 60;
    let minute = (total_secs / 60) % 60;
    let hour = (total_secs / 3600) % 24;
    let days = total_secs / 86400;

    // Howard Hinnant's algorithm for Gregorian date calculation
    let z = days as i64 + 719468;
    let era = if z >= 0 { z } else { z - 146096 } / 146097;
    let doe = (z - era * 146097) as u32;
    let yoe = (doe - doe / 1024 + doe / 1461 - doe / 142401) / 365;
    let y = yoe as i64 + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = doy - (153 * mp + 2) / 5 + 1;
    let month = if mp < 10 { mp + 3 } else { mp - 9 };
    let year = if month <= 2 { y + 1 } else { y };

    format!("{:04}-{:02}-{:02}T{:02}:{:02}:{:02}Z", year, month, day, hour, minute, second)
}

/// Atomically writes serializable data to disk via a sibling temporary file.
fn atomic_write_json<T: serde::Serialize>(path: &Path, data: &T) -> Result<(), StorageError> {
    let parent = path.parent().unwrap_or_else(|| Path::new("."));
    if !parent.exists() {
        create_dir_all(parent)
            .map_err(|e| StorageError::IoError(format!("Failed to create parent dir: {}", e)))?;
    }

    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);

    let file_stem = path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("storage_atomic");
    let temp_path = parent.join(format!("{}.tmp.{}", file_stem, timestamp));

    let json_bytes = serde_json::to_vec_pretty(data)
        .map_err(|e| StorageError::SerializationError(e.to_string()))?;

    {
        let mut file = OpenOptions::new()
            .create_new(true)
            .write(true)
            .open(&temp_path)
            .map_err(|e| StorageError::IoError(format!("Failed to create temp file: {}", e)))?;

        file.write_all(&json_bytes)
            .map_err(|e| StorageError::IoError(format!("Failed to write temp file: {}", e)))?;

        file.sync_all()
            .map_err(|e| StorageError::IoError(format!("Failed to sync temp file: {}", e)))?;
    }

    // Atomically replace target
    if let Err(rename_err) = std::fs::rename(&temp_path, path) {
        if path.exists() {
            let _ = std::fs::remove_file(path);
            if let Err(retry_err) = std::fs::rename(&temp_path, path) {
                let _ = std::fs::remove_file(&temp_path);
                return Err(StorageError::IoError(format!(
                    "Failed to replace file {:?}: initial err: {}, retry err: {}",
                    path, rename_err, retry_err
                )));
            }
        } else {
            let _ = std::fs::remove_file(&temp_path);
            return Err(StorageError::IoError(format!(
                "Failed to rename temp file to {:?}: {}",
                path, rename_err
            )));
        }
    }

    Ok(())
}

/// 100% offline local JSON storage manager.
/// Manages presets and application configuration in `%APPDATA%/equilizer/`.
#[derive(Debug, Clone)]
pub struct StorageManager {
    base_dir: PathBuf,
}

impl StorageManager {
    /// Presets filename stored in `%APPDATA%/equilizer/presets.json`.
    pub const PRESETS_FILENAME: &'static str = "presets.json";
    /// Settings filename stored in `%APPDATA%/equilizer/settings.json`.
    pub const SETTINGS_FILENAME: &'static str = "settings.json";

    /// Creates a new storage manager operating in the specified base directory.
    pub fn new<P: Into<PathBuf>>(base_dir: P) -> Self {
        Self {
            base_dir: base_dir.into(),
        }
    }

    /// Resolves the default `%APPDATA%/equilizer` directory.
    pub fn from_app_data() -> Result<Self, StorageError> {
        let app_data = std::env::var("APPDATA")
            .ok()
            .map(PathBuf::from)
            .or_else(|| {
                std::env::var("USERPROFILE")
                    .ok()
                    .map(|p| PathBuf::from(p).join("AppData").join("Roaming"))
            })
            .or_else(|| std::env::current_dir().ok())
            .ok_or(StorageError::AppDataNotFound)?;

        let base_dir = app_data.join("equilizer");
        Ok(Self::new(base_dir))
    }

    /// Full path to the `presets.json` file.
    pub fn presets_path(&self) -> PathBuf {
        self.base_dir.join(Self::PRESETS_FILENAME)
    }

    /// Full path to the `settings.json` file.
    pub fn settings_path(&self) -> PathBuf {
        self.base_dir.join(Self::SETTINGS_FILENAME)
    }

    /// Ensures the storage directory and initial seeded files exist.
    pub fn ensure_initialized(&self) -> Result<(), StorageError> {
        if !self.base_dir.exists() {
            create_dir_all(&self.base_dir)
                .map_err(|e| StorageError::IoError(format!("Failed to create base dir: {}", e)))?;
        }

        // Initialize presets.json if absent
        let presets_file = self.presets_path();
        if !presets_file.exists() {
            let factory_presets = default_factory_presets();
            atomic_write_json(&presets_file, &factory_presets)?;
        } else {
            // Verify and restore any missing factory presets if file is corrupted/partial
            let current = self.list_presets().unwrap_or_default();
            let mut merged = current;
            let mut modified = false;

            for factory in default_factory_presets() {
                if !merged.iter().any(|p| p.id == factory.id) {
                    merged.push(factory);
                    modified = true;
                }
            }

            if modified {
                atomic_write_json(&presets_file, &merged)?;
            }
        }

        // Initialize settings.json if absent
        let settings_file = self.settings_path();
        if !settings_file.exists() {
            let default_settings = AppConfig::default();
            atomic_write_json(&settings_file, &default_settings)?;
        }

        Ok(())
    }

    /// Reads and returns all presets (factory and custom user presets) from disk.
    pub fn list_presets(&self) -> Result<Vec<EqualizerPreset>, StorageError> {
        let path = self.presets_path();
        if !path.exists() {
            return Ok(default_factory_presets());
        }

        let file = File::open(&path)
            .map_err(|e| StorageError::IoError(format!("Failed to open presets file: {}", e)))?;

        let presets: Vec<EqualizerPreset> = serde_json::from_reader(file)
            .map_err(|e| StorageError::SerializationError(format!("Failed to parse presets: {}", e)))?;

        Ok(presets)
    }

    /// Looks up a preset by its unique ID.
    pub fn get_preset(&self, id: &str) -> Result<Option<EqualizerPreset>, StorageError> {
        let presets = self.list_presets()?;
        Ok(presets.into_iter().find(|p| p.id == id))
    }

    /// Saves or updates a custom user preset.
    /// Factory presets are strictly protected against modification or deletion.
    pub fn save_preset(&self, mut preset: EqualizerPreset) -> Result<EqualizerPreset, StorageError> {
        // Enforce factory protection
        if FACTORY_PRESET_IDS.contains(&preset.id.as_str()) {
            return Err(StorageError::FactoryPresetProtected(preset.id));
        }

        let mut presets = self.list_presets()?;

        // If preset ID is empty, generate an ID slug from name
        if preset.id.trim().is_empty() {
            let slug = preset
                .name
                .to_lowercase()
                .replace(|c: char| !c.is_alphanumeric() && c != '-', "-");
            let rand_suffix = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_millis() % 10000)
                .unwrap_or(0);
            preset.id = format!("{}-{}", slug.trim_matches('-'), rand_suffix);
        }

        preset.is_factory = false;
        let now_ts = current_iso_timestamp();
        preset.updated_at = Some(now_ts.clone());

        if let Some(existing) = presets.iter_mut().find(|p| p.id == preset.id) {
            if existing.is_factory {
                return Err(StorageError::FactoryPresetProtected(preset.id));
            }
            if preset.created_at.is_none() {
                preset.created_at = existing.created_at.clone();
            }
            *existing = preset.clone();
        } else {
            if preset.created_at.is_none() {
                preset.created_at = Some(now_ts);
            }
            presets.push(preset.clone());
        }

        atomic_write_json(&self.presets_path(), &presets)?;
        Ok(preset)
    }

    /// Deletes a custom preset by its ID.
    /// Factory presets return `StorageError::FactoryPresetProtected`.
    pub fn delete_preset(&self, id: &str) -> Result<(), StorageError> {
        if FACTORY_PRESET_IDS.contains(&id) {
            return Err(StorageError::FactoryPresetProtected(id.to_string()));
        }

        let mut presets = self.list_presets()?;
        let initial_len = presets.len();

        if let Some(idx) = presets.iter().position(|p| p.id == id) {
            if presets[idx].is_factory {
                return Err(StorageError::FactoryPresetProtected(id.to_string()));
            }
            presets.remove(idx);
        } else {
            return Err(StorageError::PresetNotFound(id.to_string()));
        }

        if presets.len() < initial_len {
            atomic_write_json(&self.presets_path(), &presets)?;
        }

        Ok(())
    }

    /// Loads application configuration from `settings.json`.
    pub fn load_settings(&self) -> Result<AppConfig, StorageError> {
        let path = self.settings_path();
        if !path.exists() {
            return Ok(AppConfig::default());
        }

        let file = File::open(&path)
            .map_err(|e| StorageError::IoError(format!("Failed to open settings file: {}", e)))?;

        let config: AppConfig = serde_json::from_reader(file)
            .map_err(|e| StorageError::SerializationError(format!("Failed to parse settings: {}", e)))?;

        Ok(config)
    }

    /// Atomically persists application configuration to `settings.json`.
    pub fn save_settings(&self, settings: &AppConfig) -> Result<(), StorageError> {
        atomic_write_json(&self.settings_path(), settings)
    }

    /// Resets all presets and settings back to factory default states.
    pub fn reset_to_factory_defaults(&self) -> Result<(), StorageError> {
        let factory = default_factory_presets();
        atomic_write_json(&self.presets_path(), &factory)?;
        let default_settings = AppConfig::default();
        atomic_write_json(&self.settings_path(), &default_settings)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_storage() -> (StorageManager, PathBuf) {
        let temp_dir = std::env::temp_dir().join(format!(
            "equilizer_test_storage_{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let manager = StorageManager::new(&temp_dir);
        (manager, temp_dir)
    }

    #[test]
    fn test_storage_initialization_seeds_factory_presets() {
        let (storage, temp_dir) = create_test_storage();
        let init_res = storage.ensure_initialized();
        assert!(init_res.is_ok(), "Storage init failed: {:?}", init_res);

        assert!(storage.presets_path().exists());
        assert!(storage.settings_path().exists());

        let presets = storage.list_presets().expect("Failed to list presets");
        assert_eq!(presets.len(), 5);
        assert_eq!(presets[0].id, "flat");
        assert_eq!(presets[1].id, "deep-bass");

        let settings = storage.load_settings().expect("Failed to load settings");
        assert_eq!(settings.active_preset_id, "flat");
        assert!(settings.limiter_enabled);

        let _ = std::fs::remove_dir_all(temp_dir);
    }

    #[test]
    fn test_custom_preset_lifecycle() {
        let (storage, temp_dir) = create_test_storage();
        storage.ensure_initialized().unwrap();

        let new_preset = EqualizerPreset {
            id: "my-custom-eq".to_string(),
            name: "My Custom EQ".to_string(),
            is_factory: false,
            preamp_gain_db: -1.5,
            bass_boost: BassBoostConfig {
                enabled: true,
                gain_db: 5.0,
                mode: BassBoostMode::SubBass,
                frequency_hz: 45.0,
            },
            bands: create_default_bands(&[1.0, 2.0, 3.0, 2.0, 1.0, 0.0, -1.0, -2.0, -1.0, 0.0]),
            created_at: None,
            updated_at: None,
        };

        let saved = storage.save_preset(new_preset).expect("Failed to save preset");
        assert_eq!(saved.id, "my-custom-eq");
        assert!(!saved.is_factory);
        assert!(saved.created_at.is_some());
        assert!(saved.updated_at.is_some());

        // Read back
        let retrieved = storage.get_preset("my-custom-eq").unwrap();
        assert!(retrieved.is_some());
        let p = retrieved.unwrap();
        assert_eq!(p.name, "My Custom EQ");
        assert_eq!(p.preamp_gain_db, -1.5);
        assert_eq!(p.bass_boost.gain_db, 5.0);

        // Verify total preset count is 6 (5 factory + 1 custom)
        let all_presets = storage.list_presets().unwrap();
        assert_eq!(all_presets.len(), 6);

        // Update custom preset
        let mut updated_preset = p;
        updated_preset.preamp_gain_db = -4.0;
        storage.save_preset(updated_preset).unwrap();
        let p_updated = storage.get_preset("my-custom-eq").unwrap().unwrap();
        assert_eq!(p_updated.preamp_gain_db, -4.0);

        // Delete custom preset
        assert!(storage.delete_preset("my-custom-eq").is_ok());
        assert!(storage.get_preset("my-custom-eq").unwrap().is_none());
        assert_eq!(storage.list_presets().unwrap().len(), 5);

        let _ = std::fs::remove_dir_all(temp_dir);
    }

    #[test]
    fn test_factory_preset_protection() {
        let (storage, temp_dir) = create_test_storage();
        storage.ensure_initialized().unwrap();

        // Attempt to delete factory preset
        let del_res = storage.delete_preset("flat");
        assert_eq!(
            del_res,
            Err(StorageError::FactoryPresetProtected("flat".to_string()))
        );

        let del_res2 = storage.delete_preset("deep-bass");
        assert_eq!(
            del_res2,
            Err(StorageError::FactoryPresetProtected("deep-bass".to_string()))
        );

        // Attempt to overwrite factory preset
        let mut malicious_preset = storage.get_preset("rock").unwrap().unwrap();
        malicious_preset.preamp_gain_db = 6.0;
        let save_res = storage.save_preset(malicious_preset);
        assert_eq!(
            save_res,
            Err(StorageError::FactoryPresetProtected("rock".to_string()))
        );

        let _ = std::fs::remove_dir_all(temp_dir);
    }

    #[test]
    fn test_settings_persistence() {
        let (storage, temp_dir) = create_test_storage();
        storage.ensure_initialized().unwrap();

        let mut settings = storage.load_settings().unwrap();
        settings.active_preset_id = "deep-bass".to_string();
        settings.selected_device_id = Some("Headphones 1".to_string());
        settings.is_bypassed = true;
        settings.limiter_enabled = false;

        storage.save_settings(&settings).unwrap();

        let reloaded = storage.load_settings().unwrap();
        assert_eq!(reloaded.active_preset_id, "deep-bass");
        assert_eq!(reloaded.selected_device_id, Some("Headphones 1".to_string()));
        assert!(reloaded.is_bypassed);
        assert!(!reloaded.limiter_enabled);

        let _ = std::fs::remove_dir_all(temp_dir);
    }

    #[test]
    fn test_reset_to_factory_defaults() {
        let (storage, temp_dir) = create_test_storage();
        storage.ensure_initialized().unwrap();

        let mut custom = default_factory_presets()[0].clone();
        custom.id = "user-1".to_string();
        custom.is_factory = false;
        storage.save_preset(custom).unwrap();
        assert_eq!(storage.list_presets().unwrap().len(), 6);

        storage.reset_to_factory_defaults().unwrap();
        assert_eq!(storage.list_presets().unwrap().len(), 5);

        let _ = std::fs::remove_dir_all(temp_dir);
    }
}
