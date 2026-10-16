use serde::{Deserialize, Serialize};

use crate::audio_safety::{db_to_linear, flush_denormal, linear_to_db};

/// Default limiter ceiling in decibels relative to full scale (-0.1 dBFS).
/// Provides headroom against DAC inter-sample reconstruction overshoots.
pub const DEFAULT_LIMITER_CEILING_DB: f32 = -0.1;

/// Default limiter threshold in decibels relative to full scale (-0.5 dBFS).
pub const DEFAULT_LIMITER_THRESHOLD_DB: f32 = -0.5;

/// Default soft-knee transition width in decibels (0.8 dB).
pub const DEFAULT_LIMITER_KNEE_DB: f32 = 0.8;

/// Fast attack time constant in milliseconds (1.0 ms) to catch rapid transients with 0 ms lookahead.
pub const DEFAULT_LIMITER_ATTACK_MS: f32 = 1.0;

/// Smooth release time constant in milliseconds (50.0 ms) to prevent audible distortion and pumping.
pub const DEFAULT_LIMITER_RELEASE_MS: f32 = 50.0;

/// Configuration parameters for the soft-knee brickwall limiter.
/// Serialized with camelCase keys to match frontend models.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LimiterConfig {
    pub enabled: bool,
    pub ceiling_db: f32,
    pub threshold_db: f32,
    pub knee_db: f32,
    pub attack_ms: f32,
    pub release_ms: f32,
}

impl Default for LimiterConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            ceiling_db: DEFAULT_LIMITER_CEILING_DB,
            threshold_db: DEFAULT_LIMITER_THRESHOLD_DB,
            knee_db: DEFAULT_LIMITER_KNEE_DB,
            attack_ms: DEFAULT_LIMITER_ATTACK_MS,
            release_ms: DEFAULT_LIMITER_RELEASE_MS,
        }
    }
}

/// Integrated soft-knee anti-clipping brickwall limiter.
///
/// Positioned at the very end of the audio processing pipeline (after pre-amp, biquad EQ,
/// and bass boost) to guarantee that output audio samples never exceed 0 dBFS (|sample| < 1.0).
///
/// Features:
/// - Zero lookahead latency (0 ms added delay to maintain round-trip budget <= 15 ms).
/// - Asymmetric envelope detector: 1.0 ms attack for transient catch, 50.0 ms release for smooth recovery.
/// - Continuous C^1 soft-knee curve bridging linear pass-through and brickwall limiting.
/// - Hard brickwall output clamp to ceiling level (default -0.1 dBFS ≈ 0.98855).
/// - Real-time engagement telemetry: `is_active()` and `gain_reduction_db()`.
/// - Zero heap allocations and active denormal flush-to-zero protection.
pub struct Limiter {
    config: LimiterConfig,
    sample_rate: u32,
    envelope: f32,
    ceiling_linear: f32,
    gain_reduction_db: f32,
    attack_coeff: f32,
    release_coeff: f32,
}

impl Limiter {
    /// Creates a new soft-knee limiter armed with default settings for the given sample rate.
    pub fn new(sample_rate: u32) -> Self {
        let config = LimiterConfig::default();
        let ceiling_linear = db_to_linear(config.ceiling_db);
        let attack_coeff = Self::calculate_coeff(config.attack_ms, sample_rate);
        let release_coeff = Self::calculate_coeff(config.release_ms, sample_rate);

        Self {
            config,
            sample_rate,
            envelope: 0.0,
            ceiling_linear,
            gain_reduction_db: 0.0,
            attack_coeff,
            release_coeff,
        }
    }

    /// Calculates a single-pole filter coefficient for the given time constant in milliseconds.
    #[inline]
    fn calculate_coeff(time_ms: f32, sample_rate: u32) -> f32 {
        if sample_rate == 0 || time_ms <= 0.0 {
            1.0
        } else {
            let tau = time_ms * 0.001;
            1.0 - (-1.0 / (sample_rate as f32 * tau)).exp()
        }
    }

    /// Sets the limiter configuration and updates internal ballistics.
    pub fn set_config(&mut self, config: LimiterConfig) {
        let ceiling_db = config.ceiling_db.clamp(-6.0, 0.0);
        let threshold_db = config.threshold_db.clamp(-12.0, ceiling_db);
        let knee_db = config.knee_db.clamp(0.1, 6.0);
        let attack_ms = config.attack_ms.clamp(0.1, 50.0);
        let release_ms = config.release_ms.clamp(5.0, 500.0);

        self.config = LimiterConfig {
            enabled: config.enabled,
            ceiling_db,
            threshold_db,
            knee_db,
            attack_ms,
            release_ms,
        };

        self.ceiling_linear = db_to_linear(self.config.ceiling_db);
        self.attack_coeff = Self::calculate_coeff(self.config.attack_ms, self.sample_rate);
        self.release_coeff = Self::calculate_coeff(self.config.release_ms, self.sample_rate);
    }

    /// Enables or disables the limiter stage.
    pub fn set_enabled(&mut self, enabled: bool) {
        self.config.enabled = enabled;
        if !enabled {
            self.gain_reduction_db = 0.0;
        }
    }

    /// Reconfigures ballistics for a new hardware sample rate.
    pub fn set_sample_rate(&mut self, sample_rate: u32) {
        self.sample_rate = sample_rate;
        self.attack_coeff = Self::calculate_coeff(self.config.attack_ms, sample_rate);
        self.release_coeff = Self::calculate_coeff(self.config.release_ms, sample_rate);
    }

    /// Current limiter configuration.
    #[inline]
    pub fn config(&self) -> &LimiterConfig {
        &self.config
    }

    /// True if the limiter is armed / enabled.
    #[inline]
    pub fn is_enabled(&self) -> bool {
        self.config.enabled
    }

    /// True if the limiter is actively attenuating signal peaks (> 0.05 dB reduction).
    #[inline]
    pub fn is_active(&self) -> bool {
        self.config.enabled && self.gain_reduction_db > 0.05
    }

    /// Current gain reduction in decibels (e.g. 3.0 means 3 dB of attenuation applied).
    #[inline]
    pub fn current_gain_reduction_db(&self) -> f32 {
        if self.config.enabled {
            self.gain_reduction_db
        } else {
            0.0
        }
    }

    /// Current peak envelope level.
    #[inline]
    pub fn envelope(&self) -> f32 {
        self.envelope
    }

    /// Resets the internal envelope follower back to silence.
    pub fn reset(&mut self) {
        self.envelope = 0.0;
        self.gain_reduction_db = 0.0;
    }

    /// Computes the linear gain multiplier using a continuous soft-knee brickwall curve.
    #[inline]
    fn compute_gain_reduction(&self, env_linear: f32) -> (f32, f32) {
        let env_db = linear_to_db(env_linear);
        let ceiling = self.config.ceiling_db;
        let knee = self.config.knee_db;
        let half_knee = knee * 0.5;

        // Knee region spans [ceiling - half_knee, ceiling + half_knee]
        let knee_start = ceiling - half_knee;
        let knee_end = ceiling + half_knee;

        if env_db <= knee_start {
            // Below the knee: perfectly linear pass-through (0 dB reduction)
            (1.0, 0.0)
        } else if env_db >= knee_end {
            // Above the knee: brickwall limiting clamps output strictly to ceiling
            let red_db = env_db - ceiling;
            let linear_gain = db_to_linear(-red_db);
            (linear_gain, red_db)
        } else {
            // Inside the soft-knee region: continuous C^1 quadratic polynomial
            let delta = env_db - knee_start;
            let red_db = (delta * delta) / (2.0 * knee);
            let linear_gain = db_to_linear(-red_db);
            (linear_gain, red_db)
        }
    }

    /// In-place processing of an interleaved stereo sample slice: `[L0, R0, L1, R1, ...]`.
    ///
    /// Adheres strictly to the real-time audio thread policy:
    /// - Zero heap allocations.
    /// - Zero mutex locks or blocking calls.
    /// - Smooth soft-knee gain reduction with 1 ms attack and 50 ms release.
    /// - Absolute ceiling clamp ensuring no sample can ever exceed -0.1 dBFS.
    /// - Active denormal flush-to-zero protection.
    #[inline(always)]
    pub fn process_interleaved_stereo(&mut self, samples: &mut [f32]) {
        if !self.config.enabled {
            // Bypassed: pass through with hard safety clamp against DAC damage
            for sample in samples.iter_mut() {
                *sample = sample.clamp(-1.0, 1.0);
            }
            self.gain_reduction_db = 0.0;
            return;
        }

        let (frames, _) = samples.as_chunks_mut::<2>();
        let mut max_block_reduction = 0.0f32;

        for frame in frames {
            let left_abs = frame[0].abs();
            let right_abs = frame[1].abs();
            let peak = left_abs.max(right_abs);

            // Envelope follower with asymmetric attack and release
            if peak > self.envelope {
                self.envelope += self.attack_coeff * (peak - self.envelope);
            } else {
                self.envelope += self.release_coeff * (peak - self.envelope);
            }
            self.envelope = flush_denormal(self.envelope);

            // Calculate soft-knee gain reduction for current envelope level
            let (gain, red_db) = self.compute_gain_reduction(self.envelope);
            if red_db > max_block_reduction {
                max_block_reduction = red_db;
            }

            // Apply smooth gain reduction
            let out_l = frame[0] * gain;
            let out_r = frame[1] * gain;

            // Absolute brickwall clamp ensuring zero sample escapes the ceiling
            frame[0] = flush_denormal(out_l.clamp(-self.ceiling_linear, self.ceiling_linear));
            frame[1] = flush_denormal(out_r.clamp(-self.ceiling_linear, self.ceiling_linear));
        }

        self.gain_reduction_db = max_block_reduction;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_limiter_default_initialization() {
        let limiter = Limiter::new(48000);
        assert!(limiter.is_enabled());
        assert!(!limiter.is_active());
        assert_eq!(limiter.current_gain_reduction_db(), 0.0);
        assert_eq!(limiter.config().ceiling_db, DEFAULT_LIMITER_CEILING_DB);
        assert_eq!(limiter.config().threshold_db, DEFAULT_LIMITER_THRESHOLD_DB);
        assert_eq!(limiter.config().attack_ms, 1.0);
        assert_eq!(limiter.config().release_ms, 50.0);
        assert_eq!(limiter.envelope(), 0.0);
    }

    #[test]
    fn test_sub_threshold_transparent_pass_through() {
        let mut limiter = Limiter::new(48000);

        // Low level signal: 0.5 linear = -6.02 dBFS (well below -0.9 dBFS knee start)
        let mut buffer = vec![0.5f32; 128];
        let original = buffer.clone();

        limiter.process_interleaved_stereo(&mut buffer);

        // Signal should pass through transparently
        for (i, &sample) in buffer.iter().enumerate() {
            assert!(
                (sample - original[i]).abs() < 1e-4,
                "Sample altered at index {}: got {}, expected {}",
                i,
                sample,
                original[i]
            );
        }

        assert!(!limiter.is_active());
        assert_eq!(limiter.current_gain_reduction_db(), 0.0);
    }

    #[test]
    fn test_brickwall_clamping_on_extreme_signal() {
        let mut limiter = Limiter::new(48000);
        let ceiling = limiter.ceiling_linear; // ~0.98855

        // Massive over-driven signal: 4.0 linear = +12.04 dBFS (simulates +12 dB bass boost overload)
        let mut buffer = vec![4.0f32; 256];

        limiter.process_interleaved_stereo(&mut buffer);

        // Every single output sample must be strictly <= ceiling
        for (i, &sample) in buffer.iter().enumerate() {
            assert!(
                sample.abs() <= ceiling + 1e-6,
                "Sample breached ceiling at index {}: got {}, ceiling is {}",
                i,
                sample,
                ceiling
            );
            assert!(sample.is_finite());
        }

        // Limiter must be actively engaged with significant gain reduction
        assert!(limiter.is_active());
        assert!(
            limiter.current_gain_reduction_db() > 10.0,
            "Expected >10 dB gain reduction on +12 dBFS signal, got {:.2} dB",
            limiter.current_gain_reduction_db()
        );
    }

    #[test]
    fn test_soft_knee_continuous_curve() {
        let limiter = Limiter::new(48000);

        // Test points across the knee: -2 dBFS, -1 dBFS, -0.5 dBFS, 0 dBFS, +3 dBFS
        let test_levels_db = [-2.0, -1.0, -0.6, -0.3, 0.0, 1.0, 3.0];
        let mut prev_reduction = 0.0;

        for &db in &test_levels_db {
            let env = db_to_linear(db);
            let (gain, red_db) = limiter.compute_gain_reduction(env);

            assert!(gain <= 1.0, "Gain multiplier must never exceed 1.0");
            assert!(gain > 0.0, "Gain multiplier must be positive");
            assert!(
                red_db >= prev_reduction,
                "Gain reduction must increase monotonically with input: {} vs {}",
                red_db,
                prev_reduction
            );
            prev_reduction = red_db;
        }
    }

    #[test]
    fn test_release_recovery_after_transient() {
        let mut limiter = Limiter::new(48000);

        // Feed loud burst: 2.0 linear = +6.02 dBFS across 512 samples (~5.3 ms)
        let mut loud_buffer = vec![2.0f32; 512];
        limiter.process_interleaved_stereo(&mut loud_buffer);

        assert!(limiter.is_active());
        assert!(limiter.current_gain_reduction_db() > 5.0);

        // Feed silence and monitor release over 100 ms (~4800 frames)
        let mut silent_buffer = vec![0.0f32; 256];
        for _ in 0..25 {
            limiter.process_interleaved_stereo(&mut silent_buffer);
        }

        // Envelope should decay and limiter should disengage
        assert!(
            limiter.current_gain_reduction_db() < 0.1,
            "Limiter did not release: still at {:.2} dB reduction",
            limiter.current_gain_reduction_db()
        );
        assert!(!limiter.is_active());
    }

    #[test]
    fn test_bypass_behavior() {
        let mut limiter = Limiter::new(48000);
        limiter.set_enabled(false);

        assert!(!limiter.is_enabled());
        assert!(!limiter.is_active());

        // Process audio with limiter bypassed
        let mut buffer = vec![0.8f32; 64];
        let original = buffer.clone();
        limiter.process_interleaved_stereo(&mut buffer);

        assert_eq!(buffer, original);
        assert_eq!(limiter.current_gain_reduction_db(), 0.0);
    }

    #[test]
    fn test_denormal_flush() {
        let mut limiter = Limiter::new(48000);
        let mut buffer = vec![1e-38f32; 64];

        limiter.process_interleaved_stereo(&mut buffer);

        for &sample in &buffer {
            assert_eq!(sample, 0.0);
        }
        assert_eq!(limiter.envelope(), 0.0);
    }
}
