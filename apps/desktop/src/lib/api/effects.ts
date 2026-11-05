import { hasMessage, t } from "@/lib/i18n";
import type { EffectName, EffectSpec, Rgb } from "./types";

/** Display order and labels; matches `EffectSpec::NAMES` in freergb-effects. */
export const EFFECTS: readonly { name: EffectName; label: string }[] = [
  { name: "solid", label: "Solid colour" },
  { name: "rainbow_wave", label: "Rainbow wave" },
  { name: "breathing", label: "Breathing" },
  { name: "gradient", label: "Gradient" },
  { name: "color_cycle", label: "Colour cycle" },
  { name: "radial_rainbow", label: "Radial rainbow" },
  { name: "scanner", label: "Scanner" },
  { name: "twinkle", label: "Twinkle" },
  { name: "audio_pulse", label: "Audio pulse" },
  { name: "audio_spectrum", label: "Audio spectrum" },
  { name: "screen_ambience", label: "Screen ambience" },
  { name: "layers", label: "Custom layers" },
];

/** Parameter fields per effect (besides `type`), used by editors and the contract test. */
export const EFFECT_FIELDS: Record<EffectSpec["type"], readonly string[]> = {
  marketplace: ["id", "version", "parameters"],
  solid: ["color"],
  rainbow_wave: ["speed", "scale"],
  breathing: ["color", "period_ms"],
  gradient: ["from", "to", "angle_deg"],
  color_cycle: ["period_ms"],
  radial_rainbow: ["speed", "scale"],
  scanner: ["color", "background", "period_ms", "width"],
  twinkle: ["color", "background", "density", "period_ms"],
  audio_pulse: ["color", "background", "sensitivity"],
  audio_spectrum: ["low", "high", "background"],
  screen_ambience: ["saturation"],
  layers: ["layers"],
};

export function effectLabel(name: string): string {
  const key = `effect.${name}`;
  return hasMessage(key) ? t(key) : t("effect.unknown");
}

const css = ({ r, g, b }: Rgb) => `rgb(${r} ${g} ${b})`;
const SPECTRUM = "#ef4444, #f59e0b, #10a37f, #3b82f6, #a855f7, #ef4444";

/**
 * Static CSS background that suggests what an effect looks like. This is a
 * representation for previews only; the daemon remains the only renderer.
 */
export function effectPreview(spec: EffectSpec): string {
  switch (spec.type) {
    case "marketplace":
      return "linear-gradient(135deg, #10a37f, #8b5cf6)";
    case "solid":
      return css(spec.color);
    case "breathing":
      return `radial-gradient(circle at 50% 50%, ${css(spec.color)}, rgb(0 0 0) 85%)`;
    case "gradient":
      return `linear-gradient(${90 + spec.angle_deg}deg, ${css(spec.from)}, ${css(spec.to)})`;
    case "rainbow_wave":
    case "color_cycle":
      return `linear-gradient(90deg, ${SPECTRUM})`;
    case "radial_rainbow":
      return `conic-gradient(from 0deg, ${SPECTRUM})`;
    case "scanner":
      return `linear-gradient(90deg, ${css(spec.background)} 30%, ${css(spec.color)} 50%, ${css(spec.background)} 70%)`;
    case "twinkle":
      return `radial-gradient(circle at 20% 30%, ${css(spec.color)} 0 6%, transparent 7%), radial-gradient(circle at 70% 60%, ${css(spec.color)} 0 5%, transparent 6%), ${css(spec.background)}`;
    case "audio_pulse":
      return `radial-gradient(circle, ${css(spec.color)} 0 35%, ${css(spec.background)} 70%)`;
    case "audio_spectrum":
      return `linear-gradient(90deg, ${css(spec.low)}, ${css(spec.high)})`;
    case "screen_ambience":
      return "linear-gradient(135deg, #3b82f6, #10a37f 50%, #f59e0b)";
    case "layers":
      // The bottom layer gives the best single-swatch impression.
      return spec.layers[0] ? effectPreview(spec.layers[0].effect) : "#303030";
  }
}
