pub mod bass_boost;
pub mod benchmark;
pub mod buffer;
pub mod capture;
pub mod dsp;
pub mod endpoint_policy;
pub mod limiter;
pub mod monitor;
pub mod params;
pub mod pipeline;
pub mod preamp;
pub mod render;

pub use endpoint_policy::{
    get_default_endpoint, record_original_endpoint, restore_emergency_endpoint,
    switch_default_endpoint,
};
pub use bass_boost::{BassBoostConfig, BassBoostEngine, BassBoostMode, DEFAULT_BASS_BOOST_Q};
pub use benchmark::{
    benchmark_ring_buffer_memory, calculate_theoretical_latency_ms, get_current_working_set_bytes,
    profile_dsp_resource_consumption, LatencyBenchmarkResult, ResourceProfileResult,
};
pub use capture::WasapiCapture;
pub use dsp::{BiquadCoefficients, BiquadFilterChain, EqBandConfig, FilterType, StereoBiquad};
pub use limiter::{
    Limiter, LimiterConfig, DEFAULT_LIMITER_ATTACK_MS, DEFAULT_LIMITER_CEILING_DB,
    DEFAULT_LIMITER_KNEE_DB, DEFAULT_LIMITER_RELEASE_MS, DEFAULT_LIMITER_THRESHOLD_DB,
};
pub use monitor::{spawn_device_monitor, DeviceChangeEventPayload};
pub use params::{
    create_param_channel, AtomicF32, AudioTelemetry, DspCommand, DspParamReceiver, DspParamSender,
    DspProcessor, SendError, TelemetrySnapshot,
};
pub use pipeline::AudioPipeline;
pub use preamp::{PreampStage, DEFAULT_PREAMP_GAIN_DB, MAX_PREAMP_GAIN_DB, MIN_PREAMP_GAIN_DB};
pub use render::WasapiRender;

use cpal::traits::{DeviceTrait, HostTrait};
use serde::{Deserialize, Serialize};

/// Audio output endpoint metadata.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct AudioDeviceInfo {
    pub id: String,
    pub name: String,
    pub is_default: bool,
}

/// Comprehensive audio subsystem error type.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AudioError {
    DeviceNotFound(String),
    DefaultDeviceUnavailable,
    ConfigError(String),
    StreamBuildError(String),
    StreamPlayError(String),
    StreamPauseError(String),
    BackendError(String),
}

impl std::fmt::Display for AudioError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AudioError::DeviceNotFound(name) => {
                write!(f, "Audio output device '{}' not found", name)
            }
            AudioError::DefaultDeviceUnavailable => {
                write!(f, "Default audio output device is unavailable")
            }
            AudioError::ConfigError(msg) => write!(f, "Audio device configuration error: {}", msg),
            AudioError::StreamBuildError(msg) => {
                write!(f, "Failed to build audio stream: {}", msg)
            }
            AudioError::StreamPlayError(msg) => {
                write!(f, "Failed to start audio stream: {}", msg)
            }
            AudioError::StreamPauseError(msg) => {
                write!(f, "Failed to pause audio stream: {}", msg)
            }
            AudioError::BackendError(msg) => write!(f, "Audio backend error: {}", msg),
        }
    }
}

impl std::error::Error for AudioError {}

/// Enumerates all active playback audio endpoints on the operating system.
pub fn enumerate_output_devices() -> Result<Vec<AudioDeviceInfo>, AudioError> {
    let host = cpal::default_host();
    let default_name = host
        .default_output_device()
        .and_then(|d| d.name().ok());

    let mut device_list = Vec::new();
    let devices = host
        .output_devices()
        .map_err(|e| AudioError::BackendError(e.to_string()))?;

    for device in devices {
        if let Ok(name) = device.name() {
            let is_default = default_name.as_deref() == Some(&name);
            device_list.push(AudioDeviceInfo {
                id: name.clone(),
                name,
                is_default,
            });
        }
    }

    Ok(device_list)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_audio_error_display() {
        let err = AudioError::DeviceNotFound("Headphones".to_string());
        assert_eq!(
            err.to_string(),
            "Audio output device 'Headphones' not found"
        );

        let err2 = AudioError::DefaultDeviceUnavailable;
        assert_eq!(
            err2.to_string(),
            "Default audio output device is unavailable"
        );
    }

    #[test]
    fn test_device_enumeration() {
        let res = enumerate_output_devices();
        assert!(res.is_ok(), "Device enumeration should not fail: {:?}", res);
        let list = res.unwrap();
        // In environments with sound hardware, there should be at least one device or empty list
        for dev in &list {
            assert!(!dev.name.is_empty());
        }
    }
}
