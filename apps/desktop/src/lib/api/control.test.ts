import { describe, expect, it } from "vitest";
import { chordProblem, curveProblem, DEFAULT_CURVE, macroProblem } from "./control";

describe("fan curve rules (mirror FanCurve::validate)", () => {
  it("rejects non-finite limits, rates and point values before JSON turns them into null", () => {
    for (const value of [NaN, Infinity, -Infinity]) {
      for (const key of ["minimum_percent", "critical_c", "hysteresis_c", "max_rise_percent_per_s", "max_fall_percent_per_s"] as const)
        expect(curveProblem({ ...DEFAULT_CURVE, [key]: value })).not.toBeNull();
      for (const key of ["temperature_c", "duty_percent"] as const)
        expect(curveProblem({ ...DEFAULT_CURVE, points: [{ ...DEFAULT_CURVE.points[0]!, [key]: value }, DEFAULT_CURVE.points[1]!] })).not.toBeNull();
    }
  });
  it("accepts the daemon default", () => expect(curveProblem(DEFAULT_CURVE)).toBeNull());
  it("rejects decreasing duty, unordered temperatures and points past critical", () => {
    const pts = DEFAULT_CURVE.points;
    expect(curveProblem({ ...DEFAULT_CURVE, points: [pts[0]!, { temperature_c: 50, duty_percent: 29 }] })).toMatch(/not decrease/);
    expect(curveProblem({ ...DEFAULT_CURVE, points: [pts[1]!, pts[0]!] })).toMatch(/increase/);
    expect(curveProblem({ ...DEFAULT_CURVE, critical_c: 70 })).toMatch(/critical/);
    expect(curveProblem({ ...DEFAULT_CURVE, points: [pts[0]!] })).toMatch(/2 to 64/);
    expect(curveProblem({ ...DEFAULT_CURVE, minimum_percent: 40 })).toMatch(/between the minimum/);
  });
});

describe("macro rules (mirror MacroDefinition::validate)", () => {
  it("rejects negative, fractional and non-finite delays and non-finite brightness", () => {
    for (const ms of [-1, 0.5, NaN, Infinity, -Infinity])
      expect(macroProblem({ repeat: 1, steps: [{ type: "delay", ms }] })).not.toBeNull();
    for (const value of [NaN, Infinity, -Infinity])
      expect(macroProblem({ repeat: 1, steps: [{ type: "brightness", value }] })).not.toBeNull();
    expect(macroProblem({ repeat: 1, steps: [{ type: "delay", ms: 0 }] })).toBeNull();
  });
  const ok = { repeat: 1, steps: [{ type: "brightness" as const, value: 0.5 }] };
  it("accepts a simple macro", () => expect(macroProblem(ok)).toBeNull());
  it("enforces step, repeat and delay limits", () => {
    expect(macroProblem({ ...ok, steps: [] })).not.toBeNull();
    expect(macroProblem({ ...ok, repeat: 21 })).not.toBeNull();
    expect(macroProblem({ repeat: 1, steps: [{ type: "delay", ms: 10_001 }] })).toMatch(/10 s/);
    expect(macroProblem({ repeat: 7, steps: [{ type: "delay", ms: 9_000 }] })).toMatch(/60 s/);
    expect(macroProblem({ repeat: 1, steps: [{ type: "text", text: "é".repeat(513) }] })).toMatch(/1024/);
  });
  it("validates chords like chord_plan", () => {
    expect(chordProblem(["ctrl", "c"])).toBeNull();
    expect(chordProblem(["ctrl", "shift", "alt", "meta", "f"])).toBeNull();
    expect(chordProblem(["c"])).not.toBeNull();
    expect(chordProblem(["ctrl", "ctrl", "c"])).not.toBeNull();
    expect(chordProblem(["ctrl", "a", "b"])).not.toBeNull();
    expect(chordProblem(["ctrl", "f5"])).not.toBeNull();
  });
});
