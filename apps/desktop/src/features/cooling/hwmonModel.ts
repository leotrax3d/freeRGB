import type { FanCurve } from "@/lib/api/control";

/** `cooling.hwmon.get` (Linux kernel hwmon fan headers). */
export interface HwmonView {
  started: boolean;
  board: { manufacturer: string; product: string; bios_version: string } | null;
  paused: boolean;
  unrestricted: boolean;
  floor_percent: number;
  chips: {
    id: string;
    name: string;
    read_only_hint: boolean;
    temps: { index: number; label: string; celsius: number | null }[];
    fans: { index: number; label: string; rpm: number | null }[];
    pwms: { channel_id: string; label: string; percent: number | null; enable: number | null; mode: number | null; writable: boolean; paired_fan: number | null }[];
  }[];
  assignments: HwmonAssignment[];
  status: { channel_id: string; duty_percent: number | null; rpm: number | null; floor_percent: number; state: "running" | "fallback_full" | "faulted"; message: string | null }[];
}

export type HwmonAssignment =
  | { channel_id: string; mode: "manual"; percent: number; pump: boolean; allow_stop: boolean }
  | { channel_id: string; mode: "curve"; curve: FanCurve; sensor_id: string; pump: boolean; allow_stop: boolean };

export type HwmonMode = "firmware" | "manual" | "curve";

/** Every hwmon temperature as a curve input option. */
export function hwmonTemperatures(view: HwmonView): { value: string; label: string }[] {
  return view.chips.flatMap((c) => c.temps.map((t) => ({
    value: `${c.id}/temp${t.index}`,
    label: `${c.name}: ${t.label}${t.celsius === null ? "" : ` (${Math.round(t.celsius)} °C)`}`,
  })));
}

/** RPC call for a header edit; `firmware` releases the header. */
export function hwmonRequest(channel: string, mode: HwmonMode, percent: number, sensor: string, curve: FanCurve | undefined, pump: boolean, allowStop: boolean):
  { method: "cooling.hwmon.set" | "cooling.hwmon.release"; params: Record<string, unknown> } | null {
  if (mode === "firmware") return { method: "cooling.hwmon.release", params: { channel_id: channel } };
  if (mode === "manual") return { method: "cooling.hwmon.set", params: { channel_id: channel, mode, percent: Math.max(0, Math.min(100, percent)), pump, allow_stop: allowStop } };
  if (!sensor || !curve) return null;
  return { method: "cooling.hwmon.set", params: { channel_id: channel, mode, curve, sensor_id: sensor, pump, allow_stop: allowStop } };
}
