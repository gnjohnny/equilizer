use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;
use crate::audio::{AudioError, AudioPipeline, LimiterConfig};
use crate::storage::StorageManager;

/// Thread-safe global application state managed by Tauri.
pub struct AppState {
    pub pipeline: Mutex<Option<AudioPipeline>>,
    pub storage: StorageManager,
    pub original_default_endpoint_id: Mutex<Option<String>>,
    pub is_system_routed: AtomicBool,
}

impl AppState {
    /// Automatically resolves the optimal decoupled capture and render devices.
    /// If a virtual audio sink (e.g., Boom Audio, VB-Cable, Voicemeeter) exists on the host,
    /// it is selected for loopback capture, while physical hardware is selected for render output.
    pub fn resolve_pipeline_routing(
        requested_render: Option<&str>,
        requested_capture: Option<&str>,
    ) -> (Option<String>, Option<String>) {
        let devices = crate::audio::enumerate_output_devices().unwrap_or_default();
        if devices.is_empty() {
            return (
                requested_capture.map(|s| s.to_string()),
                requested_render.map(|s| s.to_string()),
            );
        }

        let is_virtual = |name: &str| -> bool {
            let n = name.to_lowercase();
            n.contains("boom")
                || n.contains("cable")
                || n.contains("virtual")
                || n.contains("voicemeeter")
                || n.contains("stereo mix")
        };

        // 1. Determine Capture Device
        let capture_target = if let Some(req) = requested_capture.filter(|s| !s.trim().is_empty()) {
            Some(req.to_string())
        } else {
            // Auto-detect virtual device to serve as loopback recording tap
            devices
                .iter()
                .find(|d| is_virtual(&d.name))
                .map(|d| d.name.clone())
        };

        // 2. Determine Render Device
        let render_target = if let Some(req) = requested_render.filter(|s| !s.trim().is_empty()) {
            Some(req.to_string())
        } else if let Some(ref cap) = capture_target {
            // Capture is assigned to a virtual sink; route render to physical output hardware
            devices
                .iter()
                .find(|d| !is_virtual(&d.name) && d.name != *cap)
                .map(|d| d.name.clone())
                .or_else(|| {
                    devices
                        .iter()
                        .find(|d| d.is_default && d.name != *cap)
                        .map(|d| d.name.clone())
                })
        } else {
            devices
                .iter()
                .find(|d| d.is_default)
                .map(|d| d.name.clone())
        };

        // 3. Circular Feedback Protection:
        // If capture and render resolved to the same physical device, attempt to decouple via virtual sink
        let (final_capture, final_render) = match (&capture_target, &render_target) {
            (Some(c), Some(r)) if c == r && !is_virtual(c) => {
                if let Some(virt) = devices.iter().find(|d| is_virtual(&d.name)) {
                    (Some(virt.name.clone()), Some(r.clone()))
                } else {
                    (capture_target, render_target)
                }
            }
            _ => (capture_target, render_target),
        };

        (final_capture, final_render)
    }

    /// Creates and initializes application state with the given storage manager.
    pub fn new(storage: StorageManager) -> Self {
        let _ = storage.ensure_initialized();
        let settings = storage.load_settings().unwrap_or_default();

        // Query the active system default audio endpoint before any changes
        let original_endpoint = crate::audio::get_default_endpoint().ok();
        let original_id = original_endpoint.as_ref().map(|(_, id)| id.clone());
        if let Some(ref id) = original_id {
            crate::audio::record_original_endpoint(id);
        }

        let (capture_target, render_target) = Self::resolve_pipeline_routing(
            settings.selected_device_id.as_deref(),
            settings.capture_device_id.as_deref(),
        );

        let pipeline = AudioPipeline::new_routed(
            capture_target.as_deref(),
            render_target.as_deref(),
        )
        .or_else(|_| AudioPipeline::from_default_device())
        .ok();

        let mut is_routed = false;

        // If capture was resolved to a virtual sink (e.g. Boom Audio),
        // automatically switch Windows system audio to that virtual sink so all sound flows into Equilizer!
        if let Some(ref cap) = capture_target {
            let cap_lower = cap.to_lowercase();
            if cap_lower.contains("boom")
                || cap_lower.contains("cable")
                || cap_lower.contains("virtual")
                || cap_lower.contains("voicemeeter")
            {
                if let Ok(switched) = crate::audio::switch_default_endpoint(cap) {
                    println!(
                        "Successfully routed Windows system audio to virtual sink '{}'",
                        switched
                    );
                    is_routed = true;
                }
            }
        }

        if let Some(ref p) = pipeline {
            Self::sync_pipeline_parameters(p, &storage, &settings);
            if let Err(e) = p.start() {
                eprintln!("Warning: Failed to auto-start audio pipeline on launch: {}", e);
            }
        }

        Self {
            pipeline: Mutex::new(pipeline),
            storage,
            original_default_endpoint_id: Mutex::new(original_id),
            is_system_routed: AtomicBool::new(is_routed),
        }
    }

    /// Restores the original physical playback endpoint as Windows default audio device.
    pub fn restore_system_routing(&self) -> Result<(), String> {
        let lock = self.original_default_endpoint_id.lock().unwrap();
        if let Some(ref orig_id) = *lock {
            crate::audio::switch_default_endpoint(orig_id)?;
            self.is_system_routed.store(false, Ordering::SeqCst);
        }
        Ok(())
    }

    /// Activates system routing by setting Windows default audio endpoint to the capture virtual sink.
    pub fn activate_system_routing(&self) -> Result<(), String> {
        let settings = self.storage.load_settings().unwrap_or_default();
        let (capture_target, _) = Self::resolve_pipeline_routing(
            settings.selected_device_id.as_deref(),
            settings.capture_device_id.as_deref(),
        );

        if let Some(ref cap) = capture_target {
            crate::audio::switch_default_endpoint(cap)?;
            self.is_system_routed.store(true, Ordering::SeqCst);
            Ok(())
        } else {
            Err("No virtual audio sink available to route system audio".to_string())
        }
    }

    /// Toggles system audio routing to the virtual capture sink on or off.
    pub fn toggle_system_routing(&self, enable: bool) -> Result<bool, String> {
        if enable {
            self.activate_system_routing()?;
            Ok(true)
        } else {
            self.restore_system_routing()?;
            Ok(false)
        }
    }

    /// Returns whether system audio is currently routed through the virtual capture sink.
    pub fn is_system_routed(&self) -> bool {
        self.is_system_routed.load(Ordering::SeqCst)
    }

    /// Returns the currently active (capture_device_name, render_device_name) tuple if pipeline is initialized.
    pub fn active_devices(&self) -> Option<(String, String)> {
        let lock = self.pipeline.lock().ok()?;
        lock.as_ref().map(|p| {
            (
                p.capture_device_name().to_string(),
                p.render_device_name().to_string(),
            )
        })
    }

    /// Synchronizes active preset parameters (preamp, EQ bands, bass boost, limiter, bypass) to the pipeline.
    pub fn sync_pipeline_parameters(
        pipeline: &AudioPipeline,
        storage: &StorageManager,
        settings: &crate::storage::AppConfig,
    ) {
        if let Ok(Some(preset)) = storage.get_preset(&settings.active_preset_id) {
            let _ = pipeline.set_preamp_gain(preset.preamp_gain_db);
            let _ = pipeline.set_bass_boost(preset.bass_boost.into());
            let _ = pipeline.set_bypass(settings.is_bypassed);
            let _ = pipeline.set_limiter(LimiterConfig {
                enabled: settings.limiter_enabled,
                ..Default::default()
            });

            if preset.bands.len() == 10 {
                let mut arr: [crate::audio::EqBandConfig; 10] = Default::default();
                for (i, b) in preset.bands.into_iter().enumerate() {
                    arr[i] = b.into();
                }
                let _ = pipeline.update_all_bands(arr);
            }
        }
    }

    /// Sets independent capture and render routing for the audio pipeline and persists them to settings.
    pub fn set_audio_routing(
        &self,
        capture_device_id: Option<String>,
        render_device_id: Option<String>,
    ) -> Result<(), String> {
        let clean_render_id = render_device_id.filter(|s| !s.trim().is_empty());
        let clean_capture_id = capture_device_id.filter(|s| !s.trim().is_empty());

        let (capture_target, render_target) = Self::resolve_pipeline_routing(
            clean_render_id.as_deref(),
            clean_capture_id.as_deref(),
        );

        // Build target pipeline first before acquiring mutex or dropping existing stream
        let new_pipeline = AudioPipeline::new_routed(
            capture_target.as_deref(),
            render_target.as_deref(),
        )
        .map_err(|e| format!("Failed to initialize audio routing: {}", e))?;

        // Sync DSP parameters to the new pipeline
        if let Ok(settings) = self.storage.load_settings() {
            Self::sync_pipeline_parameters(&new_pipeline, &self.storage, &settings);
        }

        let mut lock = self
            .pipeline
            .lock()
            .map_err(|e| format!("Failed to acquire pipeline lock: {}", e))?;

        let was_running = lock.as_ref().map(|p| p.is_running()).unwrap_or(false);

        if was_running {
            new_pipeline
                .start()
                .map_err(|e| format!("Failed to start new audio pipeline: {}", e))?;
        }

        *lock = Some(new_pipeline);

        // Persist routing selection to disk
        if let Ok(mut settings) = self.storage.load_settings() {
            settings.selected_device_id = clean_render_id;
            settings.capture_device_id = clean_capture_id;
            let _ = self.storage.save_settings(&settings);
        }

        Ok(())
    }

    /// Reroutes audio playback to the specified output hardware device while maintaining decoupled loopback capture.
    /// If `device_id` is None or empty, routes to the system default audio endpoint.
    pub fn set_output_device(&self, device_id: Option<String>) -> Result<(), String> {
        let current_capture_id = self
            .storage
            .load_settings()
            .ok()
            .and_then(|s| s.capture_device_id);
        self.set_audio_routing(current_capture_id, device_id)
    }

    /// Gracefully handles device disconnection by resetting to the system default audio endpoint.
    pub fn handle_device_disconnect(&self, disconnected_id: &str) -> Result<String, String> {
        eprintln!(
            "Audio hardware disconnect detected for '{}'. Gracefully falling back to system default.",
            disconnected_id
        );
        self.set_output_device(None)?;
        Ok("default".to_string())
    }

    /// Starts audio playback streaming on the active pipeline.
    pub fn start_pipeline(&self) -> Result<(), AudioError> {
        let lock = self
            .pipeline
            .lock()
            .map_err(|e| AudioError::BackendError(format!("Lock error: {}", e)))?;

        if let Some(ref p) = *lock {
            p.start()
        } else {
            Err(AudioError::DefaultDeviceUnavailable)
        }
    }

    /// Pauses audio playback streaming on the active pipeline.
    pub fn pause_pipeline(&self) -> Result<(), AudioError> {
        let lock = self
            .pipeline
            .lock()
            .map_err(|e| AudioError::BackendError(format!("Lock error: {}", e)))?;

        if let Some(ref p) = *lock {
            p.pause()
        } else {
            Ok(())
        }
    }

    /// Dispatches updated 10-band equalizer settings to the active audio pipeline.
    pub fn update_eq_bands(&self, bands: Vec<crate::storage::EqBand>) -> Result<(), String> {
        let lock = self
            .pipeline
            .lock()
            .map_err(|e| format!("Failed to acquire pipeline lock: {}", e))?;

        if let Some(ref p) = *lock {
            if bands.len() == 10 {
                let mut arr: [crate::audio::EqBandConfig; 10] = Default::default();
                for (i, b) in bands.into_iter().enumerate() {
                    let mut config: crate::audio::EqBandConfig = b.into();
                    config.frequency_hz = config.frequency_hz.clamp(20.0, 20000.0);
                    config.gain_db = config.gain_db.clamp(-24.0, 24.0);
                    config.q = config.q.clamp(0.1, 10.0);
                    arr[i] = config;
                }
                p.update_all_bands(arr)
                    .map_err(|e| format!("Failed to update EQ bands: {}", e))?;
            } else {
                for (i, b) in bands.into_iter().enumerate().take(10) {
                    let mut config: crate::audio::EqBandConfig = b.into();
                    config.frequency_hz = config.frequency_hz.clamp(20.0, 20000.0);
                    config.gain_db = config.gain_db.clamp(-24.0, 24.0);
                    config.q = config.q.clamp(0.1, 10.0);
                    p.update_band(i, config)
                        .map_err(|e| format!("Failed to update band {}: {}", i, e))?;
                }
            }
        }
        Ok(())
    }

    /// Dispatches dedicated bass boost configuration to the active audio pipeline.
    pub fn update_bass_boost(&self, mut config: crate::storage::BassBoostConfig) -> Result<(), String> {
        config.gain_db = config.gain_db.clamp(0.0, 12.0);
        match config.mode {
            crate::storage::BassBoostMode::SubBass => config.frequency_hz = 45.0,
            crate::storage::BassBoostMode::Punch => config.frequency_hz = 90.0,
        }

        let lock = self
            .pipeline
            .lock()
            .map_err(|e| format!("Failed to acquire pipeline lock: {}", e))?;

        if let Some(ref p) = *lock {
            p.set_bass_boost(config.into())
                .map_err(|e| format!("Failed to update bass boost: {}", e))?;
        }
        Ok(())
    }

    /// Dispatches master pre-amp gain to the active audio pipeline.
    pub fn set_preamp_gain(&self, gain_db: f32) -> Result<(), String> {
        let clamped = gain_db.clamp(-20.0, 6.0);
        let lock = self
            .pipeline
            .lock()
            .map_err(|e| format!("Failed to acquire pipeline lock: {}", e))?;

        if let Some(ref p) = *lock {
            p.set_preamp_gain(clamped)
                .map_err(|e| format!("Failed to set preamp gain: {}", e))?;
        }
        Ok(())
    }

    /// Toggles master DSP bypass on the active audio pipeline and persists the setting.
    pub fn toggle_bypass(&self, bypassed: bool) -> Result<(), String> {
        let lock = self
            .pipeline
            .lock()
            .map_err(|e| format!("Failed to acquire pipeline lock: {}", e))?;

        if let Some(ref p) = *lock {
            p.set_bypass(bypassed)
                .map_err(|e| format!("Failed to toggle bypass: {}", e))?;
        }

        if let Ok(mut settings) = self.storage.load_settings() {
            settings.is_bypassed = bypassed;
            let _ = self.storage.save_settings(&settings);
        }

        Ok(())
    }

    /// Returns all presets (factory and custom user presets) from local storage.
    pub fn list_presets(&self) -> Result<Vec<crate::storage::EqualizerPreset>, String> {
        self.storage.list_presets().map_err(|e| e.to_string())
    }

    /// Saves or updates a custom user preset.
    /// If the saved preset is currently active, syncs its parameters to the live audio pipeline.
    pub fn save_preset(
        &self,
        preset: crate::storage::EqualizerPreset,
    ) -> Result<crate::storage::EqualizerPreset, String> {
        let saved = self.storage.save_preset(preset).map_err(|e| e.to_string())?;

        // If saving the active preset, sync to pipeline immediately
        if let Ok(settings) = self.storage.load_settings() {
            if settings.active_preset_id == saved.id {
                let lock = self
                    .pipeline
                    .lock()
                    .map_err(|e| format!("Failed to acquire pipeline lock: {}", e))?;
                if let Some(ref p) = *lock {
                    Self::sync_pipeline_parameters(p, &self.storage, &settings);
                }
            }
        }

        Ok(saved)
    }

    /// Deletes a custom user preset by its unique ID.
    /// Factory presets are protected from deletion.
    /// If the active preset was deleted, automatically falls back to the "flat" reference preset.
    pub fn delete_preset(&self, id: &str) -> Result<(), String> {
        self.storage.delete_preset(id).map_err(|e| e.to_string())?;

        // If the deleted preset was active, fall back to "flat" preset
        if let Ok(mut settings) = self.storage.load_settings() {
            if settings.active_preset_id == id {
                settings.active_preset_id = "flat".to_string();
                let _ = self.storage.save_settings(&settings);

                let lock = self
                    .pipeline
                    .lock()
                    .map_err(|e| format!("Failed to acquire pipeline lock: {}", e))?;
                if let Some(ref p) = *lock {
                    Self::sync_pipeline_parameters(p, &self.storage, &settings);
                }
            }
        }

        Ok(())
    }

    /// Activates a preset by its ID, persisting it to settings and syncing the live audio pipeline.
    pub fn set_active_preset(&self, id: &str) -> Result<crate::storage::EqualizerPreset, String> {
        let preset = self
            .storage
            .get_preset(id)
            .map_err(|e| e.to_string())?
            .ok_or_else(|| format!("Preset '{}' not found", id))?;

        if let Ok(mut settings) = self.storage.load_settings() {
            settings.active_preset_id = id.to_string();
            let _ = self.storage.save_settings(&settings);

            let lock = self
                .pipeline
                .lock()
                .map_err(|e| format!("Failed to acquire pipeline lock: {}", e))?;
            if let Some(ref p) = *lock {
                Self::sync_pipeline_parameters(p, &self.storage, &settings);
            }
        }

        Ok(preset)
    }

    /// Queries the current audio telemetry snapshot from the active pipeline.
    pub fn telemetry_snapshot(&self) -> crate::audio::TelemetrySnapshot {
        if let Ok(lock) = self.pipeline.lock() {
            if let Some(ref p) = *lock {
                return p.telemetry_snapshot();
            }
        }
        crate::audio::TelemetrySnapshot {
            peak_l: 0.0,
            peak_r: 0.0,
            limiter_active: false,
            gain_reduction_db: 0.0,
        }
    }
}

impl Drop for AppState {
    fn drop(&mut self) {
        let _ = self.restore_system_routing();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_resolve_pipeline_routing_empty_devices() {
        let (cap, ren) = AppState::resolve_pipeline_routing(Some("Speakers"), Some("Mic"));
        assert_eq!(ren, Some("Speakers".to_string()));
        assert_eq!(cap, Some("Mic".to_string()));
    }

    #[test]
    fn test_state_lifecycle_with_routing() {
        let temp_dir = std::env::temp_dir().join(format!(
            "equilizer_test_routing_state_{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let storage = StorageManager::new(&temp_dir);
        let state = AppState::new(storage);

        // Verify active devices or graceful None on headless
        let devices = state.active_devices();
        if let Some((cap, ren)) = devices {
            assert!(!cap.is_empty());
            assert!(!ren.is_empty());
        }

        // Test set_audio_routing with invalid name fails gracefully
        let res = state.set_audio_routing(
            Some("NonExistentCaptureDeviceXYZ".to_string()),
            Some("NonExistentRenderDeviceXYZ".to_string()),
        );
        assert!(res.is_err());

        let _ = std::fs::remove_dir_all(temp_dir);
    }
}
