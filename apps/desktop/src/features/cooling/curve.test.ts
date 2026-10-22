import { describe, expect, it } from "vitest";
import { curveProblem, DEFAULT_CURVE } from "@/lib/api/control";
import { addPoint, movePoint, removePoint, withMinimum } from "./curve";

describe("curve editing keeps curves valid", () => {
  it("clamps dragged points between their neighbours", () => {
    const c = movePoint(DEFAULT_CURVE, 1, 200, 0);
    expect(c.points[1]).toEqual({ temperature_c: 74, duty_percent: 30 });
    expect(curveProblem(c)).toBeNull();
    const last = movePoint(DEFAULT_CURVE, 2, 109, 100);
    expect(last.points[2]!.temperature_c).toBe(85); // critical temperature
  });

  it("adds into the widest gap and never below two points", () => {
    const added = addPoint(DEFAULT_CURVE);
    expect(added.points).toHaveLength(4);
    expect(curveProblem(added)).toBeNull();
    expect(removePoint(removePoint(DEFAULT_CURVE, 0), 0).points).toHaveLength(2);
  });

  it("raising the minimum lifts low points", () => {
    const c = withMinimum(DEFAULT_CURVE, 40);
    expect(c.points[0]!.duty_percent).toBe(40);
    expect(curveProblem(c)).toBeNull();
  });

  it("random drags never produce an invalid curve", () => {
    let c = DEFAULT_CURVE;
    for (let i = 0; i < 500; i++) {
      c = movePoint(c, i % c.points.length, Math.random() * 160 - 30, Math.random() * 140 - 20);
      if (i % 50 === 0) c = addPoint(c);
      expect(curveProblem(c)).toBeNull();
    }
  });
});
