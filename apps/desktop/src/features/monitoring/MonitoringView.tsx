import { Copy, FileDown } from "lucide-react";
import { useState, type ReactNode } from "react";
import { PageHeader, Section } from "@/components/page";
import { Button } from "@/components/ui/button";
import { transport } from "@/lib/api/transport";
import { duration, num } from "@/lib/format";
import { useDaemon, useSlices } from "@/lib/store/daemon";
import { useAction } from "@/lib/store/useAction";
import { ProviderSensors } from "./ProviderSensors";

/** Readings older than this are shown as stale (same limit the daemon uses). */
const STALE_MS = 3000;

function Stat({ label, children }: { label: string; children: ReactNode }) {
  return (
    <div className="flex flex-col gap-1 rounded-control bg-surface-sunken px-4 py-3">
      <span className="text-caption text-text-subtle">{label}</span>
      <span className="text-title font-medium tabular">{children}</span>
    </div>
  );
}

function ServiceStats() {
  const d = useDaemon((s) => s.diagnostics);
  const v = useDaemon((s) => s.connection);
  if (!d) return null;
  return (
    <Section title="Service">
      <div className="grid grid-cols-[repeat(auto-fill,minmax(160px,1fr))] gap-2">
        <Stat label="Frame rate">
          {num(d.measured_fps, 1)} <span className="text-small text-text-subtle">/ {d.target_fps} fps</span>
        </Stat>
        <Stat label="Running for">{duration(d.uptime_s)}</Stat>
        <Stat label="Devices">{d.devices}</Stat>
        <Stat label="Version">
          {v?.daemon_version ?? "—"} <span className="text-small text-text-subtle">API {v?.api_version ?? "—"}</span>
        </Stat>
      </div>
      {(d.audio || d.screen) && (
        <ul className="flex flex-col gap-1 text-small text-text-muted">
          {d.audio && <li>Audio: {d.audio}</li>}
          {d.screen && <li>Screen: {d.screen}</li>}
        </ul>
      )}
    </Section>
  );
}

function DeviceSensors() {
  const devices = useDaemon((s) => s.devices);
  const withSensors = devices?.filter((d) => d.sensors.length > 0) ?? [];
  return (
    <Section title="Device sensors">
      {withSensors.length === 0 ? (
        <p className="text-small text-text-subtle">No connected device reports sensor values.</p>
      ) : (
        <div className="overflow-hidden rounded-card bg-surface-sunken">
          <table className="w-full text-small">
            <thead className="text-left text-caption text-text-subtle">
              <tr>
                <th className="px-4 py-2 font-normal">Device</th>
                <th className="px-4 py-2 font-normal">Sensor</th>
                <th className="px-4 py-2 text-right font-normal">Value</th>
                <th className="px-4 py-2 text-right font-normal">Age</th>
              </tr>
            </thead>
            <tbody className="divide-y divide-border">
              {withSensors.flatMap((d) =>
                d.sensors.map((s) => {
                  const stale = s.age_ms > STALE_MS;
                  return (
                    <tr key={`${d.info.id}/${s.id}`}>
                      <td className="px-4 py-2 text-text-muted">{d.info.name}</td>
                      <td className="px-4 py-2">{s.name}</td>
                      <td className="px-4 py-2 text-right tabular">
                        {s.value === null || stale ? <span className="text-[#fbbf24]">{stale ? "stale" : "—"}</span> : `${num(s.value, 1)} ${s.unit}`}
                      </td>
                      <td className="px-4 py-2 text-right text-text-subtle tabular">{num(s.age_ms / 1000, 1)} s</td>
                    </tr>
                  );
                }),
              )}
            </tbody>
          </table>
        </div>
      )}
    </Section>
  );
}

function CustomSources() {
  const values = useDaemon((s) => s.sensor_values?.values);
  const entries = Object.entries(values ?? {});
  if (entries.length === 0) return null;
  return (
    <Section title="Custom temperature sources">
      <div className="grid grid-cols-[repeat(auto-fill,minmax(180px,1fr))] gap-2">
        {entries.map(([name, r]) => (
          <Stat key={name} label={name}>
            {r.value === null ? <span className="text-small text-[#fbbf24]" title={r.error ?? undefined}>unavailable</span> : `${num(r.value, 1)} °C`}
          </Stat>
        ))}
      </div>
    </Section>
  );
}

function DiagnosticsExport() {
  const [path, setPath] = useState<string | null>(null);
  const { pending, error, run } = useAction();
  return (
    <Section title="Diagnostics report">
      <div className="flex flex-col gap-3 rounded-card border border-border px-4 py-3">
        <p className="text-small text-text-muted">
          Saves an offline report for bug reports. It leaves out device IDs and serial numbers, paths, names, raw error
          texts and your macro and profile contents. Nothing is uploaded.
        </p>
        <div className="flex flex-wrap items-center gap-3">
          <Button
            variant="secondary"
            disabled={pending !== null}
            onClick={() =>
              run("export", async () => setPath(await (await transport()).exportDiagnostics()))
            }
          >
            <FileDown /> {pending ? "Saving…" : "Save report"}
          </Button>
          {path && (
            <>
              <code className="min-w-0 flex-1 truncate font-mono text-caption text-text-muted" title={path}>
                {path}
              </code>
              <Button size="sm" variant="ghost" onClick={() => void navigator.clipboard?.writeText(path)}>
                <Copy /> Copy path
              </Button>
            </>
          )}
        </div>
        {error && <p className="text-caption text-danger-text">{error}</p>}
      </div>
    </Section>
  );
}

export default function MonitoringView() {
  useSlices("sensor_values", "monitoring_sensors", "providers");
  return (
    <>
      <PageHeader title="Monitoring" description="Live service status and sensor readings." />
      <ServiceStats />
      <DeviceSensors />
      <ProviderSensors />
      <CustomSources />
      <DiagnosticsExport />
      <p className="text-caption text-text-subtle">
        Sensor history graphs and warning rules still run in freeRGB Classic; they move here in M5.
      </p>
    </>
  );
}
