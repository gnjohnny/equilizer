import React from "react";
import { Activity } from "lucide-react";
import { useAudioTelemetry } from "../../hooks/useAudioTelemetry";

export interface StereoPeakMeterProps {
  preampGainDb?: number;
  bassBoostGainDb?: number;
  isBypassed?: boolean;
  limiterEnabled?: boolean;
}

const MIN_DB = -48.0;

// Linear dB to percentage mapping (-48 dBFS = 0%, 0 dBFS = 100%)
function dbToPct(db: number): number {
  return Math.max(0, Math.min(100, ((db - MIN_DB) / (0 - MIN_DB)) * 100));
}

export const StereoPeakMeter: React.FC<StereoPeakMeterProps> = ({
  preampGainDb = 0.0,
  bassBoostGainDb = 0.0,
  isBypassed = false,
  limiterEnabled = true,
}) => {
  const {
    peakL,
    peakR,
    holdL,
    holdR,
    limiterActive,
    isClipping,
    clearClip,
  } = useAudioTelemetry({
    enabled: true,
    preampGainDb,
    bassBoostGainDb,
    isBypassed,
    limiterEnabled,
  });

  const pctL = dbToPct(peakL);
  const pctR = dbToPct(peakR);
  const holdPctL = dbToPct(holdL);
  const holdPctR = dbToPct(holdR);

  // Meter gradient fill: Green <= -12 dB (75%), Amber -12 to -3 dB (93.75%), Red -3 to 0 dBFS (100%)
  const meterGradient =
    "linear-gradient(to right, #10B981 0%, #10B981 75%, #F59E0B 75%, #F59E0B 93.75%, #EF4444 93.75%, #EF4444 100%)";

  return (
    <div className="flex flex-col justify-between p-3 rounded-lg bg-surface-card border border-surface-border-subtle shadow-sm select-none">
      {/* Header: Title, Telemetry Mode & Sticky Clip LED */}
      <div className="flex items-center justify-between">
        <div className="flex items-center gap-1.5">
          <div className="w-5 h-5 rounded flex items-center justify-center bg-surface-base border border-surface-border text-text-muted">
            <Activity
              className={`w-3 h-3 transition-colors ${
                isBypassed
                  ? "text-text-muted"
                  : limiterActive
                    ? "text-meter-warn animate-pulse"
                    : "text-accent-cyan"
              }`}
            />
          </div>
          <span className="text-xs font-semibold tracking-wider uppercase text-text-secondary">
            Stereo Output
          </span>
        </div>

        <div className="flex items-center gap-2">
          <span className="text-[10px] font-mono px-1.5 py-0.5 rounded bg-surface-base text-text-muted border border-surface-border">
            30 Hz Telemetry
          </span>

          <button
            type="button"
            onClick={clearClip}
            title={
              isClipping
                ? "CLIP DETECTED: Signal reached or exceeded 0 dBFS! Click to clear"
                : "No digital clipping detected"
            }
            className={`text-[9px] font-mono font-bold px-1.5 py-0.5 rounded border transition-all cursor-pointer ${
              isClipping
                ? "bg-red-500/20 border-red-500 text-red-400 shadow-[0_0_8px_rgba(239,68,68,0.6)] animate-pulse"
                : "bg-surface-base border-surface-border text-text-muted/60"
            }`}
          >
            CLIP
          </button>
        </div>
      </div>

      {/* Dual Channel Meters (Left & Right) */}
      <div
        className={`flex flex-col gap-2 transition-opacity ${
          isBypassed ? "opacity-40" : "opacity-100"
        }`}
      >
        {/* Left Channel */}
        <div className="flex items-center gap-2 text-[10px] font-mono">
          <span className="text-text-muted w-3 font-semibold">L</span>
          <div className="flex-1 h-2 rounded-sm bg-surface-base overflow-hidden border border-surface-border-subtle relative flex items-center">
            {/* Active Signal Bar */}
            <div
              className="h-full transition-[width] duration-75 ease-out"
              style={{
                width: `${pctL}%`,
                background: meterGradient,
              }}
            />
            {/* Peak-Hold Needle */}
            {holdPctL > 1 && (
              <div
                className="absolute top-0 bottom-0 w-0.5 bg-white shadow-[0_0_4px_rgba(255,255,255,0.8)] pointer-events-none transition-[left] duration-75"
                style={{ left: `calc(${holdPctL}% - 1px)` }}
              />
            )}
          </div>
          <span className="text-text-secondary w-14 text-right tabular-nums">
            {peakL <= MIN_DB ? "-∞ dB" : `${peakL.toFixed(1)} dB`}
          </span>
        </div>

        {/* Right Channel */}
        <div className="flex items-center gap-2 text-[10px] font-mono">
          <span className="text-text-muted w-3 font-semibold">R</span>
          <div className="flex-1 h-2 rounded-sm bg-surface-base overflow-hidden border border-surface-border-subtle relative flex items-center">
            {/* Active Signal Bar */}
            <div
              className="h-full transition-[width] duration-75 ease-out"
              style={{
                width: `${pctR}%`,
                background: meterGradient,
              }}
            />
            {/* Peak-Hold Needle */}
            {holdPctR > 1 && (
              <div
                className="absolute top-0 bottom-0 w-0.5 bg-white shadow-[0_0_4px_rgba(255,255,255,0.8)] pointer-events-none transition-[left] duration-75"
                style={{ left: `calc(${holdPctR}% - 1px)` }}
              />
            )}
          </div>
          <span className="text-text-secondary w-14 text-right tabular-nums">
            {peakR <= MIN_DB ? "-∞ dB" : `${peakR.toFixed(1)} dB`}
          </span>
        </div>
      </div>

      {/* Decibel Reference Tick Markings */}
      <div className="flex items-center justify-between text-[9px] font-mono text-text-muted pt-1 border-t border-surface-border-subtle/60 px-5">
        <span>-48 dB</span>
        <span>-24 dB</span>
        <span>-12 dB</span>
        <span>-6 dB</span>
        <span>-3 dB</span>
        <span className="text-text-secondary font-semibold">0 dBFS</span>
      </div>
    </div>
  );
};

export default StereoPeakMeter;
