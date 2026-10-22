import { describe, expect, it } from "vitest";
import { moved, normalize, nudged, resized } from "./region";

const valid = (r: { x: number; y: number; w: number; h: number }) =>
  r.x >= 0 && r.x <= 1 && r.y >= 0 && r.y <= 1 && r.w >= 0.01 && r.w <= 1 && r.h >= 0.01 && r.h <= 1;

describe("region editing", () => {
  const start = { x: 0.1, y: 0.2, w: 0.3, h: 0.4 };

  it("moves with snapping and stays inside the canvas", () => {
    expect(moved(start, 0.12, 0.01)).toEqual({ x: 0.2, y: 0.2, w: 0.3, h: 0.4 });
    expect(moved(start, 5, 5)).toEqual({ x: 0.7, y: 0.6, w: 0.3, h: 0.4 });
    expect(moved(start, -5, -5)).toEqual({ x: 0, y: 0, w: 0.3, h: 0.4 });
    expect(moved(start, 0.013, 0, false).x).toBeCloseTo(0.113);
  });

  it("resizes from the corner, never below the minimum or past the edge", () => {
    expect(resized(start, 0.2, 0.1)).toEqual({ x: 0.1, y: 0.2, w: 0.5, h: 0.5 });
    expect(resized(start, -1, -1)).toEqual({ x: 0.1, y: 0.2, w: 0.05, h: 0.05 });
    expect(resized(start, 2, 2)).toEqual({ x: 0.1, y: 0.2, w: 0.9, h: 0.8 });
  });

  it("nudges by keyboard and every result is a valid daemon region", () => {
    expect(nudged(start, "ArrowRight", 0.01, false)).toEqual({ ...start, x: 0.11 });
    expect(nudged(start, "ArrowDown", 0.05, true)).toEqual({ ...start, h: 0.45 });
    expect(nudged(start, "Enter", 0.01, false)).toBeNull();
    for (const r of [normalize({ x: -1, y: 2, w: 0, h: 9 }), normalize({ x: 0, y: 0.99, w: 0.5, h: 0.5 })]) {
      expect(valid(r)).toBe(true);
    }
  });
});
