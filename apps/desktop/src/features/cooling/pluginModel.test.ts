import { describe, expect, it } from "vitest";
import { dutyChoices, failsafeNote, pluginOf, pluginState, sourceBadge, type PluginView } from "./pluginModel";

const base: PluginView = { id: "cooling-virtual", version: "0.1.0", family: "virtual", license: "MIT", installed: true, enabled: true, virtual: true, device_found: true };

describe("plugin model", () => {
  it("derives a state from install, enable, status and device", () => {
    expect(pluginState({ ...base, installed: false })).toBe("available");
    expect(pluginState({ ...base, enabled: false })).toBe("disabled");
    expect(pluginState(base)).toBe("starting");
    expect(pluginState({ ...base, status: { id: "x", running: true, fault: null, failsafe: null, controls: [], virtual_io: true } })).toBe("running");
    expect(pluginState({ ...base, error: "crashed" })).toBe("faulted");
    expect(pluginState({ ...base, virtual: false, device_found: false })).toBe("no_device");
  });

  it("never offers a reduced duty for pumps or unknown channels", () => {
    expect(dutyChoices({ id: "p", name: "Pump", role: "pump", floor_percent: 100 })).toEqual([100]);
    expect(dutyChoices({ id: "u", name: "?", role: "unknown", floor_percent: 100 })).toEqual([100]);
    expect(dutyChoices({ id: "f", name: "Fan", role: "fan", floor_percent: 45 })).toEqual([50, 70, 100]);
  });

  it("labels sources and fail-safe outcomes", () => {
    expect(sourceBadge(base)).toBe("Plugin · cooling-virtual · virtual");
    expect(sourceBadge({ id: "corsair-link-cooling", virtual: false })).toBe("Plugin · corsair-link-cooling");
    expect(pluginOf("plugin:corsair-link-cooling/slot0_rpm")).toBe("corsair-link-cooling");
    expect(pluginOf("mainboard.lhm:x")).toBeNull();
    expect(failsafeNote(null)).toBeNull();
    expect(failsafeNote({ id: "x", running: false, fault: "hang", failsafe: { reports: 1, written: 1, errors: [] }, controls: [], virtual_io: false })).toContain("1/1");
    expect(failsafeNote({ id: "x", running: false, fault: "hang", failsafe: { reports: 1, written: 0, errors: ["disconnected"] }, controls: [], virtual_io: false })).toContain("incomplete");
  });
});
