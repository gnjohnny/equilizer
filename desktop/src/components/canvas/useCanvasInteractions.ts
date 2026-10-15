import { useState, useEffect, useCallback, useRef } from "react";
import { EqBand } from "../../types/audio";
import {
  freqToX,
  gainToY,
  xToFreq,
  yToGain,
  MIN_FREQ_HZ,
  MAX_FREQ_HZ,
  MIN_GAIN_DB,
  MAX_GAIN_DB,
} from "../../lib/coordinates";

export interface UseCanvasInteractionsProps {
  canvasRef: React.RefObject<HTMLCanvasElement | null>;
  containerRef: React.RefObject<HTMLDivElement | null>;
  bands: EqBand[];
  selectedBandId: number;
  onSelectBand?: (bandId: number) => void;
  onUpdateBand?: (updatedBand: EqBand) => void;
  isBypassed?: boolean;
}

export interface UseCanvasInteractionsReturn {
  hoveredBandId: number | null;
  draggingBandId: number | null;
  isDragging: boolean;
  isWheeling: boolean;
  activeInteractionBandId: number;
  cursorClass: string;
}

const HIT_RADIUS_PX = 16;
const BOTTOM_MARGIN_PX = 20;

export function useCanvasInteractions({
  canvasRef,
  containerRef,
  bands,
  selectedBandId,
  onSelectBand,
  onUpdateBand,
  isBypassed = false,
}: UseCanvasInteractionsProps): UseCanvasInteractionsReturn {
  const [hoveredBandId, setHoveredBandId] = useState<number | null>(null);
  const [draggingBandId, setDraggingBandId] = useState<number | null>(null);
  const [isWheeling, setIsWheeling] = useState<boolean>(false);

  const wheelTimeoutRef = useRef<NodeJS.Timeout | null>(null);
  const bandsRef = useRef<EqBand[]>(bands);
  const onUpdateBandRef = useRef(onUpdateBand);
  const onSelectBandRef = useRef(onSelectBand);
  const selectedBandIdRef = useRef(selectedBandId);

  useEffect(() => {
    bandsRef.current = bands;
    onUpdateBandRef.current = onUpdateBand;
    onSelectBandRef.current = onSelectBand;
    selectedBandIdRef.current = selectedBandId;
  }, [bands, onUpdateBand, onSelectBand, selectedBandId]);

  // Helper to find closest band to pixel coordinates
  const findHitBand = useCallback(
    (clientX: number, clientY: number): EqBand | null => {
      const container = containerRef.current;
      if (!container) return null;

      const rect = container.getBoundingClientRect();
      const x = clientX - rect.left;
      const y = clientY - rect.top;
      const width = rect.width;
      const height = rect.height - BOTTOM_MARGIN_PX;

      if (width <= 0 || height <= 0) return null;

      let closestBand: EqBand | null = null;
      let minDistance = HIT_RADIUS_PX;

      const currentBands = bandsRef.current;
      for (let i = 0; i < currentBands.length; i++) {
        const band = currentBands[i];
        if (!band) continue;

        const nodeX = freqToX(band.frequencyHz, width, MIN_FREQ_HZ, MAX_FREQ_HZ);
        const nodeY = gainToY(
          isBypassed ? 0.0 : band.gainDb,
          height,
          MIN_GAIN_DB,
          MAX_GAIN_DB
        );

        const dist = Math.hypot(x - nodeX, y - nodeY);
        if (dist < minDistance) {
          minDistance = dist;
          closestBand = band;
        }
      }

      return closestBand;
    },
    [containerRef, isBypassed]
  );

  // Mouse Move over Canvas (Hover Detection)
  useEffect(() => {
    const canvas = canvasRef.current;
    if (!canvas || isBypassed) return;

    const handleMouseMove = (event: MouseEvent) => {
      if (draggingBandId !== null) return;
      const hit = findHitBand(event.clientX, event.clientY);
      setHoveredBandId(hit ? hit.id : null);
    };

    const handleMouseLeave = () => {
      if (draggingBandId === null) {
        setHoveredBandId(null);
      }
    };

    canvas.addEventListener("mousemove", handleMouseMove);
    canvas.addEventListener("mouseleave", handleMouseLeave);

    return () => {
      canvas.removeEventListener("mousemove", handleMouseMove);
      canvas.removeEventListener("mouseleave", handleMouseLeave);
    };
  }, [canvasRef, findHitBand, draggingBandId, isBypassed]);

  // Mouse Down on Canvas (Start Drag / Select Band)
  useEffect(() => {
    const canvas = canvasRef.current;
    if (!canvas || isBypassed) return;

    const handleMouseDown = (event: MouseEvent) => {
      if (event.button !== 0) return; // Left click only

      const hit = findHitBand(event.clientX, event.clientY);
      if (hit) {
        setDraggingBandId(hit.id);
        if (onSelectBandRef.current) {
          onSelectBandRef.current(hit.id);
        }
      }
    };

    canvas.addEventListener("mousedown", handleMouseDown);
    return () => {
      canvas.removeEventListener("mousedown", handleMouseDown);
    };
  }, [canvasRef, findHitBand, isBypassed]);

  // Global Window Listeners during Active Drag
  useEffect(() => {
    if (draggingBandId === null) return;

    const handleWindowMouseMove = (event: MouseEvent) => {
      const container = containerRef.current;
      if (!container) return;

      const rect = container.getBoundingClientRect();
      const rawX = event.clientX - rect.left;
      const rawY = event.clientY - rect.top;

      const width = rect.width;
      const height = rect.height - BOTTOM_MARGIN_PX;
      if (width <= 0 || height <= 0) return;

      // Smoothly clamp coordinates inside graph bounds
      const clampedX = Math.max(0, Math.min(width, rawX));
      const clampedY = Math.max(0, Math.min(height, rawY));

      const newFreq = Math.round(xToFreq(clampedX, width, MIN_FREQ_HZ, MAX_FREQ_HZ));
      const newGain = Number(
        yToGain(clampedY, height, MIN_GAIN_DB, MAX_GAIN_DB).toFixed(1)
      );

      const targetBand = bandsRef.current.find((b) => b.id === draggingBandId);
      if (
        targetBand &&
        (targetBand.frequencyHz !== newFreq || targetBand.gainDb !== newGain)
      ) {
        if (onUpdateBandRef.current) {
          onUpdateBandRef.current({
            ...targetBand,
            frequencyHz: newFreq,
            gainDb: newGain,
          });
        }
      }
    };

    const handleWindowMouseUp = () => {
      setDraggingBandId(null);
    };

    window.addEventListener("mousemove", handleWindowMouseMove);
    window.addEventListener("mouseup", handleWindowMouseUp);

    return () => {
      window.removeEventListener("mousemove", handleWindowMouseMove);
      window.removeEventListener("mouseup", handleWindowMouseUp);
    };
  }, [draggingBandId, containerRef]);

  // Double Click (Reset Gain to 0 dB)
  useEffect(() => {
    const canvas = canvasRef.current;
    if (!canvas || isBypassed) return;

    const handleDblClick = (event: MouseEvent) => {
      const hit = findHitBand(event.clientX, event.clientY);
      if (hit && onUpdateBandRef.current) {
        onUpdateBandRef.current({
          ...hit,
          gainDb: 0.0,
        });
      }
    };

    canvas.addEventListener("dblclick", handleDblClick);
    return () => {
      canvas.removeEventListener("dblclick", handleDblClick);
    };
  }, [canvasRef, findHitBand, isBypassed]);

  // Mouse Wheel (Exponential Q Bandwidth Adjustment)
  useEffect(() => {
    const canvas = canvasRef.current;
    if (!canvas || isBypassed) return;

    const handleWheel = (event: WheelEvent) => {
      event.preventDefault(); // Prevent full window scrolling

      // Adjust hovered band, or fallback to currently selected band
      const targetId = hoveredBandId ?? selectedBandIdRef.current;
      const targetBand = bandsRef.current.find((b) => b.id === targetId);
      if (!targetBand || !onUpdateBandRef.current) return;

      const currentQ = targetBand.q;
      // Wheel up narrows bandwidth (increases Q), wheel down widens (decreases Q)
      const scale = event.deltaY < 0 ? 1.12 : 1 / 1.12;
      const newQ = Math.max(0.1, Math.min(10.0, Number((currentQ * scale).toFixed(2))));

      if (newQ !== currentQ) {
        onUpdateBandRef.current({
          ...targetBand,
          q: newQ,
        });

        // Trigger bandwidth halo illumination
        setIsWheeling(true);
        if (wheelTimeoutRef.current) clearTimeout(wheelTimeoutRef.current);
        wheelTimeoutRef.current = setTimeout(() => {
          setIsWheeling(false);
        }, 600);
      }
    };

    canvas.addEventListener("wheel", handleWheel, { passive: false });
    return () => {
      canvas.removeEventListener("wheel", handleWheel);
      if (wheelTimeoutRef.current) clearTimeout(wheelTimeoutRef.current);
    };
  }, [canvasRef, hoveredBandId, isBypassed]);

  const isDragging = draggingBandId !== null;
  const activeInteractionBandId =
    draggingBandId ?? hoveredBandId ?? selectedBandId;

  let cursorClass = "cursor-crosshair";
  if (isDragging) {
    cursorClass = "cursor-grabbing";
  } else if (hoveredBandId !== null) {
    cursorClass = "cursor-grab";
  }

  return {
    hoveredBandId,
    draggingBandId,
    isDragging,
    isWheeling,
    activeInteractionBandId,
    cursorClass,
  };
}
