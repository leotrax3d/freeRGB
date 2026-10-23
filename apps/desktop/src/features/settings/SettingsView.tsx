import { Compass, Download, Puzzle } from "lucide-react";
import { useTour } from "@/app/Tour";
import { PageHeader, Section } from "@/components/page";
import { StatusDot } from "@/components/status";
import { Button } from "@/components/ui/button";
import { deviceStatus } from "@/lib/api/types";
import { num } from "@/lib/format";
import { rpc, useDaemon, useSlices } from "@/lib/store/daemon";
import { useAction } from "@/lib/store/useAction";
import { KeyboardReactions } from "@/features/devices/KeyboardReactions";
import { transport } from "@/lib/api/transport";

function Plugins() {
  const plugins = useDaemon((s) => s.plugins);
  const { pending, error, run } = useAction();
  return (
    <Section title="Plugins" aside={error && <span className="text-caption text-danger-text">{error}</span>}>
      {!plugins?.length ? (
        <p className="text-small text-text-subtle">
          No plugins are configured. Plugins are enabled when the service starts (<code className="font-mono text-caption">--plugin</code>);
          see docs/PLUGIN_SYSTEM.md.
        </p>
      ) : (
        <ul className="flex flex-col divide-y divide-border overflow-hidden rounded-card bg-surface-sunken">
          {plugins.map((p) => {
            const state = p.status ? deviceStatus(p.status) : null;
            return (
              <li key={p.id} className="flex items-center gap-4 px-4 py-3">
                <Puzzle className="size-4 text-text-subtle" aria-hidden />
                <div className="flex min-w-0 flex-1 flex-col">
                  <span className="truncate text-body">{p.name}</span>
                  <span className="text-caption text-text-subtle">
                    {p.id} · {p.access === "hardware" ? "Hardware access" : "Virtual"} · {num(p.frames)} frames
                  </span>
                  {p.last_error && <span className="text-caption text-danger-text">{p.last_error}</span>}
                </div>
                <StatusDot
                  tone={state === "connected" ? "ok" : state === "faulted" ? "error" : "idle"}
                  label={p.enabled ? (state === "connected" ? "Running" : state === "faulted" ? "Error" : "Starting…") : "Stopped"}
                />
                <Button
                  size="sm"
                  variant="secondary"
                  disabled={!p.allowed || pending !== null}
                  title={p.allowed ? undefined : "Not allowed by the service configuration"}
                  onClick={() => run(p.id, () => rpc(p.enabled ? "plugins.stop" : "plugins.start", { id: p.id }))}
                >
                  {p.enabled ? "Stop" : "Start"}
                </Button>
              </li>
            );
          })}
        </ul>
      )}
    </Section>
  );
}

export default function SettingsView() {
  useSlices("plugins");
  const connection = useDaemon((s) => s.connection);
  const { pending, error, run } = useAction();
  return (
    <>
      <PageHeader title="Settings" />
      <Plugins />
      <KeyboardReactions />
      <Section title="Classic tools">
        <p className="text-small text-text-muted">Open Classic for application profiles, global shortcuts, sensor history, warning rules and updates. Both interfaces use the same background service.</p>
        <Button variant="secondary" disabled={pending !== null || useDaemon.getState().mode !== "tauri"} onClick={() => run("classic", async () => { const t = await transport(); if (!t.openClassic) throw new Error("Classic is unavailable in browser preview"); await t.openClassic(); })}>Open freeRGB Classic</Button>
        {error && <p role="alert" className="text-small text-danger-text">{error}</p>}
      </Section>
      <Section title="Updates">
        <div className="flex items-start gap-3 rounded-card border border-border px-4 py-3 text-small text-text-muted">
          <Download className="mt-0.5 size-4 shrink-0 text-text-subtle" aria-hidden />
          <span>Update checks and installation still run in freeRGB Classic. They move here with the shared desktop logic (M5).</span>
        </div>
      </Section>
      <SetupSettings />
      <Section title="Help">
        <div className="flex items-center justify-between gap-4 rounded-card border border-border px-4 py-3">
          <span className="text-small text-text-muted">A short walk through the most important parts of freeRGB.</span>
          <Button variant="secondary" size="sm" onClick={() => useTour.getState().start()}>
            <Compass /> Start tour
          </Button>
        </div>
      </Section>
      <Section title="About">
        <dl className="grid grid-cols-[max-content_1fr] gap-x-6 gap-y-1.5 text-small">
          <dt className="text-text-subtle">App</dt>
          <dd>freeRGB desktop {__APP_VERSION__}</dd>
          <dt className="text-text-subtle">Service</dt>
          <dd>{connection?.daemon_version ? `freergbd ${connection.daemon_version} · API ${connection.api_version}` : "not connected"}</dd>
          <dt className="text-text-subtle">Classic</dt>
          <dd className="text-text-muted">freeRGB Classic stays available for low-resource PCs; both apps control the same service and share all settings.</dd>
        </dl>
      </Section>
    </>
  );
}

/** Re-run the first-run wizard; show or withdraw the fan-control safety acknowledgement. */
function SetupSettings() {
  const onboarding = useDaemon((s) => s.onboarding);
  const { pending, error, run } = useAction();
  return (
    <Section title="Setup">
      <div className="flex flex-wrap items-center justify-between gap-4 rounded-card border border-border px-4 py-3">
        <span className="text-small text-text-muted">Run the setup again: safety notice, device detection and extension downloads, motherboard fan identification.</span>
        <Button variant="secondary" size="sm" disabled={pending !== null} onClick={() => run("reset", () => rpc("onboarding.reset"))}>Run setup wizard</Button>
      </div>
      <div className="flex flex-wrap items-center justify-between gap-4 rounded-card border border-border px-4 py-3">
        <span className="text-small text-text-muted">{onboarding?.safety_acknowledged ? "Fan-control safety notice accepted. Withdrawing it locks all fan and pump control again (lighting is unaffected)." : "Fan control is locked until the safety notice is accepted in the setup wizard."}</span>
        {onboarding?.safety_acknowledged && <Button variant="danger" size="sm" disabled={pending !== null} onClick={() => run("revoke", () => rpc("safety.revoke"))}>Withdraw acceptance</Button>}
      </div>
      {error && <p role="alert" className="text-small text-danger-text">{error}</p>}
    </Section>
  );
}
