/**
 * Biquad Filter Mathematics & Frequency Response Computation Engine
 * Implements Robert Bristow-Johnson (RBJ) Audio EQ Cookbook formulas
 */

import { EqBand } from "../types/audio";
import { generateLogFrequencies } from "./coordinates";

export const DEFAULT_SAMPLE_RATE = 48000;
export const NUM_CURVE_POINTS = 512;

export interface NormalizedBiquadCoeffs {
  b0: number;
  b1: number;
  b2: number;
  a1: number;
  a2: number;
}

/**
 * Calculates normalized biquad coefficients for a given filter band
 */
export function calculateBiquadCoefficients(
  band: EqBand,
  sampleRate: number = DEFAULT_SAMPLE_RATE
): NormalizedBiquadCoeffs {
  const { filterType, frequencyHz, gainDb, q } = band;

  // Clamp frequency below Nyquist
  const nyquist = sampleRate / 2;
  const f0 = Math.max(10, Math.min(nyquist - 100, frequencyHz));
  const Q = Math.max(0.1, Math.min(10.0, q));

  const A = Math.pow(10, gainDb / 40); // sqrt of linear gain
  const w0 = (2 * Math.PI * f0) / sampleRate;
  const cosW0 = Math.cos(w0);
  const sinW0 = Math.sin(w0);
  const alpha = sinW0 / (2 * Q);

  let b0 = 1;
  let b1 = 0;
  let b2 = 0;
  let a0 = 1;
  let a1 = 0;
  let a2 = 0;

  switch (filterType) {
    case "Peaking": {
      b0 = 1 + alpha * A;
      b1 = -2 * cosW0;
      b2 = 1 - alpha * A;
      a0 = 1 + alpha / A;
      a1 = -2 * cosW0;
      a2 = 1 - alpha / A;
      break;
    }
    case "LowShelf": {
      const twoSqrtAAlpha = 2 * Math.sqrt(A) * alpha;
      b0 = A * (A + 1 - (A - 1) * cosW0 + twoSqrtAAlpha);
      b1 = 2 * A * (A - 1 - (A + 1) * cosW0);
      b2 = A * (A + 1 - (A - 1) * cosW0 - twoSqrtAAlpha);
      a0 = A + 1 + (A - 1) * cosW0 + twoSqrtAAlpha;
      a1 = -2 * (A - 1 + (A + 1) * cosW0);
      a2 = A + 1 + (A - 1) * cosW0 - twoSqrtAAlpha;
      break;
    }
    case "HighShelf": {
      const twoSqrtAAlpha = 2 * Math.sqrt(A) * alpha;
      b0 = A * (A + 1 + (A - 1) * cosW0 + twoSqrtAAlpha);
      b1 = -2 * A * (A - 1 + (A + 1) * cosW0);
      b2 = A * (A + 1 + (A - 1) * cosW0 - twoSqrtAAlpha);
      a0 = A + 1 - (A - 1) * cosW0 + twoSqrtAAlpha;
      a1 = 2 * (A - 1 - (A + 1) * cosW0);
      a2 = A + 1 - (A - 1) * cosW0 - twoSqrtAAlpha;
      break;
    }
    case "LowPass": {
      b0 = (1 - cosW0) / 2;
      b1 = 1 - cosW0;
      b2 = (1 - cosW0) / 2;
      a0 = 1 + alpha;
      a1 = -2 * cosW0;
      a2 = 1 - alpha;
      break;
    }
    case "HighPass": {
      b0 = (1 + cosW0) / 2;
      b1 = -(1 + cosW0);
      b2 = (1 + cosW0) / 2;
      a0 = 1 + alpha;
      a1 = -2 * cosW0;
      a2 = 1 - alpha;
      break;
    }
  }

  // Normalize by a0
  if (Math.abs(a0) < 1e-9) {
    return { b0: 1, b1: 0, b2: 0, a1: 0, a2: 0 };
  }

  return {
    b0: b0 / a0,
    b1: b1 / a0,
    b2: b2 / a0,
    a1: a1 / a0,
    a2: a2 / a0,
  };
}

/**
 * Calculates magnitude response in dB of a normalized biquad at frequency f
 */
export function evaluateBiquadGainDb(
  coeffs: NormalizedBiquadCoeffs,
  freqHz: number,
  sampleRate: number = DEFAULT_SAMPLE_RATE
): number {
  const w = (2 * Math.PI * freqHz) / sampleRate;
  const cos1 = Math.cos(w);
  const cos2 = Math.cos(2 * w);
  const sin1 = Math.sin(w);
  const sin2 = Math.sin(2 * w);

  const numRe = coeffs.b0 + coeffs.b1 * cos1 + coeffs.b2 * cos2;
  const numIm = -(coeffs.b1 * sin1 + coeffs.b2 * sin2);
  const numMagSq = numRe * numRe + numIm * numIm;

  const denRe = 1 + coeffs.a1 * cos1 + coeffs.a2 * cos2;
  const denIm = -(coeffs.a1 * sin1 + coeffs.a2 * sin2);
  const denMagSq = denRe * denRe + denIm * denIm;

  if (denMagSq < 1e-12) return -100;
  if (numMagSq < 1e-12) return -100;

  const gainDb = 10 * Math.log10(numMagSq / denMagSq);
  // Guard against non-finite values
  if (!Number.isFinite(gainDb)) return 0;
  return gainDb;
}

/**
 * Pre-allocated singleton frequency array across 512 points (20 Hz - 20 kHz)
 */
export const PRECOMPUTED_FREQUENCIES: Float32Array =
  generateLogFrequencies(NUM_CURVE_POINTS);

/**
 * Evaluates composite transfer function across all active bands.
 * Mutates targetBuffer in-place to ensure zero heap allocations inside 60 FPS animation loop.
 */
export function computeCompositeResponse(
  bands: EqBand[],
  preampGainDb: number = 0.0,
  targetBuffer: Float32Array = new Float32Array(NUM_CURVE_POINTS),
  frequencies: Float32Array = PRECOMPUTED_FREQUENCIES,
  sampleRate: number = DEFAULT_SAMPLE_RATE
): Float32Array {
  // Pre-calculate coefficients only for enabled bands
  const activeCoeffs: NormalizedBiquadCoeffs[] = [];
  for (let i = 0; i < bands.length; i++) {
    const band = bands[i];
    if (band && band.enabled) {
      activeCoeffs.push(calculateBiquadCoefficients(band, sampleRate));
    }
  }

  const numPoints = frequencies.length;
  for (let p = 0; p < numPoints; p++) {
    const f = frequencies[p];
    let totalDb = preampGainDb;

    for (let c = 0; c < activeCoeffs.length; c++) {
      totalDb += evaluateBiquadGainDb(activeCoeffs[c], f, sampleRate);
    }

    // Clamp output dB within reasonable display bounds
    targetBuffer[p] = Math.max(-48.0, Math.min(36.0, totalDb));
  }

  return targetBuffer;
}
