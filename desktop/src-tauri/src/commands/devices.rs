use tauri::State;

use crate::audio::{enumerate_output_devices, AudioDeviceInfo};
use crate::state::AppState;

/// Enumerates all active playback audio endpoints on the operating system.
#[tauri::command]
pub fn get_audio_devices() -> Result<Vec<AudioDeviceInfo>, String> {
    enumerate_output_devices().map_err(|e| e.to_string())
}

/// Reroutes audio streaming to the specified output hardware device.
/// Passing `None` or an empty string selects the operating system default audio endpoint.
#[tauri::command]
pub fn set_output_device(
    state: State<'_, AppState>,
    device_id: Option<String>,
) -> Result<(), String> {
    state.set_output_device(device_id)
}

/// Starts audio playback and loopback capture streaming on the active pipeline.
#[tauri::command]
pub fn start_audio_pipeline(state: State<'_, AppState>) -> Result<(), String> {
    state.start_pipeline().map_err(|e| e.to_string())
}

/// Pauses audio playback and loopback capture streaming on the active pipeline.
#[tauri::command]
pub fn pause_audio_pipeline(state: State<'_, AppState>) -> Result<(), String> {
    state.pause_pipeline().map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::StorageManager;

    #[test]
    fn test_device_info_camel_case_serialization() {
        let info = AudioDeviceInfo {
            id: "Speakers-1".to_string(),
            name: "Speakers (Realtek Audio)".to_string(),
            is_default: true,
        };

        let json = serde_json::to_string(&info).expect("Failed to serialize AudioDeviceInfo");
        assert!(json.contains("\"isDefault\":true"));
        assert!(json.contains("\"id\":\"Speakers-1\""));
        assert!(json.contains("\"name\":\"Speakers (Realtek Audio)\""));
    }

    #[test]
    fn test_get_audio_devices_command() {
        let res = get_audio_devices();
        assert!(res.is_ok(), "Device enumeration should succeed: {:?}", res);
    }

    #[test]
    fn test_set_output_device_lifecycle() {
        let temp_dir = std::env::temp_dir().join(format!(
            "equilizer_test_app_state_{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let storage = StorageManager::new(&temp_dir);
        let app_state = AppState::new(storage);

        // Attempting to route to default device
        let res = app_state.set_output_device(None);
        // In systems with audio output, it succeeds; if headless CI, it will return an error string
        if let Err(ref e) = res {
            eprintln!("set_output_device returned (expected on headless soundless hosts): {}", e);
        }

        // Verify invalid device name returns error without crashing
        let invalid_res = app_state.set_output_device(Some("NonExistentAudioDeviceXYZ123".to_string()));
        assert!(invalid_res.is_err());

        let _ = std::fs::remove_dir_all(temp_dir);
    }

    #[test]
    fn test_pipeline_streaming_control() {
        let temp_dir = std::env::temp_dir().join(format!(
            "equilizer_test_streaming_ctrl_{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let storage = StorageManager::new(&temp_dir);
        let app_state = AppState::new(storage);

        // start and pause calls should handle state cleanly
        let _ = app_state.start_pipeline();
        let pause_res = app_state.pause_pipeline();
        assert!(pause_res.is_ok());

        let _ = std::fs::remove_dir_all(temp_dir);
    }
}
