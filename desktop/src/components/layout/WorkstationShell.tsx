import React, { useState } from "react";
import { HeaderBar } from "../header/HeaderBar";
import { ParametricCanvas } from "../canvas/ParametricCanvas";
import { BandInspector } from "../inspector/BandInspector";
import { BottomDeck } from "../deck/BottomDeck";
import {
  FACTORY_PRESETS,
  MOCK_AUDIO_DEVICES,
  EqualizerPreset,
  EqBand,
  BassBoostConfig,
} from "../../types/audio";

export const WorkstationShell: React.FC = () => {
  const [isBypassed, setIsBypassed] = useState<boolean>(false);
  const [limiterEnabled, setLimiterEnabled] = useState<boolean>(true);
  const [selectedPresetId, setSelectedPresetId] = useState<string>("flat");
  const [selectedDeviceId, setSelectedDeviceId] = useState<string>("default");
  const [selectedBandId, setSelectedBandId] = useState<number>(4);
  const [presets, setPresets] = useState<EqualizerPreset[]>(FACTORY_PRESETS);

  const activePreset = presets.find((p) => p.id === selectedPresetId) ?? presets[0];

  const handleUpdateBand = (updatedBand: EqBand) => {
    setPresets((prevPresets) =>
      prevPresets.map((preset) => {
        if (preset.id !== selectedPresetId) return preset;
        return {
          ...preset,
          bands: preset.bands.map((b) => (b.id === updatedBand.id ? updatedBand : b)),
        };
      })
    );
  };

  const handleUpdateBassBoost = (newBassBoost: BassBoostConfig) => {
    setPresets((prevPresets) =>
      prevPresets.map((preset) => {
        if (preset.id !== selectedPresetId) return preset;
        return {
          ...preset,
          bassBoost: newBassBoost,
        };
      })
    );
  };

  const handleUpdatePreamp = (newPreampGainDb: number) => {
    setPresets((prevPresets) =>
      prevPresets.map((preset) => {
        if (preset.id !== selectedPresetId) return preset;
        return {
          ...preset,
          preampGainDb: newPreampGainDb,
        };
      })
    );
  };

  const handleToggleLimiter = () => {
    setLimiterEnabled((prev) => !prev);
  };

  return (
    <div className="flex flex-col w-screen h-screen overflow-hidden bg-surface-base text-text-primary select-none">
      {/* =========================================================================
          TIER 1: HEADER BAR (h-14 / 56px Fixed)
          ========================================================================= */}
      <HeaderBar
        isBypassed={isBypassed}
        onToggleBypass={() => setIsBypassed((prev) => !prev)}
        presets={presets}
        selectedPresetId={selectedPresetId}
        onSelectPreset={(presetId) => setSelectedPresetId(presetId)}
        devices={MOCK_AUDIO_DEVICES}
        selectedDeviceId={selectedDeviceId}
        onSelectDevice={(deviceId) => setSelectedDeviceId(deviceId)}
        onSavePreset={() => {
          console.log("[Presets] Save preset requested for:", selectedPresetId);
        }}
        onNewPreset={() => {
          console.log("[Presets] New preset requested");
        }}
      />

      {/* =========================================================================
          TIER 2: CANVAS STAGE (flex-1 / Dynamic Height, Min 320px)
          ========================================================================= */}
      <section className="flex-1 min-h-80 relative overflow-hidden bg-surface-base p-3 flex flex-col">
        <ParametricCanvas
          bands={activePreset.bands}
          selectedBandId={selectedBandId}
          onSelectBand={(bandId) => setSelectedBandId(bandId)}
          onUpdateBand={handleUpdateBand}
          preampGainDb={activePreset.preampGainDb}
          bassBoost={activePreset.bassBoost}
          isBypassed={isBypassed}
        />
      </section>

      {/* =========================================================================
          TIER 3: BAND INSPECTOR STRIP (h-20 / 80px Fixed)
          ========================================================================= */}
      <BandInspector
        bands={activePreset.bands}
        selectedBandId={selectedBandId}
        onSelectBand={(bandId) => setSelectedBandId(bandId)}
        onUpdateBand={handleUpdateBand}
        isBypassed={isBypassed}
      />

      {/* =========================================================================
          TIER 4: BOTTOM CONTROL DECK (h-44 / 176px Fixed)
          ========================================================================= */}
      <BottomDeck
        bassBoost={activePreset.bassBoost}
        onUpdateBassBoost={handleUpdateBassBoost}
        preampGainDb={activePreset.preampGainDb}
        onUpdatePreamp={handleUpdatePreamp}
        limiterEnabled={limiterEnabled}
        onToggleLimiter={handleToggleLimiter}
        isBypassed={isBypassed}
      />
    </div>
  );
};

export default WorkstationShell;
