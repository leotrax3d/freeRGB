import { describe, expect, it } from "vitest";
import { approxEqual } from "./approxEqual";

describe("approxEqual", () => {
  it("treats f32 round-trips as equal but real edits as different", () => {
    expect(approxEqual({ type: "rainbow_wave", speed: 0.05, scale: 1 }, { type: "rainbow_wave", speed: 0.05000000074505806, scale: 1 })).toBe(true);
    expect(approxEqual({ speed: 0.05 }, { speed: 0.1 })).toBe(false);
    expect(approxEqual({ period_ms: 3000 }, { period_ms: 3100 })).toBe(false);
    expect(approxEqual({ a: 1 }, { a: 1, b: 2 })).toBe(false);
    expect(approxEqual([1, 2], { 0: 1, 1: 2 })).toBe(false);
  });
});
