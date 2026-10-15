import React from "react";
import { Activity, Minus, X, Power } from "lucide-react";
import { AudioDevice, EqualizerPreset } from "../../types/audio";
import { DeviceDropdown } from "./DeviceDropdown";
import { PresetDropdown } from "./PresetDropdown";

export interface HeaderBarProps {
  isBypassed: boolean;
  onToggleBypass: () => void;
  presets: EqualizerPreset[];
  selectedPresetId: string;
  onSelectPreset: (presetId: string) => void;
  devices: AudioDevice[];
  selectedDeviceId: string;
  onSelectDevice: (deviceId: string) => void;
  onSavePreset?: () => void;
  onNewPreset?: () => void;
  onDeletePreset?: (preset: EqualizerPreset) => void;
}

export const HeaderBar: React.FC<HeaderBarProps> = ({
  isBypassed,
  onToggleBypass,
  presets,
  selectedPresetId,
  onSelectPreset,
  devices,
  selectedDeviceId,
  onSelectDevice,
  onSavePreset,
  onNewPreset,
  onDeletePreset,
}) => {
  const handleMinimize = async () => {
    try {
      if (typeof window !== "undefined" && "__TAURI_INTERNALS__" in window) {
        const { getCurrentWindow } = await import("@tauri-apps/api/window");
        await getCurrentWindow().minimize();
      } else {
        console.log("[Window] Minimize clicked (browser preview mode)");
      }
    } catch (err) {
      console.warn("Could not minimize window:", err);
    }
  };

  const handleClose = async () => {
    try {
      if (typeof window !== "undefined" && "__TAURI_INTERNALS__" in window) {
        const { getCurrentWindow } = await import("@tauri-apps/api/window");
        await getCurrentWindow().close();
      } else {
        console.log("[Window] Close clicked (browser preview mode)");
      }
    } catch (err) {
      console.warn("Could not close window:", err);
    }
  };

  return (
    <header className="h-14 flex-none border-b border-surface-border bg-surface-panel/90 backdrop-blur px-4 flex items-center justify-between z-20 select-none">
      {/* =====================================================================
          LEFT SECTION: Branding & Hardware / Preset Selectors
          ===================================================================== */}
      <div className="flex-shrink-0 flex items-center gap-2.5">
        {/* Hardware Status Indicator & App Brand */}
        <div className="flex items-center gap-2">
          <div className="relative flex items-center justify-center">
            <Activity className="w-5 h-5 text-accent-cyan" />
            <span
              className={`absolute -top-0.5 -right-0.5 w-2 h-2 rounded-full ${
                isBypassed
                  ? "bg-amber-400 shadow-[0_0_8px_#F59E0B]"
                  : "bg-accent-emerald shadow-[0_0_8px_var(--accent-emerald)]"
              }`}
              title={isBypassed ? "DSP Bypassed" : "DSP Active"}
            />
          </div>
          <span className="text-base font-bold tracking-wider uppercase font-sans text-text-primary">
            Equilizer
          </span>
          <span className="hidden 2xl:inline-block text-[10px] font-semibold tracking-wider px-2 py-0.5 rounded bg-surface-card border border-surface-border text-text-secondary uppercase">
            DSP Workstation
          </span>
        </div>

        <div className="h-4 w-px bg-surface-border hidden 2xl:block" />

        {/* Dropdown Selectors */}
        <div className="flex items-center gap-2">
          <DeviceDropdown
            devices={devices}
            selectedDeviceId={selectedDeviceId}
            onSelectDevice={onSelectDevice}
          />
          <PresetDropdown
            presets={presets}
            selectedPresetId={selectedPresetId}
            onSelectPreset={onSelectPreset}
            onSavePreset={onSavePreset}
            onNewPreset={onNewPreset}
            onDeletePreset={onDeletePreset}
          />
        </div>
      </div>

      {/* =====================================================================
          CENTER SECTION: Engine & Latency Telemetry Readout (Responsive & No-Wrap)
          ===================================================================== */}
      <div className="flex-1 flex items-center justify-center min-w-0 px-2 overflow-hidden">
        {/* Expanded View for Wide Displays (xl: 1280px+) */}
        <div className="hidden xl:flex items-center gap-2.5 text-xs whitespace-nowrap">
          <div className="flex items-center gap-1.5 px-2.5 py-1 rounded bg-surface-base/80 border border-surface-border font-mono text-[11px]">
            <span className="text-text-muted">Engine:</span>
            <span className="text-text-secondary">DF-II Transposed</span>
          </div>
          <div className="flex items-center gap-1.5 px-2.5 py-1 rounded bg-surface-base/80 border border-surface-border font-mono text-[11px]">
            <span className="text-text-muted">Latency:</span>
            <span className="text-accent-cyan font-bold">&lt; 12.4 ms</span>
          </div>
          <div className="flex items-center gap-1.5 px-2.5 py-1 rounded bg-surface-base/80 border border-surface-border font-mono text-[11px]">
            <span className="text-text-muted">WASAPI:</span>
            <span
              className={
                isBypassed
                  ? "text-amber-400 font-semibold"
                  : "text-accent-emerald font-semibold"
              }
            >
              {isBypassed ? "Bypassed" : "Active (48 kHz)"}
            </span>
          </div>
        </div>

        {/* Compact Streamlined Hardware Badge for Standard Desktops (md to lg: 960px - 1279px) */}
        <div className="hidden md:flex xl:hidden items-center gap-2 text-xs whitespace-nowrap">
          <div className="flex items-center gap-2 px-2.5 py-1 rounded bg-surface-base/80 border border-surface-border font-mono text-[11px]">
            <span className="text-text-muted">Latency:</span>
            <span className="text-accent-cyan font-bold">&lt; 12.4 ms</span>
            <span className="text-surface-border">•</span>
            <span className="text-text-muted">WASAPI:</span>
            <span
              className={
                isBypassed
                  ? "text-amber-400 font-semibold"
                  : "text-accent-emerald font-semibold"
              }
            >
              {isBypassed ? "Bypassed" : "48 kHz"}
            </span>
          </div>
        </div>
      </div>

      {/* =====================================================================
          RIGHT SECTION: Master Bypass & Window Controls
          ===================================================================== */}
      <div className="flex-shrink-0 flex items-center gap-2.5">
        {/* Master Bypass Toggle */}
        <button
          type="button"
          onClick={onToggleBypass}
          title={
            isBypassed
              ? "DSP Bypassed — Audio is unaltered. Click to engage EQ."
              : "DSP Active — Audio is equalized. Click to bypass."
          }
          className={`h-8 px-3 rounded-md text-xs font-semibold flex items-center gap-2 border transition-all cursor-pointer whitespace-nowrap ${
            isBypassed
              ? "bg-amber-500/20 text-amber-300 border-amber-500/60 shadow-[0_0_12px_rgba(245,158,11,0.25)] hover:bg-amber-500/30"
              : "bg-surface-card text-text-secondary border-surface-border hover:border-slate-600 hover:text-text-primary hover:bg-surface-card-hover"
          }`}
        >
          <Power
            className={`w-3.5 h-3.5 ${
              isBypassed ? "text-amber-400" : "text-text-muted"
            }`}
          />
          <span>{isBypassed ? "BYPASSED" : "BYPASS"}</span>
          <span
            className={`w-2 h-2 rounded-full transition-colors ${
              isBypassed
                ? "bg-amber-400 shadow-[0_0_6px_#F59E0B]"
                : "bg-text-muted"
            }`}
          />
        </button>

        <div className="h-4 w-px bg-surface-border" />

        {/* Window Controls (Minimize to Tray / Close) */}
        <div className="flex items-center gap-1">
          <button
            type="button"
            onClick={handleMinimize}
            title="Minimize to Tray"
            className="w-7 h-7 rounded flex items-center justify-center text-text-muted hover:text-text-primary hover:bg-surface-card transition-colors cursor-pointer"
          >
            <Minus className="w-3.5 h-3.5" />
          </button>
          <button
            type="button"
            onClick={handleClose}
            title="Close Application"
            className="w-7 h-7 rounded flex items-center justify-center text-text-muted hover:text-rose-400 hover:bg-rose-500/10 transition-colors cursor-pointer"
          >
            <X className="w-3.5 h-3.5" />
          </button>
        </div>
      </div>
    </header>
  );
};

export default HeaderBar;
