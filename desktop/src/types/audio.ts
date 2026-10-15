/**
 * Audio Core Data Models & Schemas
 * Source of truth matching projectworkflow.md and AGENTS.md
 */

export type FilterType = 'Peaking' | 'LowShelf' | 'HighShelf' | 'HighPass' | 'LowPass';

export interface EqBand {
  id: number; // 1 through 10
  enabled: boolean;
  filterType: FilterType;
  frequencyHz: number; // 20.0 to 20000.0
  gainDb: number; // -24.0 to +24.0
  q: number; // 0.1 to 10.0
}

export type BassBoostMode = 'SubBass' | 'Punch';

export interface BassBoostConfig {
  enabled: boolean;
  gainDb: number; // 0.0 to 12.0 dB
  mode: BassBoostMode; // SubBass (45Hz) vs. Punch (90Hz)
  frequencyHz: number;
}

export interface EqualizerPreset {
  id: string;
  name: string;
  isFactory: boolean;
  preampGainDb: number; // -20.0 to +6.0 dB
  bassBoost: BassBoostConfig;
  bands: EqBand[];
  createdAt?: string;
  updatedAt?: string;
}

export interface AudioDevice {
  id: string;
  name: string;
  isDefault: boolean;
}

export interface AppConfig {
  activePresetId: string;
  selectedDeviceId: string | null; // null = System Default
  isBypassed: boolean;
  limiterEnabled: boolean;
  startMinimizedToTray: boolean;
  launchOnStartup: boolean;
}

/**
 * Standard 10-band ISO center frequencies
 */
export const DEFAULT_BAND_FREQUENCIES = [
  31.25, 62.5, 125, 250, 500, 1000, 2000, 4000, 8000, 16000,
];

export const createDefaultBands = (gains: number[] = [0, 0, 0, 0, 0, 0, 0, 0, 0, 0]): EqBand[] => {
  return DEFAULT_BAND_FREQUENCIES.map((freq, index) => {
    let filterType: FilterType = 'Peaking';
    if (index === 0) filterType = 'LowShelf';
    else if (index === 9) filterType = 'HighShelf';

    return {
      id: index + 1,
      enabled: true,
      filterType,
      frequencyHz: freq,
      gainDb: gains[index] ?? 0,
      q: 1.0,
    };
  });
};

/**
 * Bundled Factory Presets
 */
export const FACTORY_PRESETS: EqualizerPreset[] = [
  {
    id: 'flat',
    name: 'Flat',
    isFactory: true,
    preampGainDb: 0.0,
    bassBoost: {
      enabled: false,
      gainDb: 0.0,
      mode: 'SubBass',
      frequencyHz: 45.0,
    },
    bands: createDefaultBands([0, 0, 0, 0, 0, 0, 0, 0, 0, 0]),
  },
  {
    id: 'deep-bass',
    name: 'Deep Bass',
    isFactory: true,
    preampGainDb: -3.0,
    bassBoost: {
      enabled: true,
      gainDb: 6.0,
      mode: 'SubBass',
      frequencyHz: 45.0,
    },
    bands: createDefaultBands([6.5, 5.0, 3.5, 1.5, 0.0, 0.0, 0.5, 1.0, 1.5, 2.0]),
  },
  {
    id: 'punchy-gaming',
    name: 'Punchy Gaming',
    isFactory: true,
    preampGainDb: -2.0,
    bassBoost: {
      enabled: true,
      gainDb: 4.5,
      mode: 'Punch',
      frequencyHz: 90.0,
    },
    bands: createDefaultBands([4.0, 4.5, 2.0, -1.0, 1.0, 2.5, 4.0, 3.0, 2.0, 1.0]),
  },
  {
    id: 'crisp-vocals',
    name: 'Crisp Vocals',
    isFactory: true,
    preampGainDb: -1.0,
    bassBoost: {
      enabled: false,
      gainDb: 0.0,
      mode: 'Punch',
      frequencyHz: 90.0,
    },
    bands: createDefaultBands([-2.5, -2.0, -1.0, 0.5, 2.0, 3.5, 4.0, 3.0, 2.5, 1.5]),
  },
  {
    id: 'rock',
    name: 'Rock',
    isFactory: true,
    preampGainDb: -2.5,
    bassBoost: {
      enabled: true,
      gainDb: 3.0,
      mode: 'Punch',
      frequencyHz: 90.0,
    },
    bands: createDefaultBands([4.5, 3.5, 1.5, -1.0, -0.5, 1.0, 2.5, 4.0, 4.5, 4.0]),
  },
];

/**
 * Mock Output Audio Endpoints
 */
export const MOCK_AUDIO_DEVICES: AudioDevice[] = [
  {
    id: 'default',
    name: 'System Default Output',
    isDefault: true,
  },
  {
    id: 'speakers-realtek',
    name: 'Speakers (Realtek High Definition Audio)',
    isDefault: false,
  },
  {
    id: 'headphones-usb',
    name: 'Headphones (USB Gaming Audio Device)',
    isDefault: false,
  },
  {
    id: 'digital-output',
    name: 'Realtek Digital Output (Optical)',
    isDefault: false,
  },
];
