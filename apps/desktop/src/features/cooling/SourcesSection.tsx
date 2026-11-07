import { t } from "./messages";
import { useEffect, useState } from "react";
import { Plus, Trash2 } from "lucide-react";
import { Section } from "@/components/page";
import { Banner } from "@/components/status";
import { Button } from "@/components/ui/button";
import { Select } from "@/components/ui/select";
import { rpc } from "@/lib/store/daemon";
import { useAction } from "@/lib/store/useAction";
import { num } from "@/lib/format";
import { pluginOf } from "./pluginModel";
import { tierLabel } from "./support";
import { CATEGORY_LABELS, fileSensorProblem, groupSources, type CatalogEntry, type FileSensorSet, type GpuView } from "./sourceModel";

const field =
  "h-8 rounded-chip border border-border-strong bg-surface px-2 text-small text-text outline-none focus-visible:border-info";

function usePoll<T>(method: string, ms = 2000): [T | null, () => void] {
  const [value, setValue] = useState<T | null>(null);
  const load = () => void rpc<T>(method).then(setValue, () => setValue(null));
  useEffect(() => {
    load();
    const timer = setInterval(load, ms);
    return () => clearInterval(timer);
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [method, ms]);
  return [value, load];
}

/** All Celsius sources by category; every one is selectable as a curve input. */
export function SourcesSection() {
  const [catalog] = usePoll<CatalogEntry[]>("cooling.sensors.catalog");
  if (!catalog) return null;
  return (
    <Section title={t("cooling.sourcesSection.temperatureSources")}>
      <p className="text-small text-text-muted">
        {t("cooling.sourcesSection.everySourceBelowCanDriveACustom")} </p>
      {catalog.length === 0 && <p className="text-caption text-text-subtle">{t("cooling.sourcesSection.noTemperatureSourceIsReportingYet")}</p>}
      {groupSources(catalog).map((group) => (
        <div key={group.category} className="flex flex-col gap-1">
          <h3 className="text-caption font-medium text-text-subtle">{CATEGORY_LABELS[group.category]}</h3>
          <ul className="flex flex-col divide-y divide-border overflow-hidden rounded-card bg-surface-sunken">
            {group.entries.map((e) => (
              <li key={e.id} className="flex items-center gap-4 px-4 py-2" title={e.id}>
                <span className="min-w-0 flex-1 truncate text-small">{e.label}</span>
                <span className="text-caption text-text-subtle">{e.provider === "plugin" ? `Plugin · ${pluginOf(e.id) ?? e.label}` : e.provider}</span>
                <span className="w-24 text-right text-small tabular" title={e.error ?? undefined}>
                  {e.value != null ? `${num(e.value, 1)} °C` : <span className="text-[#fbbf24]">{t("cooling.sourcesSection.unavailable")}</span>}
                </span>
              </li>
            ))}
          </ul>
        </div>
      ))}
      <FileSensors />
    </Section>
  );
}

function FileSensors() {
  const [set, reload] = usePoll<FileSensorSet>("cooling.file_sensors.get", 10000);
  const [name, setName] = useState("");
  const [path, setPath] = useState("");
  const { pending, error, run } = useAction();
  if (!set) return null;
  const names = Object.keys(set.sensors);
  const problem = name || path ? fileSensorProblem(name, path, names) : null;
  const save = (sensors: FileSensorSet["sensors"], message: string) =>
    run("save", async () => {
      await rpc("cooling.file_sensors.save", { set: { sensors } });
      reload();
    }, message);
  return (
    <div className="flex flex-col gap-2">
      <h3 className="text-caption font-medium text-text-subtle">{t("cooling.sourcesSection.textFileSensors")}</h3>
      <p className="text-caption text-text-subtle">
        {t("cooling.sourcesSection.readsTheFirstNumberFromALocal")} </p>
      <ul className="flex flex-col divide-y divide-border overflow-hidden rounded-card bg-surface-sunken">
        {names.map((n) => (
          <li key={n} className="flex items-center gap-4 px-4 py-2">
            <span className="w-32 truncate font-mono text-small">{n}</span>
            <span className="min-w-0 flex-1 truncate text-caption text-text-subtle">{set.sensors[n]!.path}</span>
            <Button size="icon" variant="ghost" aria-label={t("cooling.sourcesSection.removeValue", { p0: n })} disabled={pending !== null}
              onClick={() => save(Object.fromEntries(Object.entries(set.sensors).filter(([k]) => k !== n)), t("cooling.sourcesSection.fileSensorRemoved"))}>
              <Trash2 />
            </Button>
          </li>
        ))}
        <li className="flex flex-col gap-1.5 px-4 py-3">
          <form className="flex flex-wrap items-center gap-2"
            onSubmit={(e) => {
              e.preventDefault();
              if (!name || !path || problem) return;
              save({ ...set.sensors, [name]: { path } }, t("cooling.sourcesSection.fileSensorSaved"));
              setName("");
              setPath("");
            }}>
            <input aria-label={t("cooling.sourcesSection.fileSensorName")} placeholder={t("cooling.sourcesSection.name")} value={name} onChange={(e) => setName(e.target.value)} className={`${field} w-32 font-mono`} />
            <input aria-label={t("cooling.sourcesSection.filePath")} placeholder="C:\temps\gpu.txt" value={path} onChange={(e) => setPath(e.target.value)} className={`${field} min-w-56 flex-1`} />
            <Button type="submit" size="sm" variant="secondary" disabled={!name || !path || !!problem || pending !== null}>
              <Plus /> {t("cooling.sourcesSection.add")} </Button>
          </form>
          {problem && <span className="text-caption text-danger-text">{problem}</span>}
        </li>
      </ul>
      {error && <p className="text-caption text-danger-text">{error}</p>}
    </div>
  );
}

/** NVIDIA fans via the driver's NVML; returned to driver auto on release, pause, exit or fault. */
export function GpuFansSection() {
  const [view, reload] = usePoll<GpuView>("cooling.gpu.list");
  const [outputs, setOutputs] = useState<string[]>([]);
  const [drafts, setDrafts] = useState<Record<string, string>>({});
  const { pending, error, run } = useAction();
  useEffect(() => {
    rpc<{ nodes: Record<string, unknown> }>("cooling.curve_set.get").then((s) => setOutputs(Object.keys(s.nodes)), () => setOutputs([]));
  }, []);
  if (!view?.started || view.fans.length === 0) return null;
  return (
    <Section title={t("cooling.sourcesSection.gpuFans")}>
      <p className="text-small text-text-muted">
        {t("cooling.sourcesSection.fixedDutyOrACompositeCurveFans")} </p>
      {view.paused && <Banner tone="warn">{t("cooling.sourcesSection.fanControlIsPausedGpuFansFollow")}</Banner>}
      <ul className="flex flex-col divide-y divide-border overflow-hidden rounded-card bg-surface-sunken">
        {view.fans.map((f) => {
          const id = f.fan.id;
          const current = f.assignment ? (f.assignment.mode === "manual" ? `manual:${f.assignment.percent}` : `composite:${f.assignment.output}`) : "auto";
          const choice = drafts[id] ?? current;
          const options = [
            { value: "auto", label: t("cooling.sourcesSection.driverAutomatic") },
            ...[50, 70, 100].filter((p) => p >= f.floor_percent).map((p) => ({ value: `manual:${p}`, label: t("cooling.sourcesSection.fixedValue", { p0: p }) })),
            ...outputs.map((o) => ({ value: `composite:${o}`, label: t("cooling.sourcesSection.curveValue", { p0: o }) })),
          ];
          const apply = () =>
            run(id, async () => {
              if (choice === "auto") await rpc("cooling.gpu.release", { fan_id: id });
              else {
                const [mode, arg] = [choice.slice(0, choice.indexOf(":")), choice.slice(choice.indexOf(":") + 1)];
                await rpc("cooling.gpu.set", mode === "manual" ? { fan_id: id, mode, percent: Number(arg) } : { fan_id: id, mode, output: arg });
              }
              reload();
            }, t("cooling.sourcesSection.gpuFanUpdated"));
          return (
            <li key={id} className="flex flex-wrap items-center gap-4 px-4 py-3">
              <div className="flex min-w-0 flex-1 flex-col">
                <span className="truncate text-small">{f.fan.gpu_name} {t("cooling.sourcesSection.fan")} {f.fan.index + 1}</span>
                <span className="text-caption text-text-subtle">
                  {f.fan.duty_percent != null ? t("cooling.sourcesSection.valueNow", { p0: num(f.fan.duty_percent, 0) }) : t("cooling.sourcesSection.dutyUnknown")} {t("cooling.sourcesSection.floor")} {f.floor_percent}%
                </span>
                {f.fault && <span className="text-caption text-danger-text">{f.fault}</span>}
                {!f.fan.controllable && <span className="text-caption text-text-subtle">{f.fan.reason ?? tierLabel("read_only")}</span>}
              </div>
              <Select label={t("cooling.sourcesSection.mode")} value={choice} onChange={(v) => setDrafts({ ...drafts, [id]: v })} options={options} />
              <Button size="sm" variant="secondary" disabled={!f.fan.controllable || choice === current || pending !== null} onClick={apply}>
                {t("cooling.sourcesSection.apply")} </Button>
            </li>
          );
        })}
      </ul>
      {error && <p className="text-caption text-danger-text">{error}</p>}
    </Section>
  );
}
