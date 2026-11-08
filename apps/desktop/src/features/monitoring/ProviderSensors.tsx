import { Section } from "@/components/page";
import { useDaemon } from "@/lib/store/daemon";
import { num } from "@/lib/format";
import { t } from "./messages";

export function ProviderSensors() {
  const providers = useDaemon((s) => s.providers);
  const sensors = useDaemon((s) => s.monitoring_sensors);
  const loading = useDaemon((s) => s.loading);
  const boardSensors = sensors?.filter((s) => s.provider !== "device");
  return <Section title={t("monitoring.board")}>
    {providers == null && <p role="status" className="text-small text-text-subtle">{t(loading ? "monitoring.loading" : "monitoring.board.noStatus")}</p>}
    <div role="list" aria-label={t("monitoring.board.providers")} className="contents">
    {providers?.map((p) => <p role="listitem" key={p.id} className="text-small text-text-muted">{p.name}: {p.status}{p.message ? ` · ${p.message}` : ""}</p>)}
    </div>
    {boardSensors == null ? <p role="status" className="text-small text-text-subtle">{t(loading ? "monitoring.loading" : "monitoring.board.unavailable")}</p> : boardSensors.length === 0 ? <p role="status" className="text-small text-text-subtle">{t("monitoring.board.empty")}</p> :
    <div role="list" aria-label={t("monitoring.board")} className="grid gap-2 md:grid-cols-3">
      {boardSensors.map((s) => <div role="listitem" key={`${s.provider}:${s.device_id}:${s.sensor.id}`} className="rounded-control bg-surface-sunken p-3 text-small">
        <p className="text-text-subtle">{s.device_name} · {s.sensor.name}</p>
        <p>{s.sensor.value == null || s.sensor.age_ms > 5000 ? t("monitoring.unavailable") : `${num(s.sensor.value, 1)} ${s.sensor.unit}`} · {num(s.sensor.age_ms / 1000, 1)} s</p>
      </div>)}
    </div>}
  </Section>;
}
