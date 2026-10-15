import React, { useEffect } from "react";
import { AlertTriangle, Trash2, X } from "lucide-react";
import { EqualizerPreset } from "../../types/audio";

export interface DeleteConfirmModalProps {
  isOpen: boolean;
  preset: EqualizerPreset | null;
  onClose: () => void;
  onConfirmDelete: (presetId: string) => void;
}

export const DeleteConfirmModal: React.FC<DeleteConfirmModalProps> = ({
  isOpen,
  preset,
  onClose,
  onConfirmDelete,
}) => {
  // Handle keyboard shortcuts (Escape to cancel, Enter to confirm deletion)
  useEffect(() => {
    if (!isOpen) return;

    const handleKeyDown = (e: KeyboardEvent) => {
      if (e.key === "Escape") {
        e.preventDefault();
        onClose();
      } else if (e.key === "Enter" && preset && !preset.isFactory) {
        e.preventDefault();
        onConfirmDelete(preset.id);
        onClose();
      }
    };

    window.addEventListener("keydown", handleKeyDown);
    return () => window.removeEventListener("keydown", handleKeyDown);
  }, [isOpen, preset, onClose, onConfirmDelete]);

  if (!isOpen || !preset || preset.isFactory) return null;

  const handleDelete = () => {
    onConfirmDelete(preset.id);
    onClose();
  };

  return (
    <div
      role="alertdialog"
      aria-modal="true"
      aria-labelledby="delete-preset-title"
      className="fixed inset-0 z-50 flex items-center justify-center p-4 bg-black/70 backdrop-blur-sm select-none animate-in fade-in duration-100"
      onClick={(e) => {
        if (e.target === e.currentTarget) onClose();
      }}
    >
      <div className="w-full max-w-md rounded-xl bg-surface-panel border border-surface-border shadow-2xl p-5 flex flex-col gap-4 animate-in zoom-in-95 duration-150">
        {/* Header */}
        <div className="flex items-center justify-between border-b border-surface-border-subtle pb-3">
          <div className="flex items-center gap-2">
            <div className="w-7 h-7 rounded-md flex items-center justify-center bg-red-500/15 text-red-400 border border-red-500/30">
              <AlertTriangle className="w-4 h-4" />
            </div>
            <div>
              <h2
                id="delete-preset-title"
                className="text-sm font-bold text-text-primary uppercase tracking-wider font-sans m-0"
              >
                Delete Custom Preset
              </h2>
              <p className="text-[11px] text-text-muted m-0 font-mono">
                Permanent deletion confirmation
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

        {/* Body Message */}
        <div className="flex flex-col gap-2 text-xs text-text-secondary leading-relaxed">
          <p className="m-0">
            Are you sure you want to permanently delete the custom preset{" "}
            <span className="font-bold text-text-primary font-mono text-sm px-1.5 py-0.5 rounded bg-surface-base border border-surface-border">
              &ldquo;{preset.name}&rdquo;
            </span>
            ?
          </p>

          <p className="m-0 text-text-muted text-[11px]">
            This action cannot be undone. All 10 parametric filter bands, pre-amp attenuation, and
            bass boost parameters saved in this preset will be permanently discarded.
          </p>
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
            type="button"
            onClick={handleDelete}
            className="px-4 py-1.5 rounded-md text-xs font-semibold flex items-center gap-1.5 bg-red-500/20 text-red-400 border border-red-500/40 hover:bg-red-500/30 shadow-[0_0_8px_rgba(239,68,68,0.25)] transition-all cursor-pointer"
          >
            <Trash2 className="w-3.5 h-3.5" />
            <span>Delete Preset</span>
          </button>
        </div>
      </div>
    </div>
  );
};

export default DeleteConfirmModal;
