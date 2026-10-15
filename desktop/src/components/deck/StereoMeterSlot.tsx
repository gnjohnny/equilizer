import React from "react";
import { Activity } from "lucide-react";

export interface StereoMeterSlotProps {
  isBypassed?: boolean;
}

export const StereoMeterSlot: React.FC<StereoMeterSlotProps> = ({
  isBypassed = false,
}) => {
  return (
    <div className="flex flex-col justify-between p-3 rounded-lg bg-surface-card border border-surface-border-subtle shadow-sm select-none">
      <div className="flex items-center justify-between">
        <div className="flex items-center gap-1.5">
          <div className="w-5 h-5 rounded flex items-center justify-center bg-surface-base border border-surface-border text-text-muted">
            <Activity className="w-3 h-3 text-accent-cyan" />
          </div>
          <span className="text-xs font-semibold tracking-wider uppercase text-text-secondary">
            Stereo Output
          </span>
        </div>
        <span className="text-[10px] font-mono px-1.5 py-0.5 rounded bg-surface-base text-text-muted border border-surface-border">
          30 Hz Telemetry
        </span>
      </div>

      {/* Dual Channel Meter Preview */}
      <div
        className={`flex flex-col gap-1.5 transition-opacity ${
          isBypassed ? "opacity-40" : "opacity-100"
        }`}
      >
        <div className="flex items-center gap-2 text-[10px] font-mono">
          <span className="text-text-muted w-3">L</span>
          <div className="flex-1 h-2 rounded bg-surface-base overflow-hidden border border-surface-border-subtle flex">
            <div className="h-full bg-meter-normal w-3/5" />
          </div>
          <span className="text-text-secondary w-14 text-right">-12.0 dB</span>
        </div>
        <div className="flex items-center gap-2 text-[10px] font-mono">
          <span className="text-text-muted w-3">R</span>
          <div className="flex-1 h-2 rounded bg-surface-base overflow-hidden border border-surface-border-subtle flex">
            <div className="h-full bg-meter-normal w-3/5" />
          </div>
          <span className="text-text-secondary w-14 text-right">-12.0 dB</span>
        </div>
      </div>

      <div className="flex items-center justify-between text-[9px] font-mono text-text-muted pt-1 border-t border-surface-border-subtle/60">
        <span>-48 dB</span>
        <span>-12 dB</span>
        <span>-3 dB</span>
        <span className="text-meter-clip font-semibold">CLIP</span>
      </div>
    </div>
  );
};

export default StereoMeterSlot;
