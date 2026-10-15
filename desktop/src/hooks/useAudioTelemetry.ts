import { useState, useEffect, useRef, useCallback } from "react";

export interface AudioTelemetryOptions {
  enabled?: boolean;
  preampGainDb?: number;
  bassBoostGainDb?: number;
  isBypassed?: boolean;
  limiterEnabled?: boolean;
}

export interface AudioTelemetryData {
  peakL: number;
  peakR: number;
  holdL: number;
  holdR: number;
  limiterActive: boolean;
  isClipping: boolean;
  clearClip: () => void;
}

const MIN_DB = -48.0;
const MAX_DB = 0.0;
const HOLD_TIME_MS = 1200;
const CLIP_HOLD_MS = 2000;
const DECAY_RATE_DB_PER_SEC = 25.0; // 25 dB per second falloff

export function useAudioTelemetry({
  enabled = true,
  preampGainDb = 0.0,
  bassBoostGainDb = 0.0,
  isBypassed = false,
  limiterEnabled = true,
}: AudioTelemetryOptions = {}): AudioTelemetryData {
  const [telemetry, setTelemetry] = useState({
    peakL: -24.0,
    peakR: -24.0,
    holdL: -20.0,
    holdR: -20.0,
    limiterActive: false,
    isClipping: false,
  });

  // Mutable refs for smooth ballistic calculations across 30 Hz ticks
  const currentLRef = useRef<number>(-24.0);
  const currentRRef = useRef<number>(-24.0);
  const holdLRef = useRef<number>(-20.0);
  const holdRRef = useRef<number>(-20.0);
  const holdTimerLRef = useRef<number>(0);
  const holdTimerRRef = useRef<number>(0);
  const clipExpireRef = useRef<number>(0);
  const lastTickTimeRef = useRef<number>(0);
  const phaseRef = useRef<number>(0);

  // Synchronize options in ref to avoid recreating the interval on every knob tweak
  const optionsRef = useRef({
    enabled,
    preampGainDb,
    bassBoostGainDb,
    isBypassed,
    limiterEnabled,
  });

  useEffect(() => {
    optionsRef.current = {
      enabled,
      preampGainDb,
      bassBoostGainDb,
      isBypassed,
      limiterEnabled,
    };
  }, [enabled, preampGainDb, bassBoostGainDb, isBypassed, limiterEnabled]);

  const clearClip = useCallback(() => {
    clipExpireRef.current = 0;
    setTelemetry((prev) => ({ ...prev, isClipping: false }));
  }, []);

  useEffect(() => {
    if (!enabled) return;

    lastTickTimeRef.current = Date.now();
    const intervalId = window.setInterval(() => {
      const now = Date.now();
      const dt = Math.min(0.1, Math.max(0.01, (now - lastTickTimeRef.current) / 1000));
      lastTickTimeRef.current = now;

      const opts = optionsRef.current;
      phaseRef.current += 0.25;

      // Simulated musical signal wave with slight stereo asymmetry
      const beat1 = Math.sin(phaseRef.current * 0.8) * 4.0;
      const beat2 = Math.cos(phaseRef.current * 1.7) * 3.0;
      const noiseL = (Math.random() - 0.5) * 2.5;
      const noiseR = (Math.random() - 0.5) * 2.5;

      // Base musical RMS level around -16 dBFS
      let baseTargetL = -16.0 + beat1 + noiseL;
      let baseTargetR = -16.0 + beat2 + noiseR;

      // Add preamp and bass boost influence unless bypassed
      if (!opts.isBypassed) {
        const boostEffect = opts.bassBoostGainDb * 0.25;
        baseTargetL += opts.preampGainDb + boostEffect;
        baseTargetR += opts.preampGainDb + boostEffect;
      }

      // Check limiter / clipping
      let limiterActive = false;
      let isClipping = now < clipExpireRef.current;

      if (opts.limiterEnabled && !opts.isBypassed) {
        // Soft-knee limiter compresses peaks above -0.1 dBFS
        if (baseTargetL >= -0.1 || baseTargetR >= -0.1) {
          limiterActive = true;
          baseTargetL = Math.min(-0.1, baseTargetL);
          baseTargetR = Math.min(-0.1, baseTargetR);
        }
      } else {
        // Unprotected: signal can reach or exceed 0 dBFS
        if (baseTargetL >= 0.0 || baseTargetR >= 0.0) {
          clipExpireRef.current = now + CLIP_HOLD_MS;
          isClipping = true;
        }
      }

      // Ballistics for Left Channel
      if (baseTargetL > currentLRef.current) {
        currentLRef.current = baseTargetL; // Instant attack
      } else {
        currentLRef.current = Math.max(
          MIN_DB,
          currentLRef.current - DECAY_RATE_DB_PER_SEC * dt
        );
      }

      // Peak-hold for Left Channel
      if (currentLRef.current >= holdLRef.current) {
        holdLRef.current = currentLRef.current;
        holdTimerLRef.current = now + HOLD_TIME_MS;
      } else if (now > holdTimerLRef.current) {
        holdLRef.current = Math.max(
          currentLRef.current,
          holdLRef.current - DECAY_RATE_DB_PER_SEC * dt
        );
      }

      // Ballistics for Right Channel
      if (baseTargetR > currentRRef.current) {
        currentRRef.current = baseTargetR; // Instant attack
      } else {
        currentRRef.current = Math.max(
          MIN_DB,
          currentRRef.current - DECAY_RATE_DB_PER_SEC * dt
        );
      }

      // Peak-hold for Right Channel
      if (currentRRef.current >= holdRRef.current) {
        holdRRef.current = currentRRef.current;
        holdTimerRRef.current = now + HOLD_TIME_MS;
      } else if (now > holdTimerRRef.current) {
        holdRRef.current = Math.max(
          currentRRef.current,
          holdRRef.current - DECAY_RATE_DB_PER_SEC * dt
        );
      }

      setTelemetry({
        peakL: Math.max(MIN_DB, Math.min(MAX_DB, currentLRef.current)),
        peakR: Math.max(MIN_DB, Math.min(MAX_DB, currentRRef.current)),
        holdL: Math.max(MIN_DB, Math.min(MAX_DB, holdLRef.current)),
        holdR: Math.max(MIN_DB, Math.min(MAX_DB, holdRRef.current)),
        limiterActive,
        isClipping,
      });
    }, 33); // ~30 Hz

    return () => {
      window.clearInterval(intervalId);
    };
  }, [enabled]);

  return {
    ...telemetry,
    clearClip,
  };
}

export default useAudioTelemetry;
