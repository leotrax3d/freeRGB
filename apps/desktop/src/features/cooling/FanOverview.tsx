import { useMemo, useState } from "react";
import { OctagonX, Pause, Play, Save } from "lucide-react";
import { Banner } from "@/components/status";
import { Button } from "@/components/ui/button";
import { Select } from "@/components/ui/select";
import { rpc, useDaemon, useSlices } from "@/lib/store/daemon";
import { useAction } from "@/lib/store/useAction";
import { FanCard } from "./FanCard";
import { fanViews } from "./fanModel";
import { activeCount } from "./liveModel";

/** Profile switcher, save-as and the fan pause, shown in the page header. */
export function CoolingActions() {
  useSlices("cooling_profiles", "cooling_background");
  const profiles = useDaemon((s) => s.cooling_profiles);
  const background = useDaemon((s) => s.cooling_background);
  const { pending, error, run } = useAction();
  const [saving, setSaving] = useState(false);
  const [name, setName] = useState("");
  const paused = background?.service?.paused === true;
  const applying = background?.apply.state === "applying";
  useSlices("cooling_session");
  const live = activeCount(useDaemon((s) => s.cooling_session));
  return (
    <div className="flex flex-col items-end gap-1">
      <div className="flex flex-wrap items-end gap-2">
        {(profiles?.profiles.length ?? 0) > 0 && (
          <div className="w-44">
            <Select label="Profile" placeholder="Choose profile" value={profiles?.active ?? ""}
              options={(profiles?.profiles ?? []).map((p) => ({ value: p.name, label: p.name }))}
              onChange={(v) => run("activate", () => rpc("cooling.profiles.activate", { name: v }))} />
          </div>
        )}
        {saving ? (
          <form className="flex items-end gap-2" onSubmit={(e) => { e.preventDefault(); const n = name.trim(); if (n) run("save", async () => { await rpc("cooling.profiles.save", { name: n }); setSaving(false); setName(""); }); }}>
            <input autoFocus maxLength={64} value={name} onChange={(e) => setName(e.target.value)} placeholder="Profile name" aria-label="Profile name"
              className="h-9 w-40 rounded-control border border-border bg-surface-sunken px-3 text-small" />
            <Button type="submit" size="sm" variant="primary" disabled={!name.trim() || pending !== null}>Save</Button>
            <Button type="button" size="sm" variant="ghost" onClick={() => setSaving(false)}>Cancel</Button>
          </form>
        ) : (
          <Button size="sm" variant="secondary" disabled={(background?.saved.channels.length ?? 0) === 0} onClick={() => setSaving(true)}><Save />Save as profile</Button>
        )}
        {live > 0 && (
          <Button size="sm" variant="danger" disabled={pending !== null}
            onClick={() => run("stop_all", () => rpc("cooling.sessions.stop_all"))}><OctagonX />Stop all live fans ({live})</Button>
        )}
        <Button size="sm" variant={paused ? "primary" : "secondary"} disabled={pending !== null || applying || !background?.available}
          onClick={() => run("pause", () => rpc("cooling.fans.pause", { paused: !paused }))}>
          {paused ? <Play /> : <Pause />}{paused ? "Resume fans" : "Pause fans"}
        </Button>
      </div>
      {error && <p role="alert" className="text-caption text-danger-text">{error}</p>}
    </div>
  );
}

/** One card per motherboard fan header. */
export function FanOverview() {
  useSlices("cooling_channels", "monitoring_sensors", "cooling_bindings", "cooling_background", "cooling_session", "cooling_support", "providers", "curves");
  const channels = useDaemon((s) => s.cooling_channels);
  const sensors = useDaemon((s) => s.monitoring_sensors);
  const bindings = useDaemon((s) => s.cooling_bindings);
  const background = useDaemon((s) => s.cooling_background);
  const session = useDaemon((s) => s.cooling_session);
  const support = useDaemon((s) => s.cooling_support);
  const providers = useDaemon((s) => s.providers);
  const onboarding = useDaemon((s) => s.onboarding);
  const curves = useDaemon((s) => s.curves);
  const views = useMemo(() => fanViews({
    channels, sensors, bindings: bindings?.config.bindings, saved: background?.saved.channels, service: background?.service?.channels,
    servicePaused: background?.service?.paused === true, sessions: session?.jobs ?? (session ? [session] : []), evidence: support?.channels, curves,
  }), [channels, sensors, bindings, background, session, support, curves]);
  const provider = providers?.find((p) => p.id === "mainboard.lhm");
  const applying = background?.apply.state === "applying";
  return (
    <section className="flex flex-col gap-3" aria-label="Fans">
      {onboarding && !onboarding.safety_acknowledged && (
        <Banner tone="warn">
          <span className="flex-1">Fan control is locked until you accept the safety notice. freeRGB only shows the readings.</span>
          <Button size="sm" onClick={() => void rpc("onboarding.reset")}>Open setup</Button>
        </Banner>
      )}
      {applying && <Banner tone="info">Applying your fan settings — confirm the Windows prompt.</Banner>}
      {background?.apply.state === "failed" && background.apply.error && <Banner tone="error">{background.apply.error}</Banner>}
      {views.length === 0 ? (
        <p className="rounded-card border border-border px-4 py-6 text-center text-small text-text-muted">
          {provider && provider.status !== "disabled"
            ? provider.status === "ready" ? "No controllable motherboard fan headers were found." : `Motherboard service: ${provider.message}`
            : "Motherboard fan control is not installed. Run freeRGB Setup again to add it."}
        </p>
      ) : (
        <div className="grid grid-cols-[repeat(auto-fill,minmax(320px,1fr))] gap-3">
          {views.map((v, i) => <FanCard key={v.id} view={v} index={i} />)}
        </div>
      )}
      <p className="text-caption text-text-subtle">Changes are saved in the protected service and keep running from Windows startup, also with this window closed. Windows asks for permission when you apply.</p>
    </section>
  );
}
