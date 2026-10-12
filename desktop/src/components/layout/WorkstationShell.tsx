import React from "react";
import { THEME_COLORS } from "../../styles/theme";

export const WorkstationShell: React.FC = () => {
  return (
    <div className="flex flex-col w-screen h-screen overflow-hidden bg-surface-base text-text-primary select-none">
      {/* =========================================================================
          TIER 1: HEADER BAR (h-14 / 56px Fixed)
          ========================================================================= */}
      <header className="h-14 flex-none border-b border-surface-border bg-surface-panel/90 backdrop-blur px-4 flex items-center justify-between z-10">
        {/* Left: Branding & Workstation Status */}
        <div className="flex items-center gap-3">
          <div
            className="w-3.5 h-3.5 rounded-full bg-accent-cyan shadow-[0_0_12px_var(--accent-cyan)]"
            title={`Accent Spline: ${THEME_COLORS.accentCyan}`}
          />
          <span className="text-base font-bold tracking-wider uppercase font-sans text-text-primary">
            Equilizer
          </span>
          <span className="text-[10px] font-semibold tracking-wider px-2 py-0.5 rounded bg-surface-card border border-surface-border text-text-secondary uppercase">
            DSP Workstation
          </span>
          <span className="text-xs px-2.5 py-0.5 rounded bg-surface-base border border-surface-border text-accent-cyan font-mono">
            Preset: Flat
          </span>
        </div>

        {/* Center: Engine & Latency Readout */}
        <div className="hidden md:flex items-center gap-4 text-xs">
          <span className="text-text-muted">
            Engine: <span className="text-text-secondary font-mono">Direct Form II Transposed</span>
          </span>
          <div className="h-3 w-px bg-surface-border" />
          <span className="text-text-muted">
            Round-Trip Latency: <span className="text-accent-cyan font-mono">&lt; 12.4 ms</span>
          </span>
          <div className="h-3 w-px bg-surface-border" />
          <span className="text-text-muted">
            WASAPI Loopback: <span className="text-accent-emerald font-mono">Active (48 kHz)</span>
          </span>
        </div>

        {/* Right: Master Bypass & Actions Placeholder */}
        <div className="flex items-center gap-2">
          <button
            type="button"
            className="px-3 py-1.5 rounded-md text-xs font-semibold bg-surface-card border border-surface-border hover:border-accent-cyan/50 hover:bg-surface-card-hover text-text-secondary hover:text-text-primary transition-colors cursor-pointer"
          >
            Presets
          </button>
          <button
            type="button"
            className="px-3 py-1.5 rounded-md text-xs font-bold bg-slate-800/80 border border-slate-700 text-slate-300 hover:text-white transition-colors cursor-pointer"
          >
            BYPASS
          </button>
          <div className="w-2 h-2 rounded-full bg-accent-emerald shadow-[0_0_8px_var(--accent-emerald)] ml-1" title="DSP Active" />
        </div>
      </header>

      {/* =========================================================================
          TIER 2: CANVAS STAGE (flex-1 / Dynamic Height, Min 320px)
          ========================================================================= */}
      <section className="flex-1 min-h-[320px] relative overflow-hidden bg-surface-base p-3 flex flex-col">
        <div className="relative flex-1 w-full h-full rounded-lg border border-surface-border/60 bg-gradient-to-b from-[#0C101A] to-[#080B10] flex flex-col justify-between p-4 overflow-hidden shadow-inner">
          {/* Frequency & Gain Grid Reference Indicators */}
          <div className="flex justify-between items-center text-[10px] text-text-muted font-mono border-b border-surface-border-subtle pb-2">
            <span>+24.0 dB</span>
            <span className="uppercase tracking-widest text-[9px]">Parametric Frequency Response Stage</span>
            <span>0.0 dB</span>
          </div>

          {/* Centered Graphic Curve Placeholder Frame */}
          <div className="flex flex-col items-center justify-center gap-2 text-center my-auto">
            <div className="flex items-center gap-2 px-3 py-1 rounded-full bg-surface-panel/80 border border-surface-border">
              <span className="w-2 h-2 rounded-full bg-accent-cyan animate-pulse" />
              <span className="text-xs font-semibold text-text-primary uppercase tracking-wider font-sans">
                Interactive Canvas Curve Stage
              </span>
              <span className="text-[10px] font-mono text-accent-cyan">Step 09 &bull; Step 10</span>
            </div>
            <p className="text-xs text-text-secondary max-w-md m-0">
              60 FPS Direct Form II Transposed spline visualization &bull; 10 draggable filter nodes &bull; Mouse-wheel Q adjustment
            </p>
            <div className="flex items-center gap-3 mt-2 text-[11px] font-mono text-text-muted">
              <span>View: 20 Hz – 20 kHz</span>
              <span>&bull;</span>
              <span>Scale: Logarithmic</span>
              <span>&bull;</span>
              <span>Zero-Scrollbar Viewport: Locked</span>
            </div>
          </div>

          {/* Octave Guide Markers */}
          <div className="flex justify-between items-center text-[10px] text-text-muted font-mono border-t border-surface-border-subtle pt-2">
            <span>20 Hz</span>
            <span>60 Hz</span>
            <span>125 Hz</span>
            <span>250 Hz</span>
            <span>500 Hz</span>
            <span>1 kHz</span>
            <span>2 kHz</span>
            <span>4 kHz</span>
            <span>8 kHz</span>
            <span>16 kHz</span>
            <span>20 kHz</span>
          </div>
        </div>
      </section>

      {/* =========================================================================
          TIER 3: BAND INSPECTOR STRIP (h-20 / 80px Fixed)
          ========================================================================= */}
      <section className="h-20 flex-none border-t border-surface-border bg-surface-card px-4 flex items-center justify-between gap-4 z-10">
        {/* Left: Active Band Micro-Controls Preview */}
        <div className="flex items-center gap-3 pr-3 border-r border-surface-border-subtle">
          <div className="flex flex-col">
            <span className="text-[10px] uppercase font-semibold tracking-wider text-text-muted">Selected</span>
            <span className="text-xs font-bold text-accent-cyan font-mono">BAND 04</span>
          </div>
          <div className="flex items-center gap-2 text-xs font-mono">
            <div className="px-2 py-1 rounded bg-surface-base border border-surface-border text-text-secondary">
              Type: <span className="text-text-primary">Peaking</span>
            </div>
            <div className="px-2 py-1 rounded bg-surface-base border border-surface-border text-text-secondary">
              Freq: <span className="text-text-primary">1,000 Hz</span>
            </div>
            <div className="px-2 py-1 rounded bg-surface-base border border-surface-border text-text-secondary">
              Gain: <span className="text-text-primary">+0.0 dB</span>
            </div>
            <div className="px-2 py-1 rounded bg-surface-base border border-surface-border text-text-secondary">
              Q: <span className="text-text-primary">1.00</span>
            </div>
          </div>
        </div>

        {/* Right: 10-Band Selector Strip */}
        <div className="flex-1 flex items-center justify-end gap-1.5 overflow-hidden">
          {[
            { id: 1, freq: "32 Hz" },
            { id: 2, freq: "64 Hz" },
            { id: 3, freq: "125 Hz" },
            { id: 4, freq: "250 Hz" },
            { id: 5, freq: "500 Hz" },
            { id: 6, freq: "1 kHz" },
            { id: 7, freq: "2 kHz" },
            { id: 8, freq: "4 kHz" },
            { id: 9, freq: "8 kHz" },
            { id: 10, freq: "16 kHz" },
          ].map((band) => (
            <div
              key={band.id}
              className={`flex flex-col items-center justify-center px-2 py-1 rounded border text-[10px] font-mono transition-colors cursor-pointer ${
                band.id === 4
                  ? "bg-accent-cyan/15 border-accent-cyan text-accent-cyan"
                  : "bg-surface-base border-surface-border text-text-muted hover:border-surface-border-subtle hover:text-text-secondary"
              }`}
            >
              <span className="font-semibold">{band.id}</span>
              <span className="text-[9px]">{band.freq}</span>
            </div>
          ))}
        </div>
      </section>

      {/* =========================================================================
          TIER 4: BOTTOM CONTROL DECK (h-44 / 176px Fixed)
          ========================================================================= */}
      <footer className="h-44 flex-none border-t border-surface-border bg-surface-panel px-4 py-3 grid grid-cols-4 gap-4 z-10">
        {/* Module 1: Dedicated Bass Boost */}
        <div className="flex flex-col justify-between p-3 rounded-lg bg-surface-card border border-surface-border-subtle shadow-sm">
          <div className="flex items-center justify-between">
            <span className="text-xs font-semibold tracking-wider uppercase text-text-secondary">Bass Boost</span>
            <span className="text-[10px] font-mono px-1.5 py-0.5 rounded bg-surface-base text-accent-cyan border border-surface-border">
              +0.0 dB
            </span>
          </div>
          <div className="flex items-center justify-center gap-2 py-1">
            <button
              type="button"
              className="flex-1 py-1 text-[11px] font-semibold rounded bg-surface-base border border-accent-cyan/50 text-accent-cyan"
            >
              Sub-Bass (45Hz)
            </button>
            <button
              type="button"
              className="flex-1 py-1 text-[11px] font-semibold rounded bg-surface-base border border-surface-border text-text-muted"
            >
              Punch (90Hz)
            </button>
          </div>
          <div className="w-full h-1.5 rounded-full bg-surface-base overflow-hidden border border-surface-border-subtle">
            <div className="h-full bg-accent-cyan w-0" />
          </div>
        </div>

        {/* Module 2: Pre-Amp Gain Attenuation */}
        <div className="flex flex-col justify-between p-3 rounded-lg bg-surface-card border border-surface-border-subtle shadow-sm">
          <div className="flex items-center justify-between">
            <span className="text-xs font-semibold tracking-wider uppercase text-text-secondary">Pre-Amp Gain</span>
            <span className="text-[10px] font-mono px-1.5 py-0.5 rounded bg-surface-base text-text-primary border border-surface-border">
              0.0 dB
            </span>
          </div>
          <div className="flex items-center justify-between text-[10px] font-mono text-text-muted px-1">
            <span>-20 dB</span>
            <span>0 dB</span>
            <span>+6 dB</span>
          </div>
          <div className="w-full h-1.5 rounded-full bg-surface-base overflow-hidden border border-surface-border-subtle">
            <div className="h-full bg-slate-300 w-1/2" />
          </div>
        </div>

        {/* Module 3: Dynamics & Headroom Limiter */}
        <div className="flex flex-col justify-between p-3 rounded-lg bg-surface-card border border-surface-border-subtle shadow-sm">
          <div className="flex items-center justify-between">
            <span className="text-xs font-semibold tracking-wider uppercase text-text-secondary">Limiter / Safety</span>
            <span className="text-[10px] font-mono px-1.5 py-0.5 rounded bg-meter-normal/20 border border-meter-normal/40 text-meter-normal">
              ENGAGED
            </span>
          </div>
          <p className="text-[11px] text-text-muted m-0 leading-tight">
            Soft-knee brickwall limiter active. Guards against 0 dBFS inter-sample clipping.
          </p>
          <div className="flex items-center justify-between text-[10px] font-mono text-text-muted">
            <span>Threshold: -0.1 dBFS</span>
            <span>Release: 50 ms</span>
          </div>
        </div>

        {/* Module 4: Stereo Peak VU Meters */}
        <div className="flex flex-col justify-between p-3 rounded-lg bg-surface-card border border-surface-border-subtle shadow-sm">
          <div className="flex items-center justify-between">
            <span className="text-xs font-semibold tracking-wider uppercase text-text-secondary">Stereo Output</span>
            <span className="text-[10px] font-mono px-1.5 py-0.5 rounded bg-surface-base text-text-muted border border-surface-border">
              30 Hz Telemetry
            </span>
          </div>
          {/* Dual Channel Meter Preview */}
          <div className="flex flex-col gap-1.5">
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
          <div className="flex items-center justify-between text-[9px] font-mono text-text-muted">
            <span>-48 dB</span>
            <span>-12 dB</span>
            <span>-3 dB</span>
            <span className="text-meter-clip font-semibold">CLIP</span>
          </div>
        </div>
      </footer>
    </div>
  );
};

export default WorkstationShell;
