import { useState, useEffect, useRef, useCallback } from "react";
import { invoke, isTauri } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { AudioDevice, MOCK_AUDIO_DEVICES } from "../types/audio";

export interface UseAudioDevicesReturn {
  devices: AudioDevice[];
  selectedDeviceId: string;
  selectDevice: (deviceId: string) => Promise<void>;
  isLoading: boolean;
  error: string | null;
  refreshDevices: () => Promise<void>;
}

/**
 * Hook for output audio endpoint enumeration and routing via Tauri IPC.
 * Gracefully falls back to mock endpoints in pure web or test environments.
 */
export function useAudioDevices(): UseAudioDevicesReturn {
  const [devices, setDevices] = useState<AudioDevice[]>(MOCK_AUDIO_DEVICES);
  const [selectedDeviceId, setSelectedDeviceId] = useState<string>("default");
  const [isLoading, setIsLoading] = useState<boolean>(() => isTauri());
  const [error, setError] = useState<string | null>(null);

  const selectedDeviceIdRef = useRef<string>(selectedDeviceId);
  useEffect(() => {
    selectedDeviceIdRef.current = selectedDeviceId;
  }, [selectedDeviceId]);

  // Synchronize with live OS audio endpoints on mount when running inside Tauri
  useEffect(() => {
    if (!isTauri()) return;

    let isMounted = true;
    invoke<AudioDevice[]>("get_audio_devices")
      .then((backendDevices) => {
        if (!isMounted) return;
        if (backendDevices && backendDevices.length > 0) {
          setDevices(backendDevices);
          const hasMatching = backendDevices.some(
            (d) => d.id === selectedDeviceIdRef.current
          );
          if (!hasMatching && selectedDeviceIdRef.current !== "default") {
            const defaultDev = backendDevices.find((d) => d.isDefault);
            setSelectedDeviceId(defaultDev ? defaultDev.id : "default");
          }
        }
      })
      .catch((err: unknown) => {
        if (!isMounted) return;
        const msg = err instanceof Error ? err.message : String(err);
        console.warn("Failed to enumerate audio devices from Tauri IPC:", msg);
        setError(msg);
      })
      .finally(() => {
        if (isMounted) setIsLoading(false);
      });

    return () => {
      isMounted = false;
    };
  }, []);

  const refreshDevices = useCallback(async () => {
    if (!isTauri()) return;

    try {
      setIsLoading(true);
      setError(null);
      const backendDevices = await invoke<AudioDevice[]>("get_audio_devices");
      if (backendDevices && backendDevices.length > 0) {
        setDevices(backendDevices);
        const hasMatching = backendDevices.some(
          (d) => d.id === selectedDeviceIdRef.current
        );
        if (!hasMatching && selectedDeviceIdRef.current !== "default") {
          const defaultDev = backendDevices.find((d) => d.isDefault);
          setSelectedDeviceId(defaultDev ? defaultDev.id : "default");
        }
      }
    } catch (err) {
      const msg = err instanceof Error ? err.message : String(err);
      console.warn("Failed to enumerate audio devices from Tauri IPC:", msg);
      setError(msg);
    } finally {
      setIsLoading(false);
    }
  }, []);

  // Listen for audio hardware disconnect / device change notifications
  useEffect(() => {
    if (!isTauri()) return;

    let unlisten: (() => void) | undefined;
    listen<{ disconnectedId: string; fallbackId: string }>(
      "audio:device-change",
      (event) => {
        console.warn(
          `Audio endpoint disconnected: ${event.payload.disconnectedId}. Falling back to ${event.payload.fallbackId}.`
        );
        setSelectedDeviceId("default");
        refreshDevices();
      }
    ).then((dispose) => {
      unlisten = dispose;
    });

    return () => {
      unlisten?.();
    };
  }, [refreshDevices]);

  const selectDevice = useCallback(async (deviceId: string) => {
    setSelectedDeviceId(deviceId);

    if (!isTauri()) return;

    try {
      setError(null);
      await invoke("set_output_device", {
        deviceId: deviceId === "default" ? null : deviceId,
      });
    } catch (err) {
      const msg = err instanceof Error ? err.message : String(err);
      console.error("Failed to set output device via Tauri IPC:", msg);
      setError(msg);
    }
  }, []);

  return {
    devices,
    selectedDeviceId,
    selectDevice,
    isLoading,
    error,
    refreshDevices,
  };
}

export default useAudioDevices;
