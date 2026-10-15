import React from "react";
import { Power } from "lucide-react";
import { BassBoostConfig, BassBoostMode } from "../../types/audio";

export interface BassBoostModuleProps {
  config: BassBoostConfig;
  isBypassed?: boolean;
  onUpdate: (newConfig: BassBoostConfig) => void;
}

export const BassBoostModule: React.FC<BassBoostModuleProps> = ({
  config,
  isBypassed = false,
  onUpdate,
}) => {
  const isEnabled = config.enabled && !isBypassed;

  const handleTogglePower = () => {
    onUpdate({
      ...config,
      enabled: !config.enabled,
    });
  };

  const handleModeChange = (newMode: BassBoostMode) => {
    onUpdate({
      ...config,
      mode: newMode,
      frequencyHz: newMode === "SubBass" ? 45.0 : 90.0,
    });
  };

  const handleGainChange = (e: React.ChangeEvent<HTMLInputElement>) => {
    const val = parseFloat(e.target.value);
    onUpdate({
      ...config,
      gainDb: Math.max(0, Math.min(12, val)),
    });
  };

  const handleResetGain = () => {
    onUpdate({
      ...config,
      gainDb: 0.0,
    });
  };

  // Percentage for track gradient fill (0 dB = 0%, 12 dB = 100%)
  const fillPct = Math.max(0, Math.min(100, (config.gainDb / 12) * 100));

  return (
    <div className="flex flex-col justify-between p-3 rounded-lg bg-surface-card border border-surface-border-subtle shadow-sm select-none">
      {/* Header: Module Label, Power Switch, and Value Badge */}
      <div className="flex items-center justify-between">
        <div className="flex items-center gap-1.5">
          <button
            type="button"
            onClick={handleTogglePower}
            disabled={isBypassed}
            title={
              isBypassed
                ? "Master Bypass active"
                : config.enabled
                  ? "Bass Boost Active — Click to bypass"
                  : "Bass Boost Inactive — Click to enable"
            }
            className={`w-5 h-5 rounded flex items-center justify-center border transition-all cursor-pointer ${
              isEnabled
                ? "bg-accent-emerald/20 text-accent-emerald border-accent-emerald/50 shadow-[0_0_6px_rgba(16,185,129,0.3)]"
                : "bg-surface-base text-text-muted border-surface-border hover:border-slate-600 hover:text-text-secondary"
            }`}
          >
            <Power className="w-3 h-3" />
          </button>
          <span className="text-xs font-semibold tracking-wider uppercase text-text-secondary">
            Bass Boost
          </span>
        </div>

        <button
          type="button"
          onDoubleClick={handleResetGain}
          title="Double-click to reset gain to 0.0 dB"
          className={`text-[10px] font-mono px-1.5 py-0.5 rounded border transition-colors cursor-pointer ${
            isEnabled && config.gainDb > 0
              ? "bg-accent-cyan/15 border-accent-cyan/50 text-accent-cyan font-bold"
              : "bg-surface-base border-surface-border text-text-muted"
          }`}
        >
          {config.gainDb > 0 ? `+${config.gainDb.toFixed(1)}` : "0.0"} dB
        </button>
      </div>

      {/* Dual Mode Switcher Pills */}
      <div
        className={`flex items-center justify-center gap-2 py-1 transition-opacity ${
          isEnabled ? "opacity-100" : "opacity-40"
        }`}
      >
        <button
          type="button"
          disabled={!isEnabled}
          onClick={() => handleModeChange("SubBass")}
          className={`flex-1 py-1 text-[11px] font-semibold rounded border transition-all cursor-pointer ${
            config.mode === "SubBass"
              ? "bg-surface-base border-accent-cyan/60 text-accent-cyan shadow-[0_0_8px_rgba(0,229,255,0.15)]"
              : "bg-surface-base border-surface-border text-text-muted hover:border-slate-600 hover:text-text-secondary"
          }`}
        >
          Sub-Bass (45Hz)
        </button>
        <button
          type="button"
          disabled={!isEnabled}
          onClick={() => handleModeChange("Punch")}
          className={`flex-1 py-1 text-[11px] font-semibold rounded border transition-all cursor-pointer ${
            config.mode === "Punch"
              ? "bg-surface-base border-accent-cyan/60 text-accent-cyan shadow-[0_0_8px_rgba(0,229,255,0.15)]"
              : "bg-surface-base border-surface-border text-text-muted hover:border-slate-600 hover:text-text-secondary"
          }`}
        >
          Punch (90Hz)
        </button>
      </div>

      {/* Gain Slider & Scale Labels */}
      <div
        className={`flex flex-col gap-1 transition-opacity ${
          isEnabled ? "opacity-100" : "opacity-40"
        }`}
      >
        <div className="relative flex items-center w-full h-4">
          <input
            type="range"
            min={0}
            max={12}
            step={0.5}
            disabled={!isEnabled}
            value={config.gainDb}
            onChange={handleGainChange}
            aria-label="Bass Boost Gain"
            className="w-full h-1.5 appearance-none rounded-full bg-surface-base border border-surface-border-subtle cursor-pointer accent-accent-cyan focus:outline-none"
            style={{
              background: isEnabled
                ? `linear-gradient(to right, #00E5FF 0%, #00E5FF ${fillPct}%, #0B0E14 ${fillPct}%, #0B0E14 100%)`
                : "#0B0E14",
            }}
          />
        </div>

        <div className="flex items-center justify-between text-[9px] font-mono text-text-muted px-0.5">
          <button
            type="button"
            disabled={!isEnabled}
            onClick={() => onUpdate({ ...config, gainDb: 0 })}
            className="hover:text-text-secondary cursor-pointer"
          >
            0 dB
          </button>
          <button
            type="button"
            disabled={!isEnabled}
            onClick={() => onUpdate({ ...config, gainDb: 6 })}
            className="hover:text-text-secondary cursor-pointer"
          >
            +6 dB
          </button>
          <button
            type="button"
            disabled={!isEnabled}
            onClick={() => onUpdate({ ...config, gainDb: 12 })}
            className="hover:text-text-secondary cursor-pointer"
          >
            +12 dB
          </button>
        </div>
      </div>
    </div>
  );
};

export default BassBoostModule;
