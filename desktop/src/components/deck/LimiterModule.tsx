import React from "react";
import { ShieldCheck, ShieldAlert, ShieldOff } from "lucide-react";

export interface LimiterModuleProps {
  enabled: boolean;
  preampGainDb: number;
  isBypassed?: boolean;
  onToggle: () => void;
}

export const LimiterModule: React.FC<LimiterModuleProps> = ({
  enabled,
  preampGainDb,
  isBypassed = false,
  onToggle,
}) => {
  const isLimiterArmed = enabled && !isBypassed;
  const isHighRisk = preampGainDb > 0.05;

  return (
    <div className="flex flex-col justify-between p-3 rounded-lg bg-surface-card border border-surface-border-subtle shadow-sm select-none">
      {/* Header: Label & Status Badge */}
      <div className="flex items-center justify-between">
        <div className="flex items-center gap-1.5">
          <div
            className={`w-5 h-5 rounded flex items-center justify-center border ${
              !isLimiterArmed
                ? "bg-surface-base border-red-500/30 text-red-400"
                : isHighRisk
                  ? "bg-meter-warn/20 border-meter-warn/50 text-meter-warn"
                  : "bg-surface-base border-accent-emerald/40 text-accent-emerald"
            }`}
          >
            {!isLimiterArmed ? (
              <ShieldOff className="w-3 h-3" />
            ) : isHighRisk ? (
              <ShieldAlert className="w-3 h-3" />
            ) : (
              <ShieldCheck className="w-3 h-3" />
            )}
          </div>
          <span className="text-xs font-semibold tracking-wider uppercase text-text-secondary">
            Limiter / Safety
          </span>
        </div>

        <button
          type="button"
          onClick={onToggle}
          disabled={isBypassed}
          title={
            isBypassed
              ? "Master Bypass active"
              : isLimiterArmed
                ? "Click to bypass brickwall limiter"
                : "Click to arm brickwall limiter"
          }
          className={`text-[10px] font-mono px-2 py-0.5 rounded border transition-all cursor-pointer ${
            !isLimiterArmed
              ? "bg-red-500/10 border-red-500/30 text-red-400 font-medium hover:border-red-500/50"
              : isHighRisk
                ? "bg-meter-warn/20 border-meter-warn/50 text-meter-warn font-bold shadow-[0_0_8px_rgba(245,158,11,0.25)]"
                : "bg-meter-normal/20 border-meter-normal/40 text-meter-normal font-semibold hover:border-meter-normal/60"
          }`}
        >
          {!isLimiterArmed
            ? "BYPASS"
            : isHighRisk
              ? "ACTIVE GUARD"
              : "ARMED"}
        </button>
      </div>

      {/* Descriptive Explainer */}
      <p className="text-[11px] text-text-muted m-0 leading-tight">
        {!isLimiterArmed ? (
          <span className="text-red-400/90 font-medium">
            Protection bypassed. Boosted signals may induce 0 dBFS clipping.
          </span>
        ) : isHighRisk ? (
          <span className="text-meter-warn">
            Active headroom compression guarding output against inter-sample peaks.
          </span>
        ) : (
          "Soft-knee brickwall limiter active. Guards against 0 dBFS clipping."
        )}
      </p>

      {/* Safety Invariant Specifications */}
      <div className="flex items-center justify-between text-[10px] font-mono text-text-muted pt-1 border-t border-surface-border-subtle/60">
        <span>Ceiling: -0.1 dBFS</span>
        <span>Release: 50 ms</span>
        <span>Knee: Soft (0.2 dB)</span>
      </div>
    </div>
  );
};

export default LimiterModule;
