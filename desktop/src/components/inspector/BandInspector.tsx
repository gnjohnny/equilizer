import React from "react";
import { Power } from "lucide-react";
import { EqBand } from "../../types/audio";
import { FilterTypeSelect } from "./FilterTypeSelect";
import { NumericField } from "./NumericField";

export interface BandInspectorProps {
  bands: EqBand[];
  selectedBandId: number;
  onSelectBand: (bandId: number) => void;
  onUpdateBand: (updatedBand: EqBand) => void;
  isBypassed?: boolean;
}

export const BandInspector: React.FC<BandInspectorProps> = ({
  bands,
  selectedBandId,
  onSelectBand,
  onUpdateBand,
  isBypassed = false,
}) => {
  const selectedBand =
    bands.find((b) => b.id === selectedBandId) ??
    bands[3] ??
    bands[0];

  const isBandEnabled = selectedBand ? selectedBand.enabled && !isBypassed : false;

  // Gain is disabled for LowPass and HighPass filter topologies (unity passband filters)
  const isGainDisabled =
    !isBandEnabled ||
    selectedBand.filterType === "LowPass" ||
    selectedBand.filterType === "HighPass";

  const handleToggleBandEnabled = () => {
    if (!selectedBand) return;
    onUpdateBand({
      ...selectedBand,
      enabled: !selectedBand.enabled,
    });
  };

  return (
    <section className="h-20 flex-none border-t border-surface-border bg-surface-card px-4 flex items-center justify-between gap-4 z-10 select-none">
      {/* =====================================================================
          LEFT ZONE: Band Readout, Enable/Bypass Toggle & Filter Shape
          ===================================================================== */}
      <div className="flex items-center gap-3 pr-4 border-r border-surface-border-subtle flex-shrink-0">
        {/* Band Indicator & Toggle Button */}
        <div className="flex items-center gap-2">
          <button
            type="button"
            onClick={handleToggleBandEnabled}
            title={
              selectedBand.enabled
                ? "Band Active — Click to bypass this filter"
                : "Band Bypassed — Click to activate"
            }
            className={`w-7 h-7 rounded-md flex items-center justify-center border transition-all cursor-pointer ${
              selectedBand.enabled
                ? "bg-accent-emerald/20 text-accent-emerald border-accent-emerald/50 shadow-[0_0_8px_rgba(16,185,129,0.25)]"
                : "bg-surface-base text-text-muted border-surface-border hover:border-slate-600 hover:text-text-secondary"
            }`}
          >
            <Power className="w-3.5 h-3.5" />
          </button>

          <div className="flex flex-col">
            <span className="text-[10px] uppercase font-semibold tracking-wider text-text-muted">
              Active Node
            </span>
            <span className="text-xs font-bold text-accent-cyan font-mono">
              BAND {selectedBand.id.toString().padStart(2, "0")}
            </span>
          </div>
        </div>

        {/* Filter Shape Selector */}
        <FilterTypeSelect
          value={selectedBand.filterType}
          disabled={!isBandEnabled}
          onChange={(newType) =>
            onUpdateBand({
              ...selectedBand,
              filterType: newType,
            })
          }
        />
      </div>

      {/* =====================================================================
          CENTER ZONE: Surgical Numeric Controls (Frequency, Gain, Q-Factor)
          ===================================================================== */}
      <div
        className={`flex items-center gap-2.5 transition-opacity duration-150 flex-shrink-0 ${
          isBandEnabled ? "opacity-100" : "opacity-45"
        }`}
      >
        <NumericField
          label="Freq"
          value={selectedBand.frequencyHz}
          unit="Hz"
          min={20}
          max={20000}
          step={10}
          decimals={0}
          disabled={!isBandEnabled}
          onChange={(newFreq) =>
            onUpdateBand({
              ...selectedBand,
              frequencyHz: newFreq,
            })
          }
        />

        <NumericField
          label="Gain"
          value={selectedBand.gainDb}
          unit="dB"
          min={-24.0}
          max={24.0}
          step={0.5}
          decimals={1}
          disabled={isGainDisabled}
          onChange={(newGain) =>
            onUpdateBand({
              ...selectedBand,
              gainDb: newGain,
            })
          }
        />

        <NumericField
          label="Q"
          value={selectedBand.q}
          unit=""
          min={0.1}
          max={10.0}
          step={0.05}
          decimals={2}
          disabled={!isBandEnabled}
          onChange={(newQ) =>
            onUpdateBand({
              ...selectedBand,
              q: newQ,
            })
          }
        />
      </div>

      {/* =====================================================================
          RIGHT ZONE: 10-Band Chip Navigation Strip
          ===================================================================== */}
      <div className="flex-1 flex items-center justify-end gap-1.5 overflow-hidden">
        {bands.map((band) => {
          const isSelected = band.id === selectedBandId;
          const isEnabled = band.enabled;

          return (
            <button
              key={band.id}
              type="button"
              onClick={() => onSelectBand(band.id)}
              title={`Band ${band.id}: ${Math.round(band.frequencyHz)} Hz, ${
                band.gainDb >= 0 ? "+" : ""
              }${band.gainDb.toFixed(1)} dB, Q ${band.q.toFixed(2)} (${
                band.enabled ? "Active" : "Bypassed"
              })`}
              className={`flex flex-col items-center justify-center px-2 py-1 rounded border text-[10px] font-mono transition-all cursor-pointer ${
                isSelected
                  ? "bg-accent-cyan/15 border-accent-cyan text-accent-cyan shadow-[0_0_8px_rgba(0,229,255,0.2)] font-bold"
                  : isEnabled
                    ? "bg-surface-base border-surface-border text-text-muted hover:border-slate-600 hover:text-text-secondary"
                    : "bg-surface-base/50 border-surface-border/50 text-text-muted/40 line-through"
              }`}
            >
              <span className="font-semibold">{band.id}</span>
              <span className="text-[9px]">
                {band.frequencyHz >= 1000
                  ? `${(band.frequencyHz / 1000).toFixed(
                      band.frequencyHz % 1000 === 0 ? 0 : 1
                    )}k`
                  : `${Math.round(band.frequencyHz)}`}
              </span>
            </button>
          );
        })}
      </div>
    </section>
  );
};

export default BandInspector;
