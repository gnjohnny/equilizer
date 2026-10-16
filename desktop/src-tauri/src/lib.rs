#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .run(tauri::generate_context!())
        .expect("error while running equilizer application");
}

pub mod audio;
pub mod storage;

pub mod audio_safety {
    /// Safe conversion from decibels to linear amplitude: 10^(dB / 20)
    pub fn db_to_linear(db: f32) -> f32 {
        if db.is_nan() || db.is_infinite() {
            return 1.0;
        }
        10.0f32.powf(db / 20.0)
    }

    /// Safe conversion from linear amplitude to decibels with -100 dB floor: 20 * log10(max(linear, 1e-5))
    pub fn linear_to_db(linear: f32) -> f32 {
        if linear.is_nan() || linear.is_infinite() || linear <= 0.0 {
            return -100.0;
        }
        let clamped = linear.max(1e-5);
        20.0 * clamped.log10()
    }

    /// Denormal number suppression / flush-to-zero to prevent CPU performance degradation
    pub fn flush_denormal(sample: f32) -> f32 {
        if sample.abs() < 1e-15 || sample.is_nan() || sample.is_infinite() {
            0.0
        } else {
            sample
        }
    }

    /// Clamps audio parameter values to valid operational boundaries
    pub fn clamp_gain_db(gain_db: f32) -> f32 {
        if gain_db.is_nan() {
            0.0
        } else {
            gain_db.clamp(-24.0, 24.0)
        }
    }

    pub fn clamp_frequency_hz(freq_hz: f32) -> f32 {
        if freq_hz.is_nan() {
            1000.0
        } else {
            freq_hz.clamp(20.0, 20000.0)
        }
    }

    pub fn clamp_q(q: f32) -> f32 {
        if q.is_nan() {
            1.0
        } else {
            q.clamp(0.1, 10.0)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::audio_safety::*;

    #[test]
    fn test_db_to_linear_unity() {
        let linear = db_to_linear(0.0);
        assert!((linear - 1.0).abs() < 1e-6);
    }

    #[test]
    fn test_db_to_linear_boost_and_cut() {
        let boost = db_to_linear(6.0206);
        assert!((boost - 2.0).abs() < 1e-3);

        let cut = db_to_linear(-20.0);
        assert!((cut - 0.1).abs() < 1e-4);
    }

    #[test]
    fn test_linear_to_db() {
        let db_unity = linear_to_db(1.0);
        assert!(db_unity.abs() < 1e-6);

        let db_silence = linear_to_db(0.0);
        assert_eq!(db_silence, -100.0);
    }

    #[test]
    fn test_denormal_flush() {
        assert_eq!(flush_denormal(1e-16), 0.0);
        assert_eq!(flush_denormal(-1e-20), 0.0);
        assert_eq!(flush_denormal(0.5), 0.5);
    }

    #[test]
    fn test_parameter_clamps() {
        assert_eq!(clamp_gain_db(30.0), 24.0);
        assert_eq!(clamp_gain_db(-50.0), -24.0);
        assert_eq!(clamp_frequency_hz(10.0), 20.0);
        assert_eq!(clamp_frequency_hz(25000.0), 20000.0);
        assert_eq!(clamp_q(0.01), 0.1);
        assert_eq!(clamp_q(15.0), 10.0);
    }
}
