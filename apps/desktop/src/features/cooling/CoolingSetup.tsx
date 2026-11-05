import { useMemo, useState } from "react";
import { CircleCheck, Cpu, Gauge, LoaderCircle, Waves, X } from "lucide-react";
import { Button } from "@/components/ui/button";
import { Banner } from "@/components/status";
import { rpc, useDaemon, useSlices } from "@/lib/store/daemon";
import { useAction } from "@/lib/store/useAction";
import type { BackgroundChannel } from "@/lib/api/types";
import { FanStep } from "@/features/onboarding/FanStep";
import { SupportBadge } from "./SupportBadge";
import { fanViews, floorFor, withChannel, type FanView } from "./fanModel";
import { tierDescription, tierLabel } from "./support";
import { ACTIVE_STATES } from "./liveModel";
import { chipSummaries, proposeCurve, SETUP_STEPS, type SetupStep } from "./setupModel";

function Steps({ step }: { step: SetupStep }) {
  const index = SETUP_STEPS.findIndex((s) => s.id === step);
  return (
    <ol className="flex flex-wrap gap-3 text-caption" aria-label="Setup progress">
      {SETUP_STEPS.map((s, i) => (
        <li key={s.id} aria-current={i === index ? "step" : undefined}
          className={i === index ? "font-medium text-text" : i < index ? "text-accent" : "text-text-subtle"}>
          {i + 1}. {s.label}
        </li>
      ))}
    </ol>
  );
}

function useViews(): FanView[] {
  const channels = useDaemon((s) => s.cooling_channels);
  const sensors = useDaemon((s) => s.monitoring_sensors);
  const bindings = useDaemon((s) => s.cooling_bindings);
  const background = useDaemon((s) => s.cooling_background);
  const session = useDaemon((s) => s.cooling_session);
  const support = useDaemon((s) => s.cooling_support);
  const curves = useDaemon((s) => s.curves);
  return useMemo(() => fanViews({
    channels, sensors, bindings: bindings?.config.bindings, saved: background?.saved.channels, service: background?.service?.channels,
    servicePaused: background?.service?.paused === true, sessions: session?.jobs ?? (session ? [session] : []), evidence: support?.channels, curves,
  }), [channels, sensors, bindings, background, session, support, curves]);
}

function Detect({ onNext }: { onNext: () => void }) {
  const channels = useDaemon((s) => s.cooling_channels);
  const providers = useDaemon((s) => s.providers);
  const onboarding = useDaemon((s) => s.onboarding);
  const views = useViews();
  const chips = chipSummaries(channels);
  const provider = providers?.find((p) => p.id === "mainboard.lhm");
  return (
    <div className="flex flex-col gap-4">
      <p className="text-small text-text-muted">freeRGB found these fan-controller chips through the motherboard service. Each header has a support tier that decides how far it may be slowed down.</p>
      {provider && provider.status !== "ready" && <Banner tone="warn">Motherboard service: {provider.message}</Banner>}
      {onboarding && !onboarding.safety_acknowledged && <Banner tone="warn">Fan control is locked until the safety notice is accepted. Detection works; the tests below stay disabled.</Banner>}
      {chips.length === 0 ? <p className="text-small text-text-subtle">No fan-controller chip was reported.</p> : (
        <ul className="grid gap-3 md:grid-cols-2">
          {chips.map((c) => (
            <li key={c.hardware} className="flex flex-col gap-1 rounded-card border border-border p-3 text-small">
              <span className="flex items-center gap-2 font-medium"><Cpu className="size-4" aria-hidden />{c.name}</span>
              <span className="text-text-muted">{c.controls} controllable header{c.controls === 1 ? "" : "s"}, {c.tachometers} RPM sensor{c.tachometers === 1 ? "" : "s"}</span>
              <span className="text-caption text-text-subtle">{Object.entries(c.tiers).map(([t, n]) => `${n} × ${tierLabel(t === "unknown" ? null : (t as never))}`).join(", ")}</span>
            </li>
          ))}
        </ul>
      )}
      {views.length > 0 && (
        <ul className="flex flex-col divide-y divide-border rounded-control border border-border">
          {views.map((v) => (
            <li key={v.id} className="flex flex-col gap-1 px-3 py-2 text-small">
              <span className="flex flex-wrap items-center gap-3"><span className="font-medium">{v.name}</span><SupportBadge tier={v.tier} coverage={v.evidence?.coverage} /></span>
              <span className="text-caption text-text-subtle">{v.blocked ?? tierDescription(v.tier)}</span>
            </li>
          ))}
        </ul>
      )}
      <div><Button variant="primary" onClick={onNext}>Continue</Button></div>
    </div>
  );
}

function Calibrate({ onNext }: { onNext: () => void }) {
  const session = useDaemon((s) => s.cooling_session);
  const views = useViews().filter((v) => v.role === "fan" && !v.blocked);
  const pumps = useViews().filter((v) => v.role === "pump");
  const { pending, error, run } = useAction();
  const [jobs, setJobs] = useState<Record<string, number>>({});
  const jobState = (id: string) => session?.jobs?.find((j) => j.job_id === jobs[id])?.state;
  const busy = Object.keys(jobs).some((id) => (ACTIVE_STATES as readonly string[]).includes(jobState(id) ?? ""));
  const calibrate = (v: FanView) => run(v.id, async () => {
    const r = await rpc<{ job_id: number }>("cooling.sessions.start", {
      mode: "calibration", control_id: v.id, temperature_id: v.temperature!.sensor.id, rpm_id: v.rpm!.reading.id, confirmed_fan: true, critical_c: 80,
    });
    setJobs((j) => ({ ...j, [v.id]: r.job_id }));
  });
  return (
    <div className="flex flex-col gap-4">
      <p className="text-small text-text-muted">Calibration measures each confirmed fan at 100 → 80 → 60 → 40 % (never lower) and stores its RPM range, so RPM targets and avoided ranges work. The fan returns to the BIOS afterwards. Windows asks for permission.</p>
      {pumps.length > 0 && <p className="flex items-center gap-2 text-caption text-text-subtle"><Waves className="size-4" aria-hidden />{pumps.map((p) => p.name).join(", ")}: pump, stays at 100 %, not calibrated.</p>}
      {views.length === 0 && <p className="text-small text-text-subtle">No header is confirmed as a fan yet. Go back and identify one, or skip.</p>}
      <ul className="flex flex-col gap-2">
        {views.map((v) => {
          const state = jobState(v.id);
          const running = (ACTIVE_STATES as readonly string[]).includes(state ?? "");
          const range = v.evidence?.rpm_range;
          return (
            <li key={v.id} className="flex flex-wrap items-center gap-3 rounded-control border border-border px-3 py-2 text-small">
              <Gauge className={`size-4 ${running ? "animate-pulse text-accent" : "text-text-subtle"}`} aria-hidden />
              <span className="font-medium">{v.name}</span>
              <span className="text-caption text-text-subtle">
                {running ? <span className="inline-flex items-center gap-1"><LoaderCircle className="size-3 animate-spin" />Measuring…</span>
                  : range ? `${Math.round(range.min_rpm)}–${Math.round(range.max_rpm)} RPM measured` : state === "faulted" || state === "failed" ? "Calibration failed; returned to BIOS" : "Not calibrated"}
              </span>
              <Button size="sm" variant="secondary" className="ml-auto" disabled={busy || pending !== null || !v.temperature || !v.rpm} onClick={() => calibrate(v)}>
                {range ? "Calibrate again (UAC)" : "Calibrate (UAC)"}
              </Button>
            </li>
          );
        })}
      </ul>
      {error && <p role="alert" className="text-small text-danger-text">{error}</p>}
      <div><Button variant="primary" disabled={busy} onClick={onNext}>Continue</Button></div>
    </div>
  );
}

function Curves({ onNext }: { onNext: () => void }) {
  const support = useDaemon((s) => s.cooling_support);
  const background = useDaemon((s) => s.cooling_background);
  const all = useViews();
  const fans = all.filter((v) => v.role === "fan" && !v.blocked && v.temperature && v.rpm);
  const { pending, error, run } = useAction();
  const [chosen, setChosen] = useState<Record<string, boolean>>(() => Object.fromEntries(fans.map((v) => [v.id, v.saved === null])));
  const proposals = fans.map((v) => ({ view: v, curve: proposeCurve(v.role, floorFor(v, support?.unrestricted_enabled ?? false)) }));
  const apply = () => run("apply", async () => {
    let next: BackgroundChannel[] = background?.saved.channels ?? [];
    for (const { view, curve } of proposals) {
      if (!chosen[view.id] || !curve) continue;
      next = withChannel(next, view.id, { control_id: view.id, temperature_id: view.temperature!.sensor.id, rpm_id: view.rpm!.reading.id, curve, confirmed_fan: true });
    }
    await rpc("cooling.background.apply", { channels: next });
    onNext();
  });
  return (
    <div className="flex flex-col gap-4">
      <p className="text-small text-text-muted">A starting curve per confirmed fan, following its chip temperature: quiet at idle, 100 % from 78 °C, full speed on a sensor fault. Pumps and unconfirmed headers are left alone. You can fine-tune every curve on the Cooling page afterwards.</p>
      <ul className="flex flex-col gap-2">
        {proposals.map(({ view, curve }) => (
          <li key={view.id} className="flex flex-wrap items-center gap-3 rounded-control border border-border px-3 py-2 text-small">
            <input type="checkbox" id={`propose-${view.id}`} checked={!!chosen[view.id]} disabled={!curve} onChange={(e) => setChosen((c) => ({ ...c, [view.id]: e.target.checked }))} />
            <label htmlFor={`propose-${view.id}`} className="font-medium">{view.name}</label>
            <span className="text-caption text-text-subtle">
              {curve ? curve.points.map((p) => `${p.temperature_c}°→${p.duty_percent}%`).join("  ") + ` · min ${curve.minimum_percent} %` : "No proposal"}
              {view.saved ? " · already has a curve (unchecked keeps it)" : ""}
            </span>
          </li>
        ))}
      </ul>
      {fans.length === 0 && <p className="text-small text-text-subtle">No confirmed fan with RPM and temperature sensors; nothing to propose.</p>}
      {error && <p role="alert" className="text-small text-danger-text">{error}</p>}
      <div className="flex gap-2">
        <Button variant="primary" disabled={pending !== null || !proposals.some((p) => chosen[p.view.id] && p.curve)} onClick={apply}>Apply selected curves (UAC)</Button>
        <Button variant="ghost" onClick={onNext}>Skip</Button>
      </div>
    </div>
  );
}

/**
 * Guided cooling setup: detect chips and tiers, identify/name headers and pair
 * tachometers, calibrate, then propose a curve per fan. Every hardware step
 * runs through the daemon's normal safety path (lease, floor, fail-safe).
 */
export function CoolingSetup({ onClose }: { onClose: () => void }) {
  useSlices("cooling_channels", "monitoring_sensors", "cooling_bindings", "cooling_background", "cooling_session", "cooling_support", "providers", "curves");
  const [step, setStep] = useState<SetupStep>("detect");
  return (
    <section aria-labelledby="cooling-setup-title" className="flex flex-col gap-4 rounded-card border border-accent/40 bg-surface p-4">
      <header className="flex items-start gap-3">
        <div className="flex flex-1 flex-col gap-1">
          <h2 id="cooling-setup-title" className="text-title font-semibold">Guided fan setup</h2>
          <Steps step={step} />
        </div>
        <Button size="sm" variant="ghost" aria-label="Close guided setup" onClick={onClose}><X /></Button>
      </header>
      {step === "detect" && <Detect onNext={() => setStep("identify")} />}
      {step === "identify" && <FanStep onDone={() => setStep("calibrate")} />}
      {step === "calibrate" && <Calibrate onNext={() => setStep("curves")} />}
      {step === "curves" && <Curves onNext={() => setStep("done")} />}
      {step === "done" && (
        <div className="flex flex-col gap-3">
          <p className="flex items-center gap-2 text-small"><CircleCheck className="size-4 text-accent" aria-hidden />Setup finished. Your fans are listed below; change curves, limits and avoided ranges there at any time.</p>
          <div><Button variant="primary" onClick={onClose}>Close</Button></div>
        </div>
      )}
    </section>
  );
}
