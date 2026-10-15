import React, { useRef, useEffect, useCallback } from "react";
import { EqBand } from "../../types/audio";
import {
  freqToX,
  gainToY,
  OCTAVE_GUIDES,
  GAIN_TICKS,
  MIN_FREQ_HZ,
  MAX_FREQ_HZ,
  MIN_GAIN_DB,
  MAX_GAIN_DB,
} from "../../lib/coordinates";
import {
  computeCompositeResponse,
  NUM_CURVE_POINTS,
  PRECOMPUTED_FREQUENCIES,
} from "../../lib/biquad-math";

export interface ParametricCanvasProps {
  bands: EqBand[];
  selectedBandId?: number;
  onSelectBand?: (bandId: number) => void;
  preampGainDb?: number;
  isBypassed?: boolean;
}

export const ParametricCanvas: React.FC<ParametricCanvasProps> = ({
  bands,
  selectedBandId = 4,
  onSelectBand,
  preampGainDb = 0.0,
  isBypassed = false,
}) => {
  const containerRef = useRef<HTMLDivElement>(null);
  const canvasRef = useRef<HTMLCanvasElement>(null);

  // Pre-allocated response buffer to guarantee zero GC allocations in 60 FPS loop
  const responseBufferRef = useRef<Float32Array>(
    new Float32Array(NUM_CURVE_POINTS)
  );
  const animFrameIdRef = useRef<number | null>(null);

  // Internal render function
  const renderCanvas = useCallback(() => {
    const canvas = canvasRef.current;
    const container = containerRef.current;
    if (!canvas || !container) return;

    const ctx = canvas.getContext("2d");
    if (!ctx) return;

    const rect = container.getBoundingClientRect();
    const width = rect.width;
    const height = rect.height;
    if (width <= 0 || height <= 0) return;

    const dpr = window.devicePixelRatio || 1;

    // Handle HiDPI retina scaling
    const targetCanvasWidth = Math.round(width * dpr);
    const targetCanvasHeight = Math.round(height * dpr);

    if (
      canvas.width !== targetCanvasWidth ||
      canvas.height !== targetCanvasHeight
    ) {
      canvas.width = targetCanvasWidth;
      canvas.height = targetCanvasHeight;
    }

    ctx.save();
    ctx.scale(dpr, dpr);

    // =========================================================================
    // LAYER 1: Background & Logarithmic Mesh Grid
    // =========================================================================
    // Background fill
    ctx.fillStyle = "#0C101A";
    ctx.fillRect(0, 0, width, height);

    // Subtle Logarithmic Frequency Octave Grid Lines
    ctx.font = "9px 'JetBrains Mono', monospace";
    ctx.textAlign = "center";
    ctx.textBaseline = "bottom";

    for (let i = 0; i < OCTAVE_GUIDES.length; i++) {
      const guide = OCTAVE_GUIDES[i];
      const x = Math.round(freqToX(guide.freqHz, width, MIN_FREQ_HZ, MAX_FREQ_HZ));

      // Vertical octave line
      ctx.beginPath();
      ctx.strokeStyle = "#1A2333";
      ctx.lineWidth = 1;
      ctx.moveTo(x + 0.5, 0);
      ctx.lineTo(x + 0.5, height - 16);
      ctx.stroke();

      // Octave label along bottom
      ctx.fillStyle = "#4B5563";
      ctx.fillText(guide.label, x, height - 3);
    }

    // Horizontal Decibel Reference Grid Lines
    ctx.textAlign = "left";
    ctx.textBaseline = "middle";

    for (let i = 0; i < GAIN_TICKS.length; i++) {
      const gain = GAIN_TICKS[i];
      const y = Math.round(gainToY(gain, height - 20, MIN_GAIN_DB, MAX_GAIN_DB));

      ctx.beginPath();
      if (gain === 0) {
        // Prominent 0.0 dB Center Reference Guide
        ctx.strokeStyle = "#334155";
        ctx.lineWidth = 1.5;
        ctx.setLineDash([]);
      } else {
        ctx.strokeStyle = "#161F2E";
        ctx.lineWidth = 1;
        ctx.setLineDash([3, 3]);
      }

      ctx.moveTo(0, y + 0.5);
      ctx.lineTo(width, y + 0.5);
      ctx.stroke();
      ctx.setLineDash([]);

      // Decibel tick labels on both edges
      const label = gain > 0 ? `+${gain} dB` : `${gain} dB`;
      ctx.fillStyle = gain === 0 ? "#94A3B8" : "#3B4758";
      ctx.fillText(label, 6, y - 1);
    }

    // =========================================================================
    // LAYER 2: Compute Composite Response
    // =========================================================================
    const curveBuffer = responseBufferRef.current;
    computeCompositeResponse(bands, preampGainDb, curveBuffer);

    const zeroY = gainToY(0, height - 20, MIN_GAIN_DB, MAX_GAIN_DB);

    // =========================================================================
    // LAYER 3: Translucent Under-Curve Gradient Fill
    // =========================================================================
    if (!isBypassed) {
      const fillPath = new Path2D();
      const firstX = freqToX(PRECOMPUTED_FREQUENCIES[0], width);
      fillPath.moveTo(firstX, zeroY);

      for (let p = 0; p < NUM_CURVE_POINTS; p++) {
        const x = freqToX(PRECOMPUTED_FREQUENCIES[p], width);
        const y = gainToY(curveBuffer[p], height - 20, MIN_GAIN_DB, MAX_GAIN_DB);
        fillPath.lineTo(x, y);
      }

      const lastX = freqToX(PRECOMPUTED_FREQUENCIES[NUM_CURVE_POINTS - 1], width);
      fillPath.lineTo(lastX, zeroY);
      fillPath.closePath();

      // Vertical linear gradient fill
      const gradient = ctx.createLinearGradient(0, 0, 0, height);
      gradient.addColorStop(0, "rgba(0, 229, 255, 0.22)");
      gradient.addColorStop(0.5, "rgba(0, 229, 255, 0.08)");
      gradient.addColorStop(1, "rgba(0, 229, 255, 0.0)");

      ctx.fillStyle = gradient;
      ctx.fill(fillPath);
    }

    // =========================================================================
    // LAYER 4: Composite Transfer Function Spline Stroke
    // =========================================================================
    const curvePath = new Path2D();
    for (let p = 0; p < NUM_CURVE_POINTS; p++) {
      const x = freqToX(PRECOMPUTED_FREQUENCIES[p], width);
      const y = gainToY(
        isBypassed ? 0.0 : curveBuffer[p],
        height - 20,
        MIN_GAIN_DB,
        MAX_GAIN_DB
      );

      if (p === 0) {
        curvePath.moveTo(x, y);
      } else {
        curvePath.lineTo(x, y);
      }
    }

    if (isBypassed) {
      // Bypassed state: Dimmed amber dashed curve showing original EQ curve
      const origCurvePath = new Path2D();
      for (let p = 0; p < NUM_CURVE_POINTS; p++) {
        const x = freqToX(PRECOMPUTED_FREQUENCIES[p], width);
        const y = gainToY(curveBuffer[p], height - 20, MIN_GAIN_DB, MAX_GAIN_DB);
        if (p === 0) origCurvePath.moveTo(x, y);
        else origCurvePath.lineTo(x, y);
      }

      ctx.save();
      ctx.strokeStyle = "rgba(245, 158, 11, 0.35)";
      ctx.lineWidth = 1.5;
      ctx.setLineDash([4, 4]);
      ctx.stroke(origCurvePath);
      ctx.restore();

      // Solid flat 0 dB line for active unaltered pass-through
      ctx.save();
      ctx.strokeStyle = "#F59E0B";
      ctx.lineWidth = 2.0;
      ctx.shadowColor = "rgba(245, 158, 11, 0.4)";
      ctx.shadowBlur = 8;
      ctx.stroke(curvePath);
      ctx.restore();
    } else {
      // Normal state: Glowing neon cyan composite spline
      ctx.save();
      ctx.strokeStyle = "#00E5FF";
      ctx.lineWidth = 2.5;
      ctx.lineCap = "round";
      ctx.lineJoin = "round";
      ctx.shadowColor = "rgba(0, 229, 255, 0.45)";
      ctx.shadowBlur = 8;
      ctx.stroke(curvePath);
      ctx.restore();
    }

    // =========================================================================
    // LAYER 5: Parametric Filter Nodes (10 Bands)
    // =========================================================================
    for (let i = 0; i < bands.length; i++) {
      const band = bands[i];
      if (!band) continue;

      const nodeX = freqToX(band.frequencyHz, width);
      const nodeY = gainToY(
        isBypassed ? 0.0 : band.gainDb,
        height - 20,
        MIN_GAIN_DB,
        MAX_GAIN_DB
      );

      const isSelected = band.id === selectedBandId;
      const isEnabled = band.enabled && !isBypassed;

      // Outer glow ring for selected band
      if (isSelected) {
        ctx.beginPath();
        ctx.arc(nodeX, nodeY, 12, 0, Math.PI * 2);
        ctx.fillStyle = isEnabled
          ? "rgba(0, 229, 255, 0.2)"
          : "rgba(245, 158, 11, 0.2)";
        ctx.fill();
      }

      // Outer circular rim
      ctx.beginPath();
      ctx.arc(nodeX, nodeY, isSelected ? 7 : 5.5, 0, Math.PI * 2);
      ctx.fillStyle = isEnabled
        ? isSelected
          ? "#00E5FF"
          : "#0EA5E9"
        : "#475569";
      ctx.fill();
      ctx.strokeStyle = "#0B0E14";
      ctx.lineWidth = 1.5;
      ctx.stroke();

      // Center core dot
      ctx.beginPath();
      ctx.arc(nodeX, nodeY, isSelected ? 2.5 : 2, 0, Math.PI * 2);
      ctx.fillStyle = "#FFFFFF";
      ctx.fill();

      // Band ID numeric badge
      ctx.font = "bold 9px 'JetBrains Mono', monospace";
      ctx.textAlign = "center";
      ctx.textBaseline = "middle";

      const textY = nodeY - (isSelected ? 16 : 13);
      // Small background badge for badge clarity
      ctx.fillStyle = "rgba(11, 14, 20, 0.85)";
      ctx.fillRect(nodeX - 7, textY - 6, 14, 12);
      ctx.strokeStyle = isSelected ? "#00E5FF" : "#1E293B";
      ctx.lineWidth = 1;
      ctx.strokeRect(nodeX - 7, textY - 6, 14, 12);

      ctx.fillStyle = isSelected ? "#00E5FF" : "#94A3B8";
      ctx.fillText(band.id.toString(), nodeX, textY);
    }

    // =========================================================================
    // LAYER 6: Watermark & Bypass Badge
    // =========================================================================
    if (isBypassed) {
      ctx.save();
      ctx.font = "bold 11px Inter, sans-serif";
      ctx.textAlign = "right";
      ctx.textBaseline = "top";
      ctx.fillStyle = "#F59E0B";
      ctx.fillText("MASTER BYPASS ACTIVE \u2014 RAW PASS-THROUGH", width - 16, 14);
      ctx.restore();
    }

    ctx.restore();
  }, [bands, selectedBandId, preampGainDb, isBypassed]);

  // Handle canvas clicks to select closest band
  const handleCanvasClick = (event: React.MouseEvent<HTMLCanvasElement>) => {
    if (!onSelectBand || !containerRef.current) return;
    const rect = containerRef.current.getBoundingClientRect();
    const clickX = event.clientX - rect.left;
    const clickY = event.clientY - rect.top;
    const height = rect.height;

    let closestBandId = -1;
    let minDistance = 24; // 24px click hit radius

    for (let i = 0; i < bands.length; i++) {
      const band = bands[i];
      const nodeX = freqToX(band.frequencyHz, rect.width);
      const nodeY = gainToY(
        isBypassed ? 0.0 : band.gainDb,
        height - 20,
        MIN_GAIN_DB,
        MAX_GAIN_DB
      );

      const dist = Math.hypot(clickX - nodeX, clickY - nodeY);
      if (dist < minDistance) {
        minDistance = dist;
        closestBandId = band.id;
      }
    }

    if (closestBandId !== -1) {
      onSelectBand(closestBandId);
    }
  };

  // Schedule render on requestAnimationFrame
  useEffect(() => {
    const scheduleRender = () => {
      if (animFrameIdRef.current !== null) {
        cancelAnimationFrame(animFrameIdRef.current);
      }
      animFrameIdRef.current = requestAnimationFrame(() => {
        renderCanvas();
        animFrameIdRef.current = null;
      });
    };

    scheduleRender();

    // ResizeObserver for zero-lag HiDPI adjustments
    const container = containerRef.current;
    if (!container) return;

    const observer = new ResizeObserver(() => {
      scheduleRender();
    });

    observer.observe(container);

    return () => {
      observer.disconnect();
      if (animFrameIdRef.current !== null) {
        cancelAnimationFrame(animFrameIdRef.current);
      }
    };
  }, [renderCanvas]);

  return (
    <div
      ref={containerRef}
      className="relative w-full h-full bg-[#0C101A] overflow-hidden rounded-lg border border-surface-border select-none"
    >
      <canvas
        ref={canvasRef}
        onClick={handleCanvasClick}
        className="w-full h-full block cursor-crosshair"
      />
    </div>
  );
};

export default ParametricCanvas;
