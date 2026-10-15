import React, { useState } from "react";
import { HeaderBar } from "../header/HeaderBar";
import { ParametricCanvas } from "../canvas/ParametricCanvas";
import { FACTORY_PRESETS, MOCK_AUDIO_DEVICES, EqualizerPreset } from "../../types/audio";

export const WorkstationShell: React.FC = () => {
  const [isBypassed, setIsBypassed] = useState<boolean>(false);
  const [selectedPresetId, setSelectedPresetId] = useState<string>("flat");
  const [selectedDeviceId, setSelectedDeviceId] = useState<string>("default");
  const [selectedBandId, setSelectedBandId] = useState<number>(4);
  const [presets] = useState<EqualizerPreset[]>(FACTORY_PRESETS);

  const activePreset =
    presets.find((p) => p.id === selectedPresetId) ?? presets[0];
  const selectedBand =
    activePreset.bands.find((b) => b.id === selectedBandId) ??
    activePreset.bands[3] ??
    activePreset.bands[0];

  return (
    <div className="flex flex-col w-screen h-screen overflow-hidden bg-surface-base text-text-primary select-none">
      {/* =========================================================================
          TIER 1: HEADER BAR (h-14 / 56px Fixed)
          ========================================================================= */}
      <HeaderBar
        isBypassed={isBypassed}
        onToggleBypass={() => setIsBypassed((prev) => !prev)}
        presets={presets}
        selectedPresetId={selectedPresetId}
        onSelectPreset={(presetId) => setSelectedPresetId(presetId)}
        devices={MOCK_AUDIO_DEVICES}
        selectedDeviceId={selectedDeviceId}
        onSelectDevice={(deviceId) => setSelectedDeviceId(deviceId)}
        onSavePreset={() => {
          console.log("[Presets] Save preset requested for:", selectedPresetId);
        }}
        onNewPreset={() => {
          console.log("[Presets] New preset requested");
        }}
      />

      {/* =========================================================================
          TIER 2: CANVAS STAGE (flex-1 / Dynamic Height, Min 320px)
          ========================================================================= */}
      <section className="flex-1 min-h-[320px] relative overflow-hidden bg-surface-base p-3 flex flex-col">
        <ParametricCanvas
          bands={activePreset.bands}
          selectedBandId={selectedBandId}
          onSelectBand={(bandId) => setSelectedBandId(bandId)}
          preampGainDb={activePreset.preampGainDb}
          isBypassed={isBypassed}
        />
      </section>

      {/* =========================================================================
          TIER 3: BAND INSPECTOR STRIP (h-20 / 80px Fixed)
          ========================================================================= */}
      <section className="h-20 flex-none border-t border-surface-border bg-surface-card px-4 flex items-center justify-between gap-4 z-10">
        {/* Left: Active Band Micro-Controls Preview */}
        <div className="flex items-center gap-3 pr-3 border-r border-surface-border-subtle">
          <div className="flex flex-col">
            <span className="text-[10px] uppercase font-semibold tracking-wider text-text-muted">
              Selected
            </span>
            <span className="text-xs font-bold text-accent-cyan font-mono">
              BAND {selectedBand?.id.toString().padStart(2, "0")}
            </span>
          </div>
          <div className="flex items-center gap-2 text-xs font-mono">
            <div className="px-2 py-1 rounded bg-surface-base border border-surface-border text-text-secondary">
              Type: <span className="text-text-primary">{selectedBand?.filterType}</span>
            </div>
            <div className="px-2 py-1 rounded bg-surface-base border border-surface-border text-text-secondary">
              Freq:{" "}
              <span className="text-text-primary">
                {selectedBand ? Math.round(selectedBand.frequencyHz).toLocaleString() : 0} Hz
              </span>
            </div>
            <div className="px-2 py-1 rounded bg-surface-base border border-surface-border text-text-secondary">
              Gain:{" "}
              <span className="text-text-primary">
                {selectedBand && selectedBand.gainDb >= 0
                  ? `+${selectedBand.gainDb.toFixed(1)}`
                  : selectedBand?.gainDb.toFixed(1)}{" "}
                dB
              </span>
            </div>
            <div className="px-2 py-1 rounded bg-surface-base border border-surface-border text-text-secondary">
              Q: <span className="text-text-primary">{selectedBand?.q.toFixed(2)}</span>
            </div>
          </div>
        </div>

        {/* Right: 10-Band Selector Strip */}
        <div className="flex-1 flex items-center justify-end gap-1.5 overflow-hidden">
          {activePreset.bands.map((band) => (
            <div
              key={band.id}
              onClick={() => setSelectedBandId(band.id)}
              className={`flex flex-col items-center justify-center px-2 py-1 rounded border text-[10px] font-mono transition-colors cursor-pointer ${
                band.id === selectedBandId
                  ? "bg-accent-cyan/15 border-accent-cyan text-accent-cyan shadow-[0_0_8px_rgba(0,229,255,0.2)]"
                  : "bg-surface-base border-surface-border text-text-muted hover:border-surface-border-subtle hover:text-text-secondary"
              }`}
            >
              <span className="font-semibold">{band.id}</span>
              <span className="text-[9px]">
                {band.frequencyHz >= 1000
                  ? `${(band.frequencyHz / 1000).toFixed(band.frequencyHz % 1000 === 0 ? 0 : 1)}k`
                  : `${Math.round(band.frequencyHz)}`}
              </span>
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
