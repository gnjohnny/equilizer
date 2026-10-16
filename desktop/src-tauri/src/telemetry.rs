use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;
use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter, Manager};

use crate::audio_safety::linear_to_db;
use crate::state::AppState;

/// Tauri event channel name for audio meter updates.
pub const TELEMETRY_EVENT_NAME: &str = "audio:meter-update";

/// Minimum decibel floor for audio meter displays (-48.0 dBFS).
pub const TELEMETRY_MIN_DB: f32 = -48.0;

/// Maximum decibel ceiling for audio meter displays (+6.0 dBFS).
pub const TELEMETRY_MAX_DB: f32 = 6.0;

/// Structured stereo peak meter and limiter status payload.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AudioMeterPayload {
    /// Left channel peak level in decibels (-48.0 to +6.0 dBFS).
    pub peak_l: f32,
    /// Right channel peak level in decibels (-48.0 to +6.0 dBFS).
    pub peak_r: f32,
    /// True if the soft-knee limiter is currently attenuating signal.
    pub limiter_active: bool,
}

impl Default for AudioMeterPayload {
    fn default() -> Self {
        Self {
            peak_l: TELEMETRY_MIN_DB,
            peak_r: TELEMETRY_MIN_DB,
            limiter_active: false,
        }
    }
}

/// Calibrates linear peak amplitude ($0.0 \dots 1.0+$) to decibels (-48.0 to +6.0 dBFS).
#[inline]
pub fn linear_peak_to_meter_db(linear: f32) -> f32 {
    let db = linear_to_db(linear);
    db.clamp(TELEMETRY_MIN_DB, TELEMETRY_MAX_DB)
}

/// Spawns the throttled 30 Hz audio telemetry background emitter thread.
/// Runs in a dedicated background OS thread sleeping for 33 ms between ticks,
/// ensuring zero real-time audio thread contention and minimal CPU impact.
pub fn spawn_telemetry_emitter(
    app_handle: AppHandle,
    is_running: Arc<AtomicBool>,
) -> std::thread::JoinHandle<()> {
    std::thread::Builder::new()
        .name("equilizer-telemetry-emitter".to_string())
        .spawn(move || {
            while is_running.load(Ordering::Relaxed) {
                std::thread::sleep(Duration::from_millis(33));

                let payload = if let Some(state) = app_handle.try_state::<AppState>() {
                    let snapshot = state.telemetry_snapshot();
                    AudioMeterPayload {
                        peak_l: linear_peak_to_meter_db(snapshot.peak_l),
                        peak_r: linear_peak_to_meter_db(snapshot.peak_r),
                        limiter_active: snapshot.limiter_active,
                    }
                } else {
                    AudioMeterPayload::default()
                };

                // Emit throttled 30 Hz event over Tauri's event bus
                let _ = app_handle.emit(TELEMETRY_EVENT_NAME, payload);
            }
        })
        .expect("Failed to spawn telemetry emitter background thread")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_linear_peak_to_meter_db_calibration() {
        // Unity amplitude (1.0) must map to 0.0 dBFS
        let unity = linear_peak_to_meter_db(1.0);
        assert!(unity.abs() < 1e-4, "Expected ~0.0 dBFS, got {}", unity);

        // Half amplitude (0.5) must map to ~ -6.02 dBFS
        let half = linear_peak_to_meter_db(0.5);
        assert!((half - (-6.0206)).abs() < 1e-2, "Expected ~ -6.02 dBFS, got {}", half);

        // Complete silence (0.0) must map to clamped floor -48.0 dBFS
        let silence = linear_peak_to_meter_db(0.0);
        assert_eq!(silence, TELEMETRY_MIN_DB);

        // Very low signal below floor must map to -48.0 dBFS
        let very_low = linear_peak_to_meter_db(0.001);
        assert_eq!(very_low, TELEMETRY_MIN_DB);

        // Extreme signal above unity (2.0 = +6.02 dB) must clamp to +6.0 dBFS
        let high = linear_peak_to_meter_db(2.0);
        assert_eq!(high, TELEMETRY_MAX_DB);
    }

    #[test]
    fn test_audio_meter_payload_camel_case_serialization() {
        let payload = AudioMeterPayload {
            peak_l: -12.5,
            peak_r: -10.2,
            limiter_active: true,
        };

        let json = serde_json::to_string(&payload).expect("Failed to serialize AudioMeterPayload");
        assert!(json.contains("\"peakL\":-12.5"));
        assert!(json.contains("\"peakR\":-10.2"));
        assert!(json.contains("\"limiterActive\":true"));

        let deserialized: AudioMeterPayload =
            serde_json::from_str(&json).expect("Failed to deserialize AudioMeterPayload");
        assert_eq!(payload, deserialized);
    }

    #[test]
    fn test_audio_meter_payload_default() {
        let def = AudioMeterPayload::default();
        assert_eq!(def.peak_l, TELEMETRY_MIN_DB);
        assert_eq!(def.peak_r, TELEMETRY_MIN_DB);
        assert!(!def.limiter_active);
    }
}
