import React, { useState } from "react";
import { HeaderBar } from "../header/HeaderBar";
import { ParametricCanvas } from "../canvas/ParametricCanvas";
import { BandInspector } from "../inspector/BandInspector";
import { BottomDeck } from "../deck/BottomDeck";
import { SavePresetModal } from "../modals/SavePresetModal";
import { DeleteConfirmModal } from "../modals/DeleteConfirmModal";
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
  const [isSaveModalOpen, setIsSaveModalOpen] = useState<boolean>(false);
  const [presetToDelete, setPresetToDelete] = useState<EqualizerPreset | null>(null);

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

  const handleSavePreset = (name: string, overwriteId?: string) => {
    if (overwriteId) {
      setPresets((prev) =>
        prev.map((p) => {
          if (p.id !== overwriteId) return p;
          return {
            ...p,
            name,
            preampGainDb: activePreset.preampGainDb,
            bassBoost: { ...activePreset.bassBoost },
            bands: activePreset.bands.map((b) => ({ ...b })),
            updatedAt: new Date().toISOString(),
          };
        })
      );
      setSelectedPresetId(overwriteId);
    } else {
      const newId = `user-${Date.now()}`;
      const newPreset: EqualizerPreset = {
        id: newId,
        name,
        isFactory: false,
        preampGainDb: activePreset.preampGainDb,
        bassBoost: { ...activePreset.bassBoost },
        bands: activePreset.bands.map((b) => ({ ...b })),
        createdAt: new Date().toISOString(),
        updatedAt: new Date().toISOString(),
      };
      setPresets((prev) => [...prev, newPreset]);
      setSelectedPresetId(newId);
    }
  };

  const handleDeletePreset = (presetId: string) => {
    setPresets((prev) => prev.filter((p) => p.id !== presetId));
    if (selectedPresetId === presetId) {
      setSelectedPresetId("flat");
    }
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
        onSavePreset={() => setIsSaveModalOpen(true)}
        onNewPreset={() => setIsSaveModalOpen(true)}
        onDeletePreset={(preset) => setPresetToDelete(preset)}
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

      {/* =========================================================================
          MODALS: Preset Save & Delete Confirmation Dialogs
          ========================================================================= */}
      {isSaveModalOpen && (
        <SavePresetModal
          isOpen={isSaveModalOpen}
          currentPreset={activePreset}
          existingPresets={presets}
          onClose={() => setIsSaveModalOpen(false)}
          onSave={handleSavePreset}
        />
      )}

      {presetToDelete !== null && (
        <DeleteConfirmModal
          isOpen={presetToDelete !== null}
          preset={presetToDelete}
          onClose={() => setPresetToDelete(null)}
          onConfirmDelete={handleDeletePreset}
        />
      )}
    </div>
  );
};

export default WorkstationShell;
