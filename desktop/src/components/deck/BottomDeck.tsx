import React from "react";
import { BassBoostConfig } from "../../types/audio";
import { BassBoostModule } from "./BassBoostModule";
import { PreampModule } from "./PreampModule";
import { LimiterModule } from "./LimiterModule";
import { StereoMeterSlot } from "./StereoMeterSlot";

export interface BottomDeckProps {
  bassBoost: BassBoostConfig;
  onUpdateBassBoost: (newConfig: BassBoostConfig) => void;
  preampGainDb: number;
  onUpdatePreamp: (newGainDb: number) => void;
  limiterEnabled: boolean;
  onToggleLimiter: () => void;
  isBypassed?: boolean;
}

export const BottomDeck: React.FC<BottomDeckProps> = ({
  bassBoost,
  onUpdateBassBoost,
  preampGainDb,
  onUpdatePreamp,
  limiterEnabled,
  onToggleLimiter,
  isBypassed = false,
}) => {
  return (
    <footer className="h-44 flex-none border-t border-surface-border bg-surface-panel px-4 py-3 grid grid-cols-4 gap-4 z-10 select-none">
      {/* Module 1: Dedicated Bass Boost */}
      <BassBoostModule
        config={bassBoost}
        isBypassed={isBypassed}
        onUpdate={onUpdateBassBoost}
      />

      {/* Module 2: Pre-Amp Gain Attenuation */}
      <PreampModule
        gainDb={preampGainDb}
        isBypassed={isBypassed}
        onUpdate={onUpdatePreamp}
      />

      {/* Module 3: Dynamics & Headroom Limiter */}
      <LimiterModule
        enabled={limiterEnabled}
        preampGainDb={preampGainDb}
        isBypassed={isBypassed}
        onToggle={onToggleLimiter}
      />

      {/* Module 4: Stereo Peak VU Meters */}
      <StereoMeterSlot isBypassed={isBypassed} />
    </footer>
  );
};

export default BottomDeck;
