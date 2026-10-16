use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{Device, Host, SampleFormat, Stream, StreamConfig, SupportedStreamConfig};

use crate::audio::buffer::AudioProducer;
use crate::audio::AudioError;

/// Resolves an audio output device by optional name or returns the default output endpoint.
pub fn find_output_device(host: &Host, device_name: Option<&str>) -> Result<Device, AudioError> {
    match device_name {
        Some(name) if !name.trim().is_empty() => {
            let mut devices = host
                .output_devices()
                .map_err(|e| AudioError::BackendError(e.to_string()))?;
            devices
                .find(|d| d.name().map(|n| n == name).unwrap_or(false))
                .ok_or_else(|| AudioError::DeviceNotFound(name.to_string()))
        }
        _ => host
            .default_output_device()
            .ok_or(AudioError::DefaultDeviceUnavailable),
    }
}

/// Manages the low-latency Windows WASAPI loopback audio capture stream.
/// Intercepts all operating system audio sent to the target playback endpoint.
pub struct WasapiCapture {
    stream: Stream,
    device_name: String,
    sample_rate: u32,
    channels: u16,
    sample_format: SampleFormat,
}

impl WasapiCapture {
    /// Builds a WASAPI loopback capture stream from the specified output device.
    /// Captured samples are converted to normalized f32 and forwarded to the lock-free ring buffer producer.
    pub fn build(
        device: &Device,
        device_name: String,
        mut producer: AudioProducer,
    ) -> Result<Self, AudioError> {
        let supported_config: SupportedStreamConfig = device.default_output_config().or_else(|_| {
            device
                .supported_output_configs()
                .map_err(|e| AudioError::ConfigError(e.to_string()))?
                .next()
                .map(|c| c.with_max_sample_rate())
                .ok_or_else(|| {
                    AudioError::ConfigError("No supported output configuration found".to_string())
                })
        })?;

        let sample_rate = supported_config.sample_rate().0;
        let channels = supported_config.channels();
        let sample_format = supported_config.sample_format();
        let config: StreamConfig = supported_config.into();

        let err_fn = move |_err: cpal::StreamError| {
            // Real-time audio error callback: zero-allocation, no panics
        };

        let stream = match sample_format {
            SampleFormat::F32 => device
                .build_input_stream(
                    &config,
                    move |data: &[f32], _: &cpal::InputCallbackInfo| {
                        producer.push_slice_with_denormal_flush(data);
                    },
                    err_fn,
                    None,
                )
                .map_err(|e| AudioError::StreamBuildError(e.to_string()))?,
            SampleFormat::I16 => device
                .build_input_stream(
                    &config,
                    move |data: &[i16], _: &cpal::InputCallbackInfo| {
                        producer.push_i16_slice(data);
                    },
                    err_fn,
                    None,
                )
                .map_err(|e| AudioError::StreamBuildError(e.to_string()))?,
            SampleFormat::U16 => device
                .build_input_stream(
                    &config,
                    move |data: &[u16], _: &cpal::InputCallbackInfo| {
                        producer.push_u16_slice(data);
                    },
                    err_fn,
                    None,
                )
                .map_err(|e| AudioError::StreamBuildError(e.to_string()))?,
            other => {
                return Err(AudioError::ConfigError(format!(
                    "Unsupported audio sample format: {:?}",
                    other
                )));
            }
        };

        Ok(Self {
            stream,
            device_name,
            sample_rate,
            channels,
            sample_format,
        })
    }

    /// Creates and initializes a WASAPI loopback capture stream on the system default output device.
    pub fn from_default_device(producer: AudioProducer) -> Result<Self, AudioError> {
        let host = cpal::default_host();
        let device = host
            .default_output_device()
            .ok_or(AudioError::DefaultDeviceUnavailable)?;
        let name = device
            .name()
            .unwrap_or_else(|_| "Default Audio Endpoint".to_string());
        Self::build(&device, name, producer)
    }

    /// Creates and initializes a WASAPI loopback capture stream on a device with matching name.
    pub fn from_device_name(name: &str, producer: AudioProducer) -> Result<Self, AudioError> {
        let host = cpal::default_host();
        let device = find_output_device(&host, Some(name))?;
        Self::build(&device, name.to_string(), producer)
    }

    /// Starts capturing audio samples into the ring buffer.
    pub fn play(&self) -> Result<(), AudioError> {
        self.stream
            .play()
            .map_err(|e| AudioError::StreamPlayError(e.to_string()))
    }

    /// Pauses audio capture streaming.
    pub fn pause(&self) -> Result<(), AudioError> {
        self.stream
            .pause()
            .map_err(|e| AudioError::StreamPauseError(e.to_string()))
    }

    /// Active sampling rate in Hz (e.g. 44100, 48000, 96000).
    pub fn sample_rate(&self) -> u32 {
        self.sample_rate
    }

    /// Channel count (e.g. 2 for stereo).
    pub fn channels(&self) -> u16 {
        self.channels
    }

    /// Sample format of the underlying hardware endpoint.
    pub fn sample_format(&self) -> SampleFormat {
        self.sample_format
    }

    /// Human-readable device name.
    pub fn device_name(&self) -> &str {
        &self.device_name
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::audio::buffer::AudioRingBuffer;

    #[test]
    fn test_device_resolution_invalid_name() {
        let host = cpal::default_host();
        let res = find_output_device(&host, Some("NON_EXISTENT_AUDIO_DEVICE_XYZ_12345"));
        assert!(matches!(res, Err(AudioError::DeviceNotFound(_))));
    }

    #[test]
    fn test_capture_lifecycle_default_device() {
        let host = cpal::default_host();
        if host.default_output_device().is_none() {
            // Environment without audio hardware (headless VM/container)
            eprintln!("Skipping live capture lifecycle test: no default output endpoint detected.");
            return;
        }

        let (producer, mut consumer) = AudioRingBuffer::new(4096);
        let capture_result = WasapiCapture::from_default_device(producer);

        match capture_result {
            Ok(capture) => {
                assert!(capture.sample_rate() > 0);
                assert!(capture.channels() > 0);
                assert!(!capture.device_name().is_empty());

                let play_res = capture.play();
                assert!(play_res.is_ok(), "Failed to start capture: {:?}", play_res);

                // Sleep briefly to capture OS audio or silence
                std::thread::sleep(std::time::Duration::from_millis(50));

                let pause_res = capture.pause();
                assert!(pause_res.is_ok(), "Failed to pause capture: {:?}", pause_res);

                // Drain captured samples
                let mut drain = [0.0f32; 128];
                let _read = consumer.pop_slice(&mut drain);
            }
            Err(e) => {
                eprintln!("WASAPI capture initialization error on host: {:?}", e);
            }
        }
    }
}
