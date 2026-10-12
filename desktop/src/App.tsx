import React from "react";
import { THEME_COLORS } from "./styles/theme";

export const App: React.FC = () => {
  return (
    <main className="flex flex-col items-center justify-center w-screen h-screen bg-surface-base text-text-primary p-6">
      <div className="flex flex-col items-center gap-6 p-8 rounded-lg bg-surface-panel border border-surface-border shadow-2xl max-w-lg w-full">
        {/* Header / Brand */}
        <div className="flex items-center gap-3">
          <div
            className="w-3.5 h-3.5 rounded-full bg-accent-cyan shadow-[0_0_12px_var(--accent-cyan)]"
            title={`Accent Color: ${THEME_COLORS.accentCyan}`}
          />
          <h1 className="text-xl font-bold tracking-wider uppercase m-0 text-text-primary font-sans">
            Equilizer
          </h1>
          <span className="text-[10px] font-semibold px-2 py-0.5 rounded bg-surface-card border border-surface-border text-text-secondary uppercase">
            DSP Workstation
          </span>
        </div>

        <p className="text-xs text-text-secondary text-center m-0">
          Precision Parametric Audio DSP &bull; System-wide WASAPI Loopback
        </p>

        {/* Surface and Telemetry Tokens Demo Card */}
        <div className="flex flex-col gap-3 w-full p-4 rounded-md bg-surface-card border border-surface-border-subtle">
          <div className="flex justify-between items-center text-xs">
            <span className="text-text-secondary uppercase tracking-widest font-semibold">
              Telemetry Status
            </span>
            <span className="font-mono text-accent-cyan text-xs">
              Latency: &lt;12.4 ms
            </span>
          </div>

          {/* Meter Color Badges */}
          <div className="grid grid-cols-3 gap-2 pt-2">
            <div className="flex items-center justify-center py-1.5 px-2 rounded bg-meter-normal/20 border border-meter-normal/40 text-meter-normal text-xs font-mono">
              -12.0 dBFS
            </div>
            <div className="flex items-center justify-center py-1.5 px-2 rounded bg-meter-warning/20 border border-meter-warning/40 text-meter-warning text-xs font-mono">
              -3.5 dBFS
            </div>
            <div className="flex items-center justify-center py-1.5 px-2 rounded bg-meter-clip/20 border border-meter-clip/40 text-meter-clip text-xs font-mono">
              CLIP / LIM
            </div>
          </div>
        </div>

        {/* Tabular Mono Readout Test */}
        <div className="flex items-center justify-between w-full text-xs text-text-muted border-t border-surface-border-subtle pt-3">
          <span>Theme Bridge: <code className="font-mono text-text-secondary">theme.ts</code></span>
          <span className="font-mono text-accent-cyan">Canvas: {THEME_COLORS.canvasSpline}</span>
        </div>
      </div>
    </main>
  );
};

export default App;
