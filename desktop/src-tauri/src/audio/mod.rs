pub mod buffer;
pub mod capture;
pub mod pipeline;
pub mod render;

pub use capture::WasapiCapture;
pub use pipeline::AudioPipeline;
pub use render::WasapiRender;

use cpal::traits::{DeviceTrait, HostTrait};
use serde::{Deserialize, Serialize};

/// Audio output endpoint metadata.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
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
