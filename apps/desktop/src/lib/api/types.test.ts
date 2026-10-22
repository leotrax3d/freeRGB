import { describe, expect, it } from "vitest";
import fixtures from "./fixtures/effects.json";
import { EFFECTS, EFFECT_FIELDS } from "./effects";
import type { EffectSpec } from "./types";

// fixtures/effects.json is generated from Rust (`src-tauri/tests/fixtures.rs`),
// so a renamed or added serde field fails here instead of at runtime.
describe("effect contract", () => {
  const defaults = fixtures as Record<string, EffectSpec>;

  it("knows every daemon effect, in the daemon's order", () => {
    expect(EFFECTS.map((e) => e.name).sort()).toEqual(Object.keys(defaults).sort());
  });

  it.each(Object.entries(defaults))("%s has the expected fields", (name, spec) => {
    expect(spec.type).toBe(name);
    const fields = Object.keys(spec).filter((k) => k !== "type");
    expect(fields.sort()).toEqual([...EFFECT_FIELDS[spec.type]].sort());
  });
});
