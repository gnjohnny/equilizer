use std::f32::consts::PI;
use serde::{Deserialize, Serialize};

/// Parametric equalizer filter types supported by the DSP engine.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum FilterType {
    Peaking,
    LowShelf,
    HighShelf,
    HighPass,
    LowPass,
}

/// Configuration parameters for an individual equalizer filter band.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct EqBandConfig {
    pub id: u8,
    pub enabled: bool,
    pub filter_type: FilterType,
    pub frequency_hz: f32,
    pub gain_db: f32,
    pub q: f32,
}

impl Default for EqBandConfig {
    fn default() -> Self {
        Self {
            id: 1,
            enabled: true,
            filter_type: FilterType::Peaking,
            frequency_hz: 1000.0,
            gain_db: 0.0,
            q: 1.0,
        }
    }
}

/// Normalized biquad filter transfer function coefficients:
/// H(z) = (b0 + b1*z^-1 + b2*z^-2) / (1 + a1*z^-1 + a2*z^-2)
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BiquadCoefficients {
    pub b0: f32,
    pub b1: f32,
    pub b2: f32,
    pub a1: f32,
    pub a2: f32,
}

impl Default for BiquadCoefficients {
    fn default() -> Self {
        Self::identity()
    }
}

impl BiquadCoefficients {
    /// Unity pass-through filter (H(z) = 1.0).
    pub fn identity() -> Self {
        Self {
            b0: 1.0,
            b1: 0.0,
            b2: 0.0,
            a1: 0.0,
            a2: 0.0,
        }
    }

    /// Calculates normalized biquad coefficients based on Robert Bristow-Johnson Audio EQ Cookbook formulas.
    pub fn from_config(config: &EqBandConfig, sample_rate: u32) -> Self {
        if sample_rate == 0 {
            return Self::identity();
        }

        let nyquist = sample_rate as f32 / 2.0;
        let f0 = config.frequency_hz.clamp(10.0, nyquist - 50.0);
        let q = config.q.clamp(0.1, 10.0);
        let gain_db = config.gain_db.clamp(-24.0, 24.0);

        // A = 10^(gainDb / 40) = sqrt(10^(gainDb / 20))
        let a = 10.0f32.powf(gain_db / 40.0);
        let w0 = (2.0 * PI * f0) / (sample_rate as f32);
        let cos_w0 = w0.cos();
        let sin_w0 = w0.sin();
        let alpha = sin_w0 / (2.0 * q);

        let (b0, b1, b2, a0, a1, a2) = match config.filter_type {
            FilterType::Peaking => {
                let b0 = 1.0 + alpha * a;
                let b1 = -2.0 * cos_w0;
                let b2 = 1.0 - alpha * a;
                let a0 = 1.0 + alpha / a;
                let a1 = -2.0 * cos_w0;
                let a2 = 1.0 - alpha / a;
                (b0, b1, b2, a0, a1, a2)
            }
            FilterType::LowShelf => {
                let two_sqrt_a_alpha = 2.0 * a.sqrt() * alpha;
                let b0 = a * ((a + 1.0) - (a - 1.0) * cos_w0 + two_sqrt_a_alpha);
                let b1 = 2.0 * a * ((a - 1.0) - (a + 1.0) * cos_w0);
                let b2 = a * ((a + 1.0) - (a - 1.0) * cos_w0 - two_sqrt_a_alpha);
                let a0 = (a + 1.0) + (a - 1.0) * cos_w0 + two_sqrt_a_alpha;
                let a1 = -2.0 * ((a - 1.0) + (a + 1.0) * cos_w0);
                let a2 = (a + 1.0) + (a - 1.0) * cos_w0 - two_sqrt_a_alpha;
                (b0, b1, b2, a0, a1, a2)
            }
            FilterType::HighShelf => {
                let two_sqrt_a_alpha = 2.0 * a.sqrt() * alpha;
                let b0 = a * ((a + 1.0) + (a - 1.0) * cos_w0 + two_sqrt_a_alpha);
                let b1 = -2.0 * a * ((a - 1.0) + (a + 1.0) * cos_w0);
                let b2 = a * ((a + 1.0) + (a - 1.0) * cos_w0 - two_sqrt_a_alpha);
                let a0 = (a + 1.0) - (a - 1.0) * cos_w0 + two_sqrt_a_alpha;
                let a1 = 2.0 * ((a - 1.0) - (a + 1.0) * cos_w0);
                let a2 = (a + 1.0) - (a - 1.0) * cos_w0 - two_sqrt_a_alpha;
                (b0, b1, b2, a0, a1, a2)
            }
            FilterType::LowPass => {
                let b0 = (1.0 - cos_w0) / 2.0;
                let b1 = 1.0 - cos_w0;
                let b2 = (1.0 - cos_w0) / 2.0;
                let a0 = 1.0 + alpha;
                let a1 = -2.0 * cos_w0;
                let a2 = 1.0 - alpha;
                (b0, b1, b2, a0, a1, a2)
            }
            FilterType::HighPass => {
                let b0 = (1.0 + cos_w0) / 2.0;
                let b1 = -(1.0 + cos_w0);
                let b2 = (1.0 + cos_w0) / 2.0;
                let a0 = 1.0 + alpha;
                let a1 = -2.0 * cos_w0;
                let a2 = 1.0 - alpha;
                (b0, b1, b2, a0, a1, a2)
            }
        };

        if a0.abs() < 1e-9 {
            return Self::identity();
        }

        Self {
            b0: b0 / a0,
            b1: b1 / a0,
            b2: b2 / a0,
            a1: a1 / a0,
            a2: a2 / a0,
        }
    }

    /// Evaluates the frequency response |H(e^(jw))| in decibels at a specific test frequency.
    pub fn frequency_response_db(&self, freq_hz: f32, sample_rate: u32) -> f32 {
        let w = (2.0 * PI * freq_hz) / (sample_rate as f32);
        let cos_w = w.cos();
        let sin_w = w.sin();
        let cos_2w = (2.0 * w).cos();
        let sin_2w = (2.0 * w).sin();

        // Numerator real and imaginary components
        let num_re = self.b0 + self.b1 * cos_w + self.b2 * cos_2w;
        let num_im = -self.b1 * sin_w - self.b2 * sin_2w;

        // Denominator real and imaginary components (1 + a1*z^-1 + a2*z^-2)
        let den_re = 1.0 + self.a1 * cos_w + self.a2 * cos_2w;
        let den_im = -self.a1 * sin_w - self.a2 * sin_2w;

        let num_mag_sq = num_re * num_re + num_im * num_im;
        let den_mag_sq = den_re * den_re + den_im * den_im;

        if den_mag_sq < 1e-12 {
            return -100.0;
        }

        let mag_sq = num_mag_sq / den_mag_sq;
        if mag_sq <= 1e-10 {
            return -100.0;
        }

        10.0 * mag_sq.log10()
    }
}

/// Direct Form II Transposed biquad filter channel.
/// Maintains internal state delay registers (s1, s2) with flush-to-zero denormal protection.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct BiquadChannel {
    pub s1: f32,
    pub s2: f32,
}

impl BiquadChannel {
    /// Processes a single input sample through the Direct Form II Transposed difference equations:
    /// y[n]  = b0 * x[n] + s1[n-1]
    /// s1[n] = b1 * x[n] - a1 * y[n] + s2[n-1]
    /// s2[n] = b2 * x[n] - a2 * y[n]
    #[inline(always)]
    pub fn process(&mut self, input: f32, coef: &BiquadCoefficients) -> f32 {
        let out = coef.b0 * input + self.s1;
        self.s1 = crate::audio_safety::flush_denormal(coef.b1 * input - coef.a1 * out + self.s2);
        self.s2 = crate::audio_safety::flush_denormal(coef.b2 * input - coef.a2 * out);
        crate::audio_safety::flush_denormal(out)
    }

    /// Clears delay registers to silence (e.g. on stream pause or discontinuity).
    pub fn reset(&mut self) {
        self.s1 = 0.0;
        self.s2 = 0.0;
    }
}

/// Stereo pair of biquad channels with shared filter coefficients.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct StereoBiquad {
    pub left: BiquadChannel,
    pub right: BiquadChannel,
    pub coef: BiquadCoefficients,
    pub enabled: bool,
}

impl StereoBiquad {
    pub fn new(coef: BiquadCoefficients, enabled: bool) -> Self {
        Self {
            left: BiquadChannel::default(),
            right: BiquadChannel::default(),
            coef,
            enabled,
        }
    }

    #[inline(always)]
    pub fn process_sample(&mut self, left: f32, right: f32) -> (f32, f32) {
        if !self.enabled {
            (left, right)
        } else {
            (
                self.left.process(left, &self.coef),
                self.right.process(right, &self.coef),
            )
        }
    }

    pub fn reset(&mut self) {
        self.left.reset();
        self.right.reset();
    }
}

/// 10-band cascaded Direct Form II Transposed parametric equalizer engine.
pub struct BiquadFilterChain {
    pub bands: [StereoBiquad; 10],
    pub sample_rate: u32,
}

impl BiquadFilterChain {
    /// Standard 10 ISO center frequencies: 31, 63, 125, 250, 500, 1000, 2000, 4000, 8000, 16000 Hz.
    pub const DEFAULT_FREQUENCIES: [f32; 10] = [
        31.0, 63.0, 125.0, 250.0, 500.0, 1000.0, 2000.0, 4000.0, 8000.0, 16000.0,
    ];

    /// Creates a new 10-band equalizer filter chain for the given hardware sample rate.
    pub fn new(sample_rate: u32) -> Self {
        let mut bands: [StereoBiquad; 10] = Default::default();
        for (i, &freq) in Self::DEFAULT_FREQUENCIES.iter().enumerate() {
            let filter_type = if i == 0 {
                FilterType::LowShelf
            } else if i == 9 {
                FilterType::HighShelf
            } else {
                FilterType::Peaking
            };

            let config = EqBandConfig {
                id: (i + 1) as u8,
                enabled: true,
                filter_type,
                frequency_hz: freq,
                gain_db: 0.0, // Flat initial response
                q: 1.0,
            };

            let coef = BiquadCoefficients::from_config(&config, sample_rate);
            bands[i] = StereoBiquad::new(coef, true);
        }

        Self { bands, sample_rate }
    }

    /// Updates coefficients for a specific band index (0 through 9).
    pub fn update_band(&mut self, band_idx: usize, config: &EqBandConfig) {
        if band_idx < self.bands.len() {
            self.bands[band_idx].coef = BiquadCoefficients::from_config(config, self.sample_rate);
            self.bands[band_idx].enabled = config.enabled;
        }
    }

    /// Batch updates all 10 filter bands from a configuration slice.
    pub fn update_all_bands(&mut self, configs: &[EqBandConfig]) {
        for (i, config) in configs.iter().enumerate().take(self.bands.len()) {
            self.update_band(i, config);
        }
    }

    /// Re-computes all active filter coefficients when the hardware sampling rate changes.
    pub fn set_sample_rate(&mut self, sample_rate: u32, configs: &[EqBandConfig]) {
        self.sample_rate = sample_rate;
        self.update_all_bands(configs);
    }

    /// In-place processing of an interleaved stereo sample slice: [L0, R0, L1, R1, ...]
    /// Cascades through all 10 biquad filter bands with zero heap allocations.
    #[inline(always)]
    pub fn process_interleaved_stereo(&mut self, samples: &mut [f32]) {
        let (frames, _) = samples.as_chunks_mut::<2>();
        for frame in frames {
            let mut left = frame[0];
            let mut right = frame[1];

            for band in self.bands.iter_mut() {
                if band.enabled {
                    left = band.left.process(left, &band.coef);
                    right = band.right.process(right, &band.coef);
                }
            }

            frame[0] = left;
            frame[1] = right;
        }
    }

    /// Clears delay registers in all filter stages to prevent clicks or residual energy.
    pub fn reset(&mut self) {
        for band in self.bands.iter_mut() {
            band.reset();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_identity_filter() {
        let coef = BiquadCoefficients::identity();
        let mut ch = BiquadChannel::default();

        let samples = [0.1f32, -0.5, 0.8, -0.2];
        for &s in &samples {
            let out = ch.process(s, &coef);
            assert_eq!(out, s);
        }
    }

    #[test]
    fn test_peaking_filter_response() {
        let sample_rate = 48000;
        let config = EqBandConfig {
            id: 1,
            enabled: true,
            filter_type: FilterType::Peaking,
            frequency_hz: 1000.0,
            gain_db: 6.0,
            q: 1.0,
        };

        let coef = BiquadCoefficients::from_config(&config, sample_rate);

        // At center frequency (1000 Hz), response should be approximately +6.0 dB
        let resp_center = coef.frequency_response_db(1000.0, sample_rate);
        assert!((resp_center - 6.0).abs() < 0.1, "Center resp: {}", resp_center);

        // Far away from center frequency (e.g. 50 Hz and 18,000 Hz), response should be near 0 dB
        let resp_low = coef.frequency_response_db(50.0, sample_rate);
        let resp_high = coef.frequency_response_db(18000.0, sample_rate);
        assert!(resp_low.abs() < 0.2, "Low resp: {}", resp_low);
        assert!(resp_high.abs() < 0.2, "High resp: {}", resp_high);
    }

    #[test]
    fn test_low_shelf_filter_response() {
        let sample_rate = 48000;
        let config = EqBandConfig {
            id: 1,
            enabled: true,
            filter_type: FilterType::LowShelf,
            frequency_hz: 100.0,
            gain_db: 8.0,
            q: 0.707,
        };

        let coef = BiquadCoefficients::from_config(&config, sample_rate);

        // Well below shelf frequency (e.g. 30 Hz), gain should be ~ +8.0 dB
        let resp_low = coef.frequency_response_db(30.0, sample_rate);
        assert!((resp_low - 8.0).abs() < 0.5, "Low shelf resp: {}", resp_low);

        // Well above shelf frequency (e.g. 5000 Hz), gain should be near 0 dB
        let resp_high = coef.frequency_response_db(5000.0, sample_rate);
        assert!(resp_high.abs() < 0.2, "High shelf passband resp: {}", resp_high);
    }

    #[test]
    fn test_high_shelf_filter_response() {
        let sample_rate = 48000;
        let config = EqBandConfig {
            id: 1,
            enabled: true,
            filter_type: FilterType::HighShelf,
            frequency_hz: 8000.0,
            gain_db: 5.0,
            q: 0.707,
        };

        let coef = BiquadCoefficients::from_config(&config, sample_rate);

        // Well above shelf frequency (e.g. 15000 Hz), gain should be ~ +5.0 dB
        let resp_high = coef.frequency_response_db(15000.0, sample_rate);
        assert!((resp_high - 5.0).abs() < 0.5, "High shelf resp: {}", resp_high);

        // Well below shelf frequency (e.g. 500 Hz), gain should be near 0 dB
        let resp_low = coef.frequency_response_db(500.0, sample_rate);
        assert!(resp_low.abs() < 0.2, "High shelf passband resp: {}", resp_low);
    }

    #[test]
    fn test_low_pass_filter_attenuation() {
        let sample_rate = 48000;
        let config = EqBandConfig {
            id: 1,
            enabled: true,
            filter_type: FilterType::LowPass,
            frequency_hz: 1000.0,
            gain_db: 0.0,
            q: 0.707,
        };

        let coef = BiquadCoefficients::from_config(&config, sample_rate);

        // Passband (e.g. 100 Hz): near 0 dB
        let resp_pass = coef.frequency_response_db(100.0, sample_rate);
        assert!(resp_pass.abs() < 0.2, "Passband resp: {}", resp_pass);

        // Cutoff (-3 dB for Butterworth Q=0.707)
        let resp_cutoff = coef.frequency_response_db(1000.0, sample_rate);
        assert!((resp_cutoff - (-3.01)).abs() < 0.3, "Cutoff resp: {}", resp_cutoff);

        // Stopband (e.g. 8000 Hz): heavily attenuated
        let resp_stop = coef.frequency_response_db(8000.0, sample_rate);
        assert!(resp_stop < -30.0, "Stopband resp: {}", resp_stop);
    }

    #[test]
    fn test_high_pass_filter_attenuation() {
        let sample_rate = 48000;
        let config = EqBandConfig {
            id: 1,
            enabled: true,
            filter_type: FilterType::HighPass,
            frequency_hz: 1000.0,
            gain_db: 0.0,
            q: 0.707,
        };

        let coef = BiquadCoefficients::from_config(&config, sample_rate);

        // Stopband (e.g. 100 Hz): heavily attenuated
        let resp_stop = coef.frequency_response_db(100.0, sample_rate);
        assert!(resp_stop < -30.0, "Stopband resp: {}", resp_stop);

        // Cutoff (-3 dB for Butterworth Q=0.707)
        let resp_cutoff = coef.frequency_response_db(1000.0, sample_rate);
        assert!((resp_cutoff - (-3.01)).abs() < 0.3, "Cutoff resp: {}", resp_cutoff);

        // Passband (e.g. 10,000 Hz): near 0 dB
        let resp_pass = coef.frequency_response_db(10000.0, sample_rate);
        assert!(resp_pass.abs() < 0.2, "Passband resp: {}", resp_pass);
    }

    #[test]
    fn test_cascaded_biquad_chain_stereo_processing() {
        let sample_rate = 48000;
        let mut chain = BiquadFilterChain::new(sample_rate);

        // Flat initial response should leave audio virtually unaltered
        let mut audio = vec![0.5f32, -0.2f32, 0.1f32, -0.4f32];
        let original = audio.clone();
        chain.process_interleaved_stereo(&mut audio);

        for (a, o) in audio.iter().zip(original.iter()) {
            assert!((a - o).abs() < 1e-4);
        }

        // Apply a boost on band 5 (1000 Hz)
        let boost_config = EqBandConfig {
            id: 6,
            enabled: true,
            filter_type: FilterType::Peaking,
            frequency_hz: 1000.0,
            gain_db: 12.0,
            q: 1.0,
        };
        chain.update_band(5, &boost_config);

        // Feed 1000 Hz sine wave through and verify amplification
        let mut sine_signal = Vec::new();
        for n in 0..480 {
            let t = n as f32 / sample_rate as f32;
            let val = (2.0 * PI * 1000.0 * t).sin();
            sine_signal.push(val); // Left
            sine_signal.push(val); // Right
        }

        chain.process_interleaved_stereo(&mut sine_signal);

        // In steady state (end of buffer), peak amplitude should be ~4.0 (12 dB boost is 4x linear)
        let max_val = sine_signal[800..]
            .iter()
            .cloned()
            .fold(0.0f32, |a, b| a.max(b.abs()));
        assert!((max_val - 3.98).abs() < 0.2, "Max amplitude after 12dB boost: {}", max_val);
    }
}
