import { describe, expect, it } from "vitest";
import catalog from "../../../../../effects/development/catalog.json";
import { defaults, matches, type Package } from "./types";

describe("local development collection in Lighting", () => {
  it("exposes 25 distinct packages with usable numeric controls", () => {
    const entries = catalog.effects.slice(3);
    expect(entries).toHaveLength(25);
    expect(new Set(entries.map(p => p.id)).size).toBe(25);
    for (const entry of entries) {
      const pkg: Package = {...entry, preview:[entry.preview[0]!, entry.preview[1]!]};
      const values = defaults(pkg);
      expect(pkg.parameters.length).toBeGreaterThanOrEqual(4);
      expect(pkg.parameters.length).toBeLessThanOrEqual(16);
      expect(matches(pkg, pkg.name, false, false)).toBe(true);
      expect(matches(pkg, pkg.publisher.name, false, false)).toBe(true);
      for (const parameter of pkg.parameters) {
        expect(values[parameter.key]).toBe(parameter.default);
        expect(parameter.default).toBeGreaterThanOrEqual(parameter.min);
        expect(parameter.default).toBeLessThanOrEqual(parameter.max);
        expect(parameter.step).toBeGreaterThan(0);
      }
      expect(pkg.permissions).toEqual([]);
      expect(pkg.artifact.url).toBe(`${pkg.id}-${pkg.version}.wasm`);
    }
  });
});
