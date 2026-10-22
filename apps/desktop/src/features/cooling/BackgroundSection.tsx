import { useState } from "react";
import { Section } from "@/components/page";
import { Button } from "@/components/ui/button";
import { Select } from "@/components/ui/select";
import { Checkbox } from "@/components/ui/checkbox";
import { Banner, StatusDot } from "@/components/status";
import { rpc, useDaemon, useSlices } from "@/lib/store/daemon";
import { useAction } from "@/lib/store/useAction";
import type { BackgroundChannel } from "@/lib/api/types";
import { channelTier, controlRefusal, tierLabel } from "./support";

/** Saved curves the protected service runs from boot, without this window. */
export function BackgroundSection() {
  useSlices("cooling_background", "cooling_channels", "monitoring_sensors");
  const background = useDaemon((s) => s.cooling_background);
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
  const saved = background?.saved.channels ?? [];
  const applying = background?.apply.state === "applying";
  const selected = channels?.find((c) => c.kind === "control" && c.reading.id === control);
  const refusal = controlRefusal(selected?.support);
  const curve = curves?.[curveName];
  const apply = (next: BackgroundChannel[]) => run("background", () => rpc("cooling.background.apply", { channels: next }));
  const canAdd = connected && background?.available && !applying && pending === null && confirmed && !refusal && curve
    && control && rpm && temperature && !saved.some((c) => c.control_id === control) && saved.length < 8;
  const nameOf = (id: string) => channels?.find((c) => c.reading.id === id)?.name ?? id;
  const live = (id: string) => background?.service?.channels.find((c) => c.control_id === id);
  return (
    <Section title="Background control">
      <p className="text-small text-text-muted">Saved curves run in the protected service from Windows startup, also when this window and the daemon are closed. Saving asks for Windows authorization. Each channel returns to firmware control on stop, crash, lost telemetry or watchdog expiry.</p>
      {!background?.available ? <p className="text-small text-text-subtle">Background control needs the optional motherboard provider and the current protected service.</p> : <>
        {saved.length === 0 ? <p className="text-small text-text-subtle">No background channels saved.</p> : (
          <ul className="flex flex-col gap-2">
            {saved.map((c) => {
              const status = live(c.control_id);
              return (
                <li key={c.control_id} className="flex flex-wrap items-center gap-3 text-small">
                  <span className="font-medium">{nameOf(c.control_id)}</span>
                  <StatusDot tone={status?.fault ? "error" : status?.state === "running" ? "ok" : "idle"} label={status ? `${status.state} · ${status.duty_percent} % · ${tierLabel(status.tier)}` : "not reported by the service"} />
                  {status?.fault && <span className="text-danger-text">{status.fault}</span>}
                  {status?.warning && <span className="text-[#fbbf24]">{status.warning}</span>}
                  <Button variant="secondary" disabled={!connected || applying || pending !== null} onClick={() => apply(saved.filter((x) => x.control_id !== c.control_id))}>Remove (UAC)</Button>
                </li>
              );
            })}
          </ul>
        )}
        <div className="grid gap-3 md:grid-cols-2">
          {([["Fan output", "control", control, setControl], ["Matching fan RPM", "fan", rpm, setRpm]] as const).map(([label, kind, value, set]) => (
            <Select key={kind} label={label} placeholder={label} value={value} options={(channels ?? []).filter((c) => c.kind === kind).map((c) => ({ value: c.reading.id, label: kind === "control" ? `${c.name} · ${tierLabel(channelTier(c))}` : c.name }))} onChange={(v) => { set(v); setConfirmed(false); }} />
          ))}
          <Select label="Temperature source" placeholder="Temperature source" value={temperature} options={(sensors ?? []).filter((s) => s.provider === "mainboard.lhm" && s.sensor.unit === "°C").map((s) => ({ value: s.sensor.id, label: s.sensor.name }))} onChange={(v) => { setTemperature(v); setConfirmed(false); }} />
          <Select label="Saved curve" placeholder="Saved curve" value={curveName} options={Object.keys(curves ?? {}).map((name) => ({ value: name, label: name }))} onChange={setCurveName} />
        </div>
        {refusal && <Banner tone="warn">{refusal}</Banner>}
        <Checkbox checked={confirmed} onChange={setConfirmed}>I confirm this is a normal fan, not a pump, and the selected RPM belongs to it.</Checkbox>
        <div className="flex flex-wrap gap-2">
          <Button disabled={!canAdd} onClick={() => curve && apply([...saved, { control_id: control, temperature_id: temperature, rpm_id: rpm, curve, confirmed_fan: true }])}>Add to background (UAC)</Button>
          <Button variant="secondary" disabled={!connected || applying || pending !== null || saved.length === 0} onClick={() => apply([])}>Disable all (UAC)</Button>
        </div>
        <p className="text-caption text-text-subtle">Apply: {background.apply.state}{background.service?.error ? ` · service: ${background.service.error}` : ""}</p>
      </>}
      {(error || background?.apply.error) && <p role="alert" className="text-small text-danger-text">{error || background?.apply.error}</p>}
    </Section>
  );
}
