/** `cooling.sensors.catalog`: every Celsius source a curve can use. */
export type SourceCategory = "cpu" | "gpu" | "motherboard" | "storage" | "liquid" | "file" | "other";
export interface CatalogEntry {
  id: string;
  label: string;
  provider: string;
  unit: string;
  category: SourceCategory;
  value: number | null;
  age_ms: number;
  error: string | null;
}

export const CATEGORY_LABELS: Record<SourceCategory, string> = {
  cpu: "CPU",
  gpu: "GPU",
  motherboard: "Motherboard",
  storage: "Storage",
  liquid: "Liquid",
  file: "Text files",
  other: "Other",
};
const ORDER = Object.keys(CATEGORY_LABELS) as SourceCategory[];

/** Groups in a fixed category order; entries sorted by label. Empty groups are dropped. */
export function groupSources(entries: CatalogEntry[]): { category: SourceCategory; entries: CatalogEntry[] }[] {
  return ORDER.map((category) => ({
    category,
    entries: entries.filter((e) => (ORDER.includes(e.category) ? e.category : "other") === category).sort((a, b) => a.label.localeCompare(b.label)),
  })).filter((g) => g.entries.length > 0);
}

/** `cooling.file_sensors.get` / `.save` (docs/CUSTOM_SENSORS.md). */
export interface FileSensor {
  path: string;
  scale?: number;
  offset?: number;
  min_c?: number;
  max_c?: number;
  timeout_ms?: number;
  allow_network?: boolean;
}
export interface FileSensorSet {
  sensors: Record<string, FileSensor>;
}

/** Mirrors the daemon's checks so the form can explain a refusal before saving. */
export function fileSensorProblem(name: string, path: string, existing: string[]): string | null {
  if (!/^[A-Za-z0-9_-]{1,64}$/.test(name)) return "Name: 1–64 letters, digits, - or _";
  if (existing.includes(name)) return "Name already used";
  if (!/^(\/|[A-Za-z]:[\\/])/.test(path)) {
    return /^(\\\\|\/\/)/.test(path) ? "Network paths are disabled by default" : "Use an absolute path";
  }
  return null;
}

/** `cooling.gpu.list`. */
export interface GpuFanView {
  fan: { id: string; vendor: string; gpu_name: string; index: number; duty_percent: number | null; controllable: boolean; reason: string | null; min_percent: number };
  floor_percent: number;
  assignment: ({ fan_id: string } & ({ mode: "manual"; percent: number } | { mode: "composite"; output: string })) | null;
  applied_percent: number | null;
  fault: string | null;
}
export interface GpuView {
  started: boolean;
  paused: boolean;
  backends: string[];
  fans: GpuFanView[];
}
