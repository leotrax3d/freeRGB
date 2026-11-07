import { t } from "./messages";
import { useEffect, useMemo, useRef, useState } from "react";
import { CircleCheck, CircleSlash, Fan, LoaderCircle, Waves } from "lucide-react";
import { Button } from "@/components/ui/button";
import { Banner } from "@/components/status";
import { rpc, useDaemon, useSlices } from "@/lib/store/daemon";
import type { ChannelBinding } from "@/lib/api/types";
import { fanHeaders, upsertBinding, type FanHeader } from "./fans";
import { tierLabel } from "@/features/cooling/support";
import { correlateTach, mergePeak, rpmSnapshot } from "@/features/cooling/setupModel";

export type FanOutcome = "fan" | "pump" | "none" | "blocked" | "skipped";

const IDENTIFY_SECONDS = 8;

/**
 * Motherboard fans: one header at a time is held at 100% (never lower) so the
 * user can say which fan reacted, name it and mark pumps. The guided channel
 * test (100/70/50%) is offered only for confirmed normal fans.
 */
export function FanStep({ onDone }: { onDone: (outcomes: Record<string, FanOutcome>) => void }) {
  useSlices("cooling_channels", "monitoring_sensors", "cooling_session", "cooling_bindings", "cooling_support", "providers");
  const channels = useDaemon((s) => s.cooling_channels);
  const sensors = useDaemon((s) => s.monitoring_sensors);
  const session = useDaemon((s) => s.cooling_session);
  const bindings = useDaemon((s) => s.cooling_bindings);
  const support = useDaemon((s) => s.cooling_support);
  const providers = useDaemon((s) => s.providers);
  const headers = useMemo(() => fanHeaders(channels ?? [], sensors ?? []), [channels, sensors]);
  const [outcomes, setOutcomes] = useState<Record<string, FanOutcome>>({});
  const [active, setActive] = useState<{ id: string; job: number; mode: "identify" | "channel_test" } | null>(null);
  const [asking, setAsking] = useState<string | null>(null);
  const [names, setNames] = useState<Record<string, string>>({});
  const [error, setError] = useState<string | null>(null);
  const [startedAt, setStartedAt] = useState(0);
  const [now, setNow] = useState(Date.now());
  const timer = useRef<number | undefined>(undefined);
  // Tachometer pairing: RPM before the spin-up, peak during it, chosen sensor per header.
  const [before, setBefore] = useState<Record<string, number>>({});
  const [peak, setPeak] = useState<Record<string, number>>({});
  const [tach, setTach] = useState<Record<string, string>>({});
  const [paired, setPaired] = useState<Record<string, boolean>>({});
  const tachometers = (channels ?? []).filter((c) => c.kind === "fan");
  useEffect(() => {
    if (active?.mode === "identify") setPeak((p) => mergePeak(p, rpmSnapshot(channels)));
  }, [active, channels]);

  const busy = active !== null || ["authorizing", "executing", "stopping"].includes(session?.state ?? "");
  const provider = providers?.find((p) => p.id === "mainboard.lhm");

  // Watch the running job; identification asks its question when it ends.
  useEffect(() => {
    if (!active || session?.job_id !== active.job) return;
    if (["completed", "stopped", "faulted", "failed", "blocked"].includes(session.state)) {
      if (session.state !== "completed" && session.state !== "stopped") setError(session.error ?? t("onboarding.fanStep.theModeEndedStateTheHeaderWasReturnedTo", { mode: active.mode === "identify" ? t("onboarding.fanStep.identification") : t("onboarding.fanStep.channelTest"), state: session.state }));
      else if (active.mode === "identify") {
        const auto = correlateTach(before, peak);
        if (auto) setTach((t) => ({ ...t, [active.id]: auto }));
        setPaired((p) => ({ ...p, [active.id]: auto !== null }));
        setAsking(active.id);
      }
      setActive(null);
    }
  }, [active, session, before, peak]);
  useEffect(() => {
    timer.current = window.setInterval(() => setNow(Date.now()), 250);
    return () => window.clearInterval(timer.current);
  }, []);

  const start = async (header: FanHeader, mode: "identify" | "channel_test") => {
    setError(null);
    try {
      const result = await rpc<{ job_id: number }>("cooling.sessions.start", {
        mode, control_id: header.control.reading.id, temperature_id: header.temperature!.sensor.id, rpm_id: header.rpm!.reading.id,
        confirmed_fan: mode === "channel_test",
        ...(mode === "channel_test" ? { critical_c: 80 } : {}),
      });
      setStartedAt(Date.now());
      if (mode === "identify") { const now = rpmSnapshot(channels); setBefore(now); setPeak(now); }
      setActive({ id: header.control.reading.id, job: result.job_id, mode });
    } catch (e) {
      setError(e instanceof Error ? e.message : String(e));
    }
  };

  const answer = async (header: FanHeader, outcome: "fan" | "pump" | "none") => {
    setError(null);
    try {
      if (outcome !== "none") {
        const name = (names[header.control.reading.id] ?? header.control.name).trim();
        if (!name) throw new Error(t("onboarding.fanStep.giveTheHeaderANameForExampleCPUFANOr"));
        const binding: ChannelBinding = {
          provider: "mainboard.lhm", device_id: header.control.device_id, control_id: header.control.id,
          tachometer_id: tachometers.find((t) => t.reading.id === tach[header.control.reading.id])?.id ?? header.rpm?.id ?? null, header: name.slice(0, 64), role: outcome, electrical_mode: "unknown",
        };
        const next = upsertBinding(bindings?.config.bindings ?? [], binding);
        await rpc("cooling.bindings.save", { config: { version: 1, bindings: next } });
      }
      setOutcomes((o) => ({ ...o, [header.control.reading.id]: outcome }));
      setAsking(null);
    } catch (e) {
      setError(e instanceof Error ? e.message : String(e));
    }
  };

  const finish = () => {
    const all: Record<string, FanOutcome> = {};
    for (const h of headers) all[h.control.reading.id] = h.blocked ? "blocked" : outcomes[h.control.reading.id] ?? "skipped";
    onDone(all);
  };

  if (!provider || provider.status === "disabled" || headers.length === 0) {
    return (
      <div className="flex flex-col gap-4">
        <p className="text-body text-text-muted">
          {provider && provider.status !== "disabled" && provider.status !== "ready"
            ? t("onboarding.fanStep.theMotherboardServiceIsNotReadyStatusMessage", { status: provider.status, message: provider.message ?? "" })
            : t("onboarding.fanStep.noControllableMotherboardFanHeadersWereFoundFanControl")}
        </p>
        <div><Button variant="primary" onClick={finish}>{t("onboarding.fanStep.continue")}</Button></div>
      </div>
    );
  }

  return (
    <div className="flex flex-col gap-5">
      <p className="text-body text-text-muted">
        {t("onboarding.fanStep.testOneHeaderAtATimeFreeRGBHoldsIt", { seconds: IDENTIFY_SECONDS })}
      </p>
      {error && <Banner tone="error">{error}</Banner>}
      <ul className="flex flex-col gap-3">
        {headers.map((h, i) => {
          const id = h.control.reading.id;
          const outcome = outcomes[id];
          const running = active?.id === id;
          const tested = support?.channels[id]?.test;
          const remaining = running && active!.mode === "identify" ? Math.max(0, IDENTIFY_SECONDS - Math.floor((now - startedAt) / 1000)) : null;
          return (
            <li key={id} style={{ animationDelay: `${i * 40}ms` }}
              className="flex flex-col gap-3 rounded-card border border-border bg-surface p-4 animate-[pop-in_var(--duration-base)_var(--ease-out)_both]">
              <div className="flex flex-wrap items-center gap-3">
                <Fan className={`size-5 ${running ? "animate-spin text-accent" : "text-text-subtle"}`} aria-hidden />
                <span className="font-medium">{h.control.name}</span>
                <span className="text-caption text-text-subtle">{tierLabel(h.control.support?.tier ?? null)} · {h.rpm ? `${Math.round(h.rpm.reading.value ?? 0)} RPM` : t("onboarding.fanStep.noRPMSensor")}</span>
                <span className="ms-auto text-small">
                  {h.blocked ? <span className="inline-flex items-center gap-1 text-text-subtle"><CircleSlash className="size-4" />{t("onboarding.fanStep.notTestable")}</span>
                    : outcome === "fan" ? <span className="inline-flex items-center gap-1 text-accent"><CircleCheck className="size-4" />{t("onboarding.fanStep.fan")}{tested === "passed" ? t("onboarding.fanStep.testPassed") : ""}</span>
                    : outcome === "pump" ? <span className="inline-flex items-center gap-1 text-[#fbbf24]"><Waves className="size-4" />{t("onboarding.fanStep.pumpStaysAt100")}</span>
                    : outcome === "none" ? <span className="text-text-subtle">{t("onboarding.fanStep.nothingReactedNotAssigned")}</span>
                    : running ? <span className="inline-flex items-center gap-1 text-info-text"><LoaderCircle className="size-4 animate-spin" />{active!.mode === "identify" ? t("onboarding.fanStep.fullSpeedSecondss", { seconds: remaining ?? 0 }) : t("onboarding.fanStep.channelTest1007050")}</span>
                    : null}
                </span>
              </div>
              {h.blocked && <p className="text-small text-text-subtle">{h.blocked}</p>}
              {!h.blocked && asking === id && (
                <div className="flex flex-col gap-3 animate-[fade-in_var(--duration-base)_var(--ease-out)]">
                  <label className="flex flex-col gap-1 text-small">
                    <span>{t("onboarding.fanStep.nameOfThisHeaderAsPrintedOnTheBoard")}</span>
                    <input className="max-w-sm rounded-control border border-border bg-surface-sunken px-3 py-2" maxLength={64}
                      value={names[id] ?? h.control.name} onChange={(e) => setNames((n) => ({ ...n, [id]: e.target.value }))} aria-label={t("onboarding.fanStep.headerName")} />
                  </label>
                  <label className="flex flex-col gap-1 text-small">
                    <span>{t("onboarding.fanStep.rPMSensorOfThisHeader")}{paired[id] ? t("onboarding.fanStep.pairedAutomaticallyFromTheSpeedup") : t("onboarding.fanStep.noClearSpeedupSeenPickItManually")}</span>
                    <select className="max-w-sm rounded-control border border-border bg-surface-sunken px-3 py-2" aria-label={t("onboarding.fanStep.rPMSensor")}
                      value={tach[id] ?? h.rpm?.reading.id ?? ""} onChange={(e) => setTach((t) => ({ ...t, [id]: e.target.value }))}>
                      <option value="">{t("onboarding.fanStep.none")}</option>
                      {tachometers.map((sensor) => <option key={sensor.reading.id} value={sensor.reading.id}>{sensor.name} · {Math.round(sensor.reading.value ?? 0)} RPM{peak[sensor.reading.id] !== undefined && before[sensor.reading.id] !== undefined ? t("onboarding.fanStep.peakRpm", { rpm: Math.round(peak[sensor.reading.id]!) }) : ""}</option>)}
                    </select>
                  </label>
                  <span className="text-small">{t("onboarding.fanStep.whatSpedUp")}</span>
                  <div className="flex flex-wrap gap-2">
                    <Button variant="primary" onClick={() => void answer(h, "fan")}><Fan />{t("onboarding.fanStep.aNormalFan")}</Button>
                    <Button onClick={() => void answer(h, "pump")}><Waves />{t("onboarding.fanStep.aPumpAIO")}</Button>
                    <Button variant="ghost" onClick={() => void answer(h, "none")}>{t("onboarding.fanStep.nothingChanged")}</Button>
                    <Button variant="ghost" disabled={busy} onClick={() => void start(h, "identify")}>{t("onboarding.fanStep.repeat")}</Button>
                  </div>
                </div>
              )}
              {!h.blocked && asking !== id && (
                <div className="flex flex-wrap gap-2">
                  <Button size="sm" disabled={busy} onClick={() => void start(h, "identify")}>{outcome ? t("onboarding.fanStep.testAgain") : t("onboarding.fanStep.spinUpToIdentify")}</Button>
                  {outcome === "fan" && tested !== "passed" && (
                    <Button size="sm" variant="secondary" disabled={busy} onClick={() => void start(h, "channel_test")}>{t("onboarding.fanStep.runChannelTest1007050")}</Button>
                  )}
                </div>
              )}
            </li>
          );
        })}
      </ul>
      <p className="text-caption text-text-subtle">{t("onboarding.fanStep.pumpsAndHeadersYouDidNotConfirmAreNever")}</p>
      <div className="flex justify-end">
        <Button variant="primary" disabled={busy} onClick={finish}>{Object.keys(outcomes).length ? t("onboarding.fanStep.continue") : t("onboarding.fanStep.skipFanSetup")}</Button>
      </div>
    </div>
  );
}
