use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::Arc;

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{Device, SampleFormat, Stream, StreamConfig, SupportedStreamConfig};

use crate::audio::buffer::AudioConsumer;
use crate::audio::capture::find_output_device;
use crate::audio::params::{create_param_channel, AudioTelemetry, DspParamSender, DspProcessor};
use crate::audio::AudioError;

/// Renders multi-channel or mono f32 output slices from an interleaved stereo consumer.
/// Always processes audio through the active 4-stage DSP pipeline and drains parameter commands.
#[inline]
fn render_multichannel_f32(
    consumer: &mut AudioConsumer,
    dsp: &mut DspProcessor,
    data: &mut [f32],
    channels: usize,
    underruns: &AtomicUsize,
) -> bool {
    let mut had_underrun = false;
    if channels == 1 {
        let mut stereo = [0.0f32; 128];
        let mut offset = 0;
        while offset < data.len() {
            let frames = (data.len() - offset).min(64);
            let needed_samples = frames * 2;
            let read = consumer.pop_slice(&mut stereo[..needed_samples]);
            if read < needed_samples {
                had_underrun = true;
                let shortfall = needed_samples - read;
                underruns.fetch_add(shortfall, Ordering::Relaxed);
                let last_l = if read >= 2 { stereo[read - 2] } else { 0.0 };
                let last_r = if read >= 2 { stereo[read - 1] } else { 0.0 };
                let fade_frames = (shortfall / 2).min(16);
                for f in 0..fade_frames {
                    let factor = 1.0 - ((f + 1) as f32 / (fade_frames + 1) as f32);
                    stereo[read + f * 2] = last_l * factor;
                    stereo[read + f * 2 + 1] = last_r * factor;
                }
                for s in &mut stereo[read + fade_frames * 2..needed_samples] {
                    *s = 0.0;
                }
            }
            dsp.process_interleaved_stereo(&mut stereo[..needed_samples]);
            for i in 0..frames {
                data[offset + i] = (stereo[i * 2] + stereo[i * 2 + 1]) * 0.5;
            }
            offset += frames;
        }
    } else {
        // Multi-channel: e.g. 4 (2.1 or quad), 6 (5.1), 8 (7.1) channels
        let total_frames = data.len() / channels;
        let mut frame_offset = 0;
        let mut stereo = [0.0f32; 128];

        while frame_offset < total_frames {
            let frames = (total_frames - frame_offset).min(64);
            let needed_samples = frames * 2;
            let read = consumer.pop_slice(&mut stereo[..needed_samples]);
            if read < needed_samples {
                had_underrun = true;
                let shortfall = needed_samples - read;
                underruns.fetch_add(shortfall, Ordering::Relaxed);
                let last_l = if read >= 2 { stereo[read - 2] } else { 0.0 };
                let last_r = if read >= 2 { stereo[read - 1] } else { 0.0 };
                let fade_frames = (shortfall / 2).min(16);
                for f in 0..fade_frames {
                    let factor = 1.0 - ((f + 1) as f32 / (fade_frames + 1) as f32);
                    stereo[read + f * 2] = last_l * factor;
                    stereo[read + f * 2 + 1] = last_r * factor;
                }
                for s in &mut stereo[read + fade_frames * 2..needed_samples] {
                    *s = 0.0;
                }
            }

            dsp.process_interleaved_stereo(&mut stereo[..needed_samples]);

            for i in 0..frames {
                let l = stereo[i * 2];
                let r = stereo[i * 2 + 1];
                let sub_center = (l + r) * 0.5;
                let chunk_start = (frame_offset + i) * channels;
                let chunk = &mut data[chunk_start..chunk_start + channels];

                chunk[0] = l;
                chunk[1] = r;

                if channels >= 3 {
                    chunk[2] = sub_center;
                }
                if channels >= 4 {
                    chunk[3] = sub_center;
                }
                if channels >= 5 {
                    chunk[4] = l;
                }
                if channels >= 6 {
                    chunk[5] = r;
                }
                for ch in chunk.iter_mut().skip(6) {
                    *ch = 0.0;
                }
            }
            frame_offset += frames;
        }
    }
    had_underrun
}

/// Renders multi-channel or mono i16 output slices from an interleaved stereo consumer.
#[inline]
fn render_multichannel_i16(
    consumer: &mut AudioConsumer,
    dsp: &mut DspProcessor,
    data: &mut [i16],
    channels: usize,
    underruns: &AtomicUsize,
) -> bool {
    let mut had_underrun = false;
    if channels == 1 {
        let mut stereo = [0.0f32; 128];
        let mut offset = 0;
        while offset < data.len() {
            let frames = (data.len() - offset).min(64);
            let needed = frames * 2;
            let read = consumer.pop_slice(&mut stereo[..needed]);
            if read < needed {
                had_underrun = true;
                let shortfall = needed - read;
                underruns.fetch_add(shortfall, Ordering::Relaxed);
                let last_l = if read >= 2 { stereo[read - 2] } else { 0.0 };
                let last_r = if read >= 2 { stereo[read - 1] } else { 0.0 };
                let fade_frames = (shortfall / 2).min(16);
                for f in 0..fade_frames {
                    let factor = 1.0 - ((f + 1) as f32 / (fade_frames + 1) as f32);
                    stereo[read + f * 2] = last_l * factor;
                    stereo[read + f * 2 + 1] = last_r * factor;
                }
                for s in &mut stereo[read + fade_frames * 2..needed] {
                    *s = 0.0;
                }
            }
            dsp.process_interleaved_stereo(&mut stereo[..needed]);
            for i in 0..frames {
                let m = (stereo[i * 2] + stereo[i * 2 + 1]) * 0.5;
                data[offset + i] = (m * 32767.0).clamp(-32768.0, 32767.0) as i16;
            }
            offset += frames;
        }
    } else {
        let total_frames = data.len() / channels;
        let mut frame_offset = 0;
        let mut stereo = [0.0f32; 128];
        while frame_offset < total_frames {
            let frames = (total_frames - frame_offset).min(64);
            let needed = frames * 2;
            let read = consumer.pop_slice(&mut stereo[..needed]);
            if read < needed {
                had_underrun = true;
                let shortfall = needed - read;
                underruns.fetch_add(shortfall, Ordering::Relaxed);
                let last_l = if read >= 2 { stereo[read - 2] } else { 0.0 };
                let last_r = if read >= 2 { stereo[read - 1] } else { 0.0 };
                let fade_frames = (shortfall / 2).min(16);
                for f in 0..fade_frames {
                    let factor = 1.0 - ((f + 1) as f32 / (fade_frames + 1) as f32);
                    stereo[read + f * 2] = last_l * factor;
                    stereo[read + f * 2 + 1] = last_r * factor;
                }
                for s in &mut stereo[read + fade_frames * 2..needed] {
                    *s = 0.0;
                }
            }
            dsp.process_interleaved_stereo(&mut stereo[..needed]);
            for i in 0..frames {
                let l = stereo[i * 2];
                let r = stereo[i * 2 + 1];
                let sub_center = (l + r) * 0.5;
                let chunk_start = (frame_offset + i) * channels;
                let chunk = &mut data[chunk_start..chunk_start + channels];

                chunk[0] = (l * 32767.0).clamp(-32768.0, 32767.0) as i16;
                chunk[1] = (r * 32767.0).clamp(-32768.0, 32767.0) as i16;
                let sub_i16 = (sub_center * 32767.0).clamp(-32768.0, 32767.0) as i16;

                if channels >= 3 {
                    chunk[2] = sub_i16;
                }
                if channels >= 4 {
                    chunk[3] = sub_i16;
                }
                if channels >= 5 {
                    chunk[4] = chunk[0];
                }
                if channels >= 6 {
                    chunk[5] = chunk[1];
                }
                for ch in chunk.iter_mut().skip(6) {
                    *ch = 0;
                }
            }
            frame_offset += frames;
        }
    }
    had_underrun
}

/// Renders multi-channel or mono u16 output slices from an interleaved stereo consumer.
#[inline]
fn render_multichannel_u16(
    consumer: &mut AudioConsumer,
    dsp: &mut DspProcessor,
    data: &mut [u16],
    channels: usize,
    underruns: &AtomicUsize,
) -> bool {
    let mut had_underrun = false;
    if channels == 1 {
        let mut stereo = [0.0f32; 128];
        let mut offset = 0;
        while offset < data.len() {
            let frames = (data.len() - offset).min(64);
            let needed = frames * 2;
            let read = consumer.pop_slice(&mut stereo[..needed]);
            if read < needed {
                had_underrun = true;
                let shortfall = needed - read;
                underruns.fetch_add(shortfall, Ordering::Relaxed);
                let last_l = if read >= 2 { stereo[read - 2] } else { 0.0 };
                let last_r = if read >= 2 { stereo[read - 1] } else { 0.0 };
                let fade_frames = (shortfall / 2).min(16);
                for f in 0..fade_frames {
                    let factor = 1.0 - ((f + 1) as f32 / (fade_frames + 1) as f32);
                    stereo[read + f * 2] = last_l * factor;
                    stereo[read + f * 2 + 1] = last_r * factor;
                }
                for s in &mut stereo[read + fade_frames * 2..needed] {
                    *s = 0.0;
                }
            }
            dsp.process_interleaved_stereo(&mut stereo[..needed]);
            for i in 0..frames {
                let m = (stereo[i * 2] + stereo[i * 2 + 1]) * 0.5;
                data[offset + i] = ((m * 32767.0) + 32768.0).clamp(0.0, 65535.0) as u16;
            }
            offset += frames;
        }
    } else {
        let total_frames = data.len() / channels;
        let mut frame_offset = 0;
        let mut stereo = [0.0f32; 128];
        while frame_offset < total_frames {
            let frames = (total_frames - frame_offset).min(64);
            let needed = frames * 2;
            let read = consumer.pop_slice(&mut stereo[..needed]);
            if read < needed {
                had_underrun = true;
                let shortfall = needed - read;
                underruns.fetch_add(shortfall, Ordering::Relaxed);
                let last_l = if read >= 2 { stereo[read - 2] } else { 0.0 };
                let last_r = if read >= 2 { stereo[read - 1] } else { 0.0 };
                let fade_frames = (shortfall / 2).min(16);
                for f in 0..fade_frames {
                    let factor = 1.0 - ((f + 1) as f32 / (fade_frames + 1) as f32);
                    stereo[read + f * 2] = last_l * factor;
                    stereo[read + f * 2 + 1] = last_r * factor;
                }
                for s in &mut stereo[read + fade_frames * 2..needed] {
                    *s = 0.0;
                }
            }
            dsp.process_interleaved_stereo(&mut stereo[..needed]);
            for i in 0..frames {
                let l = stereo[i * 2];
                let r = stereo[i * 2 + 1];
                let sub_center = (l + r) * 0.5;
                let chunk_start = (frame_offset + i) * channels;
                let chunk = &mut data[chunk_start..chunk_start + channels];

                chunk[0] = ((l * 32767.0) + 32768.0).clamp(0.0, 65535.0) as u16;
                chunk[1] = ((r * 32767.0) + 32768.0).clamp(0.0, 65535.0) as u16;
                let sub_u16 = ((sub_center * 32767.0) + 32768.0).clamp(0.0, 65535.0) as u16;

                if channels >= 3 {
                    chunk[2] = sub_u16;
                }
                if channels >= 4 {
                    chunk[3] = sub_u16;
                }
                if channels >= 5 {
                    chunk[4] = chunk[0];
                }
                if channels >= 6 {
                    chunk[5] = chunk[1];
                }
                for ch in chunk.iter_mut().skip(6) {
                    *ch = 32768;
                }
            }
            frame_offset += frames;
        }
    }
    had_underrun
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
    param_sender: DspParamSender,
    telemetry: Arc<AudioTelemetry>,
}

// On Windows WASAPI, cpal::Stream is safe to transfer between threads.
unsafe impl Send for WasapiRender {}

impl WasapiRender {
    /// Pre-buffering threshold in samples before active playback streaming begins (~4.0 ms @ 48 kHz stereo, 14.66 ms round-trip).
    pub const DEFAULT_PREBUFFER_THRESHOLD: usize = 384;

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

        let (param_sender, param_receiver) =
            create_param_channel(DspProcessor::DEFAULT_CHANNEL_CAPACITY);
        let telemetry = Arc::new(AudioTelemetry::new());
        let telemetry_clone = Arc::clone(&telemetry);

        let err_fn = move |_err: cpal::StreamError| {
            // Real-time audio error callback: zero-allocation, no panics
        };

        let stream = match sample_format {
            SampleFormat::F32 => {
                let mut consumer = consumer;
                let ch_usize = channels as usize;
                let mut dsp = DspProcessor::new(sample_rate, param_receiver, telemetry_clone);

                device
                    .build_output_stream(
                        &config,
                        move |data: &mut [f32], _: &cpal::OutputCallbackInfo| {
                            let needed_stereo_samples = (data.len() / ch_usize) * 2;
                            // Target watermark: 4x callback period (~40 ms cushion @ 48 kHz stereo)
                            let target_watermark = needed_stereo_samples * 4;
                            let min_startup_watermark = prebuffer_threshold.max(target_watermark);

                            let is_prebuffered = prebuffered_clone.load(Ordering::Relaxed);
                            if !is_prebuffered {
                                if consumer.len() >= min_startup_watermark {
                                    prebuffered_clone.store(true, Ordering::Relaxed);
                                } else {
                                    for sample in data.iter_mut() {
                                        *sample = 0.0;
                                    }
                                    return;
                                }
                            }

                            if ch_usize == 2 {
                                let current_level = consumer.len();
                                if current_level < needed_stereo_samples {
                                    // Starvation event: buffer cannot fulfill this callback.
                                    // Soft-knee PLC fade-out over up to 32 frames (64 samples).
                                    let read = consumer.pop_slice(&mut data[..current_level]);
                                    let shortfall = data.len() - read;
                                    underruns_clone.fetch_add(shortfall, Ordering::Relaxed);

                                    let last_l = if read >= 2 { data[read - 2] } else { 0.0 };
                                    let last_r = if read >= 2 { data[read - 1] } else { 0.0 };
                                    let fade_frames = (shortfall / 2).min(32);
                                    for f in 0..fade_frames {
                                        let factor = 1.0 - ((f + 1) as f32 / (fade_frames + 1) as f32);
                                        data[read + f * 2] = last_l * factor;
                                        data[read + f * 2 + 1] = last_r * factor;
                                    }
                                    for sample in &mut data[read + fade_frames * 2..] {
                                        *sample = 0.0;
                                    }

                                    // Instantly drop prebuffered flag to prevent the 50 Hz chopping loop!
                                    // The buffer will rebuild the full 40 ms target cushion before resuming.
                                    prebuffered_clone.store(false, Ordering::Relaxed);

                                    dsp.process_interleaved_stereo(data);
                                    return;
                                }

                                // Closed-loop Proportional Rate Matching:
                                let total_frames = data.len() / 2;
                                let deadband = needed_stereo_samples / 4; // ~120 frames = 2.5 ms
                                let low_threshold = target_watermark.saturating_sub(deadband);
                                let high_threshold = target_watermark + deadband;

                                if current_level < low_threshold && total_frames >= 4 {
                                    // Buffer is draining: consume 1 fewer frame (total_frames - 1) and stretch
                                    let in_frames = total_frames - 1;
                                    let in_samples = in_frames * 2;
                                    let read = consumer.pop_slice(&mut data[..in_samples]);
                                    if read == in_samples {
                                        let n_minus_1 = (total_frames - 1) as f32;
                                        let m_minus_1 = (in_frames - 1) as f32;
                                        for i in (0..total_frames).rev() {
                                            let p = (i as f32) * m_minus_1 / n_minus_1;
                                            let k = (p as usize).min(in_frames - 1);
                                            let alpha = p - (k as f32);
                                            let next_k = (k + 1).min(in_frames - 1);
                                            let l0 = data[k * 2];
                                            let r0 = data[k * 2 + 1];
                                            let l1 = data[next_k * 2];
                                            let r1 = data[next_k * 2 + 1];
                                            data[i * 2] = l0 + alpha * (l1 - l0);
                                            data[i * 2 + 1] = r0 + alpha * (r1 - r0);
                                        }
                                    }
                                } else if current_level > high_threshold && total_frames >= 4 {
                                    // Buffer is accumulating: consume 1 extra frame (total_frames + 1) and compress
                                    let read_base = consumer.pop_slice(data);
                                    let mut extra = [0.0f32; 2];
                                    let read_extra = consumer.pop_slice(&mut extra);
                                    if read_base == data.len() && read_extra == 2 {
                                        let in_frames = total_frames + 1;
                                        let n_minus_1 = (total_frames - 1) as f32;
                                        let m_minus_1 = (in_frames - 1) as f32;
                                        for i in 0..total_frames {
                                            let p = (i as f32) * m_minus_1 / n_minus_1;
                                            let k = p as usize;
                                            let alpha = p - (k as f32);
                                            let next_k = k + 1;
                                            let l0 = if k < total_frames { data[k * 2] } else { extra[0] };
                                            let r0 = if k < total_frames { data[k * 2 + 1] } else { extra[1] };
                                            let l1 = if next_k < total_frames { data[next_k * 2] } else { extra[0] };
                                            let r1 = if next_k < total_frames { data[next_k * 2 + 1] } else { extra[1] };
                                            data[i * 2] = l0 + alpha * (l1 - l0);
                                            data[i * 2 + 1] = r0 + alpha * (r1 - r0);
                                        }
                                    }
                                } else {
                                    // Nominal deadband: exact 1:1 bit-perfect pass-through
                                    consumer.pop_slice(data);
                                }

                                dsp.process_interleaved_stereo(data);
                            } else {
                                let had_underrun = render_multichannel_f32(
                                    &mut consumer,
                                    &mut dsp,
                                    data,
                                    ch_usize,
                                    &underruns_clone,
                                );
                                if had_underrun {
                                    prebuffered_clone.store(false, Ordering::Relaxed);
                                }
                            }
                        },
                        err_fn,
                        None,
                    )
                    .map_err(|e| AudioError::StreamBuildError(e.to_string()))?
            }
            SampleFormat::I16 => {
                let mut consumer = consumer;
                let ch_usize = channels as usize;
                let mut dsp = DspProcessor::new(sample_rate, param_receiver, telemetry_clone);

                device
                    .build_output_stream(
                        &config,
                        move |data: &mut [i16], _: &cpal::OutputCallbackInfo| {
                            let needed_stereo_samples = (data.len() / ch_usize) * 2;
                            let target_watermark = needed_stereo_samples * 4;
                            let min_startup_watermark = prebuffer_threshold.max(target_watermark);

                            let is_prebuffered = prebuffered_clone.load(Ordering::Relaxed);
                            if !is_prebuffered {
                                if consumer.len() >= min_startup_watermark {
                                    prebuffered_clone.store(true, Ordering::Relaxed);
                                } else {
                                    for sample in data.iter_mut() {
                                        *sample = 0;
                                    }
                                    return;
                                }
                            }

                            if ch_usize == 2 {
                                let current_level = consumer.len();
                                if current_level < needed_stereo_samples {
                                    prebuffered_clone.store(false, Ordering::Relaxed);
                                }

                                let mut offset = 0;
                                let mut stack_buf = [0.0f32; 128];
                                while offset < data.len() {
                                    let chunk_len = (data.len() - offset).min(128);
                                    let read = consumer.pop_slice(&mut stack_buf[..chunk_len]);
                                    if read < chunk_len {
                                        let shortfall = chunk_len - read;
                                        underruns_clone.fetch_add(shortfall, Ordering::Relaxed);
                                        let last_l = if read >= 2 { stack_buf[read - 2] } else { 0.0 };
                                        let last_r = if read >= 2 { stack_buf[read - 1] } else { 0.0 };
                                        let fade_frames = (shortfall / 2).min(16);
                                        for f in 0..fade_frames {
                                            let factor = 1.0 - ((f + 1) as f32 / (fade_frames + 1) as f32);
                                            stack_buf[read + f * 2] = last_l * factor;
                                            stack_buf[read + f * 2 + 1] = last_r * factor;
                                        }
                                        for s in &mut stack_buf[read + fade_frames * 2..chunk_len] {
                                            *s = 0.0;
                                        }
                                        prebuffered_clone.store(false, Ordering::Relaxed);
                                    }
                                    dsp.process_interleaved_stereo(&mut stack_buf[..chunk_len]);
                                    for i in 0..chunk_len {
                                        let s = stack_buf[i];
                                        let scaled = (s * 32767.0).clamp(-32768.0, 32767.0);
                                        data[offset + i] = scaled as i16;
                                    }
                                    offset += chunk_len;
                                }
                            } else {
                                let had_underrun = render_multichannel_i16(
                                    &mut consumer,
                                    &mut dsp,
                                    data,
                                    ch_usize,
                                    &underruns_clone,
                                );
                                if had_underrun {
                                    prebuffered_clone.store(false, Ordering::Relaxed);
                                }
                            }
                        },
                        err_fn,
                        None,
                    )
                    .map_err(|e| AudioError::StreamBuildError(e.to_string()))?
            }
            SampleFormat::U16 => {
                let mut consumer = consumer;
                let ch_usize = channels as usize;
                let mut dsp = DspProcessor::new(sample_rate, param_receiver, telemetry_clone);

                device
                    .build_output_stream(
                        &config,
                        move |data: &mut [u16], _: &cpal::OutputCallbackInfo| {
                            let needed_stereo_samples = (data.len() / ch_usize) * 2;
                            let target_watermark = needed_stereo_samples * 4;
                            let min_startup_watermark = prebuffer_threshold.max(target_watermark);

                            let is_prebuffered = prebuffered_clone.load(Ordering::Relaxed);
                            if !is_prebuffered {
                                if consumer.len() >= min_startup_watermark {
                                    prebuffered_clone.store(true, Ordering::Relaxed);
                                } else {
                                    for sample in data.iter_mut() {
                                        *sample = 32768;
                                    }
                                    return;
                                }
                            }

                            if ch_usize == 2 {
                                let current_level = consumer.len();
                                if current_level < needed_stereo_samples {
                                    prebuffered_clone.store(false, Ordering::Relaxed);
                                }

                                let mut offset = 0;
                                let mut stack_buf = [0.0f32; 128];
                                while offset < data.len() {
                                    let chunk_len = (data.len() - offset).min(128);
                                    let read = consumer.pop_slice(&mut stack_buf[..chunk_len]);
                                    if read < chunk_len {
                                        let shortfall = chunk_len - read;
                                        underruns_clone.fetch_add(shortfall, Ordering::Relaxed);
                                        let last_l = if read >= 2 { stack_buf[read - 2] } else { 0.0 };
                                        let last_r = if read >= 2 { stack_buf[read - 1] } else { 0.0 };
                                        let fade_frames = (shortfall / 2).min(16);
                                        for f in 0..fade_frames {
                                            let factor = 1.0 - ((f + 1) as f32 / (fade_frames + 1) as f32);
                                            stack_buf[read + f * 2] = last_l * factor;
                                            stack_buf[read + f * 2 + 1] = last_r * factor;
                                        }
                                        for s in &mut stack_buf[read + fade_frames * 2..chunk_len] {
                                            *s = 0.0;
                                        }
                                        prebuffered_clone.store(false, Ordering::Relaxed);
                                    }
                                    dsp.process_interleaved_stereo(&mut stack_buf[..chunk_len]);
                                    for i in 0..chunk_len {
                                        let s = stack_buf[i];
                                        let scaled = ((s * 32767.0) + 32768.0).clamp(0.0, 65535.0);
                                        data[offset + i] = scaled as u16;
                                    }
                                    offset += chunk_len;
                                }
                            } else {
                                let had_underrun = render_multichannel_u16(
                                    &mut consumer,
                                    &mut dsp,
                                    data,
                                    ch_usize,
                                    &underruns_clone,
                                );
                                if had_underrun {
                                    prebuffered_clone.store(false, Ordering::Relaxed);
                                }
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
            param_sender,
            telemetry,
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

    /// Handle to send lock-free parameter update commands to the real-time audio thread.
    #[inline]
    pub fn param_sender(&self) -> &DspParamSender {
        &self.param_sender
    }

    /// Handle to query real-time peak meter and limiter telemetry.
    #[inline]
    pub fn telemetry(&self) -> &Arc<AudioTelemetry> {
        &self.telemetry
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

        let (mut producer, consumer) = AudioRingBuffer::new(32768);
        // Pre-fill samples so dynamic hardware prebuffering threshold (>= 8448) is satisfied
        let dummy = vec![0.05f32; 16384];
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

    #[test]
    fn test_render_multichannel_f32_processing_and_command_draining() {
        let (mut producer, mut consumer) = AudioRingBuffer::new(256);
        let (sender, receiver) = create_param_channel(16);
        let telemetry = Arc::new(AudioTelemetry::new());
        let mut dsp = DspProcessor::new(48000, receiver, telemetry);
        let underruns = AtomicUsize::new(0);

        // Push 4 stereo frames (8 samples)
        let stereo_samples = [0.2f32, 0.4f32, 0.1f32, -0.3f32, 0.5f32, 0.5f32, -0.2f32, 0.2f32];
        producer.push_slice(&stereo_samples);

        // Send a parameter command to verify draining
        assert!(sender.set_bypass(true).is_ok());

        // Render buffer for 4 frames of 4 channels (16 samples) - typical 2.1 laptop subwoofer configuration
        let mut out_data = [0.0f32; 16];
        render_multichannel_f32(&mut consumer, &mut dsp, &mut out_data, 4, &underruns);

        // Verify that dsp drained commands
        assert!(dsp.is_bypassed());

        // Verify channel 0 (Left), channel 1 (Right), and channel 2/3 (Sub/Center mono mix)
        // Frame 0: L=0.2, R=0.4 => sub_center = 0.3
        assert!((out_data[0] - 0.2).abs() < 1e-4);
        assert!((out_data[1] - 0.4).abs() < 1e-4);
        assert!((out_data[2] - 0.3).abs() < 1e-4);
        assert!((out_data[3] - 0.3).abs() < 1e-4); // Subwoofer received 0.3!

        // Frame 1: L=0.1, R=-0.3 => sub_center = -0.1
        assert!((out_data[4] - 0.1).abs() < 1e-4);
        assert!((out_data[5] - (-0.3)).abs() < 1e-4);
        assert!((out_data[6] - (-0.1)).abs() < 1e-4);
        assert!((out_data[7] - (-0.1)).abs() < 1e-4);
    }

    #[test]
    fn test_render_multichannel_5_1_subwoofer_distribution() {
        let (mut producer, mut consumer) = AudioRingBuffer::new(256);
        let (sender, receiver) = create_param_channel(16);
        let telemetry = Arc::new(AudioTelemetry::new());
        let mut dsp = DspProcessor::new(48000, receiver, telemetry);
        let underruns = AtomicUsize::new(0);

        // Push 2 stereo frames (4 samples): Frame 0: [0.6, 0.4], Frame 1: [0.8, -0.2]
        producer.push_slice(&[0.6f32, 0.4f32, 0.8f32, -0.2f32]);
        // Set bypass so values are exact pass-through
        assert!(sender.set_bypass(true).is_ok());

        // Render buffer for 2 frames of 6 channels (12 samples)
        let mut out_5_1 = [0.0f32; 12];
        render_multichannel_f32(&mut consumer, &mut dsp, &mut out_5_1, 6, &underruns);

        // Frame 0: Ch0=0.6 (FL), Ch1=0.4 (FR), Ch2=0.5 (FC), Ch3=0.5 (LFE Sub), Ch4=0.6 (BL), Ch5=0.4 (BR)
        assert!((out_5_1[0] - 0.6).abs() < 1e-4);
        assert!((out_5_1[1] - 0.4).abs() < 1e-4);
        assert!((out_5_1[2] - 0.5).abs() < 1e-4);
        assert!((out_5_1[3] - 0.5).abs() < 1e-4); // LFE Subwoofer
        assert!((out_5_1[4] - 0.6).abs() < 1e-4);
        assert!((out_5_1[5] - 0.4).abs() < 1e-4);
    }

    #[test]
    fn test_render_multichannel_underrun_detection() {
        let (mut producer, mut consumer) = AudioRingBuffer::new(256);
        let (_sender, receiver) = create_param_channel(16);
        let telemetry = Arc::new(AudioTelemetry::new());
        let mut dsp = DspProcessor::new(48000, receiver, telemetry);
        let underruns = AtomicUsize::new(0);

        // Only push 2 samples (1 stereo frame), but render asks for 4 frames (8 samples)
        producer.push_slice(&[0.5f32, -0.5f32]);

        let mut out_stereo = [0.0f32; 8];
        let had_underrun = render_multichannel_f32(&mut consumer, &mut dsp, &mut out_stereo, 2, &underruns);

        // Must report that an underrun occurred
        assert!(had_underrun);
        assert!(underruns.load(Ordering::Relaxed) > 0);
    }

    #[test]
    fn test_closed_loop_interpolation_smoothness() {
        // Test backward stretch (N=8 frames from M=7 frames)
        let total_frames = 8usize;
        let in_frames = 7usize;
        let mut data = [0.0f32; 16];

        // Seed with ramping values [0.1, 0.1], [0.2, 0.2] ... [0.7, 0.7]
        for f in 0..in_frames {
            data[f * 2] = (f + 1) as f32 * 0.1;
            data[f * 2 + 1] = (f + 1) as f32 * 0.1;
        }

        let n_minus_1 = (total_frames - 1) as f32;
        let m_minus_1 = (in_frames - 1) as f32;
        for i in (0..total_frames).rev() {
            let p = (i as f32) * m_minus_1 / n_minus_1;
            let k = (p as usize).min(in_frames - 1);
            let alpha = p - (k as f32);
            let next_k = (k + 1).min(in_frames - 1);
            let l0 = data[k * 2];
            let r0 = data[k * 2 + 1];
            let l1 = data[next_k * 2];
            let r1 = data[next_k * 2 + 1];
            data[i * 2] = l0 + alpha * (l1 - l0);
            data[i * 2 + 1] = r0 + alpha * (r1 - r0);
        }

        // Output must start at 0.1 and end at 0.7 smoothly without NaN or discontinuities
        assert!((data[0] - 0.1).abs() < 1e-4);
        assert!((data[1] - 0.1).abs() < 1e-4);
        assert!((data[14] - 0.7).abs() < 1e-4);
        assert!((data[15] - 0.7).abs() < 1e-4);

        for w in data.windows(2) {
            assert!(!w[0].is_nan());
            assert!(!w[1].is_nan());
        }
    }
}

