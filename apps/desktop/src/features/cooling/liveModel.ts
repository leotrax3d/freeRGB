import { t } from "./messages";
import { modeLabel } from "./labels";
import type { CoolingSession } from "@/lib/api/types";

/** Live jobs that still own a channel. */
export const ACTIVE_STATES = ["authorizing", "executing", "stopping"] as const;

/** A remembered curve/manual assignment (daemon `cooling.sessions.list`). */
export interface LiveAssignment {
  control_id: string;
  mode: string;
  temperature_id: string;
  rpm_id: string;
  duty_percent: number | null;
  needs_attention: string | null;
}
export interface SessionList {
  active: CoolingSession[];
  max_active: number;
  assignments: { assignments: LiveAssignment[]; resume_pending: boolean };
}

export type LiveState = "off" | "starting" | "curve" | "manual" | "test" | "stopping" | "fault" | "needs_attention";

export interface LiveChannel {
  control_id: string;
  job_id: number | null;
  state: LiveState;
  label: string;
  error: string | null;
}

export function isActive(job: Pick<CoolingSession, "state">): boolean {
  return (ACTIVE_STATES as readonly string[]).includes(job.state);
}

/** All recent jobs from the status slice (newest first, one entry per job). */
export function jobsOf(session: CoolingSession | null | undefined): CoolingSession[] {
  if (!session) return [];
  return session.jobs ?? (session.job_id != null ? [session] : []);
}

/** One line per channel that has a live job, a recent fault or needs attention. */
export function liveChannels(session: CoolingSession | null | undefined, list: SessionList | null | undefined): LiveChannel[] {
  const out = new Map<string, LiveChannel>();
  // Oldest first so the newest job of a channel wins.
  for (const job of [...jobsOf(session)].reverse()) {
    const id = job.control_id;
    if (!id) continue;
    let state: LiveState;
    if (job.state === "authorizing") state = "starting";
    else if (job.state === "stopping") state = "stopping";
    else if (job.state === "executing") state = job.mode === "curve" || job.mode === "composite" ? "curve" : job.mode === "manual" ? "manual" : "test";
    else if (job.state === "faulted" || job.state === "failed" || job.state === "blocked") state = "fault";
    else { out.delete(id); continue; }
    out.set(id, { control_id: id, job_id: job.job_id, state, label: labelOf(state, job.mode ?? null), error: job.error ?? null });
  }
  for (const a of list?.assignments.assignments ?? []) {
    if (a.needs_attention && !out.has(a.control_id)) {
      out.set(a.control_id, { control_id: a.control_id, job_id: null, state: "needs_attention", label: t("cooling.liveModel.needsAttention"), error: a.needs_attention });
    }
  }
  return [...out.values()].sort((x, y) => x.control_id.localeCompare(y.control_id));
}

function labelOf(state: LiveState, mode: string | null): string {
  switch (state) {
    case "starting": return t("cooling.liveModel.waitingForWindowsAuthorization");
    case "curve": return mode === "composite" ? t("cooling.liveModel.compositeCurveActive") : t("cooling.liveModel.curveActive");
    case "manual": return t("cooling.liveModel.fixedSpeedActive");
    case "test": return t("cooling.liveModel.valueRunning", { p0: modeLabel(mode ?? "test") });
    case "stopping": return t("cooling.liveModel.returningToBios");
    case "fault": return t("cooling.liveModel.errorReturnedToBios");
    case "needs_attention": return t("cooling.liveModel.needsAttention");
    default: return t("cooling.liveModel.off");
  }
}

export function activeCount(session: CoolingSession | null | undefined): number {
  return jobsOf(session).filter(isActive).length;
}

/** Manual duty clamped to the channel's tier floor (pumps: always 100 %). */
export function clampDuty(duty: number, floor: number): number {
  const d = Number.isFinite(duty) ? Math.round(duty) : 100;
  return Math.max(Math.min(100, Math.max(0, floor)), Math.min(100, d));
}

/**
 * The user confirms live fan control once per app session (not per channel);
 * the daemon and protected service still authorize every channel themselves.
 */
let confirmedThisSession = false;
export const liveConfirmation = {
  needed: () => !confirmedThisSession,
  confirm: () => { confirmedThisSession = true; },
  reset: () => { confirmedThisSession = false; },
};
