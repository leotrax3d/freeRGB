import type { ModulesStatus, SetupCard, SetupPhase } from "@/lib/api/types";
import type { Tone } from "@/components/status";

export interface PhaseInfo {
  label: string;
  tone: Tone;
  /** Work in progress without a measurable amount: show an activity indicator. */
  busy: boolean;
}

export const PHASES: Record<SetupPhase, PhaseInfo> = {
  detected: { label: "Detected", tone: "idle", busy: false },
  waiting: { label: "Waiting", tone: "idle", busy: true },
  downloading: { label: "Downloading", tone: "warn", busy: false },
  verifying: { label: "Verifying", tone: "warn", busy: true },
  installing: { label: "Setting up", tone: "warn", busy: true },
  initializing: { label: "Connecting", tone: "warn", busy: true },
  ready: { label: "Ready", tone: "ok", busy: false },
  failed: { label: "Failed", tone: "error", busy: false },
  not_connected: { label: "Not connected", tone: "idle", busy: false },
  unsupported: { label: "Not yet supported", tone: "idle", busy: false },
};

/** Supported by a module, not built in. */
export const supported = (c: SetupCard) => c.module !== null && !c.builtin;

/** Cards a setup run can work on right now. */
export const selectable = (c: SetupCard) => supported(c) && (c.phase === "detected" || c.phase === "failed");

/** Pre-selection: supported devices that are not set up yet (and not dismissed). */
export function defaultSelection(cards: SetupCard[]): string[] {
  return cards.filter((c) => selectable(c) && !c.configured && !c.dismissed).map((c) => c.key);
}

/** New devices for the panel in the device overview. */
export function newDevices(cards: SetupCard[]): SetupCard[] {
  return cards.filter((c) => supported(c) && !c.configured && !c.dismissed && c.phase !== "not_connected");
}

/**
 * Download size of a selection: shared modules count once and installed
 * modules not at all. `unknown` when any needed size is missing.
 */
export function selectionSize(cards: SetupCard[], selected: Iterable<string>): { bytes: number; unknown: boolean } {
  const keys = new Set(selected);
  const modules = new Map<string, number | null>();
  for (const c of cards) {
    if (!keys.has(c.key) || !c.module || c.installed) continue;
    modules.set(c.module, c.download_size);
  }
  let bytes = 0;
  let unknown = false;
  for (const size of modules.values()) {
    if (size === null) unknown = true;
    else bytes += size;
  }
  return { bytes, unknown };
}

const WORKING: SetupPhase[] = ["waiting", "downloading", "verifying", "installing", "initializing"];

export const isWorking = (c: SetupCard) => WORKING.includes(c.phase);

/** Overall state of a set of cards (those of the current run). */
export function summary(cards: SetupCard[]) {
  return {
    working: cards.filter(isWorking).length,
    ready: cards.filter((c) => c.phase === "ready").length,
    failed: cards.filter((c) => c.phase === "failed").length,
    total: cards.length,
  };
}

/** Overall download fraction from the job counters, or null if not measurable. */
export function overallProgress(status: ModulesStatus | null | undefined): number | null {
  const job = status?.job;
  if (!job || job.bytes_total <= 0) return null;
  return Math.min(1, job.bytes_done / job.bytes_total);
}

export function bytes(n: number): string {
  if (n < 1024) return `${n} B`;
  if (n < 1024 * 1024) return `${(n / 1024).toFixed(0)} KB`;
  return `${(n / 1024 / 1024).toFixed(1)} MB`;
}

const ERROR_HINTS: Record<string, string> = {
  offline: "freeRGB could not reach the module source. Check the internet connection, then try again.",
  source: "The module source is not available.",
  permission: "freeRGB is not allowed to open this device.",
  busy: "Another program is using this device. Close other RGB software and try again.",
  invalid_package: "The downloaded module failed verification and was not installed.",
  incompatible: "No compatible module exists for this system yet.",
  initialization: "The device did not accept the module's setup.",
  timeout: "The device did not respond in time.",
  removed: "The device was disconnected during setup.",
  cancelled: "Setup was cancelled.",
};

/** One-sentence cause for a failed card; the raw message stays in details. */
export function errorHint(card: SetupCard): string | null {
  if (!card.error) return null;
  const raw = card.error.message.toLowerCase();
  if (card.error.code === "initialization" && raw.includes("permission")) return ERROR_HINTS.permission!;
  if (card.error.code === "initialization" && (raw.includes("busy") || raw.includes("unavailable"))) return ERROR_HINTS.busy!;
  return ERROR_HINTS[card.error.code] ?? "Something went wrong.";
}
