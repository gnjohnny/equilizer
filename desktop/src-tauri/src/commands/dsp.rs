use tauri::State;
use crate::state::AppState;
use crate::storage::{BassBoostConfig, EqBand};

/// Updates active parametric equalizer bands in real time.
#[tauri::command]
pub fn update_eq_bands(
    state: State<'_, AppState>,
    bands: Vec<EqBand>,
) -> Result<(), String> {
    state.update_eq_bands(bands)
}

/// Updates dedicated bass boost module parameters (gain and mode).
#[tauri::command]
pub fn update_bass_boost(
    state: State<'_, AppState>,
    config: BassBoostConfig,
) -> Result<(), String> {
    state.update_bass_boost(config)
}

/// Updates master pre-amp gain stage in decibels (-20 dB to +6 dB).
#[tauri::command]
pub fn set_preamp_gain(
    state: State<'_, AppState>,
    gain_db: f32,
) -> Result<(), String> {
    state.set_preamp_gain(gain_db)
}

/// Seamlessly toggles global master bypass for instantaneous A/B listening tests.
#[tauri::command]
pub fn toggle_bypass(
    state: State<'_, AppState>,
    bypassed: bool,
) -> Result<(), String> {
    state.toggle_bypass(bypassed)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::{create_default_bands, BassBoostMode, StorageManager};

    fn setup_test_app_state() -> (AppState, std::path::PathBuf) {
        let temp_dir = std::env::temp_dir().join(format!(
            "equilizer_dsp_cmd_test_{}",
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
    fn test_update_eq_bands_clamping() {
        let (app_state, temp_dir) = setup_test_app_state();
        let mut bands = create_default_bands(&[0.0; 10]);
        // Set extreme values to test clamping
        bands[0].gain_db = 50.0; // should clamp to 24.0
        bands[1].gain_db = -50.0; // should clamp to -24.0
        bands[2].frequency_hz = 1.0; // should clamp to 20.0
        bands[3].frequency_hz = 50000.0; // should clamp to 20000.0
        bands[4].q = 0.01; // should clamp to 0.1
        bands[5].q = 25.0; // should clamp to 10.0

        let res = app_state.update_eq_bands(bands);
        assert!(res.is_ok(), "update_eq_bands should succeed: {:?}", res);

        let _ = std::fs::remove_dir_all(temp_dir);
    }

    #[test]
    fn test_update_bass_boost() {
        let (app_state, temp_dir) = setup_test_app_state();
        let config = BassBoostConfig {
            enabled: true,
            gain_db: 15.0, // should clamp to 12.0
            mode: BassBoostMode::SubBass,
            frequency_hz: 45.0,
        };

        let res = app_state.update_bass_boost(config);
        assert!(res.is_ok(), "update_bass_boost should succeed: {:?}", res);

        let _ = std::fs::remove_dir_all(temp_dir);
    }

    #[test]
    fn test_set_preamp_gain() {
        let (app_state, temp_dir) = setup_test_app_state();
        let res = app_state.set_preamp_gain(-6.0);
        assert!(res.is_ok());

        // Extreme values clamped safely
        let res2 = app_state.set_preamp_gain(-100.0);
        assert!(res2.is_ok());
        let res3 = app_state.set_preamp_gain(100.0);
        assert!(res3.is_ok());

        let _ = std::fs::remove_dir_all(temp_dir);
    }

    #[test]
    fn test_toggle_bypass_persists_to_settings() {
        let (app_state, temp_dir) = setup_test_app_state();

        assert!(app_state.toggle_bypass(true).is_ok());
        let settings = app_state.storage.load_settings().unwrap();
        assert!(settings.is_bypassed);

        assert!(app_state.toggle_bypass(false).is_ok());
        let settings2 = app_state.storage.load_settings().unwrap();
        assert!(!settings2.is_bypassed);

        let _ = std::fs::remove_dir_all(temp_dir);
    }
}
