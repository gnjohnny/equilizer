use serde::{Deserialize, Serialize};

use crate::audio::dsp::{BiquadCoefficients, EqBandConfig, FilterType, StereoBiquad};

/// Standard Butterworth low-shelf Q factor (1 / sqrt(2) ≈ 0.7071)
/// Provides maximally flat passband with smooth shelf transition and zero resonant overshoot.
pub const DEFAULT_BASS_BOOST_Q: f32 = 0.707_106_77;

/// Minimum and maximum allowable bass boost gain in decibels.
pub const MIN_BASS_BOOST_GAIN_DB: f32 = 0.0;
pub const MAX_BASS_BOOST_GAIN_DB: f32 = 12.0;

/// Smoothing time constant for gain transitions in seconds (10 ms).
/// Eliminates audible clicks, pops, and zipper noise during continuous slider manipulation.
const SMOOTHING_TIME_SEC: f32 = 0.010;

/// Number of stereo frames per smoothing parameter update sub-block.
const CHUNK_SIZE: usize = 16;

/// Operational modes for the dedicated bass boost engine.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum BassBoostMode {
    /// Sub-Bass mode centered at 45 Hz for deep sub-low rumble, cinematic weight, and low-end extension.
    SubBass,
    /// Punch mode centered at 90 Hz for kick drum transient punch, bass guitar presence, and impact.
    Punch,
}

impl BassBoostMode {
    /// Returns the canonical center frequency for this bass boost mode.
    #[inline]
    pub fn default_frequency_hz(&self) -> f32 {
        match self {
            BassBoostMode::SubBass => 45.0,
            BassBoostMode::Punch => 90.0,
        }
    }
}

/// Configuration payload for the dedicated bass boost module.
/// Uses camelCase JSON serialization to match frontend TypeScript interfaces exactly.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BassBoostConfig {
    pub enabled: bool,
    pub gain_db: f32,
    pub mode: BassBoostMode,
    pub frequency_hz: f32,
}

impl Default for BassBoostConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            gain_db: 0.0,
            mode: BassBoostMode::SubBass,
            frequency_hz: 45.0,
        }
    }
}

/// Dedicated hardware/DAW bass boost DSP engine.
///
/// Implements a tuned Direct Form II Transposed low-shelf biquad filter with:
/// - Dual operational modes (Sub-Bass @ 45 Hz vs. Punch @ 90 Hz).
/// - Intensity control from 0.0 dB to +12.0 dB.
/// - Exponential gain smoothing (de-zippering) to prevent clicks/pops during slider movement.
/// - Zero-allocation in-place interleaved stereo audio processing.
/// - Active flush-to-zero denormal protection on internal filter states.
pub struct BassBoostEngine {
    config: BassBoostConfig,
    sample_rate: u32,
    filter: StereoBiquad,
    current_gain_db: f32,
    target_gain_db: f32,
    effective_freq_hz: f32,
}

impl BassBoostEngine {
    /// Creates a new bass boost engine initialized for the specified sample rate.
    pub fn new(sample_rate: u32) -> Self {
        let config = BassBoostConfig::default();
        let effective_freq_hz = config.mode.default_frequency_hz();
        let filter = StereoBiquad::new(BiquadCoefficients::identity(), false);

        Self {
            config,
            sample_rate,
            filter,
            current_gain_db: 0.0,
            target_gain_db: 0.0,
            effective_freq_hz,
        }
    }

    /// Sets the engine configuration with smooth parameter interpolation.
    pub fn set_config(&mut self, config: BassBoostConfig) {
        let mode = config.mode;
        let frequency_hz = if config.frequency_hz > 0.0 {
            config.frequency_hz.clamp(20.0, 300.0)
        } else {
            mode.default_frequency_hz()
        };
        let gain_db = config.gain_db.clamp(MIN_BASS_BOOST_GAIN_DB, MAX_BASS_BOOST_GAIN_DB);

        let freq_changed = (self.effective_freq_hz - frequency_hz).abs() > 0.1;
        self.effective_freq_hz = frequency_hz;
        self.config = BassBoostConfig {
            enabled: config.enabled,
            gain_db,
            mode,
            frequency_hz,
        };

        if config.enabled {
            self.target_gain_db = gain_db;
            if freq_changed {
                self.update_filter_coefficients();
            }
        } else {
            self.target_gain_db = 0.0;
        }
    }

    /// Immediately sets the configuration and snaps gain without smoothing.
    /// Ideal for preset loading or initialization where smooth transition is not desired.
    pub fn set_config_immediate(&mut self, config: BassBoostConfig) {
        self.set_config(config);
        self.current_gain_db = self.target_gain_db;
        self.update_filter_coefficients();
    }

    /// Reconfigures the engine for a new hardware sample rate.
    pub fn set_sample_rate(&mut self, sample_rate: u32) {
        self.sample_rate = sample_rate;
        self.update_filter_coefficients();
    }

    /// Current active configuration.
    #[inline]
    pub fn config(&self) -> &BassBoostConfig {
        &self.config
    }

    /// True if bass boost is enabled and active.
    #[inline]
    pub fn is_enabled(&self) -> bool {
        self.config.enabled
    }

    /// Active operational mode.
    #[inline]
    pub fn mode(&self) -> BassBoostMode {
        self.config.mode
    }

    /// Target gain setting in decibels.
    #[inline]
    pub fn gain_db(&self) -> f32 {
        self.config.gain_db
    }

    /// Current smoothed gain in decibels.
    #[inline]
    pub fn current_gain_db(&self) -> f32 {
        self.current_gain_db
    }

    /// Active center frequency in Hertz.
    #[inline]
    pub fn frequency_hz(&self) -> f32 {
        self.effective_freq_hz
    }

    /// Active hardware sample rate in Hertz.
    #[inline]
    pub fn sample_rate(&self) -> u32 {
        self.sample_rate
    }

    /// Resets the internal biquad filter delay registers back to silence.
    pub fn reset(&mut self) {
        self.filter.reset();
    }

    /// Evaluates the frequency response |H(e^(jw))| in decibels at a given test frequency.
    /// Returns 0.0 dB when disabled.
    pub fn frequency_response_db(&self, freq_hz: f32) -> f32 {
        if !self.config.enabled || self.current_gain_db <= 0.001 {
            0.0
        } else {
            self.filter.coef.frequency_response_db(freq_hz, self.sample_rate)
        }
    }

    /// In-place processing of an interleaved stereo sample slice: `[L0, R0, L1, R1, ...]`.
    ///
    /// Adheres strictly to the real-time audio thread policy:
    /// - Zero heap allocations.
    /// - Zero mutex locks or blocking calls.
    /// - Smooth de-zippering interpolation over sub-blocks.
    /// - In-place pass-through when bypassed.
    #[inline(always)]
    pub fn process_interleaved_stereo(&mut self, samples: &mut [f32]) {
        if !self.config.enabled && self.current_gain_db <= 0.001 {
            return;
        }

        let (frames, _) = samples.as_chunks_mut::<2>();
        let mut frame_idx = 0;

        while frame_idx < frames.len() {
            let chunk_len = (frames.len() - frame_idx).min(CHUNK_SIZE);

            // Interpolate gain if ramping towards target
            if (self.current_gain_db - self.target_gain_db).abs() > 0.001 {
                let alpha = 1.0
                    - (-(chunk_len as f32) / (self.sample_rate as f32 * SMOOTHING_TIME_SEC)).exp();
                self.current_gain_db += (self.target_gain_db - self.current_gain_db) * alpha;

                if (self.current_gain_db - self.target_gain_db).abs() < 0.01 {
                    self.current_gain_db = self.target_gain_db;
                }
                self.update_filter_coefficients();
            }

            if self.filter.enabled {
                for frame in &mut frames[frame_idx..frame_idx + chunk_len] {
                    let (l, r) = self.filter.process_sample(frame[0], frame[1]);
                    frame[0] = l;
                    frame[1] = r;
                }
            }

            frame_idx += chunk_len;
        }
    }

    /// Recalculates normalized biquad coefficients for the low-shelf filter.
    fn update_filter_coefficients(&mut self) {
        if self.current_gain_db <= 0.001 {
            self.filter.enabled = false;
            self.filter.coef = BiquadCoefficients::identity();
        } else {
            let band_config = EqBandConfig {
                id: 0,
                enabled: true,
                filter_type: FilterType::LowShelf,
                frequency_hz: self.effective_freq_hz,
                gain_db: self.current_gain_db,
                q: DEFAULT_BASS_BOOST_Q,
            };
            self.filter.coef = BiquadCoefficients::from_config(&band_config, self.sample_rate);
            self.filter.enabled = true;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::f32::consts::PI;

    #[test]
    fn test_bass_boost_default_initialization() {
        let engine = BassBoostEngine::new(48000);
        assert!(!engine.is_enabled());
        assert_eq!(engine.mode(), BassBoostMode::SubBass);
        assert_eq!(engine.gain_db(), 0.0);
        assert_eq!(engine.current_gain_db(), 0.0);
        assert_eq!(engine.frequency_hz(), 45.0);
        assert_eq!(engine.frequency_response_db(45.0), 0.0);
        assert_eq!(engine.frequency_response_db(1000.0), 0.0);
    }

    #[test]
    fn test_sub_bass_mode_frequency_response() {
        let mut engine = BassBoostEngine::new(48000);
        engine.set_config_immediate(BassBoostConfig {
            enabled: true,
            gain_db: 12.0,
            mode: BassBoostMode::SubBass,
            frequency_hz: 45.0,
        });

        assert!(engine.is_enabled());
        assert_eq!(engine.current_gain_db(), 12.0);

        // Sub-bass shelf response at 48 kHz:
        // Deep sub-bass (20 Hz): should approach full +12 dB boost
        let resp_20hz = engine.frequency_response_db(20.0);
        assert!(
            resp_20hz > 10.5 && resp_20hz <= 12.5,
            "Expected ~12 dB at 20 Hz, got {:.2} dB",
            resp_20hz
        );

        // Center frequency (45 Hz): should be around half-gain in dB (+6 dB)
        let resp_45hz = engine.frequency_response_db(45.0);
        assert!(
            (resp_45hz - 6.0).abs() < 1.0,
            "Expected ~6.0 dB at 45 Hz center, got {:.2} dB",
            resp_45hz
        );

        // High frequencies (1000 Hz): should have negligible boost (~0 dB)
        let resp_1000hz = engine.frequency_response_db(1000.0);
        assert!(
            resp_1000hz.abs() < 0.2,
            "Expected ~0 dB at 1000 Hz, got {:.2} dB",
            resp_1000hz
        );
    }

    #[test]
    fn test_punch_mode_frequency_response() {
        let mut engine = BassBoostEngine::new(48000);
        engine.set_config_immediate(BassBoostConfig {
            enabled: true,
            gain_db: 12.0,
            mode: BassBoostMode::Punch,
            frequency_hz: 90.0,
        });

        assert_eq!(engine.mode(), BassBoostMode::Punch);
        assert_eq!(engine.frequency_hz(), 90.0);

        // Punch shelf response at 48 kHz:
        // Low punch range (45 Hz): strong boost (> 9.0 dB)
        let resp_45hz = engine.frequency_response_db(45.0);
        assert!(
            resp_45hz > 9.0,
            "Expected >9 dB at 45 Hz for Punch mode, got {:.2} dB",
            resp_45hz
        );

        // Center frequency (90 Hz): should be around half-gain in dB (+6 dB)
        let resp_90hz = engine.frequency_response_db(90.0);
        assert!(
            (resp_90hz - 6.0).abs() < 1.0,
            "Expected ~6.0 dB at 90 Hz center, got {:.2} dB",
            resp_90hz
        );

        // High frequencies (1000 Hz): should have negligible boost (~0 dB)
        let resp_1000hz = engine.frequency_response_db(1000.0);
        assert!(
            resp_1000hz.abs() < 0.2,
            "Expected ~0 dB at 1000 Hz, got {:.2} dB",
            resp_1000hz
        );
    }

    #[test]
    fn test_bass_boost_bypass_pass_through() {
        let mut engine = BassBoostEngine::new(48000);
        engine.set_config_immediate(BassBoostConfig {
            enabled: false,
            gain_db: 10.0,
            mode: BassBoostMode::SubBass,
            frequency_hz: 45.0,
        });

        // When bypassed, response must be strictly 0 dB
        assert_eq!(engine.frequency_response_db(45.0), 0.0);
        assert_eq!(engine.frequency_response_db(20.0), 0.0);

        // Verify audio stream passes through untouched
        let mut buffer = vec![0.5f32; 128];
        let original = buffer.clone();
        engine.process_interleaved_stereo(&mut buffer);

        assert_eq!(buffer, original);
    }

    #[test]
    fn test_gain_clamping_and_mode_frequency() {
        let mut engine = BassBoostEngine::new(48000);

        // Test clamping negative and excessive gain
        engine.set_config(BassBoostConfig {
            enabled: true,
            gain_db: -5.0,
            mode: BassBoostMode::Punch,
            frequency_hz: 0.0, // Should default to Punch mode frequency (90 Hz)
        });
        assert_eq!(engine.gain_db(), 0.0);
        assert_eq!(engine.frequency_hz(), 90.0);

        engine.set_config(BassBoostConfig {
            enabled: true,
            gain_db: 25.0, // Should clamp to max 12.0 dB
            mode: BassBoostMode::SubBass,
            frequency_hz: 0.0, // Should default to SubBass mode frequency (45 Hz)
        });
        assert_eq!(engine.gain_db(), 12.0);
        assert_eq!(engine.frequency_hz(), 45.0);
    }

    #[test]
    fn test_stereo_processing_in_place() {
        let sample_rate = 48000;
        let mut engine = BassBoostEngine::new(sample_rate);
        engine.set_config_immediate(BassBoostConfig {
            enabled: true,
            gain_db: 12.0,
            mode: BassBoostMode::SubBass,
            frequency_hz: 45.0,
        });

        // Generate 45 Hz stereo sine wave
        let num_frames = 256;
        let mut samples = Vec::with_capacity(num_frames * 2);
        for i in 0..num_frames {
            let t = i as f32 / sample_rate as f32;
            let val = (2.0 * PI * 45.0 * t).sin() * 0.2;
            samples.push(val); // Left
            samples.push(val); // Right
        }

        let input_energy: f32 = samples.iter().map(|s| s * s).sum();
        engine.process_interleaved_stereo(&mut samples);
        let output_energy: f32 = samples.iter().map(|s| s * s).sum();

        // With +12 dB bass boost, energy of 45 Hz sine wave must increase significantly
        assert!(
            output_energy > input_energy * 2.0,
            "Expected boosted energy > {}, got {}",
            input_energy * 2.0,
            output_energy
        );

        // Verify left and right channels remain equal and finite
        for frame in samples.chunks_exact(2) {
            assert!((frame[0] - frame[1]).abs() < 1e-6);
            assert!(frame[0].is_finite());
            assert!(frame[1].is_finite());
        }
    }

    #[test]
    fn test_smooth_gain_interpolation_no_clicks() {
        let sample_rate = 48000;
        let mut engine = BassBoostEngine::new(sample_rate);

        // Start disabled at 0 dB
        engine.set_config_immediate(BassBoostConfig {
            enabled: false,
            gain_db: 0.0,
            mode: BassBoostMode::SubBass,
            frequency_hz: 45.0,
        });

        // Trigger smooth ramp to 12 dB
        engine.set_config(BassBoostConfig {
            enabled: true,
            gain_db: 12.0,
            mode: BassBoostMode::SubBass,
            frequency_hz: 45.0,
        });

        assert_eq!(engine.current_gain_db(), 0.0);
        assert_eq!(engine.gain_db(), 12.0);

        // Process buffers of 128 frames (256 samples)
        let mut buffer = vec![0.1f32; 256];
        let mut prev_gain = engine.current_gain_db();

        for _ in 0..10 {
            engine.process_interleaved_stereo(&mut buffer);
            let current = engine.current_gain_db();
            assert!(
                current >= prev_gain,
                "Gain should ramp monotonically upward: {} vs {}",
                current,
                prev_gain
            );
            prev_gain = current;
        }

        // After processing sufficient samples (~50-80 ms), target gain must be reached
        for _ in 0..30 {
            engine.process_interleaved_stereo(&mut buffer);
        }
        assert_eq!(engine.current_gain_db(), 12.0);
    }

    #[test]
    fn test_reset_clears_registers() {
        let mut engine = BassBoostEngine::new(48000);
        engine.set_config_immediate(BassBoostConfig {
            enabled: true,
            gain_db: 6.0,
            mode: BassBoostMode::SubBass,
            frequency_hz: 45.0,
        });

        let mut buffer = vec![0.5f32; 64];
        engine.process_interleaved_stereo(&mut buffer);

        // Filter delay registers should be non-zero after processing non-zero signal
        assert!(engine.filter.left.s1.abs() > 0.0 || engine.filter.left.s2.abs() > 0.0);

        engine.reset();
        assert_eq!(engine.filter.left.s1, 0.0);
        assert_eq!(engine.filter.left.s2, 0.0);
        assert_eq!(engine.filter.right.s1, 0.0);
        assert_eq!(engine.filter.right.s2, 0.0);
    }

    #[test]
    fn test_serde_json_compatibility() {
        let json = r#"{"enabled":true,"gainDb":9.5,"mode":"Punch","frequencyHz":90.0}"#;
        let config: BassBoostConfig = serde_json::from_str(json).expect("Deserialization failed");

        assert!(config.enabled);
        assert_eq!(config.gain_db, 9.5);
        assert_eq!(config.mode, BassBoostMode::Punch);
        assert_eq!(config.frequency_hz, 90.0);

        let serialized = serde_json::to_string(&config).expect("Serialization failed");
        assert!(serialized.contains(r#""gainDb":9.5"#));
        assert!(serialized.contains(r#""mode":"Punch""#));
    }
}
