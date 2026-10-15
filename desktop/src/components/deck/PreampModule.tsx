import React from "react";
import { SlidersHorizontal } from "lucide-react";

export interface PreampModuleProps {
  gainDb: number;
  isBypassed?: boolean;
  onUpdate: (gainDb: number) => void;
}

export const PreampModule: React.FC<PreampModuleProps> = ({
  gainDb,
  isBypassed = false,
  onUpdate,
}) => {
  const isEnabled = !isBypassed;
  const isBoosted = gainDb > 0.05;

  const handleGainChange = (e: React.ChangeEvent<HTMLInputElement>) => {
    const val = parseFloat(e.target.value);
    onUpdate(Math.max(-20, Math.min(6, val)));
  };

  const handleResetGain = () => {
    onUpdate(0.0);
  };

  // Pre-amp spans -20 to +6 (total 26 dB)
  // 0 dB is at (20 / 26) * 100 = 76.92%
  const detentPct = (20 / 26) * 100;
  const currentPct = Math.max(0, Math.min(100, ((gainDb + 20) / 26) * 100));

  return (
    <div className="flex flex-col justify-between p-3 rounded-lg bg-surface-card border border-surface-border-subtle shadow-sm select-none">
      {/* Header: Label & Value Badge */}
      <div className="flex items-center justify-between">
        <div className="flex items-center gap-1.5">
          <div className="w-5 h-5 rounded flex items-center justify-center bg-surface-base border border-surface-border text-text-muted">
            <SlidersHorizontal className="w-3 h-3" />
          </div>
          <span className="text-xs font-semibold tracking-wider uppercase text-text-secondary">
            Pre-Amp Gain
          </span>
        </div>

        <button
          type="button"
          onDoubleClick={handleResetGain}
          title="Double-click to reset gain to 0.0 dB (Unity)"
          className={`text-[10px] font-mono px-1.5 py-0.5 rounded border transition-colors cursor-pointer ${
            isBoosted
              ? "bg-meter-warn/20 border-meter-warn/50 text-meter-warn font-bold"
              : gainDb === 0
                ? "bg-surface-base border-surface-border text-text-primary font-semibold"
                : "bg-surface-base border-accent-cyan/40 text-accent-cyan"
          }`}
        >
          {gainDb > 0 ? `+${gainDb.toFixed(1)}` : gainDb.toFixed(1)} dB
        </button>
      </div>

      {/* Headroom Status Descriptor */}
      <div className="flex items-center justify-between px-0.5 text-[10px] font-mono leading-tight">
        {isBoosted ? (
          <span className="text-meter-warn flex items-center gap-1">
            <span className="w-1.5 h-1.5 rounded-full bg-meter-warn animate-pulse" />
            Reduced Headroom (+{gainDb.toFixed(1)} dB)
          </span>
        ) : gainDb < 0 ? (
          <span className="text-accent-emerald">
            Safe Headroom ({Math.abs(gainDb).toFixed(1)} dB Pad)
          </span>
        ) : (
          <span className="text-text-muted">Unity Gain (0 dBFS Reference)</span>
        )}
      </div>

      {/* Fader Track & 0 dB Center Detent */}
      <div
        className={`flex flex-col gap-1 transition-opacity ${
          isEnabled ? "opacity-100" : "opacity-40"
        }`}
      >
        <div className="relative flex items-center w-full h-4">
          {/* Visual 0 dB detent tick */}
          <div
            className="absolute top-0 bottom-0 w-0.5 bg-slate-500/70 z-0 pointer-events-none"
            style={{ left: `${detentPct}%` }}
            title="0 dB Unity Detent"
          />

          <input
            type="range"
            min={-20}
            max={6}
            step={0.5}
            disabled={!isEnabled}
            value={gainDb}
            onChange={handleGainChange}
            aria-label="Pre-Amp Gain"
            className={`w-full h-1.5 appearance-none rounded-full bg-surface-base border border-surface-border-subtle cursor-pointer focus:outline-none z-10 ${
              isBoosted ? "accent-meter-warn" : "accent-accent-cyan"
            }`}
            style={{
              background: isEnabled
                ? isBoosted
                  ? `linear-gradient(to right, #00E5FF 0%, #00E5FF ${detentPct}%, #F59E0B ${detentPct}%, #F59E0B ${currentPct}%, #0B0E14 ${currentPct}%, #0B0E14 100%)`
                  : `linear-gradient(to right, #00E5FF 0%, #00E5FF ${currentPct}%, #0B0E14 ${currentPct}%, #0B0E14 100%)`
                : "#0B0E14",
            }}
          />
        </div>

        {/* Quick-Jump Snap Labels */}
        <div className="flex items-center justify-between text-[9px] font-mono text-text-muted px-0.5">
          <button
            type="button"
            disabled={!isEnabled}
            onClick={() => onUpdate(-20)}
            className="hover:text-text-secondary cursor-pointer"
          >
            -20 dB
          </button>
          <button
            type="button"
            disabled={!isEnabled}
            onClick={() => onUpdate(-6)}
            className="hover:text-text-secondary cursor-pointer"
          >
            -6 dB
          </button>
          <button
            type="button"
            disabled={!isEnabled}
            onClick={() => onUpdate(0)}
            className={`font-semibold hover:text-text-primary cursor-pointer ${
              gainDb === 0 ? "text-text-primary underline" : "text-text-muted"
            }`}
          >
            0 dB
          </button>
          <button
            type="button"
            disabled={!isEnabled}
            onClick={() => onUpdate(3)}
            className="hover:text-text-secondary cursor-pointer"
          >
            +3 dB
          </button>
          <button
            type="button"
            disabled={!isEnabled}
            onClick={() => onUpdate(6)}
            className="hover:text-text-secondary cursor-pointer"
          >
            +6 dB
          </button>
        </div>
      </div>
    </div>
  );
};

export default PreampModule;
