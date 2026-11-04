/** `cooling.plugins.list` (API 3 cooling plugins, docs/COOLING_PLUGINS.md). */
export type PluginRole = "fan" | "pump" | "unknown";

export interface PluginChannelDecl {
  id: string;
  name: string;
  role: PluginRole;
  floor_percent: number;
  tachometer?: string;
}

export type PluginAssignment =
  | { plugin: string; channel: string; mode: "manual"; percent: number }
  | { plugin: string; channel: string; mode: "composite"; output: string };

export interface PluginReading {
  id: string;
  name: string;
  unit: "celsius" | "rpm" | "percent";
  value: number | null;
  age_ms: number | null;
}

export interface PluginStatus {
  id: string;
  running: boolean;
  fault: string | null;
  failsafe: { reports: number; written: number; errors: string[] } | null;
  controls: { channel: string; duty_percent: number }[];
  virtual_io: boolean;
}

export interface PluginView {
  id: string;
  name?: string;
  version: string;
  family: string;
  license: string;
  installed: boolean;
  enabled: boolean;
  capability?: "sensor" | "control" | "both";
  virtual?: boolean;
  device_found?: boolean;
  status?: PluginStatus | null;
  error?: string | null;
  install?: string | null;
  failures?: number;
  sensors?: PluginReading[];
  channels?: { channel: PluginChannelDecl; assignment: PluginAssignment | null; applied_percent: number | null; fault: string | null }[];
}

export interface PluginsView {
  started: boolean;
  plugins: PluginView[];
}

export type PluginState = "available" | "disabled" | "starting" | "running" | "no_device" | "faulted";

export function pluginState(p: PluginView): PluginState {
  if (!p.installed) return "available";
  if (!p.enabled) return "disabled";
  if (p.status?.running) return "running";
  if (p.status?.fault || p.error) return p.device_found === false ? "no_device" : "faulted";
  return p.device_found === false ? "no_device" : "starting";
}

export const STATE_LABELS: Record<PluginState, string> = {
  available: "Not installed",
  disabled: "Disabled",
  starting: "Starting…",
  running: "Running",
  no_device: "Device not found",
  faulted: "Stopped after a fault",
};

/** Short source badge text shared by channels and sensors. */
export function sourceBadge(p: Pick<PluginView, "id" | "virtual">): string {
  return p.virtual ? `Plugin · ${p.id} · virtual` : `Plugin · ${p.id}`;
}

/** `plugin:<id>/<sensor>` → plugin id, else null. */
export function pluginOf(id: string): string | null {
  const m = /^plugin:([^/]+)\//.exec(id);
  return m?.[1] ?? null;
}

/** Fixed duties offered for a channel: pumps and unknown channels only 100 %. */
export function dutyChoices(c: PluginChannelDecl): number[] {
  if (c.role !== "fan") return [100];
  return [40, 50, 70, 100].filter((d) => d >= c.floor_percent);
}

export function failsafeNote(s: PluginStatus | null | undefined): string | null {
  const f = s?.failsafe;
  if (!f) return null;
  if (f.reports === 0) return "Plugin stopped; no device fail-safe needed (virtual).";
  if (f.errors.length > 0) return `Fail-safe incomplete: ${f.written}/${f.reports} reports written (${f.errors.join("; ")}).`;
  return `Fail-safe sent: ${f.written}/${f.reports} reports written by freeRGB.`;
}
