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
}
