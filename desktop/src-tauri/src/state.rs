use std::sync::Mutex;
use crate::audio::{AudioError, AudioPipeline, LimiterConfig};
use crate::storage::StorageManager;

/// Thread-safe global application state managed by Tauri.
pub struct AppState {
    pub pipeline: Mutex<Option<AudioPipeline>>,
    pub storage: StorageManager,
}

impl AppState {
    /// Creates and initializes application state with the given storage manager.
    pub fn new(storage: StorageManager) -> Self {
        let _ = storage.ensure_initialized();
        let settings = storage.load_settings().unwrap_or_default();

        let pipeline = match settings.selected_device_id.as_deref() {
            Some(dev_name) if !dev_name.trim().is_empty() => {
                AudioPipeline::from_device_name(dev_name)
                    .or_else(|_| AudioPipeline::from_default_device())
                    .ok()
            }
            _ => AudioPipeline::from_default_device().ok(),
        };

        if let Some(ref p) = pipeline {
            Self::sync_pipeline_parameters(p, &storage, &settings);
            if let Err(e) = p.start() {
                eprintln!("Warning: Failed to auto-start audio pipeline on launch: {}", e);
            }
        }

        Self {
            pipeline: Mutex::new(pipeline),
            storage,
        }
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

    /// Reroutes audio streaming to the specified output hardware device.
    /// If `device_id` is None or empty, routes to the system default audio endpoint.
    pub fn set_output_device(&self, device_id: Option<String>) -> Result<(), String> {
        let clean_id = device_id.filter(|s| !s.trim().is_empty());

        // Build target pipeline first before acquiring mutex or dropping existing stream
        let new_pipeline = match clean_id.as_deref() {
            Some(name) => AudioPipeline::from_device_name(name)
                .map_err(|e| format!("Failed to initialize audio device '{}': {}", name, e))?,
            None => AudioPipeline::from_default_device()
                .map_err(|e| format!("Failed to initialize default audio device: {}", e))?,
        };

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

        // Persist device selection to disk
        if let Ok(mut settings) = self.storage.load_settings() {
            settings.selected_device_id = clean_id;
            let _ = self.storage.save_settings(&settings);
        }

        Ok(())
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
