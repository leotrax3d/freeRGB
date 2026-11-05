import { describe, expect, it } from "vitest";
import type { CoolingChannel } from "@/lib/api/types";
import { chipSummaries, correlateTach, mergePeak, proposeCurve, rpmSnapshot } from "./setupModel";

const ch = (id: string, kind: "fan" | "control", value: number, tier?: "verified" | "lhm_supported"): CoolingChannel => ({
  provider: "mainboard.lhm", id, device_id: "d", name: id, kind, control_available: kind === "control",
  reading: { id, name: id, unit: kind === "fan" ? "RPM" : "%", value, age_ms: 0 },
  support: tier ? { tier, register_readback: tier === "verified", software_mode: false, held_full: false } : null,
});

describe("guided setup model", () => {
  it("groups channels per chip with tier counts", () => {
    const chips = chipSummaries([ch("/lpc/chipa/0/control/0", "control", 50, "verified"), ch("/lpc/chipa/0/fan/0", "fan", 900), ch("/lpc/chipb/0/control/1", "control", 50, "lhm_supported")]);
    expect(chips).toHaveLength(2);
    expect(chips[0]).toMatchObject({ name: "CHIPA", controls: 1, tachometers: 1, tiers: { verified: 1 } });
    expect(chips[1]!.tiers).toEqual({ lhm_supported: 1 });
  });

  it("pairs the tachometer with a clear RPM rise", () => {
    const before = rpmSnapshot([ch("a", "fan", 800), ch("b", "fan", 1200), ch("c", "control", 50)]);
    expect(before).toEqual({ a: 800, b: 1200 });
    const peak = mergePeak(mergePeak({}, { a: 820, b: 1300 }), { a: 1600, b: 1250 });
    expect(correlateTach(before, peak)).toBe("a");
  });

  it("refuses ambiguous or tiny rises", () => {
    expect(correlateTach({ a: 800, b: 800 }, { a: 1200, b: 1150 })).toBeNull();
    expect(correlateTach({ a: 2000 }, { a: 2100 })).toBeNull();
    expect(correlateTach({}, {})).toBeNull();
  });

  it("proposes curves for fans only and never below the floor", () => {
    expect(proposeCurve("pump", 40)).toBeNull();
    expect(proposeCurve(null, 40)).toBeNull();
    const c = proposeCurve("fan", 50)!;
    expect(c.minimum_percent).toBe(50);
    expect(Math.min(...c.points.map((p) => p.duty_percent))).toBe(50);
    expect(c.points.at(-1)!.duty_percent).toBe(100);
    expect(c.critical_c).toBeLessThanOrEqual(100);
  });
});

describe("role gate", () => {
  it("lists every header with a tachometer but no role", async () => {
    const { unassignedRoles } = await import("./setupModel");
    const views = [
      { id: "c1", role: null, rpm: { id: "f1" }, rpmValue: 1700 },
      { id: "c2", role: "pump" as const, rpm: { id: "f2" }, rpmValue: 1750 },
      { id: "c3", role: null, rpm: null, rpmValue: null },
      { id: "c4", role: "fan" as const, rpm: { id: "f4" }, rpmValue: 900 },
      { id: "c5", role: null, rpm: null, rpmValue: 600 },
    ];
    expect(unassignedRoles(views).map((v) => v.id)).toEqual(["c1", "c5"]);
    expect(unassignedRoles(views.filter((v) => v.role !== null))).toEqual([]);
  });
});
