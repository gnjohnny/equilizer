import React, { useState, useRef, useEffect, useCallback } from "react";
import { SlidersHorizontal, ChevronDown, Check, Plus, Save, Trash2 } from "lucide-react";
import { EqualizerPreset } from "../../types/audio";

export interface PresetDropdownProps {
  presets: EqualizerPreset[];
  selectedPresetId: string;
  onSelectPreset: (presetId: string) => void;
  onSavePreset?: () => void;
  onNewPreset?: () => void;
  onDeletePreset?: (preset: EqualizerPreset) => void;
}

export const PresetDropdown: React.FC<PresetDropdownProps> = ({
  presets,
  selectedPresetId,
  onSelectPreset,
  onSavePreset,
  onNewPreset,
  onDeletePreset,
}) => {
  const [isOpen, setIsOpen] = useState(false);
  const containerRef = useRef<HTMLDivElement>(null);

  const selectedPreset =
    presets.find((p) => p.id === selectedPresetId) ?? presets[0];

  const handleClose = useCallback(() => {
    setIsOpen(false);
  }, []);

  // Click-outside listener
  useEffect(() => {
    if (!isOpen) return;

    const handleClickOutside = (event: MouseEvent) => {
      if (
        containerRef.current &&
        !containerRef.current.contains(event.target as Node)
      ) {
        handleClose();
      }
    };

    const handleKeyDown = (event: KeyboardEvent) => {
      if (event.key === "Escape") {
        handleClose();
      }
    };

    document.addEventListener("mousedown", handleClickOutside);
    document.addEventListener("keydown", handleKeyDown);

    return () => {
      document.removeEventListener("mousedown", handleClickOutside);
      document.removeEventListener("keydown", handleKeyDown);
    };
  }, [isOpen, handleClose]);

  return (
    <div className="relative inline-block text-left" ref={containerRef}>
      {/* Trigger Button */}
      <button
        type="button"
        onClick={() => setIsOpen((prev) => !prev)}
        aria-haspopup="listbox"
        aria-expanded={isOpen}
        title={`Preset: ${selectedPreset ? selectedPreset.name : "Select Preset"}`}
        className={`h-8 px-2.5 rounded-md text-xs font-medium flex items-center gap-2 border transition-colors cursor-pointer select-none max-w-[120px] sm:max-w-[145px] xl:max-w-[180px] ${
          isOpen
            ? "bg-surface-card border-accent-cyan text-text-primary shadow-[0_0_8px_rgba(0,229,255,0.2)]"
            : "bg-surface-card/90 border-surface-border hover:border-slate-600 hover:bg-surface-card text-text-secondary hover:text-text-primary"
        }`}
      >
        <SlidersHorizontal className="w-3.5 h-3.5 flex-none text-accent-cyan" />
        <span className="truncate flex-1 text-left font-sans text-xs">
          {selectedPreset ? selectedPreset.name : "Select Preset"}
        </span>
        <ChevronDown
          className={`w-3 h-3 flex-none text-text-muted transition-transform duration-150 ${
            isOpen ? "rotate-180 text-text-primary" : ""
          }`}
        />
      </button>

      {/* Popover Menu */}
      {isOpen && (
        <div
          role="listbox"
          aria-label="Equalizer Presets"
          className="absolute top-full mt-1.5 left-0 z-50 min-w-[260px] bg-surface-panel border border-surface-border rounded-lg shadow-2xl shadow-black p-1.5 flex flex-col gap-1 backdrop-blur-md animate-in fade-in zoom-in-95 duration-100"
        >
          <div className="px-2 py-1 text-[10px] font-semibold uppercase tracking-wider text-text-muted border-b border-surface-border-subtle flex items-center justify-between">
            <span>Equalizer Presets</span>
            <span className="font-mono text-[9px] text-accent-cyan">
              {presets.length} Available
            </span>
          </div>

          {/* Preset list */}
          <div className="flex flex-col gap-0.5 max-h-56 overflow-y-auto">
            {presets.map((preset) => {
              const isSelected = preset.id === selectedPresetId;
              return (
                <button
                  key={preset.id}
                  type="button"
                  role="option"
                  aria-selected={isSelected}
                  onClick={() => {
                    onSelectPreset(preset.id);
                    handleClose();
                  }}
                  className={`w-full text-left px-2.5 py-1.5 rounded-md text-xs flex items-center justify-between gap-2 transition-colors cursor-pointer ${
                    isSelected
                      ? "bg-accent-cyan/15 text-accent-cyan font-medium border border-accent-cyan/30"
                      : "text-text-secondary hover:text-text-primary hover:bg-surface-card border border-transparent"
                  }`}
                >
                  <div className="flex items-center gap-2 truncate">
                    <Check
                      className={`w-3.5 h-3.5 flex-none ${
                        isSelected ? "opacity-100 text-accent-cyan" : "opacity-0"
                      }`}
                    />
                    <span className="truncate">{preset.name}</span>
                  </div>

                  <div className="flex items-center gap-1.5 flex-none">
                    <span
                      className={`text-[9px] font-mono px-1.5 py-0.2 rounded border uppercase ${
                        preset.isFactory
                          ? "bg-slate-800 text-slate-300 border-slate-700"
                          : "bg-accent-purple/20 text-accent-purple border-accent-purple/40"
                      }`}
                    >
                      {preset.isFactory ? "Factory" : "Custom"}
                    </span>

                    {!preset.isFactory && onDeletePreset && (
                      <span
                        role="button"
                        tabIndex={0}
                        title={`Delete preset "${preset.name}"`}
                        onClick={(e) => {
                          e.stopPropagation();
                          onDeletePreset(preset);
                          handleClose();
                        }}
                        onKeyDown={(e) => {
                          if (e.key === "Enter" || e.key === " ") {
                            e.stopPropagation();
                            onDeletePreset(preset);
                            handleClose();
                          }
                        }}
                        className="p-1 rounded text-text-muted hover:text-red-400 hover:bg-red-500/10 transition-colors cursor-pointer"
                      >
                        <Trash2 className="w-3 h-3" />
                      </span>
                    )}
                  </div>
                </button>
              );
            })}
          </div>

          {/* Action Footer */}
          <div className="pt-1.5 mt-0.5 border-t border-surface-border-subtle grid grid-cols-2 gap-1.5">
            <button
              type="button"
              onClick={() => {
                if (onNewPreset) onNewPreset();
                handleClose();
              }}
              className="px-2 py-1.5 rounded text-[11px] font-medium bg-surface-card border border-surface-border hover:border-slate-600 hover:bg-surface-card-hover text-text-secondary hover:text-text-primary flex items-center justify-center gap-1.5 cursor-pointer transition-colors"
            >
              <Plus className="w-3 h-3 text-accent-cyan" />
              <span>New Preset</span>
            </button>
            <button
              type="button"
              onClick={() => {
                if (onSavePreset) onSavePreset();
                handleClose();
              }}
              className="px-2 py-1.5 rounded text-[11px] font-medium bg-surface-card border border-surface-border hover:border-slate-600 hover:bg-surface-card-hover text-text-secondary hover:text-text-primary flex items-center justify-center gap-1.5 cursor-pointer transition-colors"
            >
              <Save className="w-3 h-3 text-accent-emerald" />
              <span>Save</span>
            </button>
          </div>
        </div>
      )}
    </div>
  );
};

export default PresetDropdown;
