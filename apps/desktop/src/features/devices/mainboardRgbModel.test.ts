import { describe, expect, it } from "vitest";
import type { DeviceSummary } from "@/lib/api/types";
import { controllerKey, mainboardControllers, parseSettings, presetLeds, withHeader, type MainboardRgbSettings } from "./mainboardRgbModel";

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

describe("motherboard RGB import/export", () => {
  it("round-trips order and fallback and rejects bad files", () => {
    const s: MainboardRgbSettings = { enabled: true, controllers: { "0b05:19af": [{ leds: 8, order: "grb", fallback: { mode: "static", color: "#ff8800" } }, { leds: 0 }] } };
    expect(parseSettings(JSON.stringify(s))).toEqual(s);
    expect(parseSettings(JSON.stringify({ enabled: true, controllers: { "0b05:19af": [{ leds: 4, order: "rgb", fallback: { mode: "spectrum" } }] } }))).toEqual({ enabled: true, controllers: { "0b05:19af": [{ leds: 4 }] } });
    for (const bad of ["nope", "[]", '{"enabled":true,"controllers":{"0B05:19AF":[]}}', '{"enabled":true,"controllers":{"0b05:19af":[{"leds":600}]}}', '{"enabled":true,"controllers":{"0b05:19af":[{"leds":1,"order":"xyz"}]}}', '{"enabled":true,"controllers":{"0b05:19af":[{"leds":1,"fallback":{"mode":"static","color":"red"}}]}}']) {
      expect(parseSettings(bad)).toBeNull();
    }
  });
});


describe("shared Headers setup", () => {
  it("offers all six IT5711 slots and the other vendor families", () => {
    const msi = device(0x1462, 0x7b12);
    msi.info.id = "module:msi-motherboard:1462:7b12:fixture";
    const list = mainboardControllers([device(0x048d, 0x5711), device(0x048d, 0x8950), device(0x0414, 0xa100), device(0x26ce, 0x01a2), msi], null);
    expect(list.find((c) => c.key === "048d:5711")?.headers).toBe(6);
    expect(list).toHaveLength(5);
    const settings = { enabled: true, controllers: { "048d:5711": Array.from({ length: 6 }, () => ({ leds: 12 })) } };
    expect(parseSettings(JSON.stringify(settings))).toEqual(settings);
    expect(parseSettings(JSON.stringify({ ...settings, controllers: { "0b05:19af": settings.controllers["048d:5711"] } }))).toBeNull();
  });
  it("round-trips canvas mapping and limits RGB headers to a single colour", () => {
    const s = withHeader({ enabled: true, controllers: {} }, "26ce:01a2", 0, { type: "rgb", leds: 16, region: { x: .2, y: .1, w: .5, h: .5 }, order: "bgr", name: "Front" });
    expect(s.controllers["26ce:01a2"]![0]!.leds).toBe(1);
    expect(parseSettings(JSON.stringify(s))).toEqual(s);
    s.controllers["26ce:01a2"]![0]!.region!.w = 0;
    expect(parseSettings(JSON.stringify(s))).toBeNull();
  });
});
