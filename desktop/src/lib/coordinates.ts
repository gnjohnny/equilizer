/**
 * Logarithmic Frequency & Linear Decibel Coordinate Projection Utilities
 * Source of truth for Canvas graph mappings (20 Hz - 20,000 Hz, -24 dB to +24 dB)
 */

export const MIN_FREQ_HZ = 20.0;
export const MAX_FREQ_HZ = 20000.0;
export const MIN_GAIN_DB = -24.0;
export const MAX_GAIN_DB = 24.0;

const LOG_MIN_FREQ = Math.log10(MIN_FREQ_HZ);
const LOG_MAX_FREQ = Math.log10(MAX_FREQ_HZ);

/**
 * Maps frequency in Hz to horizontal canvas pixel coordinate X
 */
export function freqToX(
  freqHz: number,
  width: number,
  minFreq: number = MIN_FREQ_HZ,
  maxFreq: number = MAX_FREQ_HZ
): number {
  if (width <= 0) return 0;
  const clampedFreq = Math.max(minFreq, Math.min(maxFreq, freqHz));
  const logMin = minFreq === MIN_FREQ_HZ ? LOG_MIN_FREQ : Math.log10(minFreq);
  const logMax = maxFreq === MAX_FREQ_HZ ? LOG_MAX_FREQ : Math.log10(maxFreq);
  const range = logMax - logMin;

  const normalized = (Math.log10(clampedFreq) - logMin) / range;
  return normalized * width;
}

/**
 * Maps horizontal canvas pixel coordinate X to frequency in Hz
 */
export function xToFreq(
  x: number,
  width: number,
  minFreq: number = MIN_FREQ_HZ,
  maxFreq: number = MAX_FREQ_HZ
): number {
  if (width <= 0) return minFreq;
  const clampedX = Math.max(0, Math.min(width, x));
  const normalized = clampedX / width;

  const logMin = minFreq === MIN_FREQ_HZ ? LOG_MIN_FREQ : Math.log10(minFreq);
  const logMax = maxFreq === MAX_FREQ_HZ ? LOG_MAX_FREQ : Math.log10(maxFreq);
  const range = logMax - logMin;

  const logFreq = logMin + normalized * range;
  return Math.pow(10, logFreq);
}

/**
 * Maps decibel gain to vertical canvas pixel coordinate Y (0 dB is centered, +24 dB at top, -24 dB at bottom)
 */
export function gainToY(
  gainDb: number,
  height: number,
  minDb: number = MIN_GAIN_DB,
  maxDb: number = MAX_GAIN_DB
): number {
  if (height <= 0) return 0;
  const clampedGain = Math.max(minDb, Math.min(maxDb, gainDb));
  const range = maxDb - minDb;
  const normalized = (maxDb - clampedGain) / range;
  return normalized * height;
}

/**
 * Maps vertical canvas pixel coordinate Y to decibel gain
 */
export function yToGain(
  y: number,
  height: number,
  minDb: number = MIN_GAIN_DB,
  maxDb: number = MAX_GAIN_DB
): number {
  if (height <= 0) return 0;
  const clampedY = Math.max(0, Math.min(height, y));
  const normalized = clampedY / height;
  const range = maxDb - minDb;
  return maxDb - normalized * range;
}

/**
 * Standard Octave Frequency Guides & Labels
 */
export interface OctaveGuide {
  freqHz: number;
  label: string;
}

export const OCTAVE_GUIDES: OctaveGuide[] = [
  { freqHz: 20, label: "20 Hz" },
  { freqHz: 60, label: "60 Hz" },
  { freqHz: 125, label: "125 Hz" },
  { freqHz: 250, label: "250 Hz" },
  { freqHz: 500, label: "500 Hz" },
  { freqHz: 1000, label: "1 kHz" },
  { freqHz: 2000, label: "2 kHz" },
  { freqHz: 4000, label: "4 kHz" },
  { freqHz: 8000, label: "8 kHz" },
  { freqHz: 16000, label: "16 kHz" },
  { freqHz: 20000, label: "20 kHz" },
];

/**
 * Standard Decibel Grid References
 */
export const GAIN_TICKS: number[] = [18, 12, 6, 0, -6, -12, -18];

/**
 * Pre-generate an array of N logarithmically spaced frequencies across [minFreq, maxFreq]
 */
export function generateLogFrequencies(
  numPoints: number,
  minFreq: number = MIN_FREQ_HZ,
  maxFreq: number = MAX_FREQ_HZ
): Float32Array {
  const result = new Float32Array(numPoints);
  const logMin = minFreq === MIN_FREQ_HZ ? LOG_MIN_FREQ : Math.log10(minFreq);
  const logMax = maxFreq === MAX_FREQ_HZ ? LOG_MAX_FREQ : Math.log10(maxFreq);
  const step = (logMax - logMin) / (numPoints - 1);

  for (let i = 0; i < numPoints; i++) {
    result[i] = Math.pow(10, logMin + i * step);
  }

  return result;
}
