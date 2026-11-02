import { describe, expect, it } from "vitest";
import { curveSetProblem } from "@/lib/api/control";

describe("curveSetProblem", () => {
  it("accepts a valid composite set", () => {
    expect(curveSetProblem({ nodes: { a: { type: "constant", percent: 40 }, b: { type: "follow", source: "a", scale: 1, offset_percent: 5 } } })).toBeNull();
  });
  it("rejects loops and dangling sources", () => {
    expect(curveSetProblem({ nodes: { a: { type: "follow", source: "b", scale: 1, offset_percent: 0 }, b: { type: "follow", source: "a", scale: 1, offset_percent: 0 } } })).toMatch(/loop/);
    expect(curveSetProblem({ nodes: { m: { type: "mix", sources: ["x"], function: "sum" } } })).toMatch(/existing/);
  });
  it("rejects inverted trigger thresholds", () => {
    expect(curveSetProblem({ nodes: { t: { type: "trigger", sensor: "s", idle_c: 60, load_c: 50, idle_percent: 30, load_percent: 70, critical_c: 85, hold_ms: 0 } } })).toMatch(/idle < load/);
  });
});
