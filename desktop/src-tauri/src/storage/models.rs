use serde::{Deserialize, Serialize};

/// Supported parametric equalizer biquad filter topologies.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum FilterType {
    Peaking,
    LowShelf,
    HighShelf,
    HighPass,
    LowPass,
}

impl From<crate::audio::dsp::FilterType> for FilterType {
    fn from(ft: crate::audio::dsp::FilterType) -> Self {
        match ft {
            crate::audio::dsp::FilterType::Peaking => FilterType::Peaking,
            crate::audio::dsp::FilterType::LowShelf => FilterType::LowShelf,
            crate::audio::dsp::FilterType::HighShelf => FilterType::HighShelf,
            crate::audio::dsp::FilterType::HighPass => FilterType::HighPass,
            crate::audio::dsp::FilterType::LowPass => FilterType::LowPass,
        }
    }
}

impl From<FilterType> for crate::audio::dsp::FilterType {
    fn from(ft: FilterType) -> Self {
        match ft {
            FilterType::Peaking => crate::audio::dsp::FilterType::Peaking,
            FilterType::LowShelf => crate::audio::dsp::FilterType::LowShelf,
            FilterType::HighShelf => crate::audio::dsp::FilterType::HighShelf,
            FilterType::HighPass => crate::audio::dsp::FilterType::HighPass,
            FilterType::LowPass => crate::audio::dsp::FilterType::LowPass,
        }
    }
}

/// Dedicated bass boost operating mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum BassBoostMode {
    SubBass,
    Punch,
}

impl From<crate::audio::bass_boost::BassBoostMode> for BassBoostMode {
    fn from(mode: crate::audio::bass_boost::BassBoostMode) -> Self {
        match mode {
            crate::audio::bass_boost::BassBoostMode::SubBass => BassBoostMode::SubBass,
            crate::audio::bass_boost::BassBoostMode::Punch => BassBoostMode::Punch,
        }
    }
}

impl From<BassBoostMode> for crate::audio::bass_boost::BassBoostMode {
    fn from(mode: BassBoostMode) -> Self {
        match mode {
            BassBoostMode::SubBass => crate::audio::bass_boost::BassBoostMode::SubBass,
            BassBoostMode::Punch => crate::audio::bass_boost::BassBoostMode::Punch,
        }
    }
}

/// Single equalizer band configuration for JSON persistence.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EqBand {
    pub id: u8,
    pub enabled: bool,
    pub filter_type: FilterType,
    pub frequency_hz: f32,
    pub gain_db: f32,
    pub q: f32,
}

impl From<crate::audio::dsp::EqBandConfig> for EqBand {
    fn from(b: crate::audio::dsp::EqBandConfig) -> Self {
        Self {
            id: b.id,
            enabled: b.enabled,
            filter_type: b.filter_type.into(),
            frequency_hz: b.frequency_hz,
            gain_db: b.gain_db,
            q: b.q,
        }
    }
}

impl From<EqBand> for crate::audio::dsp::EqBandConfig {
    fn from(b: EqBand) -> Self {
        Self {
            id: b.id,
            enabled: b.enabled,
            filter_type: b.filter_type.into(),
            frequency_hz: b.frequency_hz,
            gain_db: b.gain_db,
            q: b.q,
        }
    }
}

/// Bass boost configuration for JSON persistence.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
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

impl From<crate::audio::bass_boost::BassBoostConfig> for BassBoostConfig {
    fn from(b: crate::audio::bass_boost::BassBoostConfig) -> Self {
        Self {
            enabled: b.enabled,
            gain_db: b.gain_db,
            mode: b.mode.into(),
            frequency_hz: b.frequency_hz,
        }
    }
}

impl From<BassBoostConfig> for crate::audio::bass_boost::BassBoostConfig {
    fn from(b: BassBoostConfig) -> Self {
        Self {
            enabled: b.enabled,
            gain_db: b.gain_db,
            mode: b.mode.into(),
            frequency_hz: b.frequency_hz,
        }
    }
}

/// Equalizer Preset entity matching the frontend data model.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EqualizerPreset {
    pub id: String,
    pub name: String,
    pub is_factory: bool,
    pub preamp_gain_db: f32,
    pub bass_boost: BassBoostConfig,
    pub bands: Vec<EqBand>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub created_at: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub updated_at: Option<String>,
}

/// Application settings entity matching the frontend configuration.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AppConfig {
    pub active_preset_id: String,
    pub selected_device_id: Option<String>,
    pub is_bypassed: bool,
    pub limiter_enabled: bool,
    pub start_minimized_to_tray: bool,
    pub launch_on_startup: bool,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            active_preset_id: "flat".to_string(),
            selected_device_id: None,
            is_bypassed: false,
            limiter_enabled: true,
            start_minimized_to_tray: false,
            launch_on_startup: false,
        }
    }
}

/// Standard 10 ISO band center frequencies matching the frontend curve editor.
pub const DEFAULT_BAND_FREQUENCIES: [f32; 10] = [
    31.25, 62.5, 125.0, 250.0, 500.0, 1000.0, 2000.0, 4000.0, 8000.0, 16000.0,
];

/// Protected factory preset IDs that cannot be renamed, overwritten, or deleted by users.
pub const FACTORY_PRESET_IDS: [&str; 5] = [
    "flat",
    "deep-bass",
    "punchy-gaming",
    "crisp-vocals",
    "rock",
];

/// Generates standard 10 equalizer bands with specified gain values.
pub fn create_default_bands(gains: &[f32; 10]) -> Vec<EqBand> {
    DEFAULT_BAND_FREQUENCIES
        .iter()
        .enumerate()
        .map(|(index, &freq)| {
            let filter_type = if index == 0 {
                FilterType::LowShelf
            } else if index == 9 {
                FilterType::HighShelf
            } else {
                FilterType::Peaking
            };

            EqBand {
                id: (index + 1) as u8,
                enabled: true,
                filter_type,
                frequency_hz: freq,
                gain_db: gains[index],
                q: 1.0,
            }
        })
        .collect()
}

/// Returns the 5 bundled factory presets matching `desktop/src/types/audio.ts`.
pub fn default_factory_presets() -> Vec<EqualizerPreset> {
    vec![
        EqualizerPreset {
            id: "flat".to_string(),
            name: "Flat".to_string(),
            is_factory: true,
            preamp_gain_db: 0.0,
            bass_boost: BassBoostConfig {
                enabled: false,
                gain_db: 0.0,
                mode: BassBoostMode::SubBass,
                frequency_hz: 45.0,
            },
            bands: create_default_bands(&[0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0]),
            created_at: None,
            updated_at: None,
        },
        EqualizerPreset {
            id: "deep-bass".to_string(),
            name: "Deep Bass".to_string(),
            is_factory: true,
            preamp_gain_db: -3.0,
            bass_boost: BassBoostConfig {
                enabled: true,
                gain_db: 6.0,
                mode: BassBoostMode::SubBass,
                frequency_hz: 45.0,
            },
            bands: create_default_bands(&[6.5, 5.0, 3.5, 1.5, 0.0, 0.0, 0.5, 1.0, 1.5, 2.0]),
            created_at: None,
            updated_at: None,
        },
        EqualizerPreset {
            id: "punchy-gaming".to_string(),
            name: "Punchy Gaming".to_string(),
            is_factory: true,
            preamp_gain_db: -2.0,
            bass_boost: BassBoostConfig {
                enabled: true,
                gain_db: 4.5,
                mode: BassBoostMode::Punch,
                frequency_hz: 90.0,
            },
            bands: create_default_bands(&[4.0, 4.5, 2.0, -1.0, 1.0, 2.5, 4.0, 3.0, 2.0, 1.0]),
            created_at: None,
            updated_at: None,
        },
        EqualizerPreset {
            id: "crisp-vocals".to_string(),
            name: "Crisp Vocals".to_string(),
            is_factory: true,
            preamp_gain_db: -1.0,
            bass_boost: BassBoostConfig {
                enabled: false,
                gain_db: 0.0,
                mode: BassBoostMode::Punch,
                frequency_hz: 90.0,
            },
            bands: create_default_bands(&[-2.5, -2.0, -1.0, 0.5, 2.0, 3.5, 4.0, 3.0, 2.5, 1.5]),
            created_at: None,
            updated_at: None,
        },
        EqualizerPreset {
            id: "rock".to_string(),
            name: "Rock".to_string(),
            is_factory: true,
            preamp_gain_db: -2.5,
            bass_boost: BassBoostConfig {
                enabled: true,
                gain_db: 3.0,
                mode: BassBoostMode::Punch,
                frequency_hz: 90.0,
            },
            bands: create_default_bands(&[4.5, 3.5, 1.5, -1.0, -0.5, 1.0, 2.5, 4.0, 4.5, 4.0]),
            created_at: None,
            updated_at: None,
        },
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_factory_presets_integrity() {
        let presets = default_factory_presets();
        assert_eq!(presets.len(), 5);

        for p in &presets {
            assert!(p.is_factory);
            assert!(FACTORY_PRESET_IDS.contains(&p.id.as_str()));
            assert_eq!(p.bands.len(), 10);
            assert!(p.preamp_gain_db <= 6.0 && p.preamp_gain_db >= -20.0);
            assert!(p.bass_boost.gain_db >= 0.0 && p.bass_boost.gain_db <= 12.0);
        }
    }

    #[test]
    fn test_serde_json_roundtrip() {
        let presets = default_factory_presets();
        let json = serde_json::to_string_pretty(&presets).expect("Serialization failed");

        // Verify camelCase serialization
        assert!(json.contains("\"isFactory\": true"));
        assert!(json.contains("\"preampGainDb\""));
        assert!(json.contains("\"bassBoost\""));
        assert!(json.contains("\"frequencyHz\""));
        assert!(json.contains("\"filterType\""));

        let deserialized: Vec<EqualizerPreset> =
            serde_json::from_str(&json).expect("Deserialization failed");
        assert_eq!(presets, deserialized);
    }

    #[test]
    fn test_app_config_defaults() {
        let cfg = AppConfig::default();
        assert_eq!(cfg.active_preset_id, "flat");
        assert_eq!(cfg.selected_device_id, None);
        assert!(!cfg.is_bypassed);
        assert!(cfg.limiter_enabled);
        assert!(!cfg.start_minimized_to_tray);
        assert!(!cfg.launch_on_startup);

        let json = serde_json::to_string(&cfg).expect("Config serialization failed");
        assert!(json.contains("\"activePresetId\":\"flat\""));
        assert!(json.contains("\"limiterEnabled\":true"));
    }
}
