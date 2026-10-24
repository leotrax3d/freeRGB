import type { DeviceSummary } from "@/lib/api/types";

/** Daemon `mainboard_rgb.get` view (crates/freergb-daemon/src/mainboard_rgb.rs). */
export interface MainboardHeader { leds: number; name?: string | null }
export interface MainboardRgbSettings { enabled: boolean; controllers: Record<string, MainboardHeader[]> }
export interface MainboardRgbView {
  settings: MainboardRgbSettings;
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
];

export const controllerKey = (vid: number, pid: number) =>
  `${vid.toString(16).padStart(4, "0")}:${pid.toString(16).padStart(4, "0")}`;

export interface MainboardController { key: string; label: string; headers: number; device: DeviceSummary | null }

/** Controllers to configure: detected motherboard devices plus already saved keys. */
export function mainboardControllers(devices: DeviceSummary[] | null | undefined, settings: MainboardRgbSettings | null | undefined): MainboardController[] {
  const out = new Map<string, MainboardController>();
  for (const d of devices ?? []) {
    const { usb_vid: vid, usb_pid: pid } = d.info;
    const family = FAMILIES.find((f) => f.vid === vid && pid !== null && f.pids.includes(pid));
    if (family && pid !== null) out.set(controllerKey(family.vid, pid), { key: controllerKey(family.vid, pid), label: family.label, headers: family.headers, device: d });
  }
  for (const key of Object.keys(settings?.controllers ?? {})) {
    if (out.has(key)) continue;
    const [vid, pid] = key.split(":").map((x) => parseInt(x, 16));
    const family = FAMILIES.find((f) => f.vid === vid && f.pids.includes(pid!));
    out.set(key, { key, label: family?.label ?? key, headers: family?.headers ?? 4, device: null });
  }
  return [...out.values()].sort((a, b) => a.key.localeCompare(b.key));
}

/** Settings with one header changed; LED counts are clamped, empty names dropped. */
export function withHeader(settings: MainboardRgbSettings, key: string, index: number, patch: Partial<MainboardHeader>, maxLeds = 512): MainboardRgbSettings {
  const headers = [...(settings.controllers[key] ?? [])];
  while (headers.length <= index) headers.push({ leds: 0 });
  const next = { ...headers[index]!, ...patch };
  next.leds = Math.max(0, Math.min(maxLeds, Math.round(Number.isFinite(next.leds) ? next.leds : 0)));
  const name = next.name?.slice(0, 40).trim();
  if (name) next.name = name; else delete next.name;
  headers[index] = next;
  return { ...settings, controllers: { ...settings.controllers, [key]: headers } };
}

/** Total LEDs a preset fills: `fans` fans with `per` LEDs each, chained on one header. */
export const presetLeds = (per: number, fans: number) => Math.max(0, Math.round(per)) * Math.max(1, Math.round(fans));
