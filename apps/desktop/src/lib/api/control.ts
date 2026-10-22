// Cooling, macro and custom sensor types (freergb-control, docs/LOCAL_API.md,
// docs/MACROS.md, docs/CUSTOM_SENSORS.md). Client checks mirror the Rust
// `validate` functions so editors can explain problems before saving; the
// daemon stays the authority.

import type { EffectSpec, Rgb } from "./types";

export interface CurvePoint {
  temperature_c: number;
  duty_percent: number;
}

export interface FanCurve {
  points: CurvePoint[];
  minimum_percent: number;
  critical_c: number;
  hysteresis_c: number;
  /** Optional step limits in %/s; never delay the critical-temperature override. */
  max_rise_percent_per_s?: number;
  max_fall_percent_per_s?: number;
}

export const DEFAULT_CURVE: FanCurve = {
  points: [
    { temperature_c: 25, duty_percent: 30 },
    { temperature_c: 50, duty_percent: 55 },
    { temperature_c: 75, duty_percent: 100 },
  ],
  minimum_percent: 25,
  critical_c: 85,
  hysteresis_c: 2,
};

/** Mirrors `FanCurve::validate`. */
export function curveProblem(c: FanCurve): string | null {
  if (c.points.length < 2 || c.points.length > 16) return "A curve needs 2 to 16 points.";
  // Mirrors FanCurve::validate. Native execution still enforces each channel's
  // tier floor (40 % unless the channel is unlocked as unrestricted).
  if (c.minimum_percent < 0 || c.minimum_percent > 100) return "Minimum duty must be 0–100 %.";
  if (c.critical_c < 40 || c.critical_c > 110) return "Critical temperature must be 40–110 °C.";
  if (c.hysteresis_c < 0 || c.hysteresis_c > 10) return "Hysteresis must be 0–10 °C.";
  for (const [i, p] of c.points.entries()) {
    if (p.temperature_c < -20 || p.temperature_c > 110) return "Point temperatures must be −20–110 °C.";
    if (p.duty_percent < c.minimum_percent || p.duty_percent > 100) return "Point duty must be between the minimum and 100 %.";
    const prev = c.points[i - 1];
    if (prev && (p.temperature_c <= prev.temperature_c || p.duty_percent < prev.duty_percent))
      return "Temperatures must increase and duty must not decrease.";
  }
  if (c.points[c.points.length - 1]!.temperature_c > c.critical_c) return "The curve extends beyond the critical temperature.";
  for (const rate of [c.max_rise_percent_per_s, c.max_fall_percent_per_s])
    if (rate !== undefined && (rate < 1 || rate > 100)) return "Step limits must be 1–100 %/s.";
  return null;
}

export type MacroStep =
  | { type: "delay"; ms: number }
  | { type: "color"; color: Rgb }
  | { type: "brightness"; value: number }
  | { type: "effect"; effect: EffectSpec }
  | { type: "text"; text: string }
  | { type: "key"; key: string }
  | { type: "chord"; keys: string[] };

export interface MacroDefinition {
  steps: MacroStep[];
  repeat: number;
}

export interface MacroStatus {
  running: string | null;
  input_enabled: boolean;
  error: string | null;
}

export const SINGLE_KEYS = [
  "enter", "escape", "tab", "space", "backspace", "up", "down", "left", "right",
  "media_play_pause", "volume_up", "volume_down", "volume_mute",
] as const;
export const MODIFIERS = ["ctrl", "alt", "shift", "meta"] as const;
const CHORD_KEYS = /^([a-z0-9]|enter|escape|tab|space|backspace|up|down|left|right)$/;

export function needsInput(step: MacroStep): boolean {
  return step.type === "text" || step.type === "key" || step.type === "chord";
}

/** Mirrors `chord_plan`: 1–4 distinct modifiers plus exactly one ordinary key. */
export function chordProblem(keys: string[]): string | null {
  const mods = keys.filter((k) => (MODIFIERS as readonly string[]).includes(k));
  const others = keys.filter((k) => !(MODIFIERS as readonly string[]).includes(k));
  if (mods.length < 1 || mods.length > 4 || new Set(mods).size !== mods.length) return "Use 1–4 different modifiers.";
  if (others.length !== 1 || !CHORD_KEYS.test(others[0]!)) return "Add exactly one key: a–z, 0–9, enter, escape, tab, space, backspace or an arrow.";
  return null;
}

/** Mirrors `MacroDefinition::validate` (effect parameters are checked by the daemon). */
export function macroProblem(m: MacroDefinition): string | null {
  if (m.steps.length < 1 || m.steps.length > 128) return "A macro needs 1 to 128 steps.";
  if (!Number.isInteger(m.repeat) || m.repeat < 1 || m.repeat > 20) return "Repeat must be 1–20.";
  let delay = 0;
  for (const [i, s] of m.steps.entries()) {
    const at = `Step ${i + 1}: `;
    if (s.type === "delay") {
      if (s.ms > 10_000) return at + "a delay can be at most 10 s.";
      delay += s.ms;
    }
    if (s.type === "brightness" && (s.value < 0 || s.value > 1)) return at + "brightness must be 0–100 %.";
    if (s.type === "text" && (s.text.length === 0 || new TextEncoder().encode(s.text).length > 1024)) return at + "text needs 1–1024 bytes.";
    if (s.type === "key" && !(SINGLE_KEYS as readonly string[]).includes(s.key)) return at + "unsupported key.";
    if (s.type === "chord") {
      const p = chordProblem(s.keys);
      if (p) return at + p;
    }
  }
  if (delay * m.repeat > 60_000) return "All delays together (times repeat) must stay within 60 s.";
  return null;
}

export type MixFunction = "maximum" | "minimum" | "average" | "sum" | "subtract";

export type SensorNode =
  | { type: "input"; source: string }
  | { type: "offset"; source: string; amount: number; proportional: boolean }
  | { type: "mix"; sources: string[]; function: MixFunction }
  | { type: "time_average"; source: string; window_ms: number };

export interface SensorGraph {
  version?: number;
  nodes: Record<string, SensorNode>;
}

export interface SensorReadingValue {
  value: number | null;
  sampled_at_ms: number | null;
  error: string | null;
}

export interface SensorValues {
  now_ms: number;
  values: Record<string, SensorReadingValue>;
}

export interface CoolingStatus {
  hardware_control: boolean;
  reason: string;
  error: string | null;
}
