import { Section } from "@/components/page";
import { useDaemon } from "@/lib/store/daemon";
import { num } from "@/lib/format";

export function ProviderSensors() {
  const providers = useDaemon((s) => s.providers);
  const sensors = useDaemon((s) => s.monitoring_sensors);
  return <Section title="Motherboard sensors">
    {providers == null && <p className="text-small text-text-subtle">Provider status unavailable.</p>}
    {providers?.map((p) => <p key={p.id} className="text-small text-text-muted">{p.name}: {p.status}{p.message ? ` · ${p.message}` : ""}</p>)}
    <div className="grid gap-2 md:grid-cols-3">
      {sensors?.filter((s) => s.provider !== "device").map((s) => <div key={`${s.provider}:${s.device_id}:${s.sensor.id}`} className="rounded-control bg-surface-sunken p-3 text-small">
        <p className="text-text-subtle">{s.device_name} · {s.sensor.name}</p>
        <p>{s.sensor.value == null || s.sensor.age_ms > 5000 ? "unavailable" : `${num(s.sensor.value, 1)} ${s.sensor.unit}`} · {num(s.sensor.age_ms / 1000, 1)} s</p>
      </div>)}
    </div>
  </Section>;
}
