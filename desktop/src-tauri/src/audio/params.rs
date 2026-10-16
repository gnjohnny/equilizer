use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::Arc;
use crossbeam_channel::{bounded, Receiver, Sender, TrySendError};
use serde::{Deserialize, Serialize};

use crate::audio::bass_boost::{BassBoostConfig, BassBoostEngine};
use crate::audio::dsp::{BiquadFilterChain, EqBandConfig};
use crate::audio::limiter::{Limiter, LimiterConfig};
use crate::audio::preamp::PreampStage;

/// Wait-free, single-cycle atomic 32-bit floating point primitive.
/// Implements lock-free atomic load and store operations via IEEE-754 bitwise transmutation.
#[derive(Debug)]
pub struct AtomicF32 {
    inner: AtomicU32,
}

impl Default for AtomicF32 {
    fn default() -> Self {
        Self::new(0.0)
    }
}

impl AtomicF32 {
    /// Creates a new atomic float initialized to the given value.
    pub fn new(val: f32) -> Self {
        Self {
            inner: AtomicU32::new(val.to_bits()),
        }
    }

    /// Atomically loads the current float value with the given memory ordering.
    #[inline(always)]
    pub fn load(&self, order: Ordering) -> f32 {
        f32::from_bits(self.inner.load(order))
    }

    /// Atomically stores a float value with the given memory ordering.
    #[inline(always)]
    pub fn store(&self, val: f32, order: Ordering) {
        self.inner.store(val.to_bits(), order);
    }
}

/// Structured snapshot of audio telemetry for IPC transmission to the UI.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TelemetrySnapshot {
    pub peak_l: f32,
    pub peak_r: f32,
    pub limiter_active: bool,
    pub gain_reduction_db: f32,
}

/// Lock-free audio telemetry container updated by the audio callback thread
/// and queried at ~30 Hz by the UI telemetry loop.
#[derive(Debug, Default)]
pub struct AudioTelemetry {
    pub peak_l: AtomicF32,
    pub peak_r: AtomicF32,
    pub limiter_active: AtomicBool,
    pub gain_reduction_db: AtomicF32,
}

impl AudioTelemetry {
    /// Creates a new zero-initialized audio telemetry instance.
    pub fn new() -> Self {
        Self::default()
    }

    /// Returns a snapshot of the current telemetry values.
    #[inline]
    pub fn snapshot(&self) -> TelemetrySnapshot {
        TelemetrySnapshot {
            peak_l: self.peak_l.load(Ordering::Relaxed),
            peak_r: self.peak_r.load(Ordering::Relaxed),
            limiter_active: self.limiter_active.load(Ordering::Relaxed),
            gain_reduction_db: self.gain_reduction_db.load(Ordering::Relaxed),
        }
    }
}

/// Parameter update commands sent from Tauri IPC threads to the real-time audio thread.
#[derive(Debug, Clone, PartialEq)]
pub enum DspCommand {
    /// Toggle global master bypass (raw audio pass-through vs. equalized).
    SetBypass(bool),
    /// Set master pre-amp gain in decibels (-20 dB to +6 dB).
    SetPreampGain(f32),
    /// Update dedicated bass boost configuration.
    SetBassBoost(BassBoostConfig),
    /// Update soft-knee brickwall limiter configuration.
    SetLimiter(LimiterConfig),
    /// Update an individual parametric filter band (0 through 9).
    UpdateBand(usize, EqBandConfig),
    /// Atomically update all 10 filter bands at once (e.g. preset change).
    UpdateAllBands([EqBandConfig; 10]),
    /// Clear all internal delay registers and state to silence.
    ResetState,
}

/// Error condition encountered when attempting to send a parameter command to the audio thread.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SendError {
    /// The bounded command channel queue is full.
    Full,
    /// The audio thread receiver has been disconnected/dropped.
    Disconnected,
}

impl std::fmt::Display for SendError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SendError::Full => write!(f, "DSP command channel queue is full"),
            SendError::Disconnected => write!(f, "DSP command channel is disconnected"),
        }
    }
}

impl std::error::Error for SendError {}

impl<T> From<TrySendError<T>> for SendError {
    fn from(err: TrySendError<T>) -> Self {
        match err {
            TrySendError::Full(_) => SendError::Full,
            TrySendError::Disconnected(_) => SendError::Disconnected,
        }
    }
}

/// SPSC command sender used by Tauri IPC threads to push parameter changes.
#[derive(Clone)]
pub struct DspParamSender {
    sender: Sender<DspCommand>,
}

impl DspParamSender {
    /// Attempts to enqueue a DSP parameter update command without blocking.
    #[inline]
    pub fn send(&self, cmd: DspCommand) -> Result<(), SendError> {
        self.sender.try_send(cmd).map_err(SendError::from)
    }

    /// Updates global master bypass.
    pub fn set_bypass(&self, bypass: bool) -> Result<(), SendError> {
        self.send(DspCommand::SetBypass(bypass))
    }

    /// Updates master pre-amp gain.
    pub fn set_preamp_gain(&self, gain_db: f32) -> Result<(), SendError> {
        self.send(DspCommand::SetPreampGain(gain_db))
    }

    /// Updates dedicated bass boost module.
    pub fn set_bass_boost(&self, config: BassBoostConfig) -> Result<(), SendError> {
        self.send(DspCommand::SetBassBoost(config))
    }

    /// Updates soft-knee limiter configuration.
    pub fn set_limiter(&self, config: LimiterConfig) -> Result<(), SendError> {
        self.send(DspCommand::SetLimiter(config))
    }

    /// Updates a single equalizer band.
    pub fn update_band(&self, index: usize, config: EqBandConfig) -> Result<(), SendError> {
        self.send(DspCommand::UpdateBand(index, config))
    }

    /// Updates all 10 equalizer bands atomically.
    pub fn update_all_bands(&self, bands: [EqBandConfig; 10]) -> Result<(), SendError> {
        self.send(DspCommand::UpdateAllBands(bands))
    }

    /// Resets all filter delay states to silence.
    pub fn reset_state(&self) -> Result<(), SendError> {
        self.send(DspCommand::ResetState)
    }
}

/// SPSC command receiver held by the audio processing thread.
pub struct DspParamReceiver {
    receiver: Receiver<DspCommand>,
}

/// Creates a paired lock-free bounded parameter exchange channel.
pub fn create_param_channel(capacity: usize) -> (DspParamSender, DspParamReceiver) {
    let (sender, receiver) = bounded(capacity);
    (DspParamSender { sender }, DspParamReceiver { receiver })
}

/// Unified real-time DSP processor executing the full signal chain in-place on audio buffers:
/// Pre-Amp -> 10-Band Biquad EQ -> Bass Boost -> Soft-Knee Limiter -> Peak Telemetry.
pub struct DspProcessor {
    sample_rate: u32,
    is_bypassed: bool,
    preamp: PreampStage,
    eq_chain: BiquadFilterChain,
    bass_boost: BassBoostEngine,
    limiter: Limiter,
    receiver: DspParamReceiver,
    telemetry: Arc<AudioTelemetry>,
}

impl DspProcessor {
    /// Default channel queue capacity: 128 commands.
    pub const DEFAULT_CHANNEL_CAPACITY: usize = 128;

    /// Creates a new DSP processor instance initialized for the hardware sample rate.
    pub fn new(
        sample_rate: u32,
        receiver: DspParamReceiver,
        telemetry: Arc<AudioTelemetry>,
    ) -> Self {
        Self {
            sample_rate,
            is_bypassed: false,
            preamp: PreampStage::new(sample_rate),
            eq_chain: BiquadFilterChain::new(sample_rate),
            bass_boost: BassBoostEngine::new(sample_rate),
            limiter: Limiter::new(sample_rate),
            receiver,
            telemetry,
        }
    }

    /// Drains any pending parameter commands from the SPSC channel.
    /// Non-blocking, zero heap allocations.
    #[inline(always)]
    pub fn drain_commands(&mut self) {
        while let Ok(cmd) = self.receiver.receiver.try_recv() {
            match cmd {
                DspCommand::SetBypass(bypassed) => {
                    self.is_bypassed = bypassed;
                }
                DspCommand::SetPreampGain(gain_db) => {
                    self.preamp.set_gain_db(gain_db);
                }
                DspCommand::SetBassBoost(config) => {
                    self.bass_boost.set_config(config);
                }
                DspCommand::SetLimiter(config) => {
                    self.limiter.set_config(config);
                }
                DspCommand::UpdateBand(idx, config) => {
                    self.eq_chain.update_band(idx, &config);
                }
                DspCommand::UpdateAllBands(bands) => {
                    self.eq_chain.update_all_bands(&bands);
                }
                DspCommand::ResetState => {
                    self.preamp.reset();
                    self.bass_boost.reset();
                    self.limiter.reset();
                    for band in &mut self.eq_chain.bands {
                        band.reset();
                    }
                }
            }
        }
    }

    /// In-place processing of an interleaved stereo sample slice: `[L0, R0, L1, R1, ...]`.
    ///
    /// Executes the full 4-stage DSP chain and updates atomic telemetry:
    /// 1. Drains parameter commands non-blockingly.
    /// 2. If bypassed, passes audio through untouched and updates input peak meters.
    /// 3. Otherwise: Pre-Amp -> EQ Chain -> Bass Boost -> Limiter.
    /// 4. Updates Left/Right peak amplitudes and limiter status.
    #[inline(always)]
    pub fn process_interleaved_stereo(&mut self, samples: &mut [f32]) {
        self.drain_commands();

        if samples.is_empty() {
            return;
        }

        if self.is_bypassed {
            // Bypass mode: raw pass-through, compute input peak meters for UI responsiveness
            let mut peak_l = 0.0f32;
            let mut peak_r = 0.0f32;
            let (frames, _) = samples.as_chunks::<2>();
            for frame in frames {
                let l = frame[0].abs();
                let r = frame[1].abs();
                if l > peak_l { peak_l = l; }
                if r > peak_r { peak_r = r; }
            }

            self.telemetry.peak_l.store(peak_l, Ordering::Relaxed);
            self.telemetry.peak_r.store(peak_r, Ordering::Relaxed);
            self.telemetry.limiter_active.store(false, Ordering::Relaxed);
            self.telemetry.gain_reduction_db.store(0.0, Ordering::Relaxed);
            return;
        }

        // 1. Master Pre-Amp attenuation/gain stage
        self.preamp.process_interleaved_stereo(samples);

        // 2. 10-band cascaded Direct Form II Transposed biquad equalizer
        self.eq_chain.process_interleaved_stereo(samples);

        // 3. Dedicated Bass Boost engine
        self.bass_boost.process_interleaved_stereo(samples);

        // 4. Soft-knee brickwall limiter
        self.limiter.process_interleaved_stereo(samples);

        // Compute output peak levels for Left and Right channels
        let mut peak_l = 0.0f32;
        let mut peak_r = 0.0f32;
        let (frames, _) = samples.as_chunks::<2>();
        for frame in frames {
            let l = frame[0].abs();
            let r = frame[1].abs();
            if l > peak_l { peak_l = l; }
            if r > peak_r { peak_r = r; }
        }

        self.telemetry.peak_l.store(peak_l, Ordering::Relaxed);
        self.telemetry.peak_r.store(peak_r, Ordering::Relaxed);
        self.telemetry.limiter_active.store(self.limiter.is_active(), Ordering::Relaxed);
        self.telemetry.gain_reduction_db.store(self.limiter.current_gain_reduction_db(), Ordering::Relaxed);
    }

    /// Accessors for testing and diagnostics.
    #[inline]
    pub fn is_bypassed(&self) -> bool {
        self.is_bypassed
    }

    #[inline]
    pub fn sample_rate(&self) -> u32 {
        self.sample_rate
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::audio::bass_boost::BassBoostMode;
    use crate::audio::dsp::FilterType;

    #[test]
    fn test_atomic_f32_load_store() {
        let val = AtomicF32::new(3.14159);
        assert!((val.load(Ordering::SeqCst) - 3.14159).abs() < 1e-5);

        val.store(-42.5, Ordering::SeqCst);
        assert_eq!(val.load(Ordering::SeqCst), -42.5);

        val.store(0.0, Ordering::SeqCst);
        assert_eq!(val.load(Ordering::SeqCst), 0.0);
    }

    #[test]
    fn test_telemetry_snapshot() {
        let telemetry = AudioTelemetry::new();
        assert_eq!(telemetry.peak_l.load(Ordering::Relaxed), 0.0);
        assert_eq!(telemetry.peak_r.load(Ordering::Relaxed), 0.0);
        assert!(!telemetry.limiter_active.load(Ordering::Relaxed));

        telemetry.peak_l.store(0.85, Ordering::Relaxed);
        telemetry.peak_r.store(0.92, Ordering::Relaxed);
        telemetry.limiter_active.store(true, Ordering::Relaxed);
        telemetry.gain_reduction_db.store(2.4, Ordering::Relaxed);

        let snap = telemetry.snapshot();
        assert_eq!(snap.peak_l, 0.85);
        assert_eq!(snap.peak_r, 0.92);
        assert!(snap.limiter_active);
        assert_eq!(snap.gain_reduction_db, 2.4);
    }

    #[test]
    fn test_dsp_channel_send_and_drain() {
        let (sender, receiver) = create_param_channel(16);
        let telemetry = Arc::new(AudioTelemetry::new());
        let mut dsp = DspProcessor::new(48000, receiver, telemetry);

        assert!(!dsp.is_bypassed());

        sender.set_bypass(true).unwrap();
        sender.set_preamp_gain(-6.0).unwrap();
        sender.set_bass_boost(BassBoostConfig {
            enabled: true,
            gain_db: 9.0,
            mode: BassBoostMode::Punch,
            frequency_hz: 90.0,
        }).unwrap();

        dsp.drain_commands();
        assert!(dsp.is_bypassed());
        assert_eq!(dsp.preamp.gain_db(), -6.0);
        assert_eq!(dsp.bass_boost.gain_db(), 9.0);
        assert_eq!(dsp.bass_boost.mode(), BassBoostMode::Punch);
    }

    #[test]
    fn test_dsp_processor_full_pipeline_processing() {
        let (sender, receiver) = create_param_channel(16);
        let telemetry = Arc::new(AudioTelemetry::new());
        let mut dsp = DspProcessor::new(48000, receiver, Arc::clone(&telemetry));

        // Send preamp +6 dB boost
        sender.set_preamp_gain(6.0).unwrap();

        // Send a 10-band flat update
        let mut bands = [EqBandConfig {
            id: 1,
            enabled: true,
            filter_type: FilterType::Peaking,
            frequency_hz: 1000.0,
            gain_db: 0.0,
            q: 1.0,
        }; 10];
        bands[0].filter_type = FilterType::LowShelf;
        bands[9].filter_type = FilterType::HighShelf;
        sender.update_all_bands(bands).unwrap();

        let mut buffer = vec![0.3f32; 128];
        dsp.process_interleaved_stereo(&mut buffer);

        // Peak telemetry should be updated and non-zero
        let snap = telemetry.snapshot();
        assert!(snap.peak_l > 0.0);
        assert!(snap.peak_r > 0.0);
    }

    #[test]
    fn test_dsp_processor_bypass_pass_through() {
        let (sender, receiver) = create_param_channel(16);
        let telemetry = Arc::new(AudioTelemetry::new());
        let mut dsp = DspProcessor::new(48000, receiver, Arc::clone(&telemetry));

        sender.set_bypass(true).unwrap();

        let mut buffer = vec![0.42f32; 64];
        let original = buffer.clone();
        dsp.process_interleaved_stereo(&mut buffer);

        assert_eq!(buffer, original);
        let snap = telemetry.snapshot();
        assert!((snap.peak_l - 0.42).abs() < 1e-4);
        assert!(!snap.limiter_active);
    }

    #[test]
    fn test_concurrent_parameter_flooding_stress() {
        let (sender, receiver) = create_param_channel(128);
        let telemetry = Arc::new(AudioTelemetry::new());
        let mut dsp = DspProcessor::new(48000, receiver, Arc::clone(&telemetry));

        // 100,000 samples = 50,000 stereo frames (~1.04 sec of audio)
        let total_samples = 100_000;
        let mut audio = Vec::with_capacity(total_samples);
        for n in 0..total_samples {
            let t = n as f32 / 48000.0;
            // Mix of 220 Hz and 880 Hz sine tones
            let s = (2.0 * std::f32::consts::PI * 220.0 * t).sin() * 0.4
                + (2.0 * std::f32::consts::PI * 880.0 * t).sin() * 0.2;
            audio.push(s);
        }

        let chunk_size = 128;
        let mut offset = 0;
        let mut step = 0;

        while offset < audio.len() {
            let end = (offset + chunk_size).min(audio.len());
            let chunk = &mut audio[offset..end];

            // Flood parameters continuously through the sender
            if step % 2 == 0 {
                let _ = sender.set_preamp_gain(((step % 20) as f32 - 10.0) * 0.5); // -5.0 dB to +5.0 dB
            }
            if step % 3 == 0 {
                let _ = sender.set_bass_boost(BassBoostConfig {
                    enabled: step % 6 != 0,
                    gain_db: (step % 12) as f32,
                    mode: if step % 4 == 0 { BassBoostMode::SubBass } else { BassBoostMode::Punch },
                    frequency_hz: if step % 4 == 0 { 45.0 } else { 90.0 },
                });
            }
            if step % 5 == 0 {
                let mut bands = [EqBandConfig::default(); 10];
                for (i, b) in bands.iter_mut().enumerate() {
                    b.id = (i + 1) as u8;
                    b.enabled = true;
                    b.frequency_hz = 60.0 * (1.5f32).powi(i as i32);
                    b.gain_db = ((step + i) % 15) as f32 - 7.0; // -7 dB to +7 dB
                    b.q = 1.0;
                    b.filter_type = if i == 0 { FilterType::LowShelf } else if i == 9 { FilterType::HighShelf } else { FilterType::Peaking };
                }
                let _ = sender.update_all_bands(bands);
            }

            dsp.process_interleaved_stereo(chunk);

            // Assert sample bounds and stability
            for &sample in chunk.iter() {
                assert!(!sample.is_nan() && !sample.is_infinite(), "NaN/Inf produced during stress parameter flood");
                // Bounded by soft-knee limiter (threshold 0 dBFS + soft knee, peak should never blow up above 2.0)
                assert!(
                    sample.abs() <= 2.0,
                    "DSP output exceeded limiter bounds: {} at sample offset {}",
                    sample,
                    offset
                );
            }

            offset = end;
            step += 1;
        }

        assert!(step >= 500, "Should have executed at least 500 automation steps");
    }

    #[test]
    fn test_rapid_bypass_toggle_under_signal() {
        let (sender, receiver) = create_param_channel(64);
        let telemetry = Arc::new(AudioTelemetry::new());
        let mut dsp = DspProcessor::new(48000, receiver, Arc::clone(&telemetry));

        // Heavy boost on preamp and EQ
        sender.set_preamp_gain(6.0).unwrap();
        let mut bands = [EqBandConfig::default(); 10];
        bands[0].enabled = true;
        bands[0].gain_db = 12.0;
        sender.update_all_bands(bands).unwrap();

        let mut audio = vec![0.7f32; 8192];
        let chunk_size = 16; // Rapid toggle every 16 samples
        let mut offset = 0;
        let mut bypass_state = false;

        while offset < audio.len() {
            let end = (offset + chunk_size).min(audio.len());
            let chunk = &mut audio[offset..end];

            bypass_state = !bypass_state;
            let _ = sender.set_bypass(bypass_state);

            dsp.process_interleaved_stereo(chunk);

            for &s in chunk.iter() {
                assert!(!s.is_nan() && !s.is_infinite());
                assert!(s.abs() <= 2.5, "Instability during rapid bypass toggle: {}", s);
            }

            offset = end;
        }
    }
}

