import type { FanCurve } from "@/lib/api/control";
import type {
  BackgroundChannel, BackgroundServiceChannel, ChannelBinding, ChannelEvidenceView, CoolingChannel, CoolingSession, MonitoringSensor, SupportTier,
} from "@/lib/api/types";
import { fanHeaders, type FanHeader } from "@/features/onboarding/fans";
import { channelTier } from "./support";

export type FanMode = "bios" | "curve" | "fixed";

/** Everything the Cooling tab shows about one motherboard fan header. */
export interface FanView extends FanHeader {
  id: string;
  name: string;
  role: "fan" | "pump" | null;
  binding: ChannelBinding | null;
  tier: SupportTier | null;
  rpmValue: number | null;
  duty: number | null;
  saved: BackgroundChannel | null;
  service: BackgroundServiceChannel | null;
  session: CoolingSession | null;
  evidence: ChannelEvidenceView | null;
  mode: FanMode;
  /** One-line, plain-language status. */
  status: string;
  tone: "ok" | "warn" | "error" | "idle";
}

/** A flat curve for a fixed speed; critical heat still forces 100%. */
export function fixedCurve(duty: number): FanCurve {
  const d = Math.max(0, Math.min(100, Math.round(duty)));
  return { points: [{ temperature_c: 0, duty_percent: d }, { temperature_c: 80, duty_percent: d }], minimum_percent: d, critical_c: 85, hysteresis_c: 0 };
}

/** A saved background curve that is flat is shown as "Fixed". */
export function isFixed(curve: FanCurve): boolean {
  return curve.points.every((p) => p.duty_percent === curve.points[0]!.duty_percent) && curve.minimum_percent === curve.points[0]!.duty_percent;
}

export function curveLabel(curve: FanCurve, curves: Record<string, FanCurve> | null | undefined): string {
  if (isFixed(curve)) return `${curve.points[0]!.duty_percent} % fixed`;
  const name = Object.entries(curves ?? {}).find(([, c]) => JSON.stringify(c) === JSON.stringify(curve))?.[0];
  return name ? `curve “${name}”` : "custom curve";
}

export function sensorName(id: string, sensors: MonitoringSensor[] | null | undefined): string {
  const s = sensors?.find((x) => x.sensor.id === id);
  if (!s) return id.split("/").slice(-3).join(" ");
  const device = /cpu/.test(id) ? "CPU" : /gpu/.test(id) ? "GPU" : "Board";
  return `${device} · ${s.sensor.name}`;
}

/** Temperature sources a curve can use: board, CPU and GPU (°C, from the motherboard service). */
export function temperatureSources(sensors: MonitoringSensor[] | null | undefined): MonitoringSensor[] {
  return (sensors ?? []).filter((s) => s.provider === "mainboard.lhm" && s.sensor.unit === "°C");
}

export function fanViews(input: {
  channels: CoolingChannel[] | null | undefined;
  sensors: MonitoringSensor[] | null | undefined;
  bindings: ChannelBinding[] | null | undefined;
  saved: BackgroundChannel[] | null | undefined;
  service: BackgroundServiceChannel[] | null | undefined;
  servicePaused: boolean;
  sessions: CoolingSession[] | null | undefined;
  evidence: Record<string, ChannelEvidenceView> | null | undefined;
  curves: Record<string, FanCurve> | null | undefined;
}): FanView[] {
  return fanHeaders(input.channels ?? [], input.sensors ?? []).map((h) => {
    const id = h.control.reading.id;
    const binding = input.bindings?.find((b) => b.control_id === h.control.id) ?? null;
    const saved = input.saved?.find((c) => c.control_id === id) ?? null;
    const service = input.service?.find((c) => c.control_id === id) ?? null;
    const session = input.sessions?.find((j) => j.control_id === id && ["authorizing", "executing", "stopping"].includes(j.state)) ?? null;
    const role = binding?.role ?? null;
    let status: string;
    let tone: FanView["tone"] = "idle";
    if (h.blocked && !saved) { status = h.blocked; }
    else if (session) { status = `Live session (${session.mode ?? "test"}) · ${session.state}`; tone = "warn"; }
    else if (service?.fault) { status = `Returned to BIOS: ${service.fault.replace(/_/g, " ")}`; tone = "error"; }
    else if (saved && input.servicePaused) { status = "Paused — BIOS controls this fan"; tone = "warn"; }
    else if (saved) {
      status = `${curveLabel(saved.curve, input.curves)} · ${saved.temperature_id.split(",").map((t) => sensorName(t, input.sensors)).join(" / ")}${service ? ` · now ${service.duty_percent} %` : ""}`;
      tone = service?.state === "running" ? "ok" : "warn";
    } else if (!role) { status = "Not identified yet — freeRGB leaves it to the BIOS"; }
    else { status = `BIOS controls this ${role}`; }
    return {
      ...h, id, binding, role, saved, service, session,
      name: binding?.header ?? h.control.name,
      tier: channelTier(h.control),
      rpmValue: h.rpm?.reading.value ?? null,
      duty: h.control.reading.value ?? null,
      evidence: input.evidence?.[id] ?? null,
      mode: saved ? (isFixed(saved.curve) ? "fixed" : "curve") : "bios",
      status, tone,
    };
  });
}

/** The background list with one channel replaced (or removed for BIOS). */
export function withChannel(saved: BackgroundChannel[], controlId: string, next: BackgroundChannel | null): BackgroundChannel[] {
  const others = saved.filter((c) => c.control_id !== controlId);
  return next ? [...others, next] : others;
}

/** Lowest duty the UI offers; the service enforces the same rule. */
export function floorFor(view: Pick<FanView, "role" | "evidence" | "tier">, unrestricted: boolean): number {
  const unlocked = unrestricted && (view.evidence?.unlock === "unlocked" || view.evidence?.unlock === "unlocked_pump");
  if (view.role === "pump") return view.evidence?.unlock === "unlocked_pump" && unrestricted ? 0 : 100;
  if (unlocked) return 0;
  if (view.role !== "fan") return 100;
  if (view.tier === "lhm_supported" && view.evidence?.test !== "passed") return 100;
  return 40;
}

