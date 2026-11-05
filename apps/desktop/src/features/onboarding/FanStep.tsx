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
      if (session.state !== "completed" && session.state !== "stopped") setError(session.error ?? `The ${active.mode === "identify" ? "identification" : "channel test"} ended: ${session.state}. The header was returned to firmware control.`);
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
        if (!name) throw new Error("Give the header a name, for example CPU_FAN or Front intake.");
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
            ? `The motherboard service is not ready (${provider.status}): ${provider.message}`
            : "No controllable motherboard fan headers were found. Fan control may be unavailable on this board, or the motherboard components were not installed. Run Setup again to retry."}
        </p>
        <div><Button variant="primary" onClick={finish}>Continue</Button></div>
      </div>
    );
  }

  return (
    <div className="flex flex-col gap-5">
      <p className="text-body text-text-muted">
        Test one header at a time. freeRGB holds it at <strong className="text-text">100 %</strong> for {IDENTIFY_SECONDS} seconds — it never slows anything down here — then hands it back to the motherboard. Listen and watch which fan or pump speeds up, then tell freeRGB what it was. Windows asks for permission for each test.
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
                <span className="text-caption text-text-subtle">{tierLabel(h.control.support?.tier ?? null)} · {h.rpm ? `${Math.round(h.rpm.reading.value ?? 0)} RPM` : "no RPM sensor"}</span>
                <span className="ml-auto text-small">
                  {h.blocked ? <span className="inline-flex items-center gap-1 text-text-subtle"><CircleSlash className="size-4" />Not testable</span>
                    : outcome === "fan" ? <span className="inline-flex items-center gap-1 text-accent"><CircleCheck className="size-4" />Fan{tested === "passed" ? " · test passed" : ""}</span>
                    : outcome === "pump" ? <span className="inline-flex items-center gap-1 text-[#fbbf24]"><Waves className="size-4" />Pump · stays at 100 %</span>
                    : outcome === "none" ? <span className="text-text-subtle">Nothing reacted · not assigned</span>
                    : running ? <span className="inline-flex items-center gap-1 text-info-text"><LoaderCircle className="size-4 animate-spin" />{active!.mode === "identify" ? `Full speed · ${remaining}s` : "Channel test 100 → 70 → 50 %"}</span>
                    : null}
                </span>
              </div>
              {h.blocked && <p className="text-small text-text-subtle">{h.blocked}</p>}
              {!h.blocked && asking === id && (
                <div className="flex flex-col gap-3 animate-[fade-in_var(--duration-base)_var(--ease-out)]">
                  <label className="flex flex-col gap-1 text-small">
                    <span>Name of this header (as printed on the board or where the fan sits)</span>
                    <input className="max-w-sm rounded-control border border-border bg-surface-sunken px-3 py-2" maxLength={64}
                      value={names[id] ?? h.control.name} onChange={(e) => setNames((n) => ({ ...n, [id]: e.target.value }))} aria-label="Header name" />
                  </label>
                  <label className="flex flex-col gap-1 text-small">
                    <span>RPM sensor of this header{paired[id] ? " (paired automatically from the speed-up)" : " (no clear speed-up seen; pick it manually)"}</span>
                    <select className="max-w-sm rounded-control border border-border bg-surface-sunken px-3 py-2" aria-label="RPM sensor"
                      value={tach[id] ?? h.rpm?.reading.id ?? ""} onChange={(e) => setTach((t) => ({ ...t, [id]: e.target.value }))}>
                      <option value="">None</option>
                      {tachometers.map((t) => <option key={t.reading.id} value={t.reading.id}>{t.name} · {Math.round(t.reading.value ?? 0)} RPM{peak[t.reading.id] !== undefined && before[t.reading.id] !== undefined ? ` (peak ${Math.round(peak[t.reading.id]!)})` : ""}</option>)}
                    </select>
                  </label>
                  <span className="text-small">What sped up?</span>
                  <div className="flex flex-wrap gap-2">
                    <Button variant="primary" onClick={() => void answer(h, "fan")}><Fan />A normal fan</Button>
                    <Button onClick={() => void answer(h, "pump")}><Waves />A pump / AIO</Button>
                    <Button variant="ghost" onClick={() => void answer(h, "none")}>Nothing changed</Button>
                    <Button variant="ghost" disabled={busy} onClick={() => void start(h, "identify")}>Repeat</Button>
                  </div>
                </div>
              )}
              {!h.blocked && asking !== id && (
                <div className="flex flex-wrap gap-2">
                  <Button size="sm" disabled={busy} onClick={() => void start(h, "identify")}>{outcome ? "Test again" : "Spin up to identify"}</Button>
                  {outcome === "fan" && tested !== "passed" && (
                    <Button size="sm" variant="secondary" disabled={busy} onClick={() => void start(h, "channel_test")}>Run channel test (100 → 70 → 50 %)</Button>
                  )}
                </div>
              )}
            </li>
          );
        })}
      </ul>
      <p className="text-caption text-text-subtle">Pumps and headers you did not confirm are never slowed down. The channel test runs only on confirmed normal fans, never below 50 %, and returns the fan to firmware control afterwards.</p>
      <div className="flex justify-end">
        <Button variant="primary" disabled={busy} onClick={finish}>{Object.keys(outcomes).length ? "Continue" : "Skip fan setup"}</Button>
      </div>
    </div>
  );
}
