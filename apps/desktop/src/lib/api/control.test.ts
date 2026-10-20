import { describe, expect, it } from "vitest";
import { chordProblem, curveProblem, DEFAULT_CURVE, macroProblem } from "./control";

describe("fan curve rules (mirror FanCurve::validate)", () => {
  it("accepts the daemon default", () => expect(curveProblem(DEFAULT_CURVE)).toBeNull());
  it("rejects decreasing duty, unordered temperatures and points past critical", () => {
    const pts = DEFAULT_CURVE.points;
    expect(curveProblem({ ...DEFAULT_CURVE, points: [pts[0]!, { temperature_c: 50, duty_percent: 29 }] })).toMatch(/not decrease/);
    expect(curveProblem({ ...DEFAULT_CURVE, points: [pts[1]!, pts[0]!] })).toMatch(/increase/);
    expect(curveProblem({ ...DEFAULT_CURVE, critical_c: 70 })).toMatch(/critical/);
    expect(curveProblem({ ...DEFAULT_CURVE, points: [pts[0]!] })).toMatch(/2 to 16/);
    expect(curveProblem({ ...DEFAULT_CURVE, minimum_percent: 40 })).toMatch(/between the minimum/);
  });
});

describe("macro rules (mirror MacroDefinition::validate)", () => {
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
