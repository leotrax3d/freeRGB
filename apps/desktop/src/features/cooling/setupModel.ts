import { t } from "./messages";
import type { FanCurve } from "@/lib/api/control";
import type { ChannelBinding, CoolingChannel, SupportTier } from "@/lib/api/types";
import { splitId } from "@/features/onboarding/fans";

/** One fan-controller chip as the guided setup shows it. */
export interface ChipSummary {
  hardware: string;
  name: string;
  controls: number;
  tachometers: number;
  tiers: Partial<Record<SupportTier | "unknown", number>>;
}

/** Groups the inventory per chip so the wizard can say what was detected. */
export function chipSummaries(channels: CoolingChannel[] | null | undefined): ChipSummary[] {
  const chips = new Map<string, ChipSummary>();
  for (const c of channels ?? []) {
    const hardware = splitId(c.reading.id)?.[0] ?? c.hardware_name ?? c.device_id;
    const chip = chips.get(hardware) ?? { hardware, name: hardware.split("/").filter(Boolean).slice(-2, -1)[0]?.toUpperCase() ?? hardware, controls: 0, tachometers: 0, tiers: {} };
    if (c.kind === "control") {
      chip.controls += 1;
      const tier = c.support?.tier ?? "unknown";
      chip.tiers[tier] = (chip.tiers[tier] ?? 0) + 1;
    } else chip.tachometers += 1;
    chips.set(hardware, chip);
  }
  return [...chips.values()];
}

/** RPM of every tachometer, keyed by reading id (missing values are skipped). */
export function rpmSnapshot(channels: CoolingChannel[] | null | undefined): Record<string, number> {
  const out: Record<string, number> = {};
  for (const c of channels ?? []) if (c.kind === "fan" && typeof c.reading.value === "number") out[c.reading.id] = c.reading.value;
  return out;
}

/** Keeps the highest RPM seen per tachometer while a header is held at 100 %. */
export function mergePeak(peak: Record<string, number>, now: Record<string, number>): Record<string, number> {
  const out = { ...peak };
  for (const [id, rpm] of Object.entries(now)) out[id] = Math.max(out[id] ?? 0, rpm);
  return out;
}

/**
 * Automatic tachometer pairing: the tachometer whose RPM rose the most while
 * one header was held at 100 %. Needs a clear winner (≥150 RPM and ≥15 %
 * rise, and at least twice the runner-up) — otherwise null and the user picks.
 */
export function correlateTach(before: Record<string, number>, peak: Record<string, number>): string | null {
  const rises = Object.entries(peak)
    .map(([id, rpm]) => ({ id, base: before[id] ?? 0, rise: rpm - (before[id] ?? 0) }))
    .filter((r) => r.rise >= 150 && r.rise >= 0.15 * Math.max(r.base, 1))
    .sort((a, b) => b.rise - a.rise);
  const [best, next] = rises;
  if (!best) return null;
  if (next && best.rise < 2 * next.rise) return null;
  return best.id;
}

/**
 * Proposed starting curve for a confirmed fan: quiet at idle, 100 % well
 * before critical. Never below the tier floor; pumps and unknown roles get no
 * proposal (pumps stay at 100 %, unknown channels are never treated as fans).
 */
export function proposeCurve(role: ChannelBinding["role"] | null, floor: number): FanCurve | null {
  if (role !== "fan") return null;
  const f = Math.max(0, Math.min(100, Math.round(floor)));
  const at = (d: number) => Math.max(f, d);
  return {
    points: [
      { temperature_c: 35, duty_percent: at(35) },
      { temperature_c: 55, duty_percent: at(55) },
      { temperature_c: 70, duty_percent: at(80) },
      { temperature_c: 78, duty_percent: 100 },
    ],
    minimum_percent: f,
    critical_c: 85,
    hysteresis_c: 2,
  };
}

export type SetupStep = "detect" | "identify" | "calibrate" | "curves" | "done";
export const SETUP_STEPS: { id: SetupStep; label: string }[] = [
  { id: "detect", get label() { return t("cooling.setupModel.detect"); } },
  { id: "identify", get label() { return t("cooling.setupModel.identifyName"); } },
  { id: "calibrate", get label() { return t("cooling.setupModel.calibrate"); } },
  { id: "curves", get label() { return t("cooling.setupModel.curves"); } },
  { id: "done", get label() { return t("cooling.setupModel.done"); } },
];

/**
 * Headers with a tachometer but no role (fan or pump). Guided setup blocks
 * every reduction (calibration, curves) until this list is empty; those
 * headers stay at 100 % (owner acceptance 2026-11-05: after an upgrade no
 * channel had a role and the pump was not recognized).
 */
export function unassignedRoles<T extends { role: "fan" | "pump" | null; rpm?: unknown; rpmValue?: number | null }>(views: T[]): T[] {
  return views.filter((v) => v.role === null && (v.rpm != null || v.rpmValue != null));
}

/** Label for a header without a role: it is held at full speed. */
export const roleUnknownLabel = () => t("cooling.setupModel.100RoleUnknown");
