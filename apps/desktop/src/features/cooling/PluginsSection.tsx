import { t } from "./messages";
import { useEffect, useState } from "react";
import { Download, Puzzle } from "lucide-react";
import { Section } from "@/components/page";
import { Button } from "@/components/ui/button";
import { Checkbox } from "@/components/ui/checkbox";
import { Select } from "@/components/ui/select";
import { rpc } from "@/lib/store/daemon";
import { useAction } from "@/lib/store/useAction";
import { num } from "@/lib/format";
import { dutyChoices, failsafeNote, pluginState, sourceBadge, STATE_LABELS, type PluginsView } from "./pluginModel";

function usePlugins(ms = 2000): [PluginsView | null, () => void] {
  const [view, setView] = useState<PluginsView | null>(null);
  const load = () => void rpc<PluginsView>("cooling.plugins.list").then(setView, () => setView(null));
  useEffect(() => {
    load();
    const timer = setInterval(load, ms);
    return () => clearInterval(timer);
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [ms]);
  return [view, load];
}

/** Module manager: install, enable and disable cooling plugins. */
export function CoolingPluginsManager() {
  const [view, reload] = usePlugins(3000);
  const { pending, error, run } = useAction();
  if (!view || view.plugins.length === 0) return null;
  return (
    <Section title={t("cooling.pluginsSection.coolingPlugins")}>
      <ul className="flex flex-col divide-y divide-border rounded-card border border-border">
        {view.plugins.map((p) => {
          const state = pluginState(p);
          const note = failsafeNote(p.status);
          return (
            <li key={p.id} className="flex flex-col gap-1 px-4 py-3 text-small">
              <div className="flex flex-wrap items-center gap-3">
                <Puzzle className="size-4 text-text-subtle" aria-hidden />
                <span className="font-medium text-text">{p.name ?? p.id}</span>
                <span className="text-text-subtle tabular">v{p.version}</span>
                <span className="text-text-subtle">{p.license}</span>
                {p.capability && <span className="text-text-subtle">{p.capability === "both" ? t("cooling.pluginsSection.sensorsControl") : p.capability}</span>}
                {p.virtual && <span className="text-text-subtle">{t("cooling.pluginsSection.virtual")}</span>}
                <span className="flex-1" />
                <span className="text-caption text-text-muted">{STATE_LABELS[state]}</span>
                {p.installed ? (
                  <Checkbox
                    checked={p.enabled}
                    disabled={pending !== null}
                    onChange={(enabled) =>
                      void run(p.id, async () => {
                        await rpc("cooling.plugins.enable", { id: p.id, enabled });
                        reload();
                      })
                    }
                  >
                    {t("cooling.pluginsSection.enabled")} </Checkbox>
                ) : (
                  <Button size="sm" variant="secondary" disabled={pending !== null} onClick={() => void run(p.id, () => rpc("cooling.plugins.install", { id: p.id }), t("cooling.pluginsSection.installingCoolingPlugin"))}>
                    <Download /> {t("cooling.pluginsSection.install")} </Button>
                )}
              </div>
              {(p.error || p.install?.startsWith("install failed")) && <span className="text-caption text-danger-text">{p.error ?? p.install}</span>}
              {note && <span className="text-caption text-text-subtle">{note}</span>}
            </li>
          );
        })}
      </ul>
      <p className="text-caption text-text-subtle">
        {t("cooling.pluginsSection.pluginsAreSignedPackagesThatRunAs")} </p>
      {error && <p className="text-caption text-danger-text">{error}</p>}
    </Section>
  );
}

/** Plugin channels and sensors, shown like native ones with a source badge. */
export function PluginChannelsSection() {
  const [view, reload] = usePlugins();
  const [outputs, setOutputs] = useState<string[]>([]);
  const [drafts, setDrafts] = useState<Record<string, string>>({});
  const { pending, error, run } = useAction();
  useEffect(() => {
    rpc<{ nodes: Record<string, unknown> }>("cooling.curve_set.get").then((s) => setOutputs(Object.keys(s.nodes)), () => setOutputs([]));
  }, []);
  const running = (view?.plugins ?? []).filter((p) => p.status?.running);
  if (running.length === 0) return null;
  return (
    <Section title={t("cooling.pluginsSection.pluginFansAndSensors")}>
      <p className="text-small text-text-muted">
        {t("cooling.pluginsSection.fixedDutyOrACompositeCurvePumps")} </p>
      {running.map((p) => (
        <div key={p.id} className="flex flex-col gap-2">
          <h3 className="text-caption font-medium text-text-subtle">{sourceBadge(p)}</h3>
          <ul className="flex flex-col divide-y divide-border overflow-hidden rounded-card bg-surface-sunken">
            {(p.channels ?? []).map((c) => {
              const key = `${p.id}/${c.channel.id}`;
              const a = c.assignment;
              const current = a ? (a.mode === "manual" ? `manual:${a.percent}` : `composite:${a.output}`) : "auto";
              const choice = drafts[key] ?? current;
              const options = [
                { value: "auto", label: t("cooling.pluginsSection.firmwareControl") },
                ...dutyChoices(c.channel).map((d) => ({ value: `manual:${d}`, label: t("cooling.pluginsSection.fixedValue", { p0: d }) })),
                ...(c.channel.role === "fan" ? outputs.map((o) => ({ value: `composite:${o}`, label: t("cooling.pluginsSection.curveValue", { p0: o }) })) : []),
              ];
              const tach = p.sensors?.find((s) => s.id === c.channel.tachometer);
              const apply = () =>
                run(key, async () => {
                  if (choice === "auto") await rpc("cooling.plugins.release", { plugin: p.id, channel: c.channel.id });
                  else {
                    const [mode, arg] = [choice.slice(0, choice.indexOf(":")), choice.slice(choice.indexOf(":") + 1)];
                    await rpc("cooling.plugins.set", mode === "manual" ? { plugin: p.id, channel: c.channel.id, mode, percent: Number(arg) } : { plugin: p.id, channel: c.channel.id, mode, output: arg });
                  }
                  reload();
                }, t("cooling.pluginsSection.pluginChannelUpdated"));
              return (
                <li key={key} className="flex flex-wrap items-center gap-4 px-4 py-3">
                  <div className="flex min-w-0 flex-1 flex-col">
                    <span className="truncate text-small">
                      {c.channel.name} <span className="text-caption text-text-subtle">· {c.channel.role}</span>
                    </span>
                    <span className="text-caption text-text-subtle">
                      {tach?.value != null ? `${num(tach.value, 0)} RPM` : t("cooling.pluginsSection.speedUnavailable")}
                      {c.applied_percent != null && t("cooling.pluginsSection.valueApplied", { p0: c.applied_percent })} {t("cooling.pluginsSection.floor")} {c.channel.floor_percent}%
                    </span>
                    {c.fault && <span className="text-caption text-danger-text">{c.fault}</span>}
                  </div>
                  <Select label={t("cooling.pluginsSection.mode")} value={choice} onChange={(v) => setDrafts({ ...drafts, [key]: v })} options={options} />
                  <Button size="sm" variant="secondary" disabled={choice === current || pending !== null} onClick={apply}>
                    {t("cooling.pluginsSection.apply")} </Button>
                </li>
              );
            })}
            {(p.sensors ?? [])
              .filter((s) => !(p.channels ?? []).some((c) => c.channel.tachometer === s.id))
              .map((s) => (
                <li key={s.id} className="flex items-center gap-4 px-4 py-2">
                  <span className="min-w-0 flex-1 truncate text-small">{s.name}</span>
                  <span className="w-24 text-right text-small tabular">
                    {s.value != null ? `${num(s.value, s.unit === "celsius" ? 1 : 0)} ${s.unit === "celsius" ? "°C" : s.unit === "rpm" ? "RPM" : "%"}` : <span className="text-[#fbbf24]">{t("cooling.pluginsSection.unavailable")}</span>}
                  </span>
                </li>
              ))}
          </ul>
        </div>
      ))}
      {error && <p className="text-caption text-danger-text">{error}</p>}
    </Section>
  );
}
