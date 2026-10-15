import React, { useState, useRef, useEffect, useCallback } from "react";
import { Volume2, ChevronDown, Check } from "lucide-react";
import { AudioDevice } from "../../types/audio";

export interface DeviceDropdownProps {
  devices: AudioDevice[];
  selectedDeviceId: string;
  onSelectDevice: (deviceId: string) => void;
}

export const DeviceDropdown: React.FC<DeviceDropdownProps> = ({
  devices,
  selectedDeviceId,
  onSelectDevice,
}) => {
  const [isOpen, setIsOpen] = useState(false);
  const containerRef = useRef<HTMLDivElement>(null);

  const selectedDevice = devices.find((d) => d.id === selectedDeviceId) ?? devices[0];

  const handleClose = useCallback(() => {
    setIsOpen(false);
  }, []);

  // Click-outside listener
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
        onClick={() => setIsOpen((prev) => !prev)}
        aria-haspopup="listbox"
        aria-expanded={isOpen}
        title={selectedDevice ? selectedDevice.name : "Select Device"}
        className={`h-8 px-2.5 rounded-md text-xs font-medium flex items-center gap-2 border transition-colors cursor-pointer select-none max-w-40 sm:max-w-46.25 xl:max-w-52.5 ${
          isOpen
            ? "bg-surface-card border-accent-cyan text-text-primary shadow-[0_0_8px_rgba(0,229,255,0.2)]"
            : "bg-surface-card/90 border-surface-border hover:border-slate-600 hover:bg-surface-card text-text-secondary hover:text-text-primary"
        }`}
      >
        <Volume2 className="w-3.5 h-3.5 flex-none text-accent-cyan" />
        <span className="truncate flex-1 text-left font-sans text-xs">
          {selectedDevice ? selectedDevice.name : "Select Device"}
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
          aria-label="Audio Output Devices"
          className="absolute top-full mt-1.5 left-0 z-50 min-w-70 bg-surface-panel border border-surface-border rounded-lg shadow-2xl shadow-black p-1.5 flex flex-col gap-1 backdrop-blur-md animate-in fade-in zoom-in-95 duration-100"
        >
          <div className="px-2 py-1 text-[10px] font-semibold uppercase tracking-wider text-text-muted border-b border-surface-border-subtle flex items-center justify-between">
            <span>Audio Playback Endpoint</span>
            <span className="font-mono text-[9px] text-accent-emerald">WASAPI</span>
          </div>

          <div className="flex flex-col gap-0.5 max-h-56 overflow-y-auto">
            {devices.map((device) => {
              const isSelected = device.id === selectedDeviceId;
              return (
                <button
                  key={device.id}
                  type="button"
                  role="option"
                  aria-selected={isSelected}
                  onClick={() => {
                    onSelectDevice(device.id);
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
                    <span className="truncate">{device.name}</span>
                  </div>

                  {device.isDefault && (
                    <span className="flex-none text-[9px] font-mono px-1.5 py-0.2 rounded bg-surface-base border border-surface-border text-text-muted uppercase">
                      Default
                    </span>
                  )}
                </button>
              );
            })}
          </div>
        </div>
      )}
    </div>
  );
};

export default DeviceDropdown;
