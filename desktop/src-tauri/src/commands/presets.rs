use tauri::State;
use crate::state::AppState;
use crate::storage::EqualizerPreset;

/// Loads all factory and user equalizer presets from disk.
#[tauri::command]
pub fn list_presets(state: State<'_, AppState>) -> Result<Vec<EqualizerPreset>, String> {
    state.list_presets()
}

/// Saves or updates a custom equalizer preset to disk.
#[tauri::command]
pub fn save_preset(
    state: State<'_, AppState>,
    preset: EqualizerPreset,
) -> Result<EqualizerPreset, String> {
    state.save_preset(preset)
}

/// Deletes a custom user preset from disk. Factory presets cannot be deleted.
#[tauri::command]
pub fn delete_preset(
    state: State<'_, AppState>,
    preset_id: String,
) -> Result<(), String> {
    state.delete_preset(&preset_id)
}

/// Sets the active equalizer preset, syncing parameters to the live audio pipeline and persisting selection.
#[tauri::command]
pub fn set_active_preset(
    state: State<'_, AppState>,
    preset_id: String,
) -> Result<EqualizerPreset, String> {
    state.set_active_preset(&preset_id)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::{create_default_bands, BassBoostConfig, BassBoostMode, StorageManager};

    fn setup_test_app_state() -> (AppState, std::path::PathBuf) {
        let temp_dir = std::env::temp_dir().join(format!(
            "equilizer_presets_cmd_test_{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let storage = StorageManager::new(&temp_dir);
        let app_state = AppState::new(storage);
        (app_state, temp_dir)
    }

    #[test]
    fn test_list_presets() {
        let (app_state, temp_dir) = setup_test_app_state();
        let presets = app_state.list_presets().expect("Failed to list presets");
        assert_eq!(presets.len(), 5);
        assert!(presets.iter().any(|p| p.id == "flat"));
        assert!(presets.iter().any(|p| p.id == "rock"));

        let _ = std::fs::remove_dir_all(temp_dir);
    }

    #[test]
    fn test_save_and_delete_custom_preset() {
        let (app_state, temp_dir) = setup_test_app_state();

        let custom = EqualizerPreset {
            id: "my-preset".to_string(),
            name: "My Preset".to_string(),
            is_factory: false,
            preamp_gain_db: -2.0,
            bass_boost: BassBoostConfig {
                enabled: true,
                gain_db: 4.0,
                mode: BassBoostMode::SubBass,
                frequency_hz: 45.0,
            },
            bands: create_default_bands(&[1.0; 10]),
            created_at: None,
            updated_at: None,
        };

        let saved = app_state.save_preset(custom).expect("Save preset failed");
        assert_eq!(saved.id, "my-preset");
        assert_eq!(app_state.list_presets().unwrap().len(), 6);

        // Delete preset
        assert!(app_state.delete_preset("my-preset").is_ok());
        assert_eq!(app_state.list_presets().unwrap().len(), 5);

        let _ = std::fs::remove_dir_all(temp_dir);
    }

    #[test]
    fn test_factory_preset_protection_in_commands() {
        let (app_state, temp_dir) = setup_test_app_state();

        // Attempting to delete factory preset must fail
        let del_res = app_state.delete_preset("flat");
        assert!(del_res.is_err());

        // Attempting to overwrite factory preset must fail
        let mut flat = app_state.storage.get_preset("flat").unwrap().unwrap();
        flat.preamp_gain_db = 5.0;
        let save_res = app_state.save_preset(flat);
        assert!(save_res.is_err());

        let _ = std::fs::remove_dir_all(temp_dir);
    }

    #[test]
    fn test_set_active_preset_and_deletion_fallback() {
        let (app_state, temp_dir) = setup_test_app_state();

        // Activate "rock"
        let active = app_state.set_active_preset("rock").expect("Set active failed");
        assert_eq!(active.id, "rock");
        assert_eq!(app_state.storage.load_settings().unwrap().active_preset_id, "rock");

        // Create and activate custom preset
        let custom = EqualizerPreset {
            id: "temp-active".to_string(),
            name: "Temp Active".to_string(),
            is_factory: false,
            preamp_gain_db: 0.0,
            bass_boost: BassBoostConfig::default(),
            bands: create_default_bands(&[0.0; 10]),
            created_at: None,
            updated_at: None,
        };
        app_state.save_preset(custom).unwrap();
        app_state.set_active_preset("temp-active").unwrap();
        assert_eq!(app_state.storage.load_settings().unwrap().active_preset_id, "temp-active");

        // Deleting the active preset must fallback to "flat"
        app_state.delete_preset("temp-active").unwrap();
        assert_eq!(app_state.storage.load_settings().unwrap().active_preset_id, "flat");

        let _ = std::fs::remove_dir_all(temp_dir);
    }
}
