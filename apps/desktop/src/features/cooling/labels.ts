import { t } from "./messages";

const states = ["unavailable", "authorizing", "executing", "stopping", "stopped", "completed", "faulted", "failed", "blocked", "idle", "none", "passed", "stale", "unsaved", "unlocked", "unlocked_pump", "disabled", "running", "resolved", "missing_control", "missing_tachometer", "missing_rpm", "provider_unavailable"] as const;
const modes = ["curve", "manual", "composite", "rpm", "calibration", "channel_test", "identify", "test"] as const;

/** Keep unfamiliar daemon states visible for diagnosis and older/newer services. */
export function stateLabel(state: string): string {
  const known = states.find((s) => s === state);
  return known ? t(`cooling.state.${known}`) : state;
}

export function modeLabel(mode: string): string {
  const known = modes.find((m) => m === mode);
  return known ? t(`cooling.mode.${known}`) : mode;
}

export function roleLabel(role: "fan" | "pump"): string {
  return t(`cooling.role.${role}`);
}
