import React, { useState, useEffect, useRef } from "react";
import { Bookmark, X, AlertCircle, AlertTriangle, Check } from "lucide-react";
import { EqualizerPreset } from "../../types/audio";

export interface SavePresetModalProps {
  isOpen: boolean;
  currentPreset: EqualizerPreset;
  existingPresets: EqualizerPreset[];
  onClose: () => void;
  onSave: (presetName: string, overwriteId?: string) => void;
}

// Characters forbidden in Windows filenames to prevent path traversal in local persistence
const FORBIDDEN_CHARS_REGEX = /[\\/:*?"<>|]/g;

export const SavePresetModal: React.FC<SavePresetModalProps> = ({
  isOpen,
  currentPreset,
  existingPresets,
  onClose,
  onSave,
}) => {
  const [name, setName] = useState(() =>
    currentPreset.isFactory
      ? `${currentPreset.name} (Custom)`
      : currentPreset.name
  );
  const inputRef = useRef<HTMLInputElement>(null);

  // Auto-focus and select text on mount
  useEffect(() => {
    if (!isOpen) return;
    const timer = setTimeout(() => {
      if (inputRef.current) {
        inputRef.current.focus();
        inputRef.current.select();
      }
    }, 30);
    return () => clearTimeout(timer);
  }, [isOpen]);

  // Handle keyboard shortcuts (Escape to dismiss, Enter to save)
  useEffect(() => {
    if (!isOpen) return;

    const handleKeyDown = (e: KeyboardEvent) => {
      if (e.key === "Escape") {
        e.preventDefault();
        onClose();
      }
    };

    window.addEventListener("keydown", handleKeyDown);
    return () => window.removeEventListener("keydown", handleKeyDown);
  }, [isOpen, onClose]);

  if (!isOpen) return null;

  const sanitized = name.replace(FORBIDDEN_CHARS_REGEX, "").trim();
  const matchedPreset = existingPresets.find(
    (p) => p.name.trim().toLowerCase() === sanitized.toLowerCase()
  );

  const isFactoryConflict = matchedPreset?.isFactory ?? false;
  const isCustomOverwrite = matchedPreset && !matchedPreset.isFactory;
  const isInvalidLength = sanitized.length === 0 || sanitized.length > 32;

  const handleSubmit = (e: React.FormEvent) => {
    e.preventDefault();
    if (isInvalidLength || isFactoryConflict) return;
    onSave(sanitized, isCustomOverwrite ? matchedPreset.id : undefined);
    onClose();
  };

  const enabledBandsCount = currentPreset.bands.filter((b) => b.enabled).length;

  return (
    <div
      role="dialog"
      aria-modal="true"
      aria-labelledby="save-preset-title"
      className="fixed inset-0 z-50 flex items-center justify-center p-4 bg-black/70 backdrop-blur-sm select-none animate-in fade-in duration-100"
      onClick={(e) => {
        if (e.target === e.currentTarget) onClose();
      }}
    >
      <div className="w-full max-w-md rounded-xl bg-surface-panel border border-surface-border shadow-2xl p-5 flex flex-col gap-4 animate-in zoom-in-95 duration-150">
        {/* Header */}
        <div className="flex items-center justify-between border-b border-surface-border-subtle pb-3">
          <div className="flex items-center gap-2">
            <div className="w-7 h-7 rounded-md flex items-center justify-center bg-accent-cyan/15 text-accent-cyan border border-accent-cyan/30">
              <Bookmark className="w-4 h-4" />
            </div>
            <div>
              <h2
                id="save-preset-title"
                className="text-sm font-bold text-text-primary uppercase tracking-wider font-sans m-0"
              >
                Save Equalizer Preset
              </h2>
              <p className="text-[11px] text-text-muted m-0 font-mono">
                Store current EQ curves and dynamics settings
              </p>
            </div>
          </div>

          <button
            type="button"
            onClick={onClose}
            className="w-7 h-7 rounded-md flex items-center justify-center text-text-muted hover:text-text-primary hover:bg-surface-card transition-colors cursor-pointer"
          >
            <X className="w-4 h-4" />
          </button>
        </div>

        {/* Form Body */}
        <form onSubmit={handleSubmit} className="flex flex-col gap-4">
          <div className="flex flex-col gap-1.5">
            <label
              htmlFor="preset-name-input"
              className="text-[11px] font-semibold uppercase tracking-wider text-text-secondary flex items-center justify-between"
            >
              <span>Preset Name</span>
              <span className="font-mono text-[10px] text-text-muted">
                {name.length}/32 chars
              </span>
            </label>

            <div className="relative">
              <input
                id="preset-name-input"
                ref={inputRef}
                type="text"
                maxLength={32}
                value={name}
                onChange={(e) => {
                  // Filter out forbidden characters immediately
                  const clean = e.target.value.replace(FORBIDDEN_CHARS_REGEX, "");
                  setName(clean);
                }}
                placeholder="e.g. Acoustic Master, Bass Cannon"
                className={`w-full h-9 px-3 rounded-md bg-surface-base border font-sans text-xs text-text-primary outline-none transition-colors ${
                  isFactoryConflict
                    ? "border-red-500 focus:border-red-500"
                    : isCustomOverwrite
                      ? "border-meter-warn focus:border-meter-warn"
                      : "border-surface-border focus:border-accent-cyan"
                }`}
              />
            </div>

            {/* Validation / Overwrite Advisories */}
            {isFactoryConflict ? (
              <div className="flex items-center gap-1.5 text-[11px] text-red-400 font-medium">
                <AlertCircle className="w-3.5 h-3.5 flex-none" />
                <span>Cannot overwrite factory preset. Please choose another name.</span>
              </div>
            ) : isCustomOverwrite ? (
              <div className="flex items-center gap-1.5 text-[11px] text-meter-warn font-medium">
                <AlertTriangle className="w-3.5 h-3.5 flex-none" />
                <span>
                  Preset &ldquo;{matchedPreset.name}&rdquo; will be updated with current settings.
                </span>
              </div>
            ) : (
              <span className="text-[10px] text-text-muted">
                Saved locally to your workstation preset catalog.
              </span>
            )}
          </div>

          {/* Current Settings Summary Pill */}
          <div className="rounded-md bg-surface-base/80 border border-surface-border-subtle p-2.5 flex items-center justify-between text-[11px] font-mono text-text-muted">
            <div className="flex items-center gap-1.5">
              <span className="text-text-secondary font-semibold">Bands:</span>
              <span className="text-accent-cyan">{enabledBandsCount} Active</span>
            </div>
            <div className="flex items-center gap-1.5">
              <span className="text-text-secondary font-semibold">Pre-Amp:</span>
              <span className="text-text-primary">
                {currentPreset.preampGainDb >= 0 ? "+" : ""}
                {currentPreset.preampGainDb.toFixed(1)} dB
              </span>
            </div>
            <div className="flex items-center gap-1.5">
              <span className="text-text-secondary font-semibold">Bass:</span>
              <span className="text-accent-cyan">
                {currentPreset.bassBoost.enabled
                  ? `+${currentPreset.bassBoost.gainDb.toFixed(1)} dB`
                  : "Off"}
              </span>
            </div>
          </div>

          {/* Action Buttons */}
          <div className="flex items-center justify-end gap-2 pt-2 border-t border-surface-border-subtle">
            <button
              type="button"
              onClick={onClose}
              className="px-3 py-1.5 rounded-md text-xs font-medium text-text-secondary hover:text-text-primary hover:bg-surface-card border border-surface-border transition-colors cursor-pointer"
            >
              Cancel
            </button>

            <button
              type="submit"
              disabled={isInvalidLength || isFactoryConflict}
              className={`px-4 py-1.5 rounded-md text-xs font-semibold flex items-center gap-1.5 transition-all cursor-pointer ${
                isInvalidLength || isFactoryConflict
                  ? "opacity-40 cursor-not-allowed bg-surface-card border border-surface-border text-text-muted"
                  : isCustomOverwrite
                    ? "bg-meter-warn/20 text-meter-warn border border-meter-warn/50 hover:bg-meter-warn/30 shadow-[0_0_8px_rgba(245,158,11,0.25)]"
                    : "bg-accent-cyan/20 text-accent-cyan border border-accent-cyan/50 hover:bg-accent-cyan/30 shadow-[0_0_8px_rgba(0,229,255,0.25)]"
              }`}
            >
              <Check className="w-3.5 h-3.5" />
              <span>{isCustomOverwrite ? "Update Preset" : "Save Preset"}</span>
            </button>
          </div>
        </form>
      </div>
    </div>
  );
};

export default SavePresetModal;
