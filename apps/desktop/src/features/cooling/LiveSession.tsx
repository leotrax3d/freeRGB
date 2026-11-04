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
import { activeCount, clampDuty, liveChannels, liveConfirmation, type SessionList } from "./liveModel";
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
  const active = activeCount(session);
  const name = (id: string) => channels?.find((c) => c.kind === "control" && c.reading.id === id)?.name ?? id;
  return (
    <div className="flex flex-col gap-2">
      <div className="flex flex-wrap items-center gap-3">
        <span className="text-small">{active} of {list?.max_active ?? 16} channels under live control</span>
        <Button variant="danger" className="ml-auto" disabled={!connected || active === 0 || pending !== null}
          onClick={() => run("stop_all", () => rpc("cooling.sessions.stop_all"))}><OctagonX />Stop all and return to BIOS</Button>
      </div>
      {rows.length > 0 && (
        <ul className="flex flex-col divide-y divide-border rounded-control border border-border">
          {rows.map((r) => (
            <li key={r.control_id} className="flex flex-wrap items-center gap-2 px-3 py-2 text-small">
              <span className="font-medium">{name(r.control_id)}</span>
              <span className={r.state === "fault" || r.state === "needs_attention" ? "text-danger-text" : "text-text-muted"}>{r.label}</span>
              {r.error && <span className="basis-full text-caption text-text-subtle">{r.error}</span>}
              {r.job_id !== null && ["starting", "curve", "manual", "test"].includes(r.state) && (
                <Button size="sm" variant="secondary" className="ml-auto" disabled={pending !== null}
                  onClick={() => run("stop", () => rpc("cooling.sessions.stop_channel", { control_id: r.control_id }))}>Stop</Button>
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
  const [asking, setAsking] = useState<null | (() => void)>(null);
  // Several channels can run at once; only the selected channel's own job blocks it.
  const busy = (status?.jobs ?? []).some((j) => j.control_id === control && ["authorizing", "executing", "stopping"].includes(j.state));
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
    if (liveConfirmation.needed()) setAsking(() => () => send(mode));
    else send(mode);
  };
  return (
    <Section title="Live motherboard fan control">
      <p className="text-small text-text-muted">Run curves or fixed speeds on several fans at once. Select the physical fan and its matching RPM and temperature. Windows authorization is requested when a channel starts. Closing this window leaves the daemon and its fan sessions running; remembered channels resume after a restart while their hardware is unchanged.</p>
      {status?.available && <LiveChannels />}
      {asking && (
        <Banner tone="warn">
          <span className="flex-1">freeRGB will control this fan directly. Safety limits stay active: pumps at 100 %, fans never below their tier floor, any sensor or worker fault returns the fan to the BIOS. You are asked once per session.</span>
          <Button size="sm" variant="primary" onClick={() => { liveConfirmation.confirm(); const go = asking; setAsking(null); go(); }}>Continue</Button>
          <Button size="sm" variant="ghost" onClick={() => setAsking(null)}>Cancel</Button>
        </Banner>
      )}
      {!status?.available ? <p className="text-small text-text-subtle">Live control is unavailable. Enable the optional motherboard provider through the installer and restart the daemon.</p> : <>
        <div className="grid gap-3 md:grid-cols-2">
          {([ ["Fan output", "control", control, setControl], ["Matching fan RPM", "fan", rpm, setRpm] ] as const).map(([label, kind, value, set]) => (
            <Select key={kind} label={label} placeholder={label} value={value} options={(channels ?? []).filter((c) => c.kind === kind).map((c) => ({ value: c.reading.id, label: kind === "control" ? `${c.name} · ${tierLabel(channelTier(c))} · ${c.reading.id}` : `${c.name} · ${c.reading.id}` }))} onChange={(v) => { set(v); }} />
          ))}
          <Select label="Temperature source" placeholder="Temperature source" value={temperature} options={(sensors ?? []).filter((s) => s.provider === "mainboard.lhm" && s.sensor.unit === "°C").map((s) => ({ value: s.sensor.id, label: `${s.sensor.name} · ${s.sensor.id}` }))} onChange={(v) => { setTemperature(v); }} />
          <Select label="Saved curve" placeholder="Saved curve" value={curveName} options={Object.keys(curves ?? {}).map((name) => ({ value: name, label: name }))} onChange={setCurveName} />
        </div>
        {selected && <SupportBadge tier={tier} coverage={evidence?.coverage} />}
        {refusal && <Banner tone="warn">{refusal}</Banner>}
        {selected && <p className="text-caption text-text-subtle">Channel test: {evidence?.test ?? "none"}{evidence?.tested ? " · " + evidence.tested.map((t) => `${t.duty_percent} % → ${Math.round(t.rpm)} RPM`).join(", ") : ""}. The test runs 100 → 70 → 50 % (never below 40 %) and returns the fan to firmware control; a new BIOS or board invalidates it.</p>}
        {tier === "lhm_supported" && <p className="text-caption text-text-subtle">LHM-supported channels run reduced-speed curves only after a passed channel test; calibration (100 → 40 %) is available.</p>}
        {selected && !confirmed && <p className="text-caption text-text-subtle">{role === "pump" ? "This header is assigned as a pump; live tests are for fans only." : "Identify this header as a fan on its card above first."}</p>}
        <p className="text-caption text-text-subtle">Live curves require a minimum of 40% and a critical temperature at most 100°C. Channel selection alone does not establish a physical header or PWM/DC mode.</p>
        <div className="flex flex-wrap gap-2">
          <Button disabled={!ready || !curve || (curve.minimum_percent < 40 && support?.channels[control]?.unlock !== "unlocked" && support?.channels[control]?.unlock !== "unlocked_pump") || curve.critical_c > 100} onClick={() => start("curve")}>Start saved curve (UAC)</Button>
          <Button variant="secondary" disabled={!ready || (curve?.critical_c ?? 80) > 100} onClick={() => start("calibration")}>Calibrate fan (UAC)</Button>
          <Button variant="secondary" disabled={!ready || (curve?.critical_c ?? 80) > 100} onClick={() => start("channel_test")}>Run channel test (UAC)</Button>
          <label className="flex items-center gap-2 text-small">Fixed duty
            <input type="number" min={floor} max={100} value={manualDuty} onBlur={() => setManualDuty(duty)} onChange={(e) => setManualDuty(Math.max(0, Math.min(100, Number(e.target.value) || 0)))} className="w-16 rounded-control border border-border bg-surface-sunken px-2 py-1" aria-label="Fixed duty percent" /> %
          </label>
          <Button variant="secondary" disabled={!ready} onClick={() => start("manual")}>Run fixed duty (UAC)</Button>
        </div>
        <div className="flex flex-wrap items-center gap-3 text-small">
          <label className="flex items-center gap-2">Calibration steps
            <select aria-label="Calibration steps" value={sweepStep} onChange={(e) => setSweepStep(Number(e.target.value))} className="rounded-control border border-border bg-surface-sunken px-2 py-1">
              <option value={0}>100 / 80 / 60 / 40 %</option>
              <option value={10}>every 10 %</option>
              <option value={5}>every 5 %</option>
              <option value={2}>every 2 %</option>
            </select>
          </label>
          <label className="flex items-center gap-2"><input type="checkbox" disabled={!unlocked || sweepStep === 0} checked={stallProbe && unlocked} onChange={(e) => setStallProbe(e.target.checked)} />Find stall and start speed (unrestricted fans only; the fan stops briefly)</label>
          {(evidence?.stall_percent != null || evidence?.start_percent != null) && <span className="text-caption text-text-subtle">Stored: stops at {evidence?.stall_percent ?? "?"} %, starts at {evidence?.start_percent ?? "?"} %.</span>}
        </div>
        <details className="text-small">
          <summary>Output shaping (curve, fixed duty, composite and RPM modes)</summary>
          <div className="mt-2 flex flex-wrap items-center gap-3">
            {([
              ["Offset", "offset", "%"], ["Min", "min", "%"], ["Max", "max", "%"], ["Delay", "delayS", "s"],
              ["Hysteresis up", "upPercent", "%"], ["Hysteresis down", "downPercent", "%"],
              ["Stop below", "stopBelow", "%"], ["Spin-up", "spinUpPercent", "%"], ["for", "spinUpMs", "ms"],
            ] as const).map(([label, key, unit]) => (
              <label key={key} className="flex items-center gap-1">{label}
                <input type="number" aria-label={label} value={shaping[key]} onChange={(e) => setShaping({ ...shaping, [key]: Number(e.target.value) || 0 })} className="w-16 rounded-control border border-border bg-surface-sunken px-2 py-1" />{unit}
              </label>
            ))}
            <label className="flex items-center gap-1">Avoid
              <input type="text" aria-label="Avoided ranges" placeholder="50-60, 1200-1300 rpm" value={shaping.avoid} onChange={(e) => setShaping({ ...shaping, avoid: e.target.value })} className="w-48 rounded-control border border-border bg-surface-sunken px-2 py-1" />
            </label>
          </div>
          <p className="mt-1 text-caption text-text-subtle">Applied after the curve, before the safety floor. Sensor faults and critical temperatures still go straight to 100 %. Fan stop only on unrestricted fans, never on pumps; RPM ranges use this fan's measured speeds.</p>
          {shapingError && <p role="alert" className="text-caption text-danger-text">{shapingError}</p>}
        </details>
        {(() => {
          const range = evidence?.rpm_range;
          const inRange = !!range && targetRpm >= range.min_rpm;
          return (
            <div className="flex flex-wrap items-center gap-2">
              <label className="flex items-center gap-2 text-small">Target speed
                <input type="number" min={range ? Math.ceil(range.min_rpm) : 0} step={10} value={targetRpm} onChange={(e) => setTargetRpm(Math.max(0, Math.round(Number(e.target.value) || 0)))} className="w-20 rounded-control border border-border bg-surface-sunken px-2 py-1" aria-label="Target speed in RPM" /> RPM
              </label>
              <label className="flex items-center gap-2 text-small"><input type="checkbox" checked={closedLoop} onChange={(e) => setClosedLoop(e.target.checked)} />Closed loop</label>
              <Button variant="secondary" disabled={!ready || !inRange} onClick={() => start("rpm")}>Run RPM target (UAC)</Button>
              <span className="basis-full text-caption text-text-subtle">
                {range ? `Measured ${Math.round(range.min_rpm)}–${Math.round(range.max_rpm)} RPM. The target starts from the duty of that measurement; closed loop then corrects it from the live RPM (open loop again while the RPM reading is missing). Faster targets run at 100 %, slower ones are refused.` : "Run the channel test or calibration on this fan first; RPM targets use its measured speeds."}
              </span>
            </div>
          );
        })()}
        {compositeNames.length > 0 && (
          <div className="flex flex-wrap items-center gap-2">
            <Select label="Composite curve" placeholder="Composite curve" value={composite} options={compositeNames.map((n) => ({ value: n, label: n }))} onChange={setComposite} />
            <Button variant="secondary" disabled={!ready || !composite} onClick={() => start("composite")}>Run composite curve (UAC)</Button>
            <span className="basis-full text-caption text-text-subtle">The daemon evaluates the composite curve and feeds its duty to this channel. If the daemon stops, a sensor fails or the selected temperature reaches 80 °C, the fan goes to 100 % or back to the BIOS.</span>
          </div>
        )}
      </>}
      <div className="flex items-center gap-3 text-small">
        <span>Session: {status?.state ?? "unavailable"}{status?.job_id != null ? ` · ${status.job_id}` : ""}{status?.tier ? ` · ${tierLabel(status.tier)}` : ""}</span>
        <Button variant="secondary" disabled={!connected || !busy || pending !== null} onClick={() => run("stop", () => rpc("cooling.sessions.stop_channel", { control_id: control }))}>Stop this channel</Button>
      </div>
      {(error || status?.error) && <p role="alert" className="text-small text-danger-text">{error || status?.error}</p>}
      {status?.report != null && <details className="text-small"><summary>Last execution / calibration result</summary><pre className="overflow-auto whitespace-pre-wrap text-caption">{JSON.stringify(status.report, null, 2)}</pre></details>}
    </Section>
  );
}
