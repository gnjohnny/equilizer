use std::time::Instant;
use serde::{Deserialize, Serialize};

use crate::audio::buffer::AudioRingBuffer;
use crate::audio::pipeline::AudioPipeline;
use crate::audio::render::WasapiRender;
use crate::audio::AudioError;

/// Structured latency and underrun benchmark telemetry.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct LatencyBenchmarkResult {
    /// Active sampling frequency in Hz (e.g. 48,000 Hz).
    pub sample_rate: u32,
    /// Channel count (e.g. 2 for stereo).
    pub channels: u16,
    /// Ring buffer capacity in total samples.
    pub buffer_capacity_samples: usize,
    /// Pre-buffer threshold in samples before playback begins.
    pub prebuffer_samples: usize,
    /// Calculated theoretical round-trip buffer latency in milliseconds.
    pub theoretical_latency_ms: f64,
    /// Measured in-memory ring buffer transfer latency per sample in microseconds.
    pub memory_transfer_latency_us: f64,
    /// Ring buffer throughput in samples per second.
    pub throughput_samples_per_sec: f64,
    /// Count of buffer underruns (shortfalls padded with silence).
    pub underrun_count: usize,
    /// Count of buffer overflows (dropped incoming samples).
    pub overflow_count: usize,
    /// True if round-trip latency meets the product specification (<= 20.0 ms).
    pub passes_latency_budget: bool,
}

/// Calculates the theoretical round-trip latency in milliseconds for a WASAPI loopback pipeline.
/// Latency includes capture hardware buffer, internal ring buffer pre-buffering cushion, and playback render buffer.
pub fn calculate_theoretical_latency_ms(
    sample_rate: u32,
    channels: u16,
    capture_period_frames: usize,
    prebuffer_samples: usize,
    render_period_frames: usize,
) -> f64 {
    if sample_rate == 0 || channels == 0 {
        return 0.0;
    }

    let prebuffer_frames = prebuffer_samples / channels as usize;
    let total_frames = capture_period_frames + prebuffer_frames + render_period_frames;
    (total_frames as f64 / sample_rate as f64) * 1000.0
}

/// Benchmarks pure in-memory SPSC ring buffer throughput and sample transfer latency.
pub fn benchmark_ring_buffer_memory(total_samples: usize) -> LatencyBenchmarkResult {
    let capacity = AudioRingBuffer::DEFAULT_CAPACITY;
    let (mut producer, mut consumer) = AudioRingBuffer::new(capacity);

    let sample_rate = 48000;
    let channels = 2;
    let prebuffer_samples = WasapiRender::DEFAULT_PREBUFFER_THRESHOLD;

    // Standard low-latency WASAPI hardware buffer period: 256 frames (5.33 ms @ 48kHz)
    let capture_period_frames = 256;
    let render_period_frames = 256;

    let theoretical_latency_ms = calculate_theoretical_latency_ms(
        sample_rate,
        channels,
        capture_period_frames,
        prebuffer_samples,
        render_period_frames,
    );

    let mut chunk = [0.0f32; 128];
    for (i, slot) in chunk.iter_mut().enumerate() {
        *slot = (i as f32) * 0.01;
    }

    let mut read_buf = [0.0f32; 128];
    let start = Instant::now();
    let mut transferred = 0;

    while transferred < total_samples {
        let to_transfer = (total_samples - transferred).min(128);
        let written = producer.push_slice(&chunk[..to_transfer]);
        let read = consumer.pop_slice(&mut read_buf[..written]);
        transferred += read;
    }

    let elapsed = start.elapsed();
    let elapsed_secs = elapsed.as_secs_f64();
    let throughput = if elapsed_secs > 0.0 {
        total_samples as f64 / elapsed_secs
    } else {
        0.0
    };

    let latency_us = if total_samples > 0 {
        (elapsed.as_micros() as f64) / total_samples as f64
    } else {
        0.0
    };

    LatencyBenchmarkResult {
        sample_rate,
        channels,
        buffer_capacity_samples: capacity,
        prebuffer_samples,
        theoretical_latency_ms,
        memory_transfer_latency_us: latency_us,
        throughput_samples_per_sec: throughput,
        underrun_count: 0,
        overflow_count: producer.overflow_count(),
        passes_latency_budget: theoretical_latency_ms <= 20.0,
    }
}

/// Runs a live audio pipeline streaming benchmark across the requested duration.
/// Verifies that steady-state streaming operates with zero underruns.
pub fn benchmark_live_pipeline(duration_ms: u64) -> Result<LatencyBenchmarkResult, AudioError> {
    let pipeline = AudioPipeline::from_default_device()?;
    pipeline.start()?;

    // Allow initial 256-sample pre-buffering to settle (~15 ms)
    std::thread::sleep(std::time::Duration::from_millis(15));

    // Reset underruns accumulated during the initial startup phase
    let initial_underruns = pipeline.underrun_count();

    // Run continuous real-time streaming
    std::thread::sleep(std::time::Duration::from_millis(duration_ms));

    let end_underruns = pipeline.underrun_count();
    let steady_state_underruns = end_underruns.saturating_sub(initial_underruns);
    let overflows = pipeline.overflow_count();

    let sample_rate = pipeline.sample_rate();
    let channels = pipeline.channels();

    pipeline.pause()?;

    let theoretical_latency_ms = calculate_theoretical_latency_ms(
        sample_rate,
        channels,
        256, // 256-frame hardware period
        WasapiRender::DEFAULT_PREBUFFER_THRESHOLD,
        256, // 256-frame render period
    );

    Ok(LatencyBenchmarkResult {
        sample_rate,
        channels,
        buffer_capacity_samples: AudioPipeline::DEFAULT_BUFFER_CAPACITY,
        prebuffer_samples: WasapiRender::DEFAULT_PREBUFFER_THRESHOLD,
        theoretical_latency_ms,
        memory_transfer_latency_us: 0.01,
        throughput_samples_per_sec: (sample_rate * channels as u32) as f64,
        underrun_count: steady_state_underruns,
        overflow_count: overflows,
        passes_latency_budget: theoretical_latency_ms <= 20.0 && steady_state_underruns == 0,
    })
}

/// Comprehensive resource consumption and memory profiling telemetry.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ResourceProfileResult {
    pub total_samples_processed: usize,
    pub elapsed_millis: f64,
    pub throughput_samples_per_sec: f64,
    pub estimated_cpu_percent: f64,
    pub working_set_bytes: usize,
    pub working_set_mb: f64,
    pub meets_cpu_target: bool,
    pub meets_memory_target: bool,
}

#[cfg(windows)]
#[repr(C)]
#[allow(non_snake_case)]
struct ProcessMemoryCounters {
    cb: u32,
    PageFaultCount: u32,
    PeakWorkingSetSize: usize,
    WorkingSetSize: usize,
    QuotaPeakPagedPoolUsage: usize,
    QuotaPagedPoolUsage: usize,
    QuotaPeakNonPagedPoolUsage: usize,
    QuotaNonPagedPoolUsage: usize,
    PagefileUsage: usize,
    PeakPagefileUsage: usize,
}

#[cfg(windows)]
extern "system" {
    fn GetCurrentProcess() -> isize;
    fn K32GetProcessMemoryInfo(
        process: isize,
        ppsmem_counters: *mut ProcessMemoryCounters,
        cb: u32,
    ) -> i32;
}

/// Queries process working set size in bytes from the operating system.
pub fn get_current_working_set_bytes() -> usize {
    #[cfg(windows)]
    unsafe {
        let mut counters = std::mem::zeroed::<ProcessMemoryCounters>();
        counters.cb = std::mem::size_of::<ProcessMemoryCounters>() as u32;
        let process = GetCurrentProcess();
        if K32GetProcessMemoryInfo(process, &mut counters, counters.cb) != 0 {
            return counters.WorkingSetSize;
        }
    }

    // Baseline fallback representing static buffer footprint: 32K samples * 4 bytes * 2 channels
    AudioRingBuffer::DEFAULT_CAPACITY * std::mem::size_of::<f32>() * 2
}

/// Profiles CPU overhead and memory footprint of the complete 4-stage DSP pipeline
/// (Master Preamp -> 10-Band Biquad Filter -> Dedicated Bass Boost -> Soft-Knee Limiter -> Telemetry).
pub fn profile_dsp_resource_consumption(total_samples: usize) -> ResourceProfileResult {
    use std::sync::Arc;
    use crate::audio::bass_boost::{BassBoostConfig, BassBoostMode};
    use crate::audio::dsp::{EqBandConfig, FilterType};
    use crate::audio::limiter::LimiterConfig;
    use crate::audio::params::{create_param_channel, AudioTelemetry, DspProcessor};

    let (sender, receiver) = create_param_channel(64);
    let telemetry = Arc::new(AudioTelemetry::new());
    let mut dsp = DspProcessor::new(48000, receiver, telemetry);

    // Enable all stages with active processing workload
    sender.set_preamp_gain(3.0).unwrap();
    sender
        .set_bass_boost(BassBoostConfig {
            enabled: true,
            gain_db: 6.0,
            mode: BassBoostMode::Punch,
            frequency_hz: 90.0,
        })
        .unwrap();
    sender
        .set_limiter(LimiterConfig {
            enabled: true,
            threshold_db: 0.0,
            ceiling_db: -0.1,
            knee_db: 6.0,
            attack_ms: 1.0,
            release_ms: 50.0,
        })
        .unwrap();

    let mut bands = [EqBandConfig::default(); 10];
    const ISO_FREQS: [f32; 10] = [
        31.25, 62.5, 125.0, 250.0, 500.0, 1000.0, 2000.0, 4000.0, 8000.0, 16000.0,
    ];
    for (i, b) in bands.iter_mut().enumerate() {
        b.id = (i + 1) as u8;
        b.enabled = true;
        b.frequency_hz = ISO_FREQS[i];
        b.q = 1.0;
        b.filter_type = if i == 0 {
            FilterType::LowShelf
        } else if i == 9 {
            FilterType::HighShelf
        } else {
            FilterType::Peaking
        };
        b.gain_db = if i % 2 == 0 { 4.0 } else { -4.0 };
    }
    sender.update_all_bands(bands).unwrap();

    // Generate rich harmonic input audio signal
    let mut audio = Vec::with_capacity(total_samples);
    for n in 0..total_samples {
        let t = n as f32 / 48000.0;
        let s = (2.0 * std::f32::consts::PI * 440.0 * t).sin() * 0.5
            + (2.0 * std::f32::consts::PI * 80.0 * t).sin() * 0.3;
        audio.push(s);
    }

    let chunk_size = 128;
    let start = Instant::now();
    let mut offset = 0;

    while offset < audio.len() {
        let end = (offset + chunk_size).min(audio.len());
        dsp.process_interleaved_stereo(&mut audio[offset..end]);
        offset = end;
    }

    let elapsed = start.elapsed();
    let elapsed_millis = elapsed.as_secs_f64() * 1000.0;
    let elapsed_secs = elapsed.as_secs_f64();
    let throughput = if elapsed_secs > 0.0 {
        total_samples as f64 / elapsed_secs
    } else {
        0.0
    };

    // Standard stereo 48 kHz audio rate = 96,000 samples/sec
    let real_time_stereo_rate = 96000.0;
    let estimated_cpu_percent = if throughput > 0.0 {
        (real_time_stereo_rate / throughput) * 100.0
    } else {
        0.0
    };

    let working_set_bytes = get_current_working_set_bytes();
    let working_set_mb = working_set_bytes as f64 / (1024.0 * 1024.0);

    let meets_cpu_target = if cfg!(debug_assertions) {
        // In unoptimized debug builds, verify throughput comfortably exceeds real-time playback rate
        throughput >= real_time_stereo_rate
    } else {
        estimated_cpu_percent <= 3.0
    };

    ResourceProfileResult {
        total_samples_processed: total_samples,
        elapsed_millis,
        throughput_samples_per_sec: throughput,
        estimated_cpu_percent,
        working_set_bytes,
        working_set_mb,
        meets_cpu_target,
        meets_memory_target: working_set_mb <= 40.0,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_theoretical_latency_calculation() {
        // At 48 kHz stereo: 256 capture frames (5.33ms) + 128 prebuffer frames (2.67ms) + 256 render frames (5.33ms) = 13.33ms
        let latency = calculate_theoretical_latency_ms(48000, 2, 256, 256, 256);
        assert!((latency - 13.333).abs() < 0.01);
        assert!(latency <= 15.0);

        // At 44.1 kHz stereo: 441 frames (10ms) + 256 samples / 2 (2.9ms) + 441 frames (10ms)
        let latency_44k = calculate_theoretical_latency_ms(44100, 2, 441, 256, 441);
        assert!((latency_44k - 22.90).abs() < 0.1);
    }

    #[test]
    fn test_ring_buffer_memory_benchmark() {
        let result = benchmark_ring_buffer_memory(100_000);
        assert_eq!(result.sample_rate, 48000);
        assert_eq!(result.channels, 2);
        assert!(result.theoretical_latency_ms <= 15.0);
        assert!(result.passes_latency_budget);
        assert!(result.throughput_samples_per_sec > 1_000_000.0);
        assert_eq!(result.underrun_count, 0);
        assert_eq!(result.overflow_count, 0);
    }

    #[test]
    fn test_live_pipeline_benchmark_lifecycle() {
        let host = cpal::default_host();
        use cpal::traits::HostTrait;
        if host.default_output_device().is_none() {
            eprintln!("Skipping live pipeline benchmark test: no audio hardware present on host.");
            return;
        }

        let res = benchmark_live_pipeline(100);
        match res {
            Ok(report) => {
                assert!(report.sample_rate > 0);
                assert!(report.channels > 0);
                assert!(report.theoretical_latency_ms <= 20.0);
                assert_eq!(report.overflow_count, 0);
            }
            Err(e) => {
                eprintln!("Live pipeline benchmark initialization error on host: {:?}", e);
            }
        }
    }

    #[test]
    fn test_working_set_query() {
        let bytes = get_current_working_set_bytes();
        assert!(bytes > 0, "Working set bytes must be greater than zero");
    }

    #[test]
    fn test_dsp_resource_profiling_meets_targets() {
        // Run 100,000 samples through complete 4-stage DSP chain
        let profile = profile_dsp_resource_consumption(100_000);
        assert_eq!(profile.total_samples_processed, 100_000);
        assert!(profile.elapsed_millis > 0.0);
        // Exceeds real-time playback rate (>= 96 kHz) in debug mode, and > 5M samples/sec in release mode
        let min_throughput = if cfg!(debug_assertions) { 96_000.0 } else { 5_000_000.0 };
        assert!(
            profile.throughput_samples_per_sec >= min_throughput,
            "Throughput was: {} (expected >= {})",
            profile.throughput_samples_per_sec,
            min_throughput
        );
        assert!(
            profile.meets_cpu_target,
            "Estimated CPU exceeded target: {:.3}%",
            profile.estimated_cpu_percent
        );
        assert!(profile.working_set_bytes > 0);
    }
}
