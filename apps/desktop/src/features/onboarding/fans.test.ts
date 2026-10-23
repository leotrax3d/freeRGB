import { describe, expect, it } from "vitest";
import type { ChannelBinding, CoolingChannel, MonitoringSensor } from "@/lib/api/types";
import { fanHeaders, splitId, upsertBinding } from "./fans";

const reading = (id: string, unit: string, value = 50) => ({ id, name: id, unit, value, age_ms: 0 });
const control = (hw: string, i: number, extra: Partial<CoolingChannel> = {}): CoolingChannel => ({
  provider: "mainboard.lhm", id: JSON.stringify([hw, `${hw}/control/${i}`]), device_id: `mainboard.lhm:${hw}`, name: `Fan Control #${i}`,
  kind: "control", reading: reading(`${hw}/control/${i}`, "%"), control_available: true,
  support: { tier: "verified", register_readback: true, software_mode: false, held_full: false }, ...extra,
});
const fan = (hw: string, i: number): CoolingChannel => ({
  provider: "mainboard.lhm", id: JSON.stringify([hw, `${hw}/fan/${i}`]), device_id: `mainboard.lhm:${hw}`, name: `Fan #${i}`,
  kind: "fan", reading: reading(`${hw}/fan/${i}`, "RPM", 900), control_available: false,
});
const temp: MonitoringSensor = { provider: "mainboard.lhm", device_id: "x", device_name: "x", sensor: reading("/lpc/nct/0/temperature/1", "°C", 40) };

describe("fan header pairing", () => {
  it("pairs controls with the same-index tachometer and a board temperature", () => {
    const [h] = fanHeaders([control("/lpc/nct/0", 2), fan("/lpc/nct/0", 2)], [temp]);
    expect(h!.rpm?.reading.id).toBe("/lpc/nct/0/fan/2");
    expect(h!.temperature?.sensor.id).toBe("/lpc/nct/0/temperature/1");
    expect(h!.blocked).toBeNull();
    expect(splitId("/lpc/nct/0/control/2")).toEqual(["/lpc/nct/0", "2"]);
  });

  it("marks headers that must not be tested", () => {
    const headers = fanHeaders([
      control("/lpc/nct/0", 1),
      control("/lpc/nct/0", 3, { support: { tier: "lhm_supported", register_readback: false, software_mode: true, held_full: false } }),
      control("/lpc/nct/0", 4, { control_available: false }),
      fan("/lpc/nct/0", 3),
    ], [temp]);
    expect(headers.map((h) => h.blocked)).toEqual([
      expect.stringMatching(/No matching RPM/),
      expect.stringMatching(/Another program/),
      expect.stringMatching(/No usable fan control/),
    ]);
    expect(fanHeaders([control("/lpc/nct/0", 1), fan("/lpc/nct/0", 1)], [])[0]!.blocked).toMatch(/temperature/);
  });
});

describe("binding updates", () => {
  const base: ChannelBinding = { provider: "mainboard.lhm", device_id: "d", control_id: "c1", tachometer_id: "t1", header: "CPU_FAN", role: "fan", electrical_mode: "unknown" };
  it("replaces the binding of the same control", () => {
    const next = upsertBinding([base], { ...base, header: "Rear", role: "pump" });
    expect(next).toEqual([{ ...base, header: "Rear", role: "pump" }]);
  });
  it("rejects duplicate header names and tachometers", () => {
    expect(() => upsertBinding([base], { ...base, control_id: "c2", tachometer_id: "t2", header: "cpu_fan" })).toThrow(/already used/);
    expect(() => upsertBinding([base], { ...base, control_id: "c2", header: "Other" })).toThrow(/RPM sensor/);
  });
});
