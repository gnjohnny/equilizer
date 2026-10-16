import { useState, useEffect, useRef, useCallback } from "react";
import { invoke, isTauri } from "@tauri-apps/api/core";
import {
  EqBand,
  BassBoostConfig,
  EqualizerPreset,
  FACTORY_PRESETS,
} from "../types/audio";

export interface UseEqStateOptions {
  activePreset?: EqualizerPreset;
  initialBypassed?: boolean;
}

export interface UseEqStateReturn {
  bands: EqBand[];
  bassBoost: BassBoostConfig;
  preampGainDb: number;
  isBypassed: boolean;
  updateBand: (band: EqBand) => void;
  updateBassBoost: (config: BassBoostConfig) => void;
  updatePreamp: (gainDb: number) => void;
  toggleBypass: (bypassed?: boolean) => void;
}

/**
 * Hook for live equalizer audio parameters (bands, bass boost, preamp, bypass).
 * Dispatches atomic updates to the Tauri IPC audio engine while keeping local state responsive at 60 FPS.
 */
export function useEqState({
  activePreset = FACTORY_PRESETS[0],
  initialBypassed = false,
}: UseEqStateOptions = {}): UseEqStateReturn {
  const [bands, setBands] = useState<EqBand[]>(() =>
    activePreset.bands.map((b) => ({ ...b }))
  );
  const [bassBoost, setBassBoost] = useState<BassBoostConfig>(() => ({
    ...activePreset.bassBoost,
  }));
  const [preampGainDb, setPreampGainDb] = useState<number>(
    activePreset.preampGainDb
  );
  const [isBypassed, setIsBypassed] = useState<boolean>(initialBypassed);

  // Synchronize with active preset when activePreset changes (e.g. user selected another preset)
  const lastPresetIdRef = useRef<string>(activePreset.id);

  useEffect(() => {
    if (activePreset.id !== lastPresetIdRef.current) {
      lastPresetIdRef.current = activePreset.id;
      setBands(activePreset.bands.map((b) => ({ ...b })));
      setBassBoost({ ...activePreset.bassBoost });
      setPreampGainDb(activePreset.preampGainDb);
    }
  }, [activePreset]);

  // Synchronize with external bypass changes (e.g. from System Tray right-click context menu)
  useEffect(() => {
    if (!isTauri()) return;

    let unlisten: (() => void) | undefined;
    import("@tauri-apps/api/event").then(({ listen }) => {
      listen<boolean>("app:bypass-changed", (event) => {
        setIsBypassed(event.payload);
      }).then((dispose) => {
        unlisten = dispose;
      });
    });

    return () => {
      unlisten?.();
    };
  }, []);

  // IPC Dispatch for EQ Bands
  const updateBand = useCallback((updatedBand: EqBand) => {
    setBands((prevBands) => {
      const nextBands = prevBands.map((b) =>
        b.id === updatedBand.id ? updatedBand : b
      );

      if (isTauri()) {
        invoke("update_eq_bands", { bands: nextBands }).catch((err) => {
          console.error("Failed to update EQ bands via Tauri IPC:", err);
        });
      }

      return nextBands;
    });
  }, []);

  // IPC Dispatch for Bass Boost
  const updateBassBoost = useCallback((newBassBoost: BassBoostConfig) => {
    setBassBoost(newBassBoost);

    if (isTauri()) {
      invoke("update_bass_boost", { config: newBassBoost }).catch((err) => {
        console.error("Failed to update Bass Boost via Tauri IPC:", err);
      });
    }
  }, []);

  // IPC Dispatch for Master Pre-Amp
  const updatePreamp = useCallback((newPreampGainDb: number) => {
    setPreampGainDb(newPreampGainDb);

    if (isTauri()) {
      invoke("set_preamp_gain", { gainDb: newPreampGainDb }).catch((err) => {
        console.error("Failed to set preamp gain via Tauri IPC:", err);
      });
    }
  }, []);

  // IPC Dispatch for Master Bypass Toggle
  const toggleBypass = useCallback((bypassed?: boolean) => {
    setIsBypassed((prev) => {
      const nextBypassed = typeof bypassed === "boolean" ? bypassed : !prev;

      if (isTauri()) {
        invoke("toggle_bypass", { bypassed: nextBypassed }).catch((err) => {
          console.error("Failed to toggle bypass via Tauri IPC:", err);
        });
      }

      return nextBypassed;
    });
  }, []);

  return {
    bands,
    bassBoost,
    preampGainDb,
    isBypassed,
    updateBand,
    updateBassBoost,
    updatePreamp,
    toggleBypass,
  };
}

export default useEqState;
