import { describe, expect, it } from "vitest";
import type { DeviceSummary } from "@/lib/api/types";
import { controllerKey, mainboardControllers, presetLeds, withHeader, type MainboardRgbSettings } from "./mainboardRgb";

const device = (vid: number, pid: number) => ({ info: { id: `hid:${controllerKey(vid, pid)}:x`, name: "", vendor: "", kind: "LightingController", usb_vid: vid, usb_pid: pid, serial: null, firmware: null } }) as unknown as DeviceSummary;

describe("motherboard RGB settings", () => {
  it("lists detected controllers and saved ones", () => {
    const settings: MainboardRgbSettings = { enabled: true, controllers: { "048d:5702": [{ leds: 16 }] } };
    const list = mainboardControllers([device(0x0b05, 0x19af), device(0x1b1c, 0x1bfd)], settings);
    expect(list.map((c) => [c.key, c.label, c.headers, c.device !== null])).toEqual([
      ["048d:5702", "Gigabyte RGB Fusion 2", 2, false],
      ["0b05:19af", "ASUS Aura", 4, true],
    ]);
  });

  it("edits one header, clamps counts and drops blank names", () => {
    let s: MainboardRgbSettings = { enabled: true, controllers: {} };
    s = withHeader(s, "0b05:19af", 2, { leds: 700, name: "  Front fans  " });
    expect(s.controllers["0b05:19af"]).toEqual([{ leds: 0 }, { leds: 0 }, { leds: 512, name: "Front fans" }]);
    s = withHeader(s, "0b05:19af", 2, { leds: -3, name: " " });
    expect(s.controllers["0b05:19af"]![2]).toEqual({ leds: 0 });
    expect(presetLeds(16, 3)).toBe(48);
    expect(presetLeds(8, 0)).toBe(8);
  });
});
