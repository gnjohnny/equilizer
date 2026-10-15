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
import { useCanvasInteractions } from "./useCanvasInteractions";

export interface ParametricCanvasProps {
  bands: EqBand[];
  selectedBandId?: number;
  onSelectBand?: (bandId: number) => void;
  onUpdateBand?: (updatedBand: EqBand) => void;
  preampGainDb?: number;
  isBypassed?: boolean;
}

export const ParametricCanvas: React.FC<ParametricCanvasProps> = ({
  bands,
  selectedBandId = 4,
  onSelectBand,
  onUpdateBand,
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

  // Integrate interactive mouse drag, hit-testing, and wheel handling
  const {
    hoveredBandId,
    draggingBandId,
    isWheeling,
    cursorClass,
  } = useCanvasInteractions({
    canvasRef,
    containerRef,
    bands,
    selectedBandId,
    onSelectBand,
    onUpdateBand,
    isBypassed,
  });

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
    ctx.fillStyle = "#0C101A";
    ctx.fillRect(0, 0, width, height);

    // Subtle Logarithmic Frequency Octave Grid Lines
    ctx.font = "9px 'JetBrains Mono', monospace";
    ctx.textAlign = "center";
    ctx.textBaseline = "bottom";

    for (let i = 0; i < OCTAVE_GUIDES.length; i++) {
      const guide = OCTAVE_GUIDES[i];
      const x = Math.round(freqToX(guide.freqHz, width, MIN_FREQ_HZ, MAX_FREQ_HZ));

      ctx.beginPath();
      ctx.strokeStyle = "#1A2333";
      ctx.lineWidth = 1;
      ctx.moveTo(x + 0.5, 0);
      ctx.lineTo(x + 0.5, height - 16);
      ctx.stroke();

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

      ctx.save();
      ctx.strokeStyle = "#F59E0B";
      ctx.lineWidth = 2.0;
      ctx.shadowColor = "rgba(245, 158, 11, 0.4)";
      ctx.shadowBlur = 8;
      ctx.stroke(curvePath);
      ctx.restore();
    } else {
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
    // LAYER 5: Parametric Filter Nodes & Q-Bandwidth Halos (10 Bands)
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
      const isHovered = band.id === hoveredBandId;
      const isDraggingThis = band.id === draggingBandId;
      const isEnabled = band.enabled && !isBypassed;

      // Draw Q Bandwidth Guide on active or hovered nodes
      if (!isBypassed && (isSelected || isHovered || isDraggingThis || isWheeling)) {
        const Q = Math.max(0.1, band.q);
        const f0 = band.frequencyHz;
        const sqrtTerm = Math.sqrt(1 + 1 / (4 * Q * Q));
        const f1 = Math.max(MIN_FREQ_HZ, f0 * (sqrtTerm - 1 / (2 * Q)));
        const f2 = Math.min(MAX_FREQ_HZ, f0 * (sqrtTerm + 1 / (2 * Q)));

        const bwX1 = freqToX(f1, width);
        const bwX2 = freqToX(f2, width);

        // Bandwidth bracket guide line
        ctx.save();
        ctx.beginPath();
        ctx.moveTo(bwX1, nodeY);
        ctx.lineTo(bwX2, nodeY);
        ctx.strokeStyle = isDraggingThis || isWheeling
          ? "rgba(0, 229, 255, 0.65)"
          : "rgba(0, 229, 255, 0.35)";
        ctx.lineWidth = 2;
        ctx.stroke();

        // Left & Right limit ticks
        ctx.beginPath();
        ctx.moveTo(bwX1, nodeY - 5);
        ctx.lineTo(bwX1, nodeY + 5);
        ctx.moveTo(bwX2, nodeY - 5);
        ctx.lineTo(bwX2, nodeY + 5);
        ctx.strokeStyle = "rgba(0, 229, 255, 0.8)";
        ctx.lineWidth = 1.5;
        ctx.stroke();
        ctx.restore();
      }

      // Outer glow halo for selected / hovered / dragged band
      if (isSelected || isHovered || isDraggingThis) {
        ctx.beginPath();
        const haloRadius = isDraggingThis ? 16 : isHovered ? 14 : 12;
        ctx.arc(nodeX, nodeY, haloRadius, 0, Math.PI * 2);
        ctx.fillStyle = isEnabled
          ? isDraggingThis
            ? "rgba(0, 229, 255, 0.35)"
            : "rgba(0, 229, 255, 0.2)"
          : "rgba(245, 158, 11, 0.2)";
        ctx.fill();
      }

      // Outer circular handle
      const handleRadius = isDraggingThis ? 8 : isHovered ? 7.5 : isSelected ? 7 : 5.5;
      ctx.beginPath();
      ctx.arc(nodeX, nodeY, handleRadius, 0, Math.PI * 2);
      ctx.fillStyle = isEnabled
        ? isSelected || isHovered || isDraggingThis
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

      const textY = nodeY - (isSelected || isDraggingThis ? 18 : 14);
      ctx.fillStyle = "rgba(11, 14, 20, 0.9)";
      ctx.fillRect(nodeX - 7, textY - 6, 14, 12);
      ctx.strokeStyle = isSelected || isHovered ? "#00E5FF" : "#1E293B";
      ctx.lineWidth = 1;
      ctx.strokeRect(nodeX - 7, textY - 6, 14, 12);

      ctx.fillStyle = isSelected || isHovered ? "#00E5FF" : "#94A3B8";
      ctx.fillText(band.id.toString(), nodeX, textY);

      // Real-time floating parameter tooltip when dragging or wheeling this band
      if (!isBypassed && (isDraggingThis || (isWheeling && isSelected))) {
        const readout = `${Math.round(band.frequencyHz).toLocaleString()} Hz \u2022 ${
          band.gainDb >= 0 ? "+" : ""
        }${band.gainDb.toFixed(1)} dB \u2022 Q: ${band.q.toFixed(2)}`;

        ctx.font = "bold 10px 'JetBrains Mono', monospace";
        const metrics = ctx.measureText(readout);
        const pillW = metrics.width + 16;
        const pillH = 18;
        const pillX = Math.max(10, Math.min(width - pillW - 10, nodeX - pillW / 2));
        const pillY = Math.max(10, nodeY - 34);

        ctx.fillStyle = "rgba(11, 14, 20, 0.95)";
        ctx.fillRect(pillX, pillY, pillW, pillH);
        ctx.strokeStyle = "#00E5FF";
        ctx.lineWidth = 1;
        ctx.strokeRect(pillX, pillY, pillW, pillH);

        ctx.fillStyle = "#00E5FF";
        ctx.textAlign = "center";
        ctx.textBaseline = "middle";
        ctx.fillText(readout, pillX + pillW / 2, pillY + pillH / 2);
      }
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
  }, [
    bands,
    selectedBandId,
    preampGainDb,
    isBypassed,
    hoveredBandId,
    draggingBandId,
    isWheeling,
  ]);

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
        className={`w-full h-full block ${cursorClass}`}
      />
    </div>
  );
};

export default ParametricCanvas;
