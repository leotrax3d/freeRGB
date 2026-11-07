import { t } from "./messages";
import { roleLabel } from "./labels";
import { useEffect, useState } from "react";
import { Fan, FlaskConical, LoaderCircle, Search, Waves } from "lucide-react";
import { Button } from "@/components/ui/button";
import { Select } from "@/components/ui/select";
import { Slider } from "@/components/ui/slider";
import { StatusDot } from "@/components/status";
import { rpc, useDaemon } from "@/lib/store/daemon";
import { useAction } from "@/lib/store/useAction";
import type { BackgroundChannel, ChannelBinding } from "@/lib/api/types";
import { upsertBinding } from "@/features/onboarding/fans";
import { SupportBadge } from "./SupportBadge";
import { fixedCurve, floorFor, sensorName, temperatureSources, withChannel, type FanMode, type FanView } from "./fanModel";

const MODES: { id: FanMode; label: string }[] = [
  { id: "bios", label: "BIOS" },
  { id: "curve", get label() { return t("cooling.fanCard.curve"); } },
  { id: "fixed", get label() { return t("cooling.fanCard.fixed"); } },
];

/** One motherboard fan header: what controls it now and a simple way to change it. */
export function FanCard({ view, index }: { view: FanView; index: number }) {
  const curves = useDaemon((s) => s.curves);
  const sensors = useDaemon((s) => s.monitoring_sensors);
  const background = useDaemon((s) => s.cooling_background);
  const bindings = useDaemon((s) => s.cooling_bindings);
  const support = useDaemon((s) => s.cooling_support);
  const session = useDaemon((s) => s.cooling_session);
  const { pending, error, run } = useAction();
  const sources = temperatureSources(sensors);
  const floor = floorFor(view, support?.unrestricted_enabled ?? false);
  const [mode, setMode] = useState<FanMode>(view.mode);
  const [curveName, setCurveName] = useState("");
  const [fixed, setFixed] = useState(Math.max(floor, view.saved && view.mode === "fixed" ? view.saved.curve.points[0]!.duty_percent : 60));
  const [temps, setTemps] = useState<string[]>(view.saved ? view.saved.temperature_id.split(",") : view.temperature ? [view.temperature.sensor.id] : []);
  const [identifying, setIdentifying] = useState<number | null>(null);
  const [asking, setAsking] = useState(false);
  useEffect(() => setMode(view.mode), [view.mode]);
  // A new role/test/unlock changes the allowed floor; keep the suggestion sensible.
  useEffect(() => { if (view.mode !== "fixed") setFixed(Math.max(floor, 60)); }, [floor, view.mode]);

  // Identification job finished -> ask what reacted.
  useEffect(() => {
    if (identifying === null || !session) return;
    // Several jobs can run at once; look this card's job up by id.
    const job = session.jobs?.find((j) => j.job_id === identifying) ?? (session.job_id === identifying ? session : null);
    if (!job) return;
    if (!["authorizing", "executing", "stopping"].includes(job.state)) {
      setIdentifying(null);
      if (job.state === "completed" || job.state === "stopped") setAsking(true);
    }
  }, [identifying, session]);

  const busy = pending !== null || background?.apply.state === "applying" || identifying !== null;
  const changed = mode !== view.mode || (mode === "curve" && curveName !== "") || (mode === "fixed" && view.mode === "fixed" && fixed !== view.saved?.curve.points[0]?.duty_percent)
    || (mode !== "bios" && view.saved !== null && temps.join(",") !== view.saved.temperature_id);
  const curve = mode === "fixed" ? fixedCurve(fixed) : curveName ? curves?.[curveName] : view.saved && view.mode === "curve" ? view.saved.curve : undefined;
  const belowFloor = curve !== undefined && curve.minimum_percent < floor;
  const canApply = !busy && changed && (mode === "bios" || (curve !== undefined && temps.length > 0 && !belowFloor && !!view.rpm));

  const apply = () => run("apply", async () => {
    const saved = background?.saved.channels ?? [];
    const next: BackgroundChannel | null = mode === "bios" ? null : {
      control_id: view.id, temperature_id: temps.join(","), rpm_id: view.rpm?.reading.id ?? "",
      curve: curve!, confirmed_fan: view.role === "fan",
    };
    await rpc("cooling.background.apply", { channels: withChannel(saved, view.id, next) });
  });
  const identify = () => run("identify", async () => {
    setAsking(false);
    const result = await rpc<{ job_id: number }>("cooling.sessions.start", {
      mode: "identify", control_id: view.id, temperature_id: view.temperature!.sensor.id, rpm_id: view.rpm!.reading.id,
      confirmed_fan: false,
    });
    setIdentifying(result.job_id);
  });
  const assign = (role: "fan" | "pump" | null) => run("assign", async () => {
    const current = bindings?.config.bindings ?? [];
    const next: ChannelBinding[] = role === null ? current.filter((b) => b.control_id !== view.control.id)
      : upsertBinding(current, { provider: "mainboard.lhm", device_id: view.control.device_id, control_id: view.control.id,
          tachometer_id: view.rpm?.id ?? null, header: view.name.slice(0, 64), role, electrical_mode: "unknown" });
    await rpc("cooling.bindings.save", { config: { version: 1, bindings: next } });
    setAsking(false);
  });
  const test = () => run("test", async () => {
    const result = await rpc<{ job_id: number }>("cooling.sessions.start", {
      mode: "channel_test", control_id: view.id, temperature_id: view.temperature!.sensor.id, rpm_id: view.rpm!.reading.id, confirmed_fan: true, critical_c: 80,
    });
    setIdentifying(result.job_id);
  });

  const Icon = view.role === "pump" ? Waves : Fan;
  const spinning = view.service?.state === "running" || identifying !== null;
  return (
    <article style={{ animationDelay: `${index * 40}ms` }}
      className="flex flex-col gap-4 rounded-card border border-border bg-surface p-4 animate-[pop-in_var(--duration-base)_var(--ease-out)_both]">
      <header className="flex items-start gap-3">
        <Icon className={`mt-0.5 size-6 shrink-0 ${spinning ? "animate-spin text-accent" : "text-text-subtle"}`} aria-hidden />
        <div className="flex min-w-0 flex-1 flex-col gap-0.5">
          <h3 className="truncate font-medium">{view.name}</h3>
          <StatusDot tone={view.tone} label={view.status} />
        </div>
        <div className="text-right tabular">
          <div className="text-title font-semibold">{view.rpmValue !== null ? Math.round(view.rpmValue) : "—"}<span className="ml-1 text-small font-normal text-text-subtle">RPM</span></div>
          <div className="text-caption text-text-subtle">{view.duty !== null ? `${Math.round(view.duty)} %` : ""}</div>
        </div>
      </header>

      {view.role === null && !view.blocked && (
        <div className="flex flex-col gap-2 rounded-control bg-surface-sunken p-3 text-small">
          {asking ? (
            <>
              <span>{t("cooling.fanCard.whichOneSpedUp")}</span>
              <div className="flex flex-wrap gap-2">
                <Button size="sm" variant="primary" disabled={busy} onClick={() => assign("fan")}><Fan />{t("cooling.fanCard.aFan")}</Button>
                <Button size="sm" disabled={busy} onClick={() => assign("pump")}><Waves />{t("cooling.fanCard.aPumpAio")}</Button>
                <Button size="sm" variant="ghost" onClick={() => setAsking(false)}>{t("cooling.fanCard.nothing")}</Button>
              </div>
            </>
          ) : (
            <>
              <span className="text-text-muted">{t("cooling.fanCard.spinItUpFor8SecondsAt")}</span>
              <div><Button size="sm" disabled={busy || !view.temperature || !view.rpm} onClick={identify}>
                {identifying !== null ? <LoaderCircle className="animate-spin" /> : <Search />}{identifying !== null ? t("cooling.fanCard.spinningUp") : t("cooling.fanCard.identify")}
              </Button></div>
            </>
          )}
        </div>
      )}

      {view.role !== null && !view.blocked && (
        <div className="flex flex-col gap-3">
          <div role="radiogroup" aria-label={t("cooling.fanCard.valueControl", { p0: view.name })} className="inline-flex w-fit rounded-control bg-surface-sunken p-0.5">
            {MODES.map((m) => (
              <button key={m.id} type="button" role="radio" aria-checked={mode === m.id} onClick={() => setMode(m.id)}
                className={`rounded-control px-3 py-1 text-small transition-colors duration-(--duration-base) ${mode === m.id ? "bg-surface-raised text-text" : "text-text-muted hover:text-text"}`}>
                {m.label}
              </button>
            ))}
          </div>
          {mode === "bios" && <p className="text-small text-text-muted">{t("cooling.fan.firmware", { role: roleLabel(view.role) })}</p>}
          {mode === "curve" && (
            <Select label={t("cooling.fanCard.curve")} placeholder={view.saved && view.mode === "curve" ? t("cooling.fanCard.keepCurrentCurve") : t("cooling.fanCard.chooseACurve")} value={curveName}
              options={Object.keys(curves ?? {}).map((n) => ({ value: n, label: n }))} onChange={setCurveName} />
          )}
          {mode === "fixed" && (
            <div className="flex flex-col gap-1.5">
              <div className="flex justify-between text-small"><span>{t("cooling.fanCard.speed")}</span><span className="tabular text-text-muted">{fixed} %</span></div>
              <Slider aria-label={t("cooling.fanCard.valueFixedSpeed", { p0: view.name })} min={floor} max={100} step={1} value={[Math.max(fixed, floor)]} onValueChange={([v]) => setFixed(v!)} />
              {floor >= 100 && <span className="text-caption text-text-subtle">{view.role === "pump" ? t("cooling.fanCard.pumpsStayAt100") : t("cooling.fanCard.runTheChannelTestFirstToAllow")}</span>}
            </div>
          )}
          {mode !== "bios" && (
            <div className="flex flex-col gap-1.5">
              <span className="text-small">{t("cooling.fanCard.follows")} {temps.length > 1 ? t("cooling.fanCard.theHottestOf") : ""}</span>
              <div className="flex flex-wrap gap-1.5">
                {sources.map((s) => {
                  const on = temps.includes(s.sensor.id);
                  return (
                    <button key={s.sensor.id} type="button" aria-pressed={on} disabled={!on && temps.length >= 3}
                      onClick={() => setTemps(on ? temps.filter((t) => t !== s.sensor.id) : [...temps, s.sensor.id])}
                      className={`rounded-full border px-2.5 py-1 text-caption transition-colors duration-(--duration-base) ${on ? "border-accent bg-accent/15 text-text" : "border-border text-text-muted hover:text-text disabled:opacity-40"}`}>
                      {sensorName(s.sensor.id, sensors)} {s.sensor.value !== null ? `· ${Math.round(s.sensor.value)} °C` : ""}
                    </button>
                  );
                })}
              </div>
            </div>
          )}
          {belowFloor && <p className="text-caption text-[#fbbf24]">{t("cooling.fan.belowFloor", { floor, role: roleLabel(view.role) })}</p>}
          <div className="flex flex-wrap items-center gap-2">
            <Button size="sm" variant="primary" disabled={!canApply} onClick={apply}>{pending === "apply" ? t("cooling.fanCard.applying") : t("cooling.fanCard.apply")}</Button>
            {view.role === "fan" && view.evidence?.test !== "passed" && (
              <Button size="sm" variant="ghost" disabled={busy || !view.temperature || !view.rpm} onClick={test}><FlaskConical />{t("cooling.fanCard.channelTest")}</Button>
            )}
            {view.session && (
              <Button size="sm" variant="danger" disabled={pending !== null}
                onClick={() => run("stop", () => rpc("cooling.sessions.stop_channel", { control_id: view.id }))}>{t("cooling.fanCard.stopLive")}</Button>
            )}
            <span className="ml-auto flex items-center gap-2 text-caption text-text-subtle">
              <SupportBadge tier={view.tier} coverage={view.evidence?.coverage} />
              <button type="button" className="underline-offset-2 hover:underline" onClick={() => void assign(null)}>{t("cooling.fanCard.reassign")}</button>
            </span>
          </div>
        </div>
      )}
      {error && <p role="alert" className="text-small text-danger-text">{error}</p>}
    </article>
  );
}
