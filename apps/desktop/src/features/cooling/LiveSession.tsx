import { useState } from "react";
import { Section } from "@/components/page";
import { Button } from "@/components/ui/button";
import { Select } from "@/components/ui/select";
import { Checkbox } from "@/components/ui/checkbox";
import { rpc, useDaemon, useSlices } from "@/lib/store/daemon";
import { useAction } from "@/lib/store/useAction";
import { Banner } from "@/components/status";
import { SupportBadge } from "./SupportBadge";
import { channelTier, controlRefusal, tierLabel } from "./support";

export function LiveSession() {
  useSlices("cooling_session", "cooling_channels", "monitoring_sensors", "providers", "cooling_support");
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
  const [confirmed, setConfirmed] = useState(false);
  const { pending, error, run } = useAction();
  const busy = ["authorizing", "executing", "stopping"].includes(status?.state ?? "");
  const curve = curves?.[curveName];
  const selected = channels?.find((c) => c.kind === "control" && c.reading.id === control);
  const tier = channelTier(selected);
  const refusal = controlRefusal(selected?.support);
  const selectionValid = channels?.some((c) => c.kind === "control" && c.reading.id === control)
    && channels?.some((c) => c.kind === "fan" && c.reading.id === rpm)
    && sensors?.some((s) => s.provider === "mainboard.lhm" && s.sensor.unit === "°C" && s.sensor.id === temperature);
  const ready = connected && status?.available && status.state !== "blocked" && !busy && confirmed && selectionValid && !refusal && pending === null;
  const evidence = support?.channels[control];
  const start = (mode: "curve" | "calibration" | "channel_test") => run("start", () => rpc("cooling.sessions.start", {
    mode, control_id: control, rpm_id: rpm, temperature_id: temperature, confirmed_fan: confirmed,
    ...(mode === "curve" ? { curve } : { critical_c: curve?.critical_c ?? 80 }),
  }));
  return (
    <Section title="Live motherboard fan control">
      <p className="text-small text-text-muted">Experimental. Select the physical fan and its matching RPM and temperature. Starting requests Windows authorization. Closing this window leaves the daemon and fan session running.</p>
      {!status?.available ? <p className="text-small text-text-subtle">Live control is unavailable. Enable the optional motherboard provider through the installer and restart the daemon.</p> : <>
        <div className="grid gap-3 md:grid-cols-2">
          {([ ["Fan output", "control", control, setControl], ["Matching fan RPM", "fan", rpm, setRpm] ] as const).map(([label, kind, value, set]) => (
            <Select key={kind} label={label} placeholder={label} value={value} options={(channels ?? []).filter((c) => c.kind === kind).map((c) => ({ value: c.reading.id, label: kind === "control" ? `${c.name} · ${tierLabel(channelTier(c))} · ${c.reading.id}` : `${c.name} · ${c.reading.id}` }))} onChange={(v) => { set(v); setConfirmed(false); }} />
          ))}
          <Select label="Temperature source" placeholder="Temperature source" value={temperature} options={(sensors ?? []).filter((s) => s.provider === "mainboard.lhm" && s.sensor.unit === "°C").map((s) => ({ value: s.sensor.id, label: `${s.sensor.name} · ${s.sensor.id}` }))} onChange={(v) => { setTemperature(v); setConfirmed(false); }} />
          <Select label="Saved curve" placeholder="Saved curve" value={curveName} options={Object.keys(curves ?? {}).map((name) => ({ value: name, label: name }))} onChange={setCurveName} />
        </div>
        {selected && <SupportBadge tier={tier} />}
        {refusal && <Banner tone="warn">{refusal}</Banner>}
        {selected && <p className="text-caption text-text-subtle">Channel test: {evidence?.test ?? "none"}{evidence?.tested ? " · " + evidence.tested.map((t) => `${t.duty_percent} % → ${Math.round(t.rpm)} RPM`).join(", ") : ""}. The test runs 100 → 70 → 50 % (never below 40 %) and returns the fan to firmware control; a new BIOS or board invalidates it.</p>}
        {tier === "lhm_supported" && <p className="text-caption text-text-subtle">LHM-supported channels run reduced-speed curves only after a passed channel test; calibration (100 → 40 %) is available.</p>}
        <Checkbox checked={confirmed} onChange={setConfirmed}>I confirm this is a normal fan, not a pump, and the selected RPM belongs to it.</Checkbox>
        <p className="text-caption text-text-subtle">Live curves require a minimum of 40% and a critical temperature at most 100°C. Channel selection alone does not establish a physical header or PWM/DC mode.</p>
        <div className="flex flex-wrap gap-2">
          <Button disabled={!ready || !curve || curve.minimum_percent < 40 || curve.critical_c > 100} onClick={() => start("curve")}>Start saved curve (UAC)</Button>
          <Button variant="secondary" disabled={!ready || (curve?.critical_c ?? 80) > 100} onClick={() => start("calibration")}>Calibrate fan (UAC)</Button>
          <Button variant="secondary" disabled={!ready || (curve?.critical_c ?? 80) > 100} onClick={() => start("channel_test")}>Run channel test (UAC)</Button>
        </div>
      </>}
      <div className="flex items-center gap-3 text-small">
        <span>Session: {status?.state ?? "unavailable"}{status?.job_id != null ? ` · ${status.job_id}` : ""}{status?.tier ? ` · ${tierLabel(status.tier)}` : ""}</span>
        <Button variant="secondary" disabled={!connected || !busy || status?.job_id == null || pending !== null} onClick={() => run("stop", () => rpc("cooling.sessions.stop", { job_id: status?.job_id }))}>Stop and return to firmware</Button>
      </div>
      {(error || status?.error) && <p role="alert" className="text-small text-danger-text">{error || status?.error}</p>}
      {status?.report != null && <details className="text-small"><summary>Last execution / calibration result</summary><pre className="overflow-auto whitespace-pre-wrap text-caption">{JSON.stringify(status.report, null, 2)}</pre></details>}
    </Section>
  );
}
