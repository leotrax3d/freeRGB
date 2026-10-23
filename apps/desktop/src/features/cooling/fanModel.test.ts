import { describe, expect, it } from "vitest";
import type { BackgroundChannel, CoolingChannel, MonitoringSensor } from "@/lib/api/types";
import { DEFAULT_CURVE } from "@/lib/api/control";
import { fanViews, fixedCurve, floorFor, isFixed, withChannel } from "./fanModel";

const hw = "/lpc/nct/0";
const reading = (id: string, unit: string, value: number) => ({ id, name: id, unit, value, age_ms: 0 });
const channels: CoolingChannel[] = [
  { provider: "mainboard.lhm", id: JSON.stringify([hw, `${hw}/control/2`]), device_id: "d", name: "Fan Control #2", kind: "control", reading: reading(`${hw}/control/2`, "%", 55), control_available: true,
    support: { tier: "verified", register_readback: true, software_mode: false, held_full: false } },
  { provider: "mainboard.lhm", id: JSON.stringify([hw, `${hw}/fan/2`]), device_id: "d", name: "Fan #2", kind: "fan", reading: reading(`${hw}/fan/2`, "RPM", 1200), control_available: false },
];
const sensors: MonitoringSensor[] = [{ provider: "mainboard.lhm", device_id: "d", device_name: "d", sensor: reading(`${hw}/temperature/1`, "°C", 40) }];
const saved = (curve = DEFAULT_CURVE): BackgroundChannel => ({ control_id: `${hw}/control/2`, temperature_id: `${hw}/temperature/1`, rpm_id: `${hw}/fan/2`, curve, confirmed_fan: true });
const base = { channels, sensors, bindings: [], saved: [], service: [], servicePaused: false, sessions: [], evidence: {}, curves: { Quiet: DEFAULT_CURVE } };

describe("fan cards", () => {
  it("explains unidentified, BIOS, curve, fixed, paused and faulted headers", () => {
    expect(fanViews(base)[0]!.status).toMatch(/Not identified/);
    const fan = { ...base, bindings: [{ provider: "mainboard.lhm", device_id: "d", control_id: channels[0]!.id, tachometer_id: null, header: "CPU_FAN", role: "fan" as const, electrical_mode: "unknown" as const }] };
    expect(fanViews(fan)[0]).toMatchObject({ name: "CPU_FAN", role: "fan", mode: "bios", status: "BIOS controls this fan", rpmValue: 1200 });
    const curve = fanViews({ ...fan, saved: [saved()], service: [{ control_id: `${hw}/control/2`, tier: "verified", state: "running", duty_percent: 45, fault: null, warning: null }] })[0]!;
    expect(curve.mode).toBe("curve");
    expect(curve.status).toMatch(/curve “Quiet”.*now 45 %/);
    expect(curve.tone).toBe("ok");
    expect(fanViews({ ...fan, saved: [saved(fixedCurve(60))] })[0]!.mode).toBe("fixed");
    expect(fanViews({ ...fan, saved: [saved()], servicePaused: true })[0]!.status).toMatch(/Paused/);
    expect(fanViews({ ...fan, saved: [saved()], service: [{ control_id: `${hw}/control/2`, tier: "verified", state: "faulted", duty_percent: 100, fault: "rpm_implausible", warning: null }] })[0]!.tone).toBe("error");
  });

  it("keeps fixed curves flat and replaces one channel at a time", () => {
    expect(isFixed(fixedCurve(55))).toBe(true);
    expect(isFixed(DEFAULT_CURVE)).toBe(false);
    expect(fixedCurve(140).minimum_percent).toBe(100);
    const other = { ...saved(), control_id: `${hw}/control/3` };
    expect(withChannel([saved(), other], `${hw}/control/2`, null)).toEqual([other]);
    expect(withChannel([other], `${hw}/control/2`, saved())).toHaveLength(2);
  });

  it("offers the same floors the service enforces", () => {
    expect(floorFor({ role: null, evidence: null, tier: "verified" }, false)).toBe(100);
    expect(floorFor({ role: "pump", evidence: null, tier: "verified" }, false)).toBe(100);
    expect(floorFor({ role: "fan", evidence: null, tier: "verified" }, false)).toBe(40);
    expect(floorFor({ role: "fan", evidence: { test: "none", tested: null, unlock: "none" }, tier: "lhm_supported" }, false)).toBe(100);
    expect(floorFor({ role: "fan", evidence: { test: "none", tested: null, unlock: "unlocked" }, tier: "lhm_supported" }, true)).toBe(0);
    expect(floorFor({ role: "pump", evidence: { test: "none", tested: null, unlock: "unlocked_pump" }, tier: "verified" }, true)).toBe(0);
  });
});
