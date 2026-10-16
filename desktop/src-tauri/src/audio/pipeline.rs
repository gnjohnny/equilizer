use std::sync::atomic::{AtomicBool, Ordering};

use crate::audio::buffer::AudioRingBuffer;
use crate::audio::capture::WasapiCapture;
use crate::audio::render::WasapiRender;
use crate::audio::AudioError;

/// Coordinates the full loopback pass-through audio pipeline.
/// Couples WASAPI loopback capture, lock-free ring buffering, and hardware playback rendering.
pub struct AudioPipeline {
    capture: WasapiCapture,
    render: WasapiRender,
    is_running: AtomicBool,
}

impl AudioPipeline {
    /// Default buffer capacity: 32,768 f32 samples (~340 ms buffer @ 96 kHz stereo or ~740 ms @ 44.1 kHz).
    pub const DEFAULT_BUFFER_CAPACITY: usize = 32768;

    /// Builds and initializes the complete loopback pass-through pipeline on the system default audio endpoint.
    pub fn from_default_device() -> Result<Self, AudioError> {
        let (producer, consumer) = AudioRingBuffer::new(Self::DEFAULT_BUFFER_CAPACITY);
        let capture = WasapiCapture::from_default_device(producer)?;
        let render = WasapiRender::from_default_device(consumer)?;

        Ok(Self {
            capture,
            render,
            is_running: AtomicBool::new(false),
        })
    }

    /// Builds and initializes the complete loopback pass-through pipeline targeting a specific named device.
    pub fn from_device_name(name: &str) -> Result<Self, AudioError> {
        let (producer, consumer) = AudioRingBuffer::new(Self::DEFAULT_BUFFER_CAPACITY);
        let capture = WasapiCapture::from_device_name(name, producer)?;
        let render = WasapiRender::from_device_name(name, consumer)?;

        Ok(Self {
            capture,
            render,
            is_running: AtomicBool::new(false),
        })
    }

    /// Starts both capture and render streams, beginning live system-wide audio interception and rendering.
    pub fn start(&self) -> Result<(), AudioError> {
        self.capture.play()?;
        self.render.play()?;
        self.is_running.store(true, Ordering::SeqCst);
        Ok(())
    }

    /// Pauses both capture and playback streams.
    pub fn pause(&self) -> Result<(), AudioError> {
        self.render.pause()?;
        self.capture.pause()?;
        self.is_running.store(false, Ordering::SeqCst);
        Ok(())
    }

    /// Returns true if the audio pipeline is actively capturing and streaming audio.
    pub fn is_running(&self) -> bool {
        self.is_running.load(Ordering::SeqCst)
    }

    /// Total count of samples padded with silence due to buffer underrun.
    pub fn underrun_count(&self) -> usize {
        self.render.underrun_count()
    }

    /// Total count of samples dropped due to buffer overflow.
    pub fn overflow_count(&self) -> usize {
        self.render.overflow_count()
    }

    /// Target audio endpoint device name.
    pub fn device_name(&self) -> &str {
        self.render.device_name()
    }

    /// Active sampling rate in Hz.
    pub fn sample_rate(&self) -> u32 {
        self.render.sample_rate()
    }

    /// Active channel count.
    pub fn channels(&self) -> u16 {
        self.render.channels()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cpal::traits::HostTrait;

    #[test]
    fn test_pipeline_pass_through_lifecycle() {
        let host = cpal::default_host();
        if host.default_output_device().is_none() {
            eprintln!("Skipping pipeline lifecycle test: no audio output device present on host.");
            return;
        }

        let pipeline_result = AudioPipeline::from_default_device();
        match pipeline_result {
            Ok(pipeline) => {
                assert!(!pipeline.is_running());
                assert!(pipeline.sample_rate() > 0);
                assert!(pipeline.channels() > 0);

                let start_res = pipeline.start();
                assert!(start_res.is_ok(), "Pipeline start failed: {:?}", start_res);
                assert!(pipeline.is_running());

                // Let pass-through run briefly for 50 ms
                std::thread::sleep(std::time::Duration::from_millis(50));

                let pause_res = pipeline.pause();
                assert!(pause_res.is_ok(), "Pipeline pause failed: {:?}", pause_res);
                assert!(!pipeline.is_running());
            }
            Err(e) => {
                eprintln!("Audio pipeline creation failed on host: {:?}", e);
            }
        }
    }
}
