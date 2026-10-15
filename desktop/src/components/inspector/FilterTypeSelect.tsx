import React, { useState, useRef, useEffect, useCallback } from "react";
import { ChevronDown, Check } from "lucide-react";
import { FilterType } from "../../types/audio";

export interface FilterTypeSelectProps {
  value: FilterType;
  onChange: (newType: FilterType) => void;
  disabled?: boolean;
}

interface FilterOption {
  type: FilterType;
  label: string;
  shortLabel: string;
}

const FILTER_OPTIONS: FilterOption[] = [
  { type: "Peaking", label: "Peaking (Bell)", shortLabel: "Bell" },
  { type: "LowShelf", label: "Low Shelf", shortLabel: "Low Shelf" },
  { type: "HighShelf", label: "High Shelf", shortLabel: "High Shelf" },
  { type: "HighPass", label: "High Pass (Low Cut)", shortLabel: "High Pass" },
  { type: "LowPass", label: "Low Pass (High Cut)", shortLabel: "Low Pass" },
];

/**
 * Mini SVG graphic representation of filter slope shapes
 */
const FilterShapeIcon: React.FC<{ type: FilterType; className?: string }> = ({
  type,
  className = "w-3.5 h-3.5",
}) => {
  switch (type) {
    case "Peaking":
      return (
        <svg
          viewBox="0 0 24 24"
          fill="none"
          stroke="currentColor"
          strokeWidth="2.5"
          className={className}
        >
          <path d="M2 18 C8 18 10 6 12 6 C14 6 16 18 22 18" strokeLinecap="round" />
        </svg>
      );
    case "LowShelf":
      return (
        <svg
          viewBox="0 0 24 24"
          fill="none"
          stroke="currentColor"
          strokeWidth="2.5"
          className={className}
        >
          <path d="M2 8 L8 8 C11 8 13 18 16 18 L22 18" strokeLinecap="round" />
        </svg>
      );
    case "HighShelf":
      return (
        <svg
          viewBox="0 0 24 24"
          fill="none"
          stroke="currentColor"
          strokeWidth="2.5"
          className={className}
        >
          <path d="M2 18 L8 18 C11 18 13 8 16 8 L22 8" strokeLinecap="round" />
        </svg>
      );
    case "HighPass":
      return (
        <svg
          viewBox="0 0 24 24"
          fill="none"
          stroke="currentColor"
          strokeWidth="2.5"
          className={className}
        >
          <path d="M2 20 C6 20 10 18 13 10 L22 10" strokeLinecap="round" />
        </svg>
      );
    case "LowPass":
      return (
        <svg
          viewBox="0 0 24 24"
          fill="none"
          stroke="currentColor"
          strokeWidth="2.5"
          className={className}
        >
          <path d="M2 10 L11 10 C14 18 18 20 22 20" strokeLinecap="round" />
        </svg>
      );
  }
};

export const FilterTypeSelect: React.FC<FilterTypeSelectProps> = ({
  value,
  onChange,
  disabled = false,
}) => {
  const [isOpen, setIsOpen] = useState(false);
  const containerRef = useRef<HTMLDivElement>(null);

  const selectedOption = FILTER_OPTIONS.find((o) => o.type === value) ?? FILTER_OPTIONS[0];

  const handleClose = useCallback(() => {
    setIsOpen(false);
  }, []);

  useEffect(() => {
    if (!isOpen) return;

    const handleClickOutside = (event: MouseEvent) => {
      if (containerRef.current && !containerRef.current.contains(event.target as Node)) {
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
        disabled={disabled}
        onClick={() => !disabled && setIsOpen((prev) => !prev)}
        aria-haspopup="listbox"
        aria-expanded={isOpen}
        title={`Filter Shape: ${selectedOption.label}`}
        className={`h-8 px-2.5 rounded-md text-xs font-medium flex items-center gap-2 border transition-all select-none ${
          disabled
            ? "opacity-40 cursor-not-allowed bg-surface-base border-surface-border text-text-muted"
            : isOpen
              ? "bg-surface-panel border-accent-cyan text-text-primary shadow-[0_0_8px_rgba(0,229,255,0.25)] cursor-pointer"
              : "bg-surface-base border-surface-border hover:border-slate-600 hover:text-text-primary text-text-secondary cursor-pointer"
        }`}
      >
        <FilterShapeIcon type={value} className="w-3.5 h-3.5 flex-none text-accent-cyan" />
        <span className="truncate font-sans font-medium text-xs">{selectedOption.shortLabel}</span>
        <ChevronDown
          className={`w-3 h-3 flex-none text-text-muted transition-transform duration-150 ${
            isOpen ? "rotate-180 text-text-primary" : ""
          }`}
        />
      </button>

      {/* Popover Dropdown (Opens upward above the inspector) */}
      {isOpen && (
        <div
          role="listbox"
          aria-label="Filter Topologies"
          className="absolute bottom-full mb-1.5 left-0 z-50 min-w-52.5 bg-surface-panel border border-surface-border rounded-lg shadow-2xl shadow-black p-1.5 flex flex-col gap-0.5 backdrop-blur-md animate-in fade-in zoom-in-95 duration-100"
        >
          <div className="px-2 py-1 text-[10px] font-semibold uppercase tracking-wider text-text-muted border-b border-surface-border-subtle">
            Filter Topology Shape
          </div>

          <div className="flex flex-col gap-0.5 pt-1">
            {FILTER_OPTIONS.map((option) => {
              const isSelected = option.type === value;
              return (
                <button
                  key={option.type}
                  type="button"
                  role="option"
                  aria-selected={isSelected}
                  onClick={() => {
                    onChange(option.type);
                    handleClose();
                  }}
                  className={`w-full text-left px-2.5 py-1.5 rounded-md text-xs flex items-center justify-between gap-2 transition-colors cursor-pointer ${
                    isSelected
                      ? "bg-accent-cyan/15 text-accent-cyan font-medium border border-accent-cyan/30"
                      : "text-text-secondary hover:text-text-primary hover:bg-surface-card border border-transparent"
                  }`}
                >
                  <div className="flex items-center gap-2 truncate">
                    <FilterShapeIcon
                      type={option.type}
                      className={`w-3.5 h-3.5 flex-none ${
                        isSelected ? "text-accent-cyan" : "text-text-muted"
                      }`}
                    />
                    <span className="truncate">{option.label}</span>
                  </div>

                  <Check
                    className={`w-3.5 h-3.5 flex-none ${
                      isSelected ? "opacity-100 text-accent-cyan" : "opacity-0"
                    }`}
                  />
                </button>
              );
            })}
          </div>
        </div>
      )}
    </div>
  );
};

export default FilterTypeSelect;
