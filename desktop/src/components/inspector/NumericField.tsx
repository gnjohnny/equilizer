import React, { useState, useRef } from "react";

export interface NumericFieldProps {
  label: string;
  value: number;
  unit: string;
  min: number;
  max: number;
  step: number;
  decimals?: number;
  disabled?: boolean;
  onChange: (newValue: number) => void;
}

export const NumericField: React.FC<NumericFieldProps> = ({
  label,
  value,
  unit,
  min,
  max,
  step,
  decimals = 0,
  disabled = false,
  onChange,
}) => {
  const [isEditing, setIsEditing] = useState(false);
  const [draft, setDraft] = useState("");
  const inputRef = useRef<HTMLInputElement>(null);

  const commitValue = (rawString: string) => {
    const cleaned = rawString.trim().replace(",", "");
    const parsed = parseFloat(cleaned);

    if (Number.isFinite(parsed)) {
      const clamped = Math.max(min, Math.min(max, parsed));
      const rounded =
        decimals === 0
          ? Math.round(clamped)
          : Number(clamped.toFixed(decimals));
      onChange(rounded);
      setDraft(
        decimals === 0 ? rounded.toString() : rounded.toFixed(decimals)
      );
    } else {
      // Revert to current value on invalid input
      setDraft(
        decimals === 0
          ? Math.round(value).toString()
          : value.toFixed(decimals)
      );
    }
    setIsEditing(false);
  };

  const handleKeyDown = (event: React.KeyboardEvent<HTMLInputElement>) => {
    if (event.key === "Enter") {
      event.preventDefault();
      commitValue(draft);
      inputRef.current?.blur();
    } else if (event.key === "Escape") {
      event.preventDefault();
      setDraft(
        decimals === 0
          ? Math.round(value).toString()
          : value.toFixed(decimals)
      );
      setIsEditing(false);
      inputRef.current?.blur();
    } else if (event.key === "ArrowUp") {
      event.preventDefault();
      const current = parseFloat(draft) || value;
      const next = Math.min(max, current + step);
      const rounded =
        decimals === 0 ? Math.round(next) : Number(next.toFixed(decimals));
      onChange(rounded);
      setDraft(
        decimals === 0 ? rounded.toString() : rounded.toFixed(decimals)
      );
    } else if (event.key === "ArrowDown") {
      event.preventDefault();
      const current = parseFloat(draft) || value;
      const next = Math.max(min, current - step);
      const rounded =
        decimals === 0 ? Math.round(next) : Number(next.toFixed(decimals));
      onChange(rounded);
      setDraft(
        decimals === 0 ? rounded.toString() : rounded.toFixed(decimals)
      );
    }
  };

  return (
    <div
      className={`flex items-center h-8 px-2 rounded-md border bg-surface-base font-mono text-xs transition-colors ${
        disabled
          ? "opacity-40 cursor-not-allowed border-surface-border"
          : isEditing
            ? "border-accent-cyan shadow-[0_0_8px_rgba(0,229,255,0.2)] bg-surface-panel"
            : "border-surface-border hover:border-slate-600"
      }`}
    >
      <span className="text-[10px] uppercase font-semibold text-text-muted mr-1.5 select-none">
        {label}:
      </span>

      <input
        ref={inputRef}
        type="text"
        disabled={disabled}
        value={
          disabled
            ? "N/A"
            : isEditing
              ? draft
              : decimals === 0
                ? Math.round(value).toLocaleString()
                : `${value >= 0 && label === "Gain" ? "+" : ""}${value.toFixed(decimals)}`
        }
        onFocus={() => {
          if (!disabled) {
            setIsEditing(true);
            setDraft(
              decimals === 0
                ? Math.round(value).toString()
                : value.toFixed(decimals)
            );
            setTimeout(() => inputRef.current?.select(), 10);
          }
        }}
        onBlur={() => commitValue(draft)}
        onChange={(e) => setDraft(e.target.value)}
        onKeyDown={handleKeyDown}
        className={`w-16 bg-transparent outline-none font-mono text-xs text-text-primary ${
          disabled ? "cursor-not-allowed text-text-muted" : "cursor-text"
        }`}
      />

      {unit && (
        <span className="text-[10px] text-text-muted ml-0.5 select-none font-mono">
          {unit}
        </span>
      )}
    </div>
  );
};

export default NumericField;
