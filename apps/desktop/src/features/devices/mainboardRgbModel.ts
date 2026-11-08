import type { DeviceSummary } from "@/lib/api/types";

/** Daemon `mainboard_rgb.get` view (crates/freergb-daemon/src/mainboard_rgb.rs). */
export const COLOR_ORDERS = ["rgb", "rbg", "grb", "gbr", "brg", "bgr"] as const;
export type ColorOrder = (typeof COLOR_ORDERS)[number];
export type Fallback = { mode: "spectrum" } | { mode: "off" } | { mode: "static"; color: string };
export interface MainboardHeader { type?: "rgb" | "argb"; region?: { x: number; y: number; w: number; h: number }; leds: number; name?: string | null; order?: ColorOrder; fallback?: Fallback }
export interface DetectedHeader { present: boolean; max_leds: number; firmware_leds?: number; firmware_max_leds?: number; start?: number; count: number }
export interface DetectedController { fixed_leds: number; headers: DetectedHeader[] }
export interface MainboardRgbSettings { enabled: boolean; controllers: Record<string, MainboardHeader[]> }
export interface MainboardRgbView {
  settings: MainboardRgbSettings;
  fixed_headers?: Record<string, MainboardHeader[]>;
  detected?: Record<string, DetectedController>;
  max_headers: number;
  max_leds: number;
  fan_presets: number[];
  vendor_software: string[];
  error: string | null;
}

/** USB IDs of motherboard RGB controllers freeRGB drives, with their header count. */
const FAMILIES: { vid: number; pids: number[]; headers: number; label: string }[] = [
  { vid: 0x0b05, pids: [0x18f3, 0x1939, 0x19af, 0x1aa6, 0x1bed], headers: 4, label: "ASUS Aura" },
  { vid: 0x048d, pids: [0x5702, 0x8297], headers: 2, label: "Gigabyte RGB Fusion 2" },
  { vid: 0x048d, pids: [0x5711], headers: 6, label: "Gigabyte IT5711 (4 external + 2 onboard)" },
  { vid: 0x048d, pids: [0x8950], headers: 2, label: "Gigabyte RGB Fusion 2" },
  { vid: 0x0414, pids: [0xa100], headers: 2, label: "Gigabyte RGB Fusion 2" },
  { vid: 0x26ce, pids: [0x01a2, 0x01a6], headers: 3, label: "ASRock Polychrome" },
];

export const controllerKey = (vid: number, pid: number) =>
  `${vid.toString(16).padStart(4, "0")}:${pid.toString(16).padStart(4, "0")}`;

export interface MainboardController { key: string; label: string; headers: number; device: DeviceSummary | null }

/** Controllers to configure: detected motherboard devices plus already saved keys. */
export function mainboardControllers(devices: DeviceSummary[] | null | undefined, settings: MainboardRgbSettings | null | undefined): MainboardController[] {
  const out = new Map<string, MainboardController>();
  for (const d of devices ?? []) {
    const { usb_vid: vid, usb_pid: pid } = d.info;
    const family = FAMILIES.find((f) => f.vid === vid && pid !== null && f.pids.includes(pid))
      ?? (d.info.id.startsWith("module:msi-motherboard:") ? { vid: vid!, headers: 4, label: "MSI Mystic Light" } : undefined);
    if (family && pid !== null) out.set(controllerKey(family.vid, pid), { key: controllerKey(family.vid, pid), label: family.label, headers: family.headers, device: d });
  }
  for (const key of Object.keys(settings?.controllers ?? {})) {
    if (out.has(key)) continue;
    const [vid, pid] = key.split(":").map((x) => parseInt(x, 16));
    const family = FAMILIES.find((f) => f.vid === vid && f.pids.includes(pid!));
    out.set(key, { key, label: family?.label ?? ((vid === 0x1462 || vid === 0x0db0) ? "MSI Mystic Light" : key), headers: family?.headers ?? 4, device: null });
  }
  return [...out.values()].sort((a, b) => a.key.localeCompare(b.key));
}

/** Settings with one header changed; LED counts are clamped, empty names dropped. */
export function withHeader(settings: MainboardRgbSettings, key: string, index: number, patch: Partial<MainboardHeader>, maxLeds = 512): MainboardRgbSettings {
  const headers = [...(settings.controllers[key] ?? [])];
  while (headers.length <= index) headers.push({ leds: 0 });
  const next = { ...headers[index]!, ...patch };
  next.leds = Math.max(0, Math.min(maxLeds, Math.round(Number.isFinite(next.leds) ? next.leds : 0)));
  if (next.type === "rgb") next.leds = Math.min(1, next.leds);
  const name = next.name?.slice(0, 40).trim();
  if (name) next.name = name; else delete next.name;
  headers[index] = normalizeHeader(next);
  return { ...settings, controllers: { ...settings.controllers, [key]: headers } };
}

/** Total LEDs a preset fills: `fans` fans with `per` LEDs each, chained on one header. */
export const presetLeds = (per: number, fans: number) => Math.max(0, Math.round(per)) * Math.max(1, Math.round(fans));

/** Header families whose adapter honours colour order and release fallback. */
export const supportsHeaderOptions = (_key: string) => true;

const HEX = /^#[0-9a-f]{6}$/;

/** Drops default order/fallback so saved files stay minimal and old files stay valid. */
export function normalizeHeader(header: MainboardHeader): MainboardHeader {
  const out = { ...header };
  if (out.order === "rgb") delete out.order;
  if (out.fallback?.mode === "spectrum") delete out.fallback;
  return out;
}

/** Parses an exported settings file; returns null for anything that is not valid settings. */
export function parseSettings(text: string, maxLeds = 512): MainboardRgbSettings | null {
  let value: unknown;
  try { value = JSON.parse(text); } catch { return null; }
  if (typeof value !== "object" || value === null) return null;
  const v = value as { enabled?: unknown; controllers?: unknown };
  if (typeof v.enabled !== "boolean" || typeof v.controllers !== "object" || v.controllers === null) return null;
  const controllers: Record<string, MainboardHeader[]> = {};
  for (const [key, list] of Object.entries(v.controllers)) {
    if (!/^[0-9a-f]{4}:[0-9a-f]{4}$/.test(key) || !Array.isArray(list) || list.length > (key === "048d:5711" ? 6 : 4)) return null;
    const headers: MainboardHeader[] = [];
    for (const h of list as Record<string, unknown>[]) {
      if (typeof h !== "object" || h === null) return null;
      const leds = h.leds ?? 0;
      if (typeof leds !== "number" || !Number.isInteger(leds) || leds < 0 || leds > maxLeds) return null;
      const header: MainboardHeader = { leds };
      if (h.type != null) { if (h.type !== "rgb" && h.type !== "argb") return null; header.type = h.type; }
      if (header.type === "rgb" && leds > 1) return null;
      if (h.region != null) {
        const r = h.region as MainboardHeader["region"];
        if (!r || ![r.x, r.y, r.w, r.h].every(Number.isFinite) || r.x < 0 || r.x > 1 || r.y < 0 || r.y > 1 || r.w < .01 || r.w > 1 || r.h < .01 || r.h > 1) return null;
        header.region = r;
      }
      if (h.name != null) { if (typeof h.name !== "string" || !h.name.trim() || h.name.length > 40) return null; header.name = h.name; }
      if (h.order != null) { if (!(COLOR_ORDERS as readonly unknown[]).includes(h.order)) return null; header.order = h.order as ColorOrder; }
      if (h.fallback != null) {
        const f = h.fallback as { mode?: unknown; color?: unknown };
        if (f.mode === "spectrum" || f.mode === "off") header.fallback = { mode: f.mode };
        else if (f.mode === "static" && typeof f.color === "string" && HEX.test(f.color)) header.fallback = { mode: "static", color: f.color };
        else return null;
      }
      headers.push(normalizeHeader(header));
    }
    controllers[key] = headers;
  }
  return { enabled: v.enabled, controllers };
}
