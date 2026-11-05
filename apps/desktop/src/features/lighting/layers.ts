import fixtures from "@/lib/api/fixtures/effects.json";
import type { BlendMode, EffectName, Layer } from "@/lib/api/types";
import type { LightingKey } from "./messages";

/** Mirrors `freergb_effects::layers::MAX_LAYERS`. */
export const MAX_LAYERS = 8;

export const BLENDS: readonly { mode: BlendMode; label: LightingKey }[] = [
  { mode: "normal", label: "lighting.blend.normal" },
  { mode: "add", label: "lighting.blend.add" },
  { mode: "multiply", label: "lighting.blend.multiply" },
  { mode: "screen", label: "lighting.blend.screen" },
  { mode: "max", label: "lighting.blend.max" },
];

type LayerEffect = Layer["effect"];
const DEFAULTS = fixtures as unknown as Record<Exclude<EffectName, "layers">, LayerEffect>;

/** Pure stack operations; indexes are daemon order (0 = bottom). */
export function addLayer(layers: Layer[]): Layer[] {
  if (layers.length >= MAX_LAYERS) return layers;
  return [...layers, { effect: DEFAULTS.breathing, blend: "screen", opacity: 1, region: null }];
}

export function removeLayer(layers: Layer[], index: number): Layer[] {
  // The daemon requires at least one layer.
  return layers.length <= 1 ? layers : layers.filter((_, i) => i !== index);
}

/** Moves the layer at `index` by `by` positions (positive = towards the top). */
export function moveLayer(layers: Layer[], index: number, by: number): Layer[] {
  const target = index + by;
  if (target < 0 || target >= layers.length) return layers;
  const next = [...layers];
  const [layer] = next.splice(index, 1);
  next.splice(target, 0, layer!);
  return next;
}

export function updateLayer(layers: Layer[], index: number, patch: Partial<Layer>): Layer[] {
  return layers.map((l, i) => (i === index ? { ...l, ...patch } : l));
}

/** Switching a layer's effect starts from that effect's defaults, like Classic. */
export function setLayerEffect(layers: Layer[], index: number, name: Exclude<EffectName, "layers">): Layer[] {
  return updateLayer(layers, index, { effect: DEFAULTS[name] });
}
