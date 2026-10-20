import { describe, expect, it } from "vitest";
import fixtures from "@/lib/api/fixtures/effects.json";
import { EFFECT_FIELDS } from "@/lib/api/effects";
import { SCHEMA } from "./schema";

describe("effect parameter schema", () => {
  it.each(Object.entries(SCHEMA))("%s edits exactly the daemon's fields", (name, { fields }) => {
    expect(fields.map((f) => f.key).sort()).toEqual([...EFFECT_FIELDS[name as keyof typeof SCHEMA]].sort());
  });

  it.each(Object.entries(SCHEMA))("%s ranges stay inside daemon bounds and contain the default", (name, { fields }) => {
    const defaults = (fixtures as Record<string, Record<string, unknown>>)[name]!;
    for (const field of fields) {
      if (field.kind !== "range") continue;
      expect(field.min).toBeGreaterThanOrEqual(field.daemon[0]);
      expect(field.max).toBeLessThanOrEqual(field.daemon[1]);
      const value = defaults[field.key] as number;
      expect(value, `${name}.${field.key}`).toBeGreaterThanOrEqual(field.min);
      expect(value, `${name}.${field.key}`).toBeLessThanOrEqual(field.max);
    }
  });
});
