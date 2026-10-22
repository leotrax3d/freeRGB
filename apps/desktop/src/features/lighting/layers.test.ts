import { describe, expect, it } from "vitest";
import type { Layer } from "@/lib/api/types";
import { addLayer, MAX_LAYERS, moveLayer, removeLayer, setLayerEffect } from "./layers";

const layer = (r: number): Layer => ({ effect: { type: "solid", color: { r, g: 0, b: 0 } }, blend: "normal", opacity: 1 });
const reds = (layers: Layer[]) => layers.map((l) => (l.effect.type === "solid" ? l.effect.color.r : -1));

describe("layer stack operations", () => {
  it("respects the daemon's 1..=8 layer bounds", () => {
    let stack = [layer(1)];
    expect(removeLayer(stack, 0)).toBe(stack);
    for (let i = 0; i < 20; i++) stack = addLayer(stack);
    expect(stack).toHaveLength(MAX_LAYERS);
  });

  it("moves within bounds and keeps other layers in order", () => {
    const stack = [layer(1), layer(2), layer(3)];
    expect(reds(moveLayer(stack, 0, 1))).toEqual([2, 1, 3]);
    expect(reds(moveLayer(stack, 2, -2))).toEqual([3, 1, 2]);
    expect(moveLayer(stack, 2, 1)).toBe(stack);
  });

  it("never creates nested layers and resets parameters on effect change", () => {
    const next = setLayerEffect([layer(9)], 0, "scanner");
    expect(next[0]!.effect.type).toBe("scanner");
    expect(next[0]!.blend).toBe("normal");
    expect(addLayer([layer(1)]).every((l) => l.effect.type !== ("layers" as string))).toBe(true);
  });
});
