import { t } from "./messages";
import { stateLabel } from "./labels";
import { useEffect, useState } from "react";
import { OctagonX } from "lucide-react";
import { Section } from "@/components/page";
import { Button } from "@/components/ui/button";
import { Select } from "@/components/ui/select";
import { rpc, useDaemon, useSlices } from "@/lib/store/daemon";
import { useAction } from "@/lib/store/useAction";
import { Banner } from "@/components/status";
import { SupportBadge } from "./SupportBadge";
import { channelTier, controlRefusal, tierLabel } from "./support";
import { floorFor } from "./fanModel";
import { clampDuty, isActive, jobsOf, liveChannels, liveConfirmation, type SessionList } from "./liveModel";
import type { CurveSet } from "@/lib/api/control";
import { EMPTY_SHAPING, shapingPayload, type ShapingDraft } from "./shapingModel";

type LiveMode = "curve" | "calibration" | "channel_test" | "manual" | "composite" | "rpm";

/** Live state of every channel with per-channel and global stop. */
export function LiveChannels() {
  useSlices("cooling_session", "cooling_channels");
  const session = useDaemon((s) => s.cooling_session);
  const channels = useDaemon((s) => s.cooling_channels);
  const connected = useDaemon((s) => s.connection?.connected);
  const [list, setList] = useState<SessionList | null>(null);
  const { pending, error, run } = useAction();
  useEffect(() => {
    let live = true;
    rpc<SessionList>("cooling.sessions.list").then((l) => { if (live) setList(l); }, () => { if (live) setList(null); });
    return () => { live = false; };
  }, [session]);
  const rows = liveChannels(session, list);
  const active = rows.filter((r) => r.job_id !== null && ["starting", "curve", "manual", "test", "stopping"].includes(r.state)).length;
  const name = (id: string) => channels?.find((c) => c.kind === "control" && c.reading.id === id)?.name ?? id;
  return (
    <div className="flex flex-col gap-2">
      <div className="flex flex-wrap items-center gap-3">
        <span className="text-small">{t("cooling.live.count", { active, max: list?.max_active ?? 16 })}</span>
        <Button variant="danger" className="ms-auto" disabled={!connected || active === 0 || pending !== null}
          onClick={() => run("stop_all", () => rpc("cooling.sessions.stop_all"))}><OctagonX />{t("cooling.liveSession.stopAllAndReturnToBios")}</Button>
      </div>
      {rows.length > 0 && (
        <ul className="flex flex-col divide-y divide-border rounded-control border border-border">
          {rows.map((r) => (
            <li key={r.control_id} className="flex flex-wrap items-center gap-2 px-3 py-2 text-small">
              <span className="font-medium">{name(r.control_id)}</span>
              <span className={r.state === "fault" || r.state === "needs_attention" ? "text-danger-text" : "text-text-muted"}>{r.label}</span>
              {r.error && <span className="basis-full text-caption text-text-subtle">{r.error}</span>}
              {r.job_id !== null && ["starting", "curve", "manual", "test"].includes(r.state) && (
                <Button size="sm" variant="secondary" className="ms-auto" disabled={!connected || pending !== null}
                  onClick={() => run("stop", () => rpc("cooling.sessions.stop_channel", { control_id: r.control_id }))}>{t("cooling.liveSession.stop")}</Button>
              )}
            </li>
          ))}
        </ul>
      )}
      {error && <p role="alert" className="text-small text-danger-text">{error}</p>}
    </div>
  );
}

export function LiveSession() {
  useSlices("cooling_bindings", "cooling_session", "cooling_channels", "monitoring_sensors", "providers", "cooling_support");
  const support = useDaemon((s) => s.cooling_support);
  const status = useDaemon((s) => s.cooling_session);
  const channels = useDaemon((s) => s.cooling_channels);
  const sensors = useDaemon((s) => s.monitoring_sensors);
  const curves = useDaemon((s) => s.curves);
  const connected = useDaemon((s) => s.connection?.connected);
  const [control, setControl] = useState("");
  const [rpm, setRpm] = useState("");
  const [temperature, setTemperature] = useState("");
  const [curveName, setCurveName] = useState("");
  const [manualDuty, setManualDuty] = useState(60);
  const [composite, setComposite] = useState("");
  const [targetRpm, setTargetRpm] = useState(900);
  const [closedLoop, setClosedLoop] = useState(true);
  const [sweepStep, setSweepStep] = useState(0);
  const [stallProbe, setStallProbe] = useState(false);
  const [shaping, setShaping] = useState<ShapingDraft>(EMPTY_SHAPING);
  const [compositeNames, setCompositeNames] = useState<string[]>([]);
  useEffect(() => {
    let live = true;
    rpc<CurveSet>("cooling.curve_set.get").then((set) => { if (live) setCompositeNames(Object.keys(set.nodes)); }, () => {});
    return () => { live = false; };
  }, []);
  const { pending, error, run } = useAction();
  const [asking, setAsking] = useState<LiveMode | null>(null);
  useEffect(() => { setAsking(null); }, [control, rpm, temperature, curveName]);
  // Several channels can run at once; only the selected channel's own job blocks it.
  const selectedJob = jobsOf(status).find((j) => j.control_id === control);
  const busy = selectedJob != null && isActive(selectedJob);
  const curve = curves?.[curveName];
  const selected = channels?.find((c) => c.kind === "control" && c.reading.id === control);
  const tier = channelTier(selected);
  // The role from fan identification replaces a per-session confirmation.
  const bindings = useDaemon((s) => s.cooling_bindings);
  const role = bindings?.config.bindings.find((b) => b.control_id === selected?.id)?.role ?? null;
  const confirmed = role === "fan";
  const refusal = controlRefusal(selected?.support);
  const selectionValid = channels?.some((c) => c.kind === "control" && c.reading.id === control)
    && channels?.some((c) => c.kind === "fan" && c.reading.id === rpm)
    && sensors?.some((s) => s.provider === "mainboard.lhm" && s.sensor.unit === "°C" && s.sensor.id === temperature);
  const ready = connected && status?.available && status.state !== "blocked" && !busy && confirmed && selectionValid && !refusal && pending === null;
  const evidence = support?.channels[control];
  const floor = floorFor({ role, evidence: evidence ?? null, tier }, support?.unrestricted_enabled ?? false);
  const duty = clampDuty(manualDuty, floor);
  const unlocked = evidence?.unlock === "unlocked";
  const shaped = shapingPayload(shaping, unlocked && role === "fan");
  const shapingError = typeof shaped === "string" ? shaped : null;
  const send = (mode: LiveMode) => run("start", () => rpc("cooling.sessions.start", {
    mode, control_id: control, rpm_id: rpm, temperature_id: temperature, confirmed_fan: confirmed,
    ...(mode === "curve" ? { curve } : mode === "manual" ? { duty_percent: duty } : mode === "composite" ? { composite_output: composite, critical_c: 80 } : mode === "rpm" ? { target_rpm: targetRpm, closed_loop: closedLoop } : { critical_c: curve?.critical_c ?? 80 }),
    ...(mode === "calibration" && sweepStep > 0 ? { sweep: { step_percent: sweepStep, lowest_percent: stallProbe && unlocked ? 0 : 40, probe_stall: stallProbe && unlocked } } : {}),
    ...(["curve", "manual", "composite", "rpm"].includes(mode) && shaped && typeof shaped !== "string" ? { shaping: shaped } : {}),
  }));
  // One confirmation per app session, not per channel; the service still checks each.
  const start = (mode: LiveMode) => {
    if (shapingError && mode !== "calibration" && mode !== "channel_test") return;
    if (liveConfirmation.needed()) setAsking(mode);
    else send(mode);
  };
  return (
    <Section title={t("cooling.liveSession.liveMotherboardFanControl")}>
      <p className="text-small text-text-muted">{t("cooling.liveSession.runCurvesOrFixedSpeedsOnSeveral")}</p>
      {status?.available && <LiveChannels />}
      {asking && (
        <Banner tone="warn">
          <span className="flex-1">{t("cooling.liveSession.freergbWillControlThisFanDirectlySafety")}</span>
          <Button size="sm" variant="primary" disabled={!ready} onClick={() => { liveConfirmation.confirm(); const mode = asking; setAsking(null); send(mode); }}>{t("cooling.liveSession.continue")}</Button>
          <Button size="sm" variant="ghost" onClick={() => setAsking(null)}>{t("cooling.liveSession.cancel")}</Button>
        </Banner>
      )}
      {!status?.available ? <p className="text-small text-text-subtle">{t("cooling.liveSession.liveControlIsUnavailableEnableTheOptional")}</p> : <>
        <div className="grid gap-3 md:grid-cols-2">
          {([ [t("cooling.liveSession.fanOutput"), "control", control, setControl], [t("cooling.liveSession.matchingFanRpm"), "fan", rpm, setRpm] ] as const).map(([label, kind, value, set]) => (
            <Select key={kind} label={label} placeholder={label} value={value} options={(channels ?? []).filter((c) => c.kind === kind).map((c) => ({ value: c.reading.id, label: kind === "control" ? `${c.name} · ${tierLabel(channelTier(c))} · ${c.reading.id}` : `${c.name} · ${c.reading.id}` }))} onChange={(v) => { set(v); }} />
          ))}
          <Select label={t("cooling.liveSession.temperatureSource")} placeholder={t("cooling.liveSession.temperatureSource")} value={temperature} options={(sensors ?? []).filter((s) => s.provider === "mainboard.lhm" && s.sensor.unit === "°C").map((s) => ({ value: s.sensor.id, label: `${s.sensor.name} · ${s.sensor.id}` }))} onChange={(v) => { setTemperature(v); }} />
          <Select label={t("cooling.liveSession.savedCurve")} placeholder={t("cooling.liveSession.savedCurve")} value={curveName} options={Object.keys(curves ?? {}).map((name) => ({ value: name, label: name }))} onChange={setCurveName} />
        </div>
        {selected && <SupportBadge tier={tier} coverage={evidence?.coverage} />}
        {refusal && <Banner tone="warn">{refusal}</Banner>}
        {selected && <p className="text-caption text-text-subtle">{t("cooling.liveSession.channelTest")} {stateLabel(evidence?.test ?? "none")}{evidence?.tested ? " · " + evidence.tested.map((t) => `${t.duty_percent} % → ${Math.round(t.rpm)} RPM`).join(", ") : ""}{t("cooling.liveSession.theTestRuns1007050Never")}</p>}
        {tier === "lhm_supported" && <p className="text-caption text-text-subtle">{t("cooling.liveSession.lhmSupportedChannelsRunReducedSpeedCurves")}</p>}
        {selected && !confirmed && <p className="text-caption text-text-subtle">{role === "pump" ? t("cooling.liveSession.thisHeaderIsAssignedAsAPump") : t("cooling.liveSession.identifyThisHeaderAsAFanOn")}</p>}
        <p className="text-caption text-text-subtle">{t("cooling.liveSession.liveCurvesRequireAMinimumOf40")}</p>
        <div className="flex flex-wrap gap-2">
          <Button disabled={!ready || !curve || (curve.minimum_percent < 40 && support?.channels[control]?.unlock !== "unlocked" && support?.channels[control]?.unlock !== "unlocked_pump") || curve.critical_c > 100} onClick={() => start("curve")}>{t("cooling.liveSession.startSavedCurveUac")}</Button>
          <Button variant="secondary" disabled={!ready || (curve?.critical_c ?? 80) > 100} onClick={() => start("calibration")}>{t("cooling.liveSession.calibrateFanUac")}</Button>
          <Button variant="secondary" disabled={!ready || (curve?.critical_c ?? 80) > 100} onClick={() => start("channel_test")}>{t("cooling.liveSession.runChannelTestUac")}</Button>
          <label className="flex items-center gap-2 text-small">{t("cooling.liveSession.fixedDuty")} <input type="number" min={floor} max={100} value={manualDuty} onBlur={() => setManualDuty(duty)} onChange={(e) => setManualDuty(Math.max(0, Math.min(100, Number(e.target.value) || 0)))} className="w-16 rounded-control border border-border bg-surface-sunken px-2 py-1" aria-label={t("cooling.liveSession.fixedDutyPercent")} /> %
          </label>
          <Button variant="secondary" disabled={!ready} onClick={() => start("manual")}>{t("cooling.liveSession.runFixedDutyUac")}</Button>
        </div>
        <div className="flex flex-wrap items-center gap-3 text-small">
          <label className="flex items-center gap-2">{t("cooling.liveSession.calibrationSteps")} <select aria-label={t("cooling.liveSession.calibrationSteps")} value={sweepStep} onChange={(e) => setSweepStep(Number(e.target.value))} className="rounded-control border border-border bg-surface-sunken px-2 py-1">
              <option value={0}>100 / 80 / 60 / 40 %</option>
              <option value={10}>{t("cooling.liveSession.every10")}</option>
              <option value={5}>{t("cooling.liveSession.every5")}</option>
              <option value={2}>{t("cooling.liveSession.every2")}</option>
            </select>
          </label>
          <label className="flex items-center gap-2"><input type="checkbox" disabled={!unlocked || sweepStep === 0} checked={stallProbe && unlocked} onChange={(e) => setStallProbe(e.target.checked)} />{t("cooling.liveSession.findStallAndStartSpeedUnrestrictedFans")}</label>
          {(evidence?.stall_percent != null || evidence?.start_percent != null) && <span className="text-caption text-text-subtle">{t("cooling.liveSession.storedStopsAt")} {evidence?.stall_percent ?? "?"} {t("cooling.liveSession.startsAt")} {evidence?.start_percent ?? "?"} %.</span>}
        </div>
        <details className="text-small">
          <summary>{t("cooling.liveSession.outputShapingCurveFixedDutyCompositeAnd")}</summary>
          <div className="mt-2 flex flex-wrap items-center gap-3">
            {([
              [t("cooling.liveSession.offset"), "offset", "%"], [t("cooling.liveSession.min"), "min", "%"], [t("cooling.liveSession.max"), "max", "%"], [t("cooling.liveSession.delay"), "delayS", "s"],
              [t("cooling.liveSession.hysteresisUp"), "upPercent", "%"], [t("cooling.liveSession.hysteresisDown"), "downPercent", "%"],
              [t("cooling.liveSession.stopBelow"), "stopBelow", "%"], [t("cooling.liveSession.spinUp"), "spinUpPercent", "%"], [t("cooling.shaping.for"), "spinUpMs", "ms"],
            ] as const).map(([label, key, unit]) => (
              <label key={key} className="flex items-center gap-1">{label}
                <input type="number" aria-label={label} value={shaping[key]} onChange={(e) => setShaping({ ...shaping, [key]: Number(e.target.value) || 0 })} className="w-16 rounded-control border border-border bg-surface-sunken px-2 py-1" />{unit}
              </label>
            ))}
            <label className="flex items-center gap-1">{t("cooling.liveSession.avoid")} <input type="text" aria-label={t("cooling.liveSession.avoidedRanges")} placeholder="50-60, 1200-1300 rpm" value={shaping.avoid} onChange={(e) => setShaping({ ...shaping, avoid: e.target.value })} className="w-48 rounded-control border border-border bg-surface-sunken px-2 py-1" />
            </label>
          </div>
          <p className="mt-1 text-caption text-text-subtle">{t("cooling.liveSession.appliedAfterTheCurveBeforeTheSafety")}</p>
          {shapingError && <p role="alert" className="text-caption text-danger-text">{shapingError}</p>}
        </details>
        {(() => {
          const range = evidence?.rpm_range;
          const inRange = !!range && targetRpm >= range.min_rpm;
          return (
            <div className="flex flex-wrap items-center gap-2">
              <label className="flex items-center gap-2 text-small">{t("cooling.liveSession.targetSpeed")} <input type="number" min={range ? Math.ceil(range.min_rpm) : 0} step={10} value={targetRpm} onChange={(e) => setTargetRpm(Math.max(0, Math.round(Number(e.target.value) || 0)))} className="w-20 rounded-control border border-border bg-surface-sunken px-2 py-1" aria-label={t("cooling.liveSession.targetSpeedInRpm")} /> RPM
              </label>
              <label className="flex items-center gap-2 text-small"><input type="checkbox" checked={closedLoop} onChange={(e) => setClosedLoop(e.target.checked)} />{t("cooling.liveSession.closedLoop")}</label>
              <Button variant="secondary" disabled={!ready || !inRange} onClick={() => start("rpm")}>{t("cooling.liveSession.runRpmTargetUac")}</Button>
              <span className="basis-full text-caption text-text-subtle">
                {range ? t("cooling.liveSession.measuredValueValueRpmTheTargetStarts", { p0: Math.round(range.min_rpm), p1: Math.round(range.max_rpm) }) : t("cooling.liveSession.runTheChannelTestOrCalibrationOn")}
              </span>
            </div>
          );
        })()}
        {compositeNames.length > 0 && (
          <div className="flex flex-wrap items-center gap-2">
            <Select label={t("cooling.liveSession.compositeCurve")} placeholder={t("cooling.liveSession.compositeCurve")} value={composite} options={compositeNames.map((n) => ({ value: n, label: n }))} onChange={setComposite} />
            <Button variant="secondary" disabled={!ready || !composite} onClick={() => start("composite")}>{t("cooling.liveSession.runCompositeCurveUac")}</Button>
            <span className="basis-full text-caption text-text-subtle">{t("cooling.liveSession.theDaemonEvaluatesTheCompositeCurveAnd")}</span>
          </div>
        )}
      </>}
      <div className="flex items-center gap-3 text-small">
        <span>{t("cooling.liveSession.session")} {stateLabel(selectedJob?.state ?? "unavailable")}{selectedJob?.job_id != null ? ` · ${selectedJob.job_id}` : ""}{selectedJob?.tier ? ` · ${tierLabel(selectedJob.tier)}` : ""}</span>
        <Button variant="secondary" disabled={!connected || !busy || pending !== null} onClick={() => run("stop", () => rpc("cooling.sessions.stop_channel", { control_id: control }))}>{t("cooling.liveSession.stopThisChannel")}</Button>
      </div>
      {(error || selectedJob?.error || status?.error) && <p role="alert" className="text-small text-danger-text">{error || selectedJob?.error || status?.error}</p>}
      {selectedJob?.report != null && <details className="text-small"><summary>{t("cooling.liveSession.lastExecutionCalibrationResult")}</summary><pre className="overflow-auto whitespace-pre-wrap text-caption">{JSON.stringify(selectedJob.report, null, 2)}</pre></details>}
    </Section>
  );
}
