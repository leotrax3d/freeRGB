import type { EffectName } from "@/lib/api/types";
import type { LightingKey } from "./messages";

/**
 * Editable parameters per effect. Slider ranges match freeRGB Classic and lie
 * inside the daemon's validation bounds (docs/LOCAL_API.md), which stay the
 * authority; `daemon` records those bounds so a test can check the subset.
 */
export type Field =
  | { key: string; kind: "color"; label: LightingKey }
  | {
      key: string;
      kind: "range";
      /** Message keys, translated when rendered. */
      label: LightingKey;
      min: number;
      max: number;
      step: number;
      daemon: [number, number];
      format: (value: number) => string;
      /** Optional display hint, e.g. negative speed reverses direction. */
      hint?: LightingKey;
    };

const seconds = (ms: number) => `${(ms / 1000).toFixed(ms < 10_000 ? 1 : 0)} s`;
const fixed = (digits: number, unit = "") => (v: number) => `${v.toFixed(digits)}${unit}`;
const percent = (v: number) => `${Math.round(v * 100)}%`;

const PERIOD = [1, 86_400_000] as [number, number];
const period = (min: number, max: number, label: LightingKey = "lighting.f.duration"): Field => ({
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
  label: "lighting.f.speed",
  min: -2,
  max: 2,
  step: 0.05,
  daemon: [-100, 100],
  format: fixed(2),
  hint: "lighting.f.speedHint",
};
const cycles: Field = { key: "scale", kind: "range", label: "lighting.f.cycles", min: 0.1, max: 5, step: 0.1, daemon: [0.001, 1000], format: fixed(1) };
const color = (key: string, label: LightingKey): Field => ({ key, kind: "color", label });

export const SCHEMA: Record<Exclude<EffectName, "layers">, { fields: Field[]; note?: LightingKey }> = {
  solid: { fields: [color("color", "lighting.f.colour")] },
  rainbow_wave: { fields: [speed, cycles] },
  radial_rainbow: { fields: [speed, cycles] },
  breathing: { fields: [color("color", "lighting.f.colour"), period(500, 10_000, "lighting.f.breath")] },
  gradient: {
    fields: [
      color("from", "lighting.f.from"),
      color("to", "lighting.f.to"),
      { key: "angle_deg", kind: "range", label: "lighting.f.angle", min: 0, max: 360, step: 1, daemon: [-360_000, 360_000], format: fixed(0, "°") },
    ],
  },
  color_cycle: { fields: [period(1_000, 60_000, "lighting.f.cycle")] },
  scanner: {
    fields: [
      color("color", "lighting.f.colour"),
      color("background", "lighting.f.background"),
      period(300, 10_000, "lighting.f.sweep"),
      { key: "width", kind: "range", label: "lighting.f.width", min: 0.02, max: 0.5, step: 0.01, daemon: [0.001, 1], format: percent },
    ],
  },
  twinkle: {
    fields: [
      color("color", "lighting.f.colour"),
      color("background", "lighting.f.background"),
      { key: "density", kind: "range", label: "lighting.f.density", min: 0, max: 1, step: 0.01, daemon: [0, 1], format: percent },
      period(200, 5_000, "lighting.f.sparkle"),
    ],
  },
  audio_pulse: {
    fields: [
      color("color", "lighting.f.colour"),
      color("background", "lighting.f.background"),
      { key: "sensitivity", kind: "range", label: "lighting.f.sensitivity", min: 0.5, max: 4, step: 0.1, daemon: [0, 100], format: fixed(1, "×") },
    ],
    note: "lighting.note.audioPulse",
  },
  audio_spectrum: {
    fields: [color("low", "lighting.f.bass"), color("high", "lighting.f.treble"), color("background", "lighting.f.background")],
    note: "lighting.note.audioSpectrum",
  },
  screen_ambience: {
    fields: [
      { key: "saturation", kind: "range", label: "lighting.f.saturation", min: 0.5, max: 2.5, step: 0.05, daemon: [0, 10], format: fixed(2, "×") },
    ],
    note: "lighting.note.screen",
  },
};
