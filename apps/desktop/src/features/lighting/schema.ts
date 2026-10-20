import type { EffectName } from "@/lib/api/types";

/**
 * Editable parameters per effect. Slider ranges match freeRGB Classic and lie
 * inside the daemon's validation bounds (docs/LOCAL_API.md), which stay the
 * authority; `daemon` records those bounds so a test can check the subset.
 */
export type Field =
  | { key: string; kind: "color"; label: string }
  | {
      key: string;
      kind: "range";
      label: string;
      min: number;
      max: number;
      step: number;
      daemon: [number, number];
      format: (value: number) => string;
      /** Optional display hint, e.g. negative speed reverses direction. */
      hint?: string;
    };

const seconds = (ms: number) => `${(ms / 1000).toFixed(ms < 10_000 ? 1 : 0)} s`;
const fixed = (digits: number, unit = "") => (v: number) => `${v.toFixed(digits)}${unit}`;
const percent = (v: number) => `${Math.round(v * 100)}%`;

const PERIOD = [1, 86_400_000] as [number, number];
const period = (min: number, max: number, label = "Duration"): Field => ({
  key: "period_ms",
  kind: "range",
  label,
  min,
  max,
  step: 100,
  daemon: PERIOD,
  format: seconds,
});
const speed: Field = {
  key: "speed",
  kind: "range",
  label: "Speed",
  min: -2,
  max: 2,
  step: 0.05,
  daemon: [-100, 100],
  format: fixed(2),
  hint: "Negative values reverse the direction.",
};
const cycles: Field = { key: "scale", kind: "range", label: "Cycles", min: 0.1, max: 5, step: 0.1, daemon: [0.001, 1000], format: fixed(1) };
const color = (key: string, label: string): Field => ({ key, kind: "color", label });

export const SCHEMA: Record<Exclude<EffectName, "layers">, { fields: Field[]; note?: string }> = {
  solid: { fields: [color("color", "Colour")] },
  rainbow_wave: { fields: [speed, cycles] },
  radial_rainbow: { fields: [speed, cycles] },
  breathing: { fields: [color("color", "Colour"), period(500, 10_000, "Breath")] },
  gradient: {
    fields: [
      color("from", "From"),
      color("to", "To"),
      { key: "angle_deg", kind: "range", label: "Angle", min: 0, max: 360, step: 1, daemon: [-360_000, 360_000], format: fixed(0, "°") },
    ],
  },
  color_cycle: { fields: [period(1_000, 60_000, "Cycle")] },
  scanner: {
    fields: [
      color("color", "Colour"),
      color("background", "Background"),
      period(300, 10_000, "Sweep"),
      { key: "width", kind: "range", label: "Width", min: 0.02, max: 0.5, step: 0.01, daemon: [0.001, 1], format: percent },
    ],
  },
  twinkle: {
    fields: [
      color("color", "Colour"),
      color("background", "Background"),
      { key: "density", kind: "range", label: "Density", min: 0, max: 1, step: 0.01, daemon: [0, 1], format: percent },
      period(200, 5_000, "Sparkle"),
    ],
  },
  audio_pulse: {
    fields: [
      color("color", "Colour"),
      color("background", "Background"),
      { key: "sensitivity", kind: "range", label: "Sensitivity", min: 0.5, max: 4, step: 0.1, daemon: [0, 100], format: fixed(1, "×") },
    ],
    note: "Reacts to the sound your PC plays. Audio capture runs only while this effect is active.",
  },
  audio_spectrum: {
    fields: [color("low", "Bass"), color("high", "Treble"), color("background", "Background")],
    note: "Eight frequency bands across the canvas, bass on the left.",
  },
  screen_ambience: {
    fields: [
      { key: "saturation", kind: "range", label: "Saturation", min: 0.5, max: 2.5, step: 0.05, daemon: [0, 10], format: fixed(2, "×") },
    ],
    note: "Mirrors the primary monitor. Place devices in Layout to match where they sit around the screen.",
  },
};
