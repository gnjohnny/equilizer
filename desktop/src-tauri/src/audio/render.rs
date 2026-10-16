use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::Arc;

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{Device, SampleFormat, Stream, StreamConfig, SupportedStreamConfig};

use crate::audio::buffer::AudioConsumer;
use crate::audio::capture::find_output_device;
use crate::audio::AudioError;

/// Renders multi-channel or mono f32 output slices from an interleaved stereo consumer.
#[inline]
fn render_multichannel_f32(
    consumer: &mut AudioConsumer,
    data: &mut [f32],
    channels: usize,
    underruns: &AtomicUsize,
) {
    if channels == 1 {
        let mut stereo = [0.0f32; 2];
        for out in data.iter_mut() {
            let read = consumer.pop_slice(&mut stereo);
            if read == 2 {
                *out = (stereo[0] + stereo[1]) * 0.5;
            } else if read == 1 {
                *out = stereo[0];
                underruns.fetch_add(1, Ordering::Relaxed);
            } else {
                *out = 0.0;
                underruns.fetch_add(1, Ordering::Relaxed);
            }
        }
    } else {
        // e.g. 4, 6 (5.1), 8 (7.1) channels
        // Map L to channel 0, R to channel 1, and zero remaining surround/LFE channels
        let mut frame_stereo = [0.0f32; 2];
        for chunk in data.chunks_mut(channels) {
            let read = consumer.pop_slice(&mut frame_stereo);
            if read == 2 {
                chunk[0] = frame_stereo[0];
                chunk[1] = frame_stereo[1];
            } else if read == 1 {
                chunk[0] = frame_stereo[0];
                if chunk.len() > 1 {
                    chunk[1] = 0.0;
                }
                underruns.fetch_add(1, Ordering::Relaxed);
            } else {
                chunk[0] = 0.0;
                if chunk.len() > 1 {
                    chunk[1] = 0.0;
                }
                underruns.fetch_add(2, Ordering::Relaxed);
            }
            for ch in chunk.iter_mut().skip(2) {
                *ch = 0.0;
            }
        }
    }
}

/// Manages the low-latency Windows WASAPI playback output stream.
/// Consumes samples from the lock-free ring buffer and feeds them directly to the sound card.
pub struct WasapiRender {
    stream: Stream,
    device_name: String,
    sample_rate: u32,
    channels: u16,
    sample_format: SampleFormat,
    underrun_count: Arc<AtomicUsize>,
    overflow_count: Arc<AtomicUsize>,
}

impl WasapiRender {
    /// Pre-buffering threshold in samples before active playback streaming begins (~5.3 ms @ 48 kHz).
    pub const DEFAULT_PREBUFFER_THRESHOLD: usize = 256;

    /// Builds a WASAPI playback stream targeting the specified output hardware device.
    pub fn build(
        device: &Device,
        device_name: String,
        consumer: AudioConsumer,
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

        let underrun_count = Arc::new(AtomicUsize::new(0));
        let underruns_clone = Arc::clone(&underrun_count);

        let overflow_count = Arc::new(AtomicUsize::new(consumer.overflow_count()));

        let prebuffered = Arc::new(AtomicBool::new(false));
        let prebuffered_clone = Arc::clone(&prebuffered);

        let prebuffer_threshold = Self::DEFAULT_PREBUFFER_THRESHOLD;

        let err_fn = move |_err: cpal::StreamError| {
            // Real-time audio error callback: zero-allocation, no panics
        };

        let stream = match sample_format {
            SampleFormat::F32 => {
                let mut consumer = consumer;
                let ch_usize = channels as usize;
                device
                    .build_output_stream(
                        &config,
                        move |data: &mut [f32], _: &cpal::OutputCallbackInfo| {
                            let is_prebuffered = prebuffered_clone.load(Ordering::Relaxed);
                            if !is_prebuffered {
                                if consumer.len() >= prebuffer_threshold {
                                    prebuffered_clone.store(true, Ordering::Relaxed);
                                } else {
                                    for sample in data.iter_mut() {
                                        *sample = 0.0;
                                    }
                                    return;
                                }
                            }

                            if ch_usize == 2 {
                                let read = consumer.pop_slice(data);
                                if read < data.len() {
                                    for sample in &mut data[read..] {
                                        *sample = 0.0;
                                    }
                                    underruns_clone
                                        .fetch_add(data.len() - read, Ordering::Relaxed);
                                }
                            } else {
                                render_multichannel_f32(
                                    &mut consumer,
                                    data,
                                    ch_usize,
                                    &underruns_clone,
                                );
                            }
                        },
                        err_fn,
                        None,
                    )
                    .map_err(|e| AudioError::StreamBuildError(e.to_string()))?
            }
            SampleFormat::I16 => {
                let mut consumer = consumer;
                device
                    .build_output_stream(
                        &config,
                        move |data: &mut [i16], _: &cpal::OutputCallbackInfo| {
                            let is_prebuffered = prebuffered_clone.load(Ordering::Relaxed);
                            if !is_prebuffered {
                                if consumer.len() >= prebuffer_threshold {
                                    prebuffered_clone.store(true, Ordering::Relaxed);
                                } else {
                                    for sample in data.iter_mut() {
                                        *sample = 0;
                                    }
                                    return;
                                }
                            }

                            let mut offset = 0;
                            let mut stack_buf = [0.0f32; 128];
                            while offset < data.len() {
                                let chunk_len = (data.len() - offset).min(128);
                                let read = consumer.pop_slice(&mut stack_buf[..chunk_len]);
                                for i in 0..read {
                                    let s = stack_buf[i];
                                    let scaled = (s * 32767.0).clamp(-32768.0, 32767.0);
                                    data[offset + i] = scaled as i16;
                                }
                                if read < chunk_len {
                                    for i in read..chunk_len {
                                        data[offset + i] = 0;
                                    }
                                    underruns_clone
                                        .fetch_add(chunk_len - read, Ordering::Relaxed);
                                }
                                offset += chunk_len;
                            }
                        },
                        err_fn,
                        None,
                    )
                    .map_err(|e| AudioError::StreamBuildError(e.to_string()))?
            }
            SampleFormat::U16 => {
                let mut consumer = consumer;
                device
                    .build_output_stream(
                        &config,
                        move |data: &mut [u16], _: &cpal::OutputCallbackInfo| {
                            let is_prebuffered = prebuffered_clone.load(Ordering::Relaxed);
                            if !is_prebuffered {
                                if consumer.len() >= prebuffer_threshold {
                                    prebuffered_clone.store(true, Ordering::Relaxed);
                                } else {
                                    for sample in data.iter_mut() {
                                        *sample = 32768;
                                    }
                                    return;
                                }
                            }

                            let mut offset = 0;
                            let mut stack_buf = [0.0f32; 128];
                            while offset < data.len() {
                                let chunk_len = (data.len() - offset).min(128);
                                let read = consumer.pop_slice(&mut stack_buf[..chunk_len]);
                                for i in 0..read {
                                    let s = stack_buf[i];
                                    let scaled = ((s * 32767.0) + 32768.0).clamp(0.0, 65535.0);
                                    data[offset + i] = scaled as u16;
                                }
                                if read < chunk_len {
                                    for i in read..chunk_len {
                                        data[offset + i] = 32768;
                                    }
                                    underruns_clone
                                        .fetch_add(chunk_len - read, Ordering::Relaxed);
                                }
                                offset += chunk_len;
                            }
                        },
                        err_fn,
                        None,
                    )
                    .map_err(|e| AudioError::StreamBuildError(e.to_string()))?
            }
            other => {
                return Err(AudioError::ConfigError(format!(
                    "Unsupported output audio sample format: {:?}",
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
            underrun_count,
            overflow_count,
        })
    }

    /// Creates and initializes a WASAPI playback stream on the system default output device.
    pub fn from_default_device(consumer: AudioConsumer) -> Result<Self, AudioError> {
        let host = cpal::default_host();
        let device = host
            .default_output_device()
            .ok_or(AudioError::DefaultDeviceUnavailable)?;
        let name = device
            .name()
            .unwrap_or_else(|_| "Default Audio Endpoint".to_string());
        Self::build(&device, name, consumer)
    }

    /// Creates and initializes a WASAPI playback stream on a device with matching name.
    pub fn from_device_name(name: &str, consumer: AudioConsumer) -> Result<Self, AudioError> {
        let host = cpal::default_host();
        let device = find_output_device(&host, Some(name))?;
        Self::build(&device, name.to_string(), consumer)
    }

    /// Starts audio playback streaming to hardware.
    pub fn play(&self) -> Result<(), AudioError> {
        self.stream
            .play()
            .map_err(|e| AudioError::StreamPlayError(e.to_string()))
    }

    /// Pauses audio playback streaming.
    pub fn pause(&self) -> Result<(), AudioError> {
        self.stream
            .pause()
            .map_err(|e| AudioError::StreamPauseError(e.to_string()))
    }

    /// Active hardware sampling rate in Hz (e.g. 44100, 48000, 96000).
    pub fn sample_rate(&self) -> u32 {
        self.sample_rate
    }

    /// Active hardware channel count (e.g. 2 for stereo).
    pub fn channels(&self) -> u16 {
        self.channels
    }

    /// Sample format of the hardware output endpoint.
    pub fn sample_format(&self) -> SampleFormat {
        self.sample_format
    }

    /// Human-readable device name.
    pub fn device_name(&self) -> &str {
        &self.device_name
    }

    /// Total count of samples padded with silence due to buffer underrun.
    pub fn underrun_count(&self) -> usize {
        self.underrun_count.load(Ordering::Relaxed)
    }

    /// Total count of samples dropped due to buffer overflow.
    pub fn overflow_count(&self) -> usize {
        self.overflow_count.load(Ordering::Relaxed)
    }

    /// Resets the underrun counter to zero and returns the previous count.
    pub fn reset_underrun_count(&self) -> usize {
        self.underrun_count.swap(0, Ordering::Relaxed)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::audio::buffer::AudioRingBuffer;

    #[test]
    fn test_render_resolution_invalid_name() {
        let (_, consumer) = AudioRingBuffer::new(1024);
        let res = WasapiRender::from_device_name("INVALID_DEVICE_FOR_TEST_XYZ_99999", consumer);
        assert!(matches!(res, Err(AudioError::DeviceNotFound(_))));
    }

    #[test]
    fn test_render_lifecycle_default_device() {
        let host = cpal::default_host();
        if host.default_output_device().is_none() {
            eprintln!("Skipping render lifecycle test: no default audio endpoint found.");
            return;
        }

        let (mut producer, consumer) = AudioRingBuffer::new(2048);
        // Pre-fill some samples so prebuffering threshold is satisfied
        let dummy = vec![0.05f32; 512];
        producer.push_slice(&dummy);

        let render_result = WasapiRender::from_default_device(consumer);
        match render_result {
            Ok(render) => {
                assert!(render.sample_rate() > 0);
                assert!(render.channels() > 0);
                assert!(!render.device_name().is_empty());

                let play_res = render.play();
                assert!(play_res.is_ok(), "Failed to play render stream: {:?}", play_res);

                std::thread::sleep(std::time::Duration::from_millis(50));

                let pause_res = render.pause();
                assert!(pause_res.is_ok(), "Failed to pause render stream: {:?}", pause_res);
            }
            Err(e) => {
                eprintln!("Render stream initialization error on host: {:?}", e);
            }
        }
    }
}
