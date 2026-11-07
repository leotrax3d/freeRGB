import { t } from "./messages";
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
  get available() { return t("cooling.pluginModel.notInstalled"); },
  get disabled() { return t("cooling.pluginModel.disabled"); },
  get starting() { return t("cooling.pluginModel.starting"); },
  get running() { return t("cooling.pluginModel.running"); },
  get no_device() { return t("cooling.pluginModel.deviceNotFound"); },
  get faulted() { return t("cooling.pluginModel.stoppedAfterAFault"); },
};

/** Short source badge text shared by channels and sensors. */
export function sourceBadge(p: Pick<PluginView, "id" | "virtual">): string {
  return p.virtual ? t("cooling.pluginModel.pluginValueVirtual", { p0: p.id }) : t("cooling.pluginModel.pluginValue", { p0: p.id });
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
  if (f.reports === 0) return t("cooling.pluginModel.pluginStoppedNoDeviceFailSafeNeeded");
  if (f.errors.length > 0) return t("cooling.pluginModel.failSafeIncompleteValueValueReportsWritten", { p0: f.written, p1: f.reports, p2: f.errors.join("; ") });
  return t("cooling.pluginModel.failSafeSentValueValueReportsWritten", { p0: f.written, p1: f.reports });
}
