import { describe, expect, it } from "vitest";
import { EMPTY_SHAPING, parseAvoid, shapingPayload } from "./shapingModel";

describe("output shaping draft", () => {
  it("sends nothing for an untouched draft", () => {
    expect(shapingPayload(EMPTY_SHAPING, false)).toBeNull();
  });

  it("parses percent and RPM resonance ranges", () => {
    expect(parseAvoid("50-60, 1200-1300 rpm")).toEqual([{ from: 50, to: 60 }, { from: 1200, to: 1300, unit: "rpm" }]);
    expect(parseAvoid("60-50")).toMatch(/from < to/);
    expect(parseAvoid("abc")).toMatch(/Cannot read/);
  });

  it("maps the draft to the daemon fields", () => {
    expect(shapingPayload({ ...EMPTY_SHAPING, offset: 5, delayS: 2.5, upPercent: 3, spinUpPercent: 70, spinUpMs: 800 }, false)).toEqual({
      offset_percent: 5,
      response_delay_ms: 2500,
      hysteresis_up_percent: 3,
      spin_up: { percent: 70, ms: 800 },
    });
  });

  it("allows fan stop only on unrestricted channels", () => {
    expect(shapingPayload({ ...EMPTY_SHAPING, stopBelow: 20 }, false)).toMatch(/unrestricted/);
    expect(shapingPayload({ ...EMPTY_SHAPING, stopBelow: 20 }, true)).toEqual({ stop_below_percent: 20 });
    expect(shapingPayload({ ...EMPTY_SHAPING, min: 70, max: 60 }, true)).toMatch(/min/);
  });
});
