use crate::audio_safety::{db_to_linear, flush_denormal, linear_to_db};

/// Minimum allowable master pre-amp gain in decibels (-20 dB attenuation / pad).
pub const MIN_PREAMP_GAIN_DB: f32 = -20.0;

/// Maximum allowable master pre-amp gain in decibels (+6 dB boost).
pub const MAX_PREAMP_GAIN_DB: f32 = 6.0;

/// Default master pre-amp gain in decibels (0 dB reference / unity gain).
pub const DEFAULT_PREAMP_GAIN_DB: f32 = 0.0;

/// Smoothing time constant for gain transitions in seconds (10 ms).
/// Eliminates audible clicks, pops, and zipper noise during continuous slider movement.
const SMOOTHING_TIME_SEC: f32 = 0.010;

/// Number of stereo frames per smoothing parameter update sub-block.
const CHUNK_SIZE: usize = 16;

/// Master Pre-Amp attenuation and gain stage.
///
/// Applied before the cascaded biquad equalizer and bass boost stages to provide headroom
/// protection against digital clipping (|sample| >= 1.0) when positive EQ boosts are applied.
///
/// Features:
/// - Linear gain conversion: `gain_linear = 10^(gainDb / 20)`.
/// - Gain range: -20.0 dB (0.10x) to +6.0 dB (~1.995x).
/// - Fast-path zero-cost unity pass-through when at 0.0 dB or bypassed.
/// - Exponential gain smoothing (de-zippering) over 10 ms sub-blocks.
/// - In-place interleaved stereo processing with zero allocations and denormal protection.
#[derive(Debug, Clone, PartialEq)]
pub struct PreampStage {
    gain_db: f32,
    target_linear_gain: f32,
    current_linear_gain: f32,
    sample_rate: u32,
    bypassed: bool,
}

impl PreampStage {
    /// Creates a new pre-amp stage initialized at unity gain (0.0 dB) for the given sample rate.
    pub fn new(sample_rate: u32) -> Self {
        Self {
            gain_db: DEFAULT_PREAMP_GAIN_DB,
            target_linear_gain: 1.0,
            current_linear_gain: 1.0,
            sample_rate,
            bypassed: false,
        }
    }

    /// Sets the target pre-amp gain in decibels with smooth de-zippering interpolation.
    /// Values are clamped strictly between MIN_PREAMP_GAIN_DB (-20 dB) and MAX_PREAMP_GAIN_DB (+6 dB).
    pub fn set_gain_db(&mut self, gain_db: f32) {
        let clamped = gain_db.clamp(MIN_PREAMP_GAIN_DB, MAX_PREAMP_GAIN_DB);
        self.gain_db = clamped;
        if self.bypassed {
            self.target_linear_gain = 1.0;
        } else {
            self.target_linear_gain = db_to_linear(clamped);
        }
    }

    /// Immediately snaps the pre-amp gain to the target level without smoothing.
    /// Intended for preset loading or stream initialization.
    pub fn set_gain_db_immediate(&mut self, gain_db: f32) {
        self.set_gain_db(gain_db);
        self.current_linear_gain = self.target_linear_gain;
    }

    /// Sets the bypass state of the pre-amp stage.
    pub fn set_bypassed(&mut self, bypassed: bool) {
        self.bypassed = bypassed;
        if bypassed {
            self.target_linear_gain = 1.0;
        } else {
            self.target_linear_gain = db_to_linear(self.gain_db);
        }
    }

    /// Reconfigures the stage for a new hardware sample rate.
    pub fn set_sample_rate(&mut self, sample_rate: u32) {
        self.sample_rate = sample_rate;
    }

    /// Target gain setting in decibels.
    #[inline]
    pub fn gain_db(&self) -> f32 {
        self.gain_db
    }

    /// Current effective gain in decibels based on the smoothed linear multiplier.
    #[inline]
    pub fn current_gain_db(&self) -> f32 {
        linear_to_db(self.current_linear_gain)
    }

    /// Target linear amplitude multiplier.
    #[inline]
    pub fn target_linear_gain(&self) -> f32 {
        self.target_linear_gain
    }

    /// Current smoothed linear amplitude multiplier.
    #[inline]
    pub fn current_linear_gain(&self) -> f32 {
        self.current_linear_gain
    }

    /// Returns whether the pre-amp stage is currently bypassed.
    #[inline]
    pub fn is_bypassed(&self) -> bool {
        self.bypassed
    }

    /// Current hardware sample rate in Hertz.
    #[inline]
    pub fn sample_rate(&self) -> u32 {
        self.sample_rate
    }

    /// Snaps current linear gain to target linear gain immediately.
    pub fn reset(&mut self) {
        self.current_linear_gain = self.target_linear_gain;
    }

    /// In-place processing of an interleaved stereo sample slice: `[L0, R0, L1, R1, ...]`.
    ///
    /// Adheres strictly to the real-time audio thread policy:
    /// - Zero heap allocations.
    /// - Zero mutex locks or blocking operations.
    /// - Fast-path return when at unity gain (1.0).
    /// - Smooth de-zippering interpolation over sub-blocks.
    /// - SIMD-vectorizable sample multiplication.
    #[inline(always)]
    pub fn process_interleaved_stereo(&mut self, samples: &mut [f32]) {
        let is_unity = (self.current_linear_gain - 1.0).abs() < 1e-4
            && (self.target_linear_gain - 1.0).abs() < 1e-4;

        if is_unity {
            return;
        }

        let is_ramping = (self.current_linear_gain - self.target_linear_gain).abs() > 1e-4;

        if is_ramping {
            let (frames, _) = samples.as_chunks_mut::<2>();
            let mut frame_idx = 0;

            while frame_idx < frames.len() {
                let chunk_len = (frames.len() - frame_idx).min(CHUNK_SIZE);

                // Smoothly interpolate linear gain towards target
                let alpha = 1.0
                    - (-(chunk_len as f32) / (self.sample_rate as f32 * SMOOTHING_TIME_SEC)).exp();
                self.current_linear_gain +=
                    (self.target_linear_gain - self.current_linear_gain) * alpha;

                if (self.current_linear_gain - self.target_linear_gain).abs() < 1e-4 {
                    self.current_linear_gain = self.target_linear_gain;
                }

                let gain = self.current_linear_gain;
                for frame in &mut frames[frame_idx..frame_idx + chunk_len] {
                    frame[0] = flush_denormal(frame[0] * gain);
                    frame[1] = flush_denormal(frame[1] * gain);
                }

                frame_idx += chunk_len;
            }
        } else {
            // Steady state at non-unity gain: auto-vectorizable slice loop
            let gain = self.current_linear_gain;
            for sample in samples.iter_mut() {
                *sample = flush_denormal(*sample * gain);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_preamp_default_initialization() {
        let preamp = PreampStage::new(48000);
        assert_eq!(preamp.gain_db(), 0.0);
        assert_eq!(preamp.current_gain_db(), 0.0);
        assert_eq!(preamp.target_linear_gain(), 1.0);
        assert_eq!(preamp.current_linear_gain(), 1.0);
        assert!(!preamp.is_bypassed());
        assert_eq!(preamp.sample_rate(), 48000);
    }

    #[test]
    fn test_preamp_gain_clamping() {
        let mut preamp = PreampStage::new(48000);

        preamp.set_gain_db(-35.0);
        assert_eq!(preamp.gain_db(), -20.0);

        preamp.set_gain_db(18.0);
        assert_eq!(preamp.gain_db(), 6.0);

        preamp.set_gain_db(-12.5);
        assert_eq!(preamp.gain_db(), -12.5);
    }

    #[test]
    fn test_preamp_linear_multipliers() {
        let mut preamp = PreampStage::new(48000);

        // 0.0 dB -> exactly 1.0
        preamp.set_gain_db_immediate(0.0);
        assert_eq!(preamp.target_linear_gain(), 1.0);
        assert_eq!(preamp.current_linear_gain(), 1.0);

        // -20.0 dB -> 0.10
        preamp.set_gain_db_immediate(-20.0);
        assert!((preamp.target_linear_gain() - 0.10).abs() < 1e-4);
        assert!((preamp.current_linear_gain() - 0.10).abs() < 1e-4);

        // -6.0 dB -> ~0.501187
        preamp.set_gain_db_immediate(-6.0);
        assert!((preamp.target_linear_gain() - 0.501187).abs() < 1e-3);

        // +6.0 dB -> ~1.99526
        preamp.set_gain_db_immediate(6.0);
        assert!((preamp.target_linear_gain() - 1.99526).abs() < 1e-3);
    }

    #[test]
    fn test_preamp_audio_attenuation_and_boost() {
        let mut preamp = PreampStage::new(48000);

        // Attenuate by -6 dB (approx half amplitude)
        preamp.set_gain_db_immediate(-6.0);
        let mut buffer = vec![0.8f32; 64];
        preamp.process_interleaved_stereo(&mut buffer);

        let expected_attenuated = 0.8 * db_to_linear(-6.0);
        for &sample in &buffer {
            assert!((sample - expected_attenuated).abs() < 1e-4);
        }

        // Boost by +6 dB (approx double amplitude)
        preamp.set_gain_db_immediate(6.0);
        let mut buffer = vec![0.3f32; 64];
        preamp.process_interleaved_stereo(&mut buffer);

        let expected_boosted = 0.3 * db_to_linear(6.0);
        for &sample in &buffer {
            assert!((sample - expected_boosted).abs() < 1e-4);
        }
    }

    #[test]
    fn test_preamp_unity_pass_through_optimization() {
        let mut preamp = PreampStage::new(48000);
        preamp.set_gain_db_immediate(0.0);

        let mut buffer = vec![0.42f32; 128];
        let original = buffer.clone();
        preamp.process_interleaved_stereo(&mut buffer);

        assert_eq!(buffer, original);
    }

    #[test]
    fn test_preamp_smooth_gain_interpolation() {
        let mut preamp = PreampStage::new(48000);
        preamp.set_gain_db_immediate(0.0);

        // Move to -12 dB with smooth interpolation
        preamp.set_gain_db(-12.0);
        assert_eq!(preamp.current_linear_gain(), 1.0);
        let target = preamp.target_linear_gain();

        let mut buffer = vec![0.5f32; 256];
        let mut prev_gain = preamp.current_linear_gain();

        for _ in 0..10 {
            preamp.process_interleaved_stereo(&mut buffer);
            let current = preamp.current_linear_gain();
            assert!(
                current <= prev_gain,
                "Gain should ramp monotonically downward: {} vs {}",
                current,
                prev_gain
            );
            prev_gain = current;
        }

        // After processing enough samples (~50 ms), target is reached
        for _ in 0..30 {
            preamp.process_interleaved_stereo(&mut buffer);
        }
        assert!((preamp.current_linear_gain() - target).abs() < 1e-3);
    }

    #[test]
    fn test_preamp_bypass() {
        let mut preamp = PreampStage::new(48000);
        preamp.set_gain_db_immediate(-12.0);
        assert!((preamp.current_linear_gain() - 1.0).abs() > 0.1);

        preamp.set_bypassed(true);
        preamp.reset();
        assert_eq!(preamp.current_linear_gain(), 1.0);

        let mut buffer = vec![0.75f32; 64];
        let original = buffer.clone();
        preamp.process_interleaved_stereo(&mut buffer);
        assert_eq!(buffer, original);

        // Disabling bypass restores target gain
        preamp.set_bypassed(false);
        preamp.reset();
        assert!((preamp.current_linear_gain() - db_to_linear(-12.0)).abs() < 1e-4);
    }

    #[test]
    fn test_preamp_denormal_flush() {
        let mut preamp = PreampStage::new(48000);
        preamp.set_gain_db_immediate(-6.0);

        let mut buffer = vec![1e-38f32; 32];
        preamp.process_interleaved_stereo(&mut buffer);

        for &sample in &buffer {
            assert_eq!(sample, 0.0);
        }
    }
}
