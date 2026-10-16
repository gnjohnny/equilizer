import React, { useState } from "react";
import { HeaderBar } from "../header/HeaderBar";
import { ParametricCanvas } from "../canvas/ParametricCanvas";
import { BandInspector } from "../inspector/BandInspector";
import { BottomDeck } from "../deck/BottomDeck";
import { SavePresetModal } from "../modals/SavePresetModal";
import { DeleteConfirmModal } from "../modals/DeleteConfirmModal";
import { EqualizerPreset } from "../../types/audio";
import { useAudioDevices, usePresets, useEqState } from "../../hooks";

export const WorkstationShell: React.FC = () => {
  const { devices, selectedDeviceId, selectDevice } = useAudioDevices();

  const {
    presets,
    activePresetId,
    activePreset,
    selectPreset,
    savePreset,
    deletePreset,
  } = usePresets();

  const {
    bands,
    bassBoost,
    preampGainDb,
    isBypassed,
    updateBand,
    updateBassBoost,
    updatePreamp,
    toggleBypass,
  } = useEqState({ activePreset });

  const [limiterEnabled, setLimiterEnabled] = useState<boolean>(true);
  const [selectedBandId, setSelectedBandId] = useState<number>(4);
  const [isSaveModalOpen, setIsSaveModalOpen] = useState<boolean>(false);
  const [presetToDelete, setPresetToDelete] = useState<EqualizerPreset | null>(null);

  // Snapshot combining preset metadata with current live curve edits for save modal
  const livePresetSnapshot: EqualizerPreset = {
    ...activePreset,
    preampGainDb,
    bassBoost,
    bands,
  };

  const handleSavePreset = async (name: string, overwriteId?: string) => {
    try {
      await savePreset(name, overwriteId, {
        preampGainDb,
        bassBoost,
        bands,
      });
      setIsSaveModalOpen(false);
    } catch (err) {
      console.error("Failed to save preset:", err);
    }
  };

  const handleDeletePreset = async (presetId: string) => {
    try {
      await deletePreset(presetId);
      setPresetToDelete(null);
    } catch (err) {
      console.error("Failed to delete preset:", err);
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
        onToggleBypass={() => toggleBypass()}
        presets={presets}
        selectedPresetId={activePresetId}
        onSelectPreset={(presetId) => void selectPreset(presetId)}
        devices={devices}
        selectedDeviceId={selectedDeviceId}
        onSelectDevice={(deviceId) => void selectDevice(deviceId)}
        onSavePreset={() => setIsSaveModalOpen(true)}
        onNewPreset={() => setIsSaveModalOpen(true)}
        onDeletePreset={(preset) => setPresetToDelete(preset)}
      />

      {/* =========================================================================
          TIER 2: CANVAS STAGE (flex-1 / Dynamic Height, Min 320px)
          ========================================================================= */}
      <section className="flex-1 min-h-80 relative overflow-hidden bg-surface-base p-3 flex flex-col">
        <ParametricCanvas
          bands={bands}
          selectedBandId={selectedBandId}
          onSelectBand={(bandId) => setSelectedBandId(bandId)}
          onUpdateBand={updateBand}
          preampGainDb={preampGainDb}
          bassBoost={bassBoost}
          isBypassed={isBypassed}
        />
      </section>

      {/* =========================================================================
          TIER 3: BAND INSPECTOR STRIP (h-20 / 80px Fixed)
          ========================================================================= */}
      <BandInspector
        bands={bands}
        selectedBandId={selectedBandId}
        onSelectBand={(bandId) => setSelectedBandId(bandId)}
        onUpdateBand={updateBand}
        isBypassed={isBypassed}
      />

      {/* =========================================================================
          TIER 4: BOTTOM CONTROL DECK (h-44 / 176px Fixed)
          ========================================================================= */}
      <BottomDeck
        bassBoost={bassBoost}
        onUpdateBassBoost={updateBassBoost}
        preampGainDb={preampGainDb}
        onUpdatePreamp={updatePreamp}
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
          currentPreset={livePresetSnapshot}
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
