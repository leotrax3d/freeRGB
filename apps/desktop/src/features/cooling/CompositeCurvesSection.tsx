import { t } from "./messages";
import { Plus, RotateCcw, Trash2 } from "lucide-react";
import { useEffect, useState } from "react";
import { Section } from "@/components/page";
import { Button } from "@/components/ui/button";
import { Checkbox } from "@/components/ui/checkbox";
import { Select } from "@/components/ui/select";
import { DEFAULT_CURVE, curveSetProblem, type CurveMixFunction, type CurveNode, type CurveSet, type CurveSetValues } from "@/lib/api/control";
import { num } from "@/lib/format";
import { rpc, useDaemon } from "@/lib/store/daemon";
import { useAction } from "@/lib/store/useAction";
import { nodeNameProblem } from "./SensorsSection";
import { CurveEditor } from "./CurveEditor";
import type { FanCurve } from "@/lib/api/control";

/** Drag editor for graph and linear nodes (linear = two draggable points). */
function DragCurve({ curve, onChange }: { curve: FanCurve; onChange: (c: FanCurve) => void }) {
  const [selected, setSelected] = useState<number | null>(null);
  return <div className="w-full max-w-xl"><CurveEditor curve={curve} selected={selected} onSelect={setSelected} onChange={onChange} /></div>;
}

const field =
  "h-8 rounded-chip border border-border-strong bg-surface px-2 text-small text-text outline-none focus-visible:border-info";

const TYPES: { type: CurveNode["type"]; label: string }[] = [
  { type: "graph", get label() { return t("cooling.compositeCurvesSection.graphCurve"); } },
  { type: "constant", get label() { return t("cooling.compositeCurvesSection.constant"); } },
  { type: "trigger", get label() { return t("cooling.compositeCurvesSection.twoThresholdTrigger"); } },
  { type: "mix", get label() { return t("cooling.compositeCurvesSection.mix"); } },
  { type: "follow", get label() { return t("cooling.compositeCurvesSection.follow"); } },
  { type: "linear", get label() { return t("cooling.compositeCurvesSection.linear"); } },
  { type: "equilibrium", get label() { return t("cooling.compositeCurvesSection.adaptiveEquilibrium"); } },
];
const MIX: { fn: CurveMixFunction; label: string }[] = [
  { fn: "maximum", get label() { return t("cooling.compositeCurvesSection.maximum"); } },
  { fn: "minimum", get label() { return t("cooling.compositeCurvesSection.minimum"); } },
  { fn: "average", get label() { return t("cooling.compositeCurvesSection.average"); } },
  { fn: "sum", get label() { return t("cooling.compositeCurvesSection.sum"); } },
];

function blank(type: CurveNode["type"], sensor: string, other: string): CurveNode {
  switch (type) {
    case "graph":
      return { type, sensor, curve: DEFAULT_CURVE };
    case "constant":
      return { type, percent: 50 };
    case "trigger":
      return { type, sensor, idle_c: 45, load_c: 60, idle_percent: 30, load_percent: 70, critical_c: 85, hold_ms: 5000 };
    case "mix":
      return { type, sources: other ? [other] : [], function: "maximum" };
    case "follow":
      return { type, source: other, scale: 1, offset_percent: 0 };
    case "linear":
      return { type, sensor, min_c: 35, max_c: 75, min_percent: 30, max_percent: 100, critical_c: 85 };
    case "equilibrium":
      return { type, sensor, target_c: 65, critical_c: 90, min_percent: 30, max_percent: 100, gain_percent_per_c: 1, max_change_percent_per_s: 2, trend_s: 10 };
  }
}

function Num({ label, value, onChange, step = 1, unit }: { label: string; value: number; onChange: (v: number) => void; step?: number; unit?: string }) {
  return (
    <label className="flex items-center gap-1 text-caption text-text-subtle">
      {label}
      <input type="number" step={step} aria-label={label} className={`${field} w-16 tabular`} value={value} onChange={(e) => onChange(Number(e.target.value))} />
      {unit}
    </label>
  );
}

function NodeFields({ node, others, sensors, onChange }: { node: CurveNode; others: string[]; sensors: Record<string, string>; onChange: (n: CurveNode) => void }) {
  const sensorSelect = (value: string, set: (v: string) => void) => (
    <Select
      label={t("cooling.compositeCurvesSection.sensor")}
      value={value in sensors ? value : ""}
      placeholder={value ? t("cooling.compositeCurvesSection.unavailableSensor") : t("cooling.compositeCurvesSection.chooseASensor")}
      onChange={set}
      options={Object.entries(sensors).map(([v, label]) => ({ value: v, label }))}
    />
  );
  switch (node.type) {
    case "graph":
      return (
        <div className="flex flex-wrap items-center gap-2">
          {sensorSelect(node.sensor, (sensor) => onChange({ ...node, sensor }))}
          <span className="text-caption text-text-subtle">
            {node.curve.points.map((p) => `${num(p.temperature_c, 0)}°→${num(p.duty_percent, 0)}%`).join("  ")}
          </span>
          <DragCurve curve={node.curve} onChange={(curve) => onChange({ ...node, curve })} />
        </div>
      );
    case "constant":
      return <Num label={t("cooling.compositeCurvesSection.duty")} unit="%" value={node.percent} onChange={(percent) => onChange({ ...node, percent })} />;
    case "trigger":
      return (
        <div className="flex flex-wrap items-center gap-2">
          {sensorSelect(node.sensor, (sensor) => onChange({ ...node, sensor }))}
          <Num label={t("cooling.compositeCurvesSection.idleTemperature")} unit="°C" value={node.idle_c} onChange={(idle_c) => onChange({ ...node, idle_c })} />
          <Num label={t("cooling.compositeCurvesSection.loadTemperature")} unit="°C" value={node.load_c} onChange={(load_c) => onChange({ ...node, load_c })} />
          <Num label={t("cooling.compositeCurvesSection.idleDuty")} unit="%" value={node.idle_percent} onChange={(idle_percent) => onChange({ ...node, idle_percent })} />
          <Num label={t("cooling.compositeCurvesSection.loadDuty")} unit="%" value={node.load_percent} onChange={(load_percent) => onChange({ ...node, load_percent })} />
          <Num label={t("cooling.compositeCurvesSection.critical")} unit="°C" value={node.critical_c} onChange={(critical_c) => onChange({ ...node, critical_c })} />
          <Num label={t("cooling.compositeCurvesSection.hold")} unit="s" value={node.hold_ms / 1000} onChange={(s) => onChange({ ...node, hold_ms: Math.round(s * 1000) })} />
        </div>
      );
    case "mix":
      return (
        <div className="flex flex-wrap items-center gap-2">
          <Select label={t("cooling.compositeCurvesSection.function")} value={node.function} onChange={(fn) => onChange({ ...node, function: fn })} options={MIX.map((m) => ({ value: m.fn, label: m.label }))} />
          <span className="text-caption text-text-subtle">{t("cooling.compositeCurvesSection.of")}</span>
          {others.map((n) => (
            <Checkbox key={n} checked={node.sources.includes(n)} onChange={(on) => onChange({ ...node, sources: on ? [...node.sources, n] : node.sources.filter((s) => s !== n) })}>
              {n}
            </Checkbox>
          ))}
        </div>
      );
    case "follow":
      return (
        <div className="flex flex-wrap items-center gap-2">
          <Select label={t("cooling.compositeCurvesSection.curve")} value={others.includes(node.source) ? node.source : ""} onChange={(source) => onChange({ ...node, source })} options={others.map((n) => ({ value: n, label: n }))} />
          <Num label="×" step={0.05} value={node.scale} onChange={(scale) => onChange({ ...node, scale })} />
          <Num label="+" unit="%" value={node.offset_percent} onChange={(offset_percent) => onChange({ ...node, offset_percent })} />
        </div>
      );
    case "linear":
      return (
        <div className="flex flex-wrap items-center gap-2">
          {sensorSelect(node.sensor, (sensor) => onChange({ ...node, sensor }))}
          <Num label={t("cooling.compositeCurvesSection.from")} unit="°C" value={node.min_c} onChange={(min_c) => onChange({ ...node, min_c })} />
          <Num label={t("cooling.compositeCurvesSection.at")} unit="%" value={node.min_percent} onChange={(min_percent) => onChange({ ...node, min_percent })} />
          <Num label={t("cooling.compositeCurvesSection.to")} unit="°C" value={node.max_c} onChange={(max_c) => onChange({ ...node, max_c })} />
          <Num label={t("cooling.compositeCurvesSection.at")} unit="%" value={node.max_percent} onChange={(max_percent) => onChange({ ...node, max_percent })} />
          <Num label={t("cooling.compositeCurvesSection.critical")} unit="°C" value={node.critical_c} onChange={(critical_c) => onChange({ ...node, critical_c })} />
          <DragCurve
            curve={{ points: [{ temperature_c: node.min_c, duty_percent: node.min_percent }, { temperature_c: node.max_c, duty_percent: node.max_percent }], minimum_percent: Math.min(node.min_percent, node.max_percent), critical_c: node.critical_c, hysteresis_c: 0 }}
            onChange={(c) => onChange({ ...node, min_c: c.points[0]!.temperature_c, min_percent: c.points[0]!.duty_percent, max_c: c.points[1]!.temperature_c, max_percent: c.points[1]!.duty_percent })} />
        </div>
      );
    case "equilibrium":
      return (
        <div className="flex flex-wrap items-center gap-2">
          {sensorSelect(node.sensor, (sensor) => onChange({ ...node, sensor }))}
          <Num label={t("cooling.compositeCurvesSection.hold")} unit="°C" value={node.target_c} onChange={(target_c) => onChange({ ...node, target_c })} />
          <Num label={t("cooling.compositeCurvesSection.critical")} unit="°C" value={node.critical_c} onChange={(critical_c) => onChange({ ...node, critical_c })} />
          <Num label={t("cooling.compositeCurvesSection.min")} unit="%" value={node.min_percent} onChange={(min_percent) => onChange({ ...node, min_percent })} />
          <Num label={t("cooling.compositeCurvesSection.max")} unit="%" value={node.max_percent} onChange={(max_percent) => onChange({ ...node, max_percent })} />
          <Num label={t("cooling.compositeCurvesSection.gain")} unit="%/°C" step={0.1} value={node.gain_percent_per_c} onChange={(gain_percent_per_c) => onChange({ ...node, gain_percent_per_c })} />
          <Num label={t("cooling.compositeCurvesSection.maxChange")} unit="%/s" step={0.1} value={node.max_change_percent_per_s} onChange={(max_change_percent_per_s) => onChange({ ...node, max_change_percent_per_s })} />
          <Num label={t("cooling.compositeCurvesSection.lookAhead")} unit="s" value={node.trend_s} onChange={(trend_s) => onChange({ ...node, trend_s })} />
        </div>
      );
  }
}

/**
 * Composite FanControl-style curves evaluated by the daemon as a preview.
 * They are not bound to fan channels yet; outputs never drive hardware.
 */
export function CompositeCurvesSection() {
  const sensors = useDaemon((s) => s.sensor_sources) ?? {};
  const [saved, setSaved] = useState<CurveSet | null>(null);
  const [draft, setDraft] = useState<CurveSet | null>(null);
  const [values, setValues] = useState<CurveSetValues | null>(null);
  const [newName, setNewName] = useState("");
  const [newType, setNewType] = useState<CurveNode["type"]>("graph");
  const { pending, error, run } = useAction();

  useEffect(() => {
    let alive = true;
    rpc<CurveSet>("cooling.curve_set.get").then((set) => alive && setSaved(set), () => alive && setSaved({ nodes: {} }));
    const poll = () => rpc<CurveSetValues>("cooling.curve_set.values").then((v) => alive && setValues(v), () => {});
    poll();
    const timer = setInterval(poll, 2000);
    return () => {
      alive = false;
      clearInterval(timer);
    };
  }, []);

  if (!saved) return null;
  const set = draft ?? saved;
  const names = Object.keys(set.nodes);
  const dirty = draft !== null;
  const nameProblem = nodeNameProblem(newName, names);
  const setProblem = curveSetProblem(set);
  const update = (nodes: Record<string, CurveNode>) => setDraft({ nodes });
  const save = () =>
    run("save", async () => {
      await rpc("cooling.curve_set.save", { set });
      setSaved(set);
      setDraft(null);
    }, t("cooling.compositeCurvesSection.compositeCurvesSaved"));

  return (
    <Section
      title={t("cooling.compositeCurvesSection.compositeCurvesPreview")}
      aside={
        dirty && (
          <div className="flex items-center gap-2">
            <Button size="sm" variant="ghost" onClick={() => setDraft(null)}>
              <RotateCcw /> {t("cooling.compositeCurvesSection.discard")} </Button>
            <Button size="sm" variant="primary" disabled={pending !== null || !!setProblem} onClick={save}>
              {t("cooling.compositeCurvesSection.saveCurves")} </Button>
          </div>
        )
      }
    >
      <p className="text-small text-text-muted">
        {t("cooling.compositeCurvesSection.buildCurvesFromOtherCurvesConstantValues")} </p>
      <ul className="flex flex-col divide-y divide-border overflow-hidden rounded-card bg-surface-sunken">
        {names.map((name) => {
          const node = set.nodes[name]!;
          const out = values?.outputs[name];
          return (
            <li key={name} className="flex flex-wrap items-center gap-x-4 gap-y-2 px-4 py-3">
              <div className="flex w-40 flex-col">
                <span className="truncate font-mono text-small">{name}</span>
                <span className="text-caption text-text-subtle">{TYPES.find((t) => t.type === node.type)?.label}</span>
              </div>
              <div className="min-w-0 flex-1">
                <NodeFields node={node} others={names.filter((n) => n !== name)} sensors={sensors} onChange={(next) => update({ ...set.nodes, [name]: next })} />
              </div>
              <span className="w-24 shrink-0 text-end text-small tabular">
                {dirty ? (
                  <span className="text-text-subtle">{t("cooling.compositeCurvesSection.unsaved")}</span>
                ) : out ? (
                  <span className={out.failsafe ? "text-[#fbbf24]" : undefined} title={out.failsafe ? t("cooling.compositeCurvesSection.failSafeMissingStaleOrCriticalInput") : undefined}>
                    {num(out.percent, 0)} %{out.failsafe ? " !" : ""}
                  </span>
                ) : (
                  "—"
                )}
              </span>
              <Button size="icon" variant="ghost" aria-label={t("cooling.compositeCurvesSection.removeValue", { p0: name })} onClick={() => update(Object.fromEntries(Object.entries(set.nodes).filter(([n]) => n !== name)))}>
                <Trash2 />
              </Button>
            </li>
          );
        })}
        <li className="flex flex-col gap-1.5 px-4 py-3">
          <form
            className="flex flex-wrap items-center gap-2"
            onSubmit={(e) => {
              e.preventDefault();
              if (!newName || nameProblem) return;
              update({ ...set.nodes, [newName]: blank(newType, Object.keys(sensors)[0] ?? "", names[0] ?? "") });
              setNewName("");
            }}
          >
            <input aria-label={t("cooling.compositeCurvesSection.newCurveName")} placeholder={t("cooling.compositeCurvesSection.newCurveNameAlternative")} value={newName} onChange={(e) => setNewName(e.target.value)} className={`${field} w-44 font-mono`} />
            <Select
              label={t("cooling.compositeCurvesSection.type")}
              value={newType}
              onChange={setNewType}
              options={TYPES.map((t) => ({ value: t.type, label: t.label, disabled: (t.type === "mix" || t.type === "follow") && names.length === 0 }))}
            />
            <Button type="submit" size="sm" variant="secondary" disabled={!newName || !!nameProblem}>
              <Plus /> {t("cooling.compositeCurvesSection.add")} </Button>
          </form>
          {nameProblem && <span className="text-caption text-danger-text">{nameProblem}</span>}
          {setProblem && <span className="text-caption text-danger-text">{setProblem}</span>}
        </li>
      </ul>
      {error && <p className="text-caption text-danger-text">{error}</p>}
    </Section>
  );
}
