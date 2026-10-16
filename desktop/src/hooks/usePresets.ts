import { useState, useEffect, useCallback, useMemo } from "react";
import { invoke, isTauri } from "@tauri-apps/api/core";
import {
  EqualizerPreset,
  FACTORY_PRESETS,
  EqBand,
  BassBoostConfig,
} from "../types/audio";

export interface CustomEqData {
  preampGainDb?: number;
  bassBoost?: BassBoostConfig;
  bands?: EqBand[];
}

export interface UsePresetsReturn {
  presets: EqualizerPreset[];
  activePresetId: string;
  activePreset: EqualizerPreset;
  isLoading: boolean;
  error: string | null;
  selectPreset: (presetId: string) => Promise<void>;
  savePreset: (
    name: string,
    overwriteId?: string,
    customEq?: CustomEqData
  ) => Promise<EqualizerPreset>;
  deletePreset: (presetId: string) => Promise<void>;
  reloadPresets: () => Promise<void>;
}

/**
 * Hook for equalizer preset persistence, loading, saving, and activation via Tauri IPC.
 * Protects factory presets and safely manages local/IPC state.
 */
export function usePresets(): UsePresetsReturn {
  const [presets, setPresets] = useState<EqualizerPreset[]>(FACTORY_PRESETS);
  const [activePresetId, setActivePresetId] = useState<string>("flat");
  const [isLoading, setIsLoading] = useState<boolean>(() => isTauri());
  const [error, setError] = useState<string | null>(null);

  const activePreset = useMemo(() => {
    return (
      presets.find((p) => p.id === activePresetId) ??
      presets[0] ??
      FACTORY_PRESETS[0]
    );
  }, [presets, activePresetId]);

  // Synchronize with stored presets on mount when running inside Tauri
  useEffect(() => {
    if (!isTauri()) return;

    let isMounted = true;
    invoke<EqualizerPreset[]>("list_presets")
      .then((loadedPresets) => {
        if (!isMounted) return;
        if (loadedPresets && loadedPresets.length > 0) {
          setPresets(loadedPresets);
        }
      })
      .catch((err: unknown) => {
        if (!isMounted) return;
        const msg = err instanceof Error ? err.message : String(err);
        console.warn("Failed to load presets via Tauri IPC:", msg);
        setError(msg);
      })
      .finally(() => {
        if (isMounted) setIsLoading(false);
      });

    return () => {
      isMounted = false;
    };
  }, []);

  // Synchronize with external active preset updates (e.g. from System Tray right-click context menu)
  useEffect(() => {
    if (!isTauri()) return;

    let unlisten: (() => void) | undefined;
    import("@tauri-apps/api/event").then(({ listen }) => {
      listen<EqualizerPreset>("app:preset-changed", (event) => {
        if (event.payload?.id) {
          setActivePresetId(event.payload.id);
        }
      }).then((dispose) => {
        unlisten = dispose;
      });
    });

    return () => {
      unlisten?.();
    };
  }, []);

  const reloadPresets = useCallback(async () => {
    if (!isTauri()) return;

    try {
      setIsLoading(true);
      setError(null);
      const loadedPresets = await invoke<EqualizerPreset[]>("list_presets");
      if (loadedPresets && loadedPresets.length > 0) {
        setPresets(loadedPresets);
      }
    } catch (err) {
      const msg = err instanceof Error ? err.message : String(err);
      console.warn("Failed to load presets via Tauri IPC:", msg);
      setError(msg);
    } finally {
      setIsLoading(false);
    }
  }, []);

  const selectPreset = useCallback(async (presetId: string) => {
    setActivePresetId(presetId);

    if (!isTauri()) return;

    try {
      setError(null);
      const activated = await invoke<EqualizerPreset>("set_active_preset", {
        presetId,
      });
      if (activated) {
        setPresets((prev) =>
          prev.map((p) => (p.id === activated.id ? activated : p))
        );
      }
    } catch (err) {
      const msg = err instanceof Error ? err.message : String(err);
      console.error("Failed to set active preset via Tauri IPC:", msg);
      setError(msg);
    }
  }, []);

  const savePreset = useCallback(
    async (
      name: string,
      overwriteId?: string,
      customEq?: CustomEqData
    ): Promise<EqualizerPreset> => {
      const nowIso = new Date().toISOString();

      if (overwriteId) {
        const existing = presets.find((p) => p.id === overwriteId);
        const updatedPreset: EqualizerPreset = {
          id: overwriteId,
          name,
          isFactory: false,
          preampGainDb: customEq?.preampGainDb ?? activePreset.preampGainDb,
          bassBoost: customEq?.bassBoost
            ? { ...customEq.bassBoost }
            : { ...activePreset.bassBoost },
          bands: customEq?.bands
            ? customEq.bands.map((b) => ({ ...b }))
            : activePreset.bands.map((b) => ({ ...b })),
          createdAt: existing?.createdAt ?? nowIso,
          updatedAt: nowIso,
        };

        if (isTauri()) {
          try {
            setError(null);
            const saved = await invoke<EqualizerPreset>("save_preset", {
              preset: updatedPreset,
            });
            setPresets((prev) =>
              prev.map((p) => (p.id === saved.id ? saved : p))
            );
            setActivePresetId(saved.id);
            return saved;
          } catch (err) {
            const msg = err instanceof Error ? err.message : String(err);
            console.error("Failed to save preset via Tauri IPC:", msg);
            setError(msg);
            throw err;
          }
        } else {
          setPresets((prev) =>
            prev.map((p) => (p.id === overwriteId ? updatedPreset : p))
          );
          setActivePresetId(overwriteId);
          return updatedPreset;
        }
      } else {
        const newId = `user-${Date.now()}`;
        const newPreset: EqualizerPreset = {
          id: newId,
          name,
          isFactory: false,
          preampGainDb: customEq?.preampGainDb ?? activePreset.preampGainDb,
          bassBoost: customEq?.bassBoost
            ? { ...customEq.bassBoost }
            : { ...activePreset.bassBoost },
          bands: customEq?.bands
            ? customEq.bands.map((b) => ({ ...b }))
            : activePreset.bands.map((b) => ({ ...b })),
          createdAt: nowIso,
          updatedAt: nowIso,
        };

        if (isTauri()) {
          try {
            setError(null);
            const saved = await invoke<EqualizerPreset>("save_preset", {
              preset: newPreset,
            });
            setPresets((prev) => [...prev, saved]);
            setActivePresetId(saved.id);
            return saved;
          } catch (err) {
            const msg = err instanceof Error ? err.message : String(err);
            console.error("Failed to create preset via Tauri IPC:", msg);
            setError(msg);
            throw err;
          }
        } else {
          setPresets((prev) => [...prev, newPreset]);
          setActivePresetId(newId);
          return newPreset;
        }
      }
    },
    [activePreset, presets]
  );

  const deletePreset = useCallback(
    async (presetId: string) => {
      if (isTauri()) {
        try {
          setError(null);
          await invoke("delete_preset", { presetId });
          setPresets((prev) => prev.filter((p) => p.id !== presetId));
          if (activePresetId === presetId) {
            setActivePresetId("flat");
          }
        } catch (err) {
          const msg = err instanceof Error ? err.message : String(err);
          console.error("Failed to delete preset via Tauri IPC:", msg);
          setError(msg);
          throw err;
        }
      } else {
        setPresets((prev) => prev.filter((p) => p.id !== presetId));
        if (activePresetId === presetId) {
          setActivePresetId("flat");
        }
      }
    },
    [activePresetId]
  );

  return {
    presets,
    activePresetId,
    activePreset,
    isLoading,
    error,
    selectPreset,
    savePreset,
    deletePreset,
    reloadPresets,
  };
}

export default usePresets;
