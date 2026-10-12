/**
 * Equilizer Theme Tokens & Canvas Color Bridge
 *
 * Direct hex and rgba values matching `globals.css` and `.context/ui-tokens.md`.
 * Used by HTML5 Canvas (curve editor and VU meters) to avoid expensive
 * `window.getComputedStyle()` calls during 60 FPS animation loops.
 */

export const THEME_COLORS = {
  // Surface Layers (Matte DAW Obsidian & Slate)
  surfaceBase: "#0B0E14",
  surfacePanel: "#111622",
  surfaceCard: "#161D2B",
  surfaceCardHover: "#1C2537",
  surfaceBorder: "#1E293B",
  surfaceBorderSubtle: "#151D2A",

  // Text & Readout Typography
  textPrimary: "#F8FAFC",
  textSecondary: "#94A3B8",
  textMuted: "#475569",
  textInverse: "#0B0E14",

  // Brand & Filter Accents
  accentCyan: "#00E5FF",
  accentCyanGlow: "rgba(0, 229, 255, 0.25)",
  accentEmerald: "#10B981",
  accentPurple: "#8B5CF6",

  // Audio Telemetry & Alert Levels
  meterNormal: "#10B981",
  meterWarning: "#F59E0B",
  meterClip: "#EF4444",
  meterPeakHold: "#38BDF8",

  // Canvas Graph Elements
  canvasBg: "#0C101A",
  canvasGridMajor: "#1E293B",
  canvasGridMinor: "#131A29",
  canvasZeroLine: "#334155",
  canvasSpline: "#00E5FF",
  canvasSplineFillTop: "rgba(0, 229, 255, 0.18)",
  canvasSplineFillBottom: "rgba(0, 229, 255, 0.0)",
} as const;

export type ThemeColors = typeof THEME_COLORS;
