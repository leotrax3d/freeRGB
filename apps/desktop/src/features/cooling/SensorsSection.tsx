import { t } from "./messages";
import { Plus, RotateCcw, Trash2 } from "lucide-react";
import { Checkbox } from "@/components/ui/checkbox";
import { Select } from "@/components/ui/select";
import { useEffect, useState } from "react";
import { Section } from "@/components/page";
import { Button } from "@/components/ui/button";
import type { MixFunction, SensorGraph, SensorNode } from "@/lib/api/control";
import { num } from "@/lib/format";
import { approxEqual } from "@/lib/store/approxEqual";
import { rpc, useDaemon } from "@/lib/store/daemon";
import { useAction } from "@/lib/store/useAction";

const field =
  "h-8 rounded-chip border border-border-strong bg-surface px-2 text-small text-text outline-none focus-visible:border-info";

const TYPES: { type: SensorNode["type"]; label: string }[] = [
  { type: "input", get label() { return t("cooling.sensorsSection.sensorInput"); } },
  { type: "offset", get label() { return t("cooling.sensorsSection.correction"); } },
  { type: "mix", get label() { return t("cooling.sensorsSection.combination"); } },
  { type: "time_average", get label() { return t("cooling.sensorsSection.rollingAverage"); } },
];
const MIX: { fn: MixFunction; label: string }[] = [
  { fn: "maximum", get label() { return t("cooling.sensorsSection.maximum"); } },
  { fn: "minimum", get label() { return t("cooling.sensorsSection.minimum"); } },
  { fn: "average", get label() { return t("cooling.sensorsSection.average"); } },
  { fn: "sum", get label() { return t("cooling.sensorsSection.sum"); } },
  { fn: "subtract", get label() { return t("cooling.sensorsSection.firstMinusOthers"); } },
];

/** Mirrors the node-name rule in `SensorGraph::validate`. */
export function nodeNameProblem(name: string, taken: string[]): string | null {
  if (!name) return null;
  if (new TextEncoder().encode(name).length > 64 || !/^[\p{L}\p{N}_-]+$/u.test(name)) return t("cooling.sensorsSection.lettersDigitsAndOnlyNoSpaces");
  if (taken.includes(name)) return t("cooling.sensorsSection.thisNameIsAlreadyUsed");
  return null;
}

function blank(type: SensorNode["type"], firstSource: string, firstNode: string): SensorNode {
  switch (type) {
    case "input":
      return { type, source: firstSource };
    case "offset":
      return { type, source: firstNode, amount: 0, proportional: false };
    case "mix":
      return { type, sources: firstNode ? [firstNode] : [], function: "maximum" };
    case "time_average":
      return { type, source: firstNode, window_ms: 10_000 };
  }
}

function NodeFields({
  node,
  others,
  sources,
  onChange,
}: {
  node: SensorNode;
  others: string[];
  sources: Record<string, string>;
  onChange: (node: SensorNode) => void;
}) {
  const nodeSelect = (value: string, set: (v: string) => void) => (
    <Select label={t("cooling.sensorsSection.sourceNode")} value={others.includes(value) ? value : ""} onChange={set} options={others.map((n) => ({ value: n, label: n }))} />
  );
  switch (node.type) {
    case "input":
      return (
        <Select
          label={t("cooling.sensorsSection.sensor")}
          className="w-full"
          value={node.source in sources ? node.source : ""}
          placeholder={node.source ? t("cooling.sensorsSection.unavailableSensor") : t("cooling.sensorsSection.chooseASensor")}
          onChange={(source) => onChange({ ...node, source })}
          options={Object.entries(sources).map(([value, label]) => ({ value, label }))}
        />
      );
    case "offset":
      return (
        <div className="flex flex-wrap items-center gap-2">
          {nodeSelect(node.source, (source) => onChange({ ...node, source }))}
          <input
            type="number"
            step={0.5}
            min={-200}
            max={200}
            aria-label={t("cooling.sensorsSection.amount")}
            className={`${field} w-20 tabular`}
            value={node.amount}
            onChange={(e) => onChange({ ...node, amount: Number(e.target.value) })}
          />
          <Select
            label={t("cooling.sensorsSection.unit")}
            value={node.proportional ? "pct" : "abs"}
            onChange={(v) => onChange({ ...node, proportional: v === "pct" })}
            options={[{ value: "abs", label: "°C" }, { value: "pct", label: "%" }]}
          />
        </div>
      );
    case "mix":
      return (
        <div className="flex flex-wrap items-center gap-2">
          <Select label={t("cooling.sensorsSection.function")} value={node.function} onChange={(fn) => onChange({ ...node, function: fn })} options={MIX.map((m) => ({ value: m.fn, label: m.label }))} />
          <span className="text-caption text-text-subtle">{t("cooling.sensorsSection.of")}</span>
          {others.map((n) => (
            <Checkbox
              key={n}
              checked={node.sources.includes(n)}
              onChange={(on) => onChange({ ...node, sources: on ? [...node.sources, n] : node.sources.filter((s) => s !== n) })}
            >
              {n}
            </Checkbox>
          ))}
        </div>
      );
    case "time_average":
      return (
        <div className="flex flex-wrap items-center gap-2">
          {nodeSelect(node.source, (source) => onChange({ ...node, source }))}
          <span className="text-caption text-text-subtle">{t("cooling.sensorsSection.over")}</span>
          <input
            type="number"
            min={1}
            max={600}
            aria-label={t("cooling.sensorsSection.windowInSeconds")}
            className={`${field} w-20 tabular`}
            value={node.window_ms / 1000}
            onChange={(e) => onChange({ ...node, window_ms: Math.round(Number(e.target.value) * 1000) })}
          />
          <span className="text-caption text-text-subtle">s</span>
        </div>
      );
  }
}

/**
 * Custom Celsius sources evaluated by the daemon (docs/CUSTOM_SENSORS.md).
 * Edits stay local until saved; readings always show the saved graph.
 */
export function SensorsSection() {
  const saved = useDaemon((s) => s.sensor_graph);
  const sources = useDaemon((s) => s.sensor_sources) ?? {};
  const values = useDaemon((s) => s.sensor_values?.values);
  const [draft, setDraft] = useState<SensorGraph | null>(null);
  const [newName, setNewName] = useState("");
  const [newType, setNewType] = useState<SensorNode["type"]>("input");
  const { pending, error, run } = useAction();

  useEffect(() => {
    if (draft && saved && approxEqual(draft, saved)) setDraft(null);
  }, [draft, saved]);

  if (!saved) return null;
  const graph = draft ?? saved;
  const names = Object.keys(graph.nodes);
  const dirty = draft !== null;
  const problem = nodeNameProblem(newName, names);
  const update = (nodes: Record<string, SensorNode>) => setDraft({ version: 1, nodes });

  return (
    <Section
      title={t("cooling.sensorsSection.customTemperatureSources")}
      aside={
        dirty && (
          <div className="flex items-center gap-2">
            <Button size="sm" variant="ghost" onClick={() => setDraft(null)}>
              <RotateCcw /> {t("cooling.sensorsSection.discard")} </Button>
            <Button size="sm" variant="primary" disabled={pending !== null} onClick={() => run("save", () => rpc("cooling.sensors.save", { graph }), t("cooling.sensorsSection.temperatureSourcesSaved"))}>
              {t("cooling.sensorsSection.saveSources")} </Button>
          </div>
        )
      }
    >
      <p className="text-small text-text-muted">
        {t("cooling.sensorsSection.combineRealTemperatureReadingsIntoNewSources")} </p>
      {Object.keys(sources).length === 0 && (
        <p className="text-caption text-text-subtle">{t("cooling.sensorsSection.noConnectedDeviceCurrentlyReportsATemperature")}</p>
      )}
      <ul className="flex flex-col divide-y divide-border overflow-hidden rounded-card bg-surface-sunken">
        {names.map((name) => {
          const node = graph.nodes[name]!;
          const reading = values?.[name];
          return (
            <li key={name} className="flex flex-wrap items-center gap-x-4 gap-y-2 px-4 py-3">
              <div className="flex w-40 flex-col">
                <span className="truncate font-mono text-small">{name}</span>
                <span className="text-caption text-text-subtle">{TYPES.find((t) => t.type === node.type)?.label}</span>
              </div>
              <div className="min-w-0 flex-1">
                <NodeFields
                  node={node}
                  others={names.filter((n) => n !== name)}
                  sources={sources}
                  onChange={(next) => update({ ...graph.nodes, [name]: next })}
                />
              </div>
              <span className="w-24 shrink-0 text-end text-small tabular" title={reading?.error ?? undefined}>
                {dirty ? (
                  <span className="text-text-subtle">{t("cooling.sensorsSection.unsaved")}</span>
                ) : reading?.value != null ? (
                  `${num(reading.value, 1)} °C`
                ) : (
                  <span className="text-[#fbbf24]">{reading?.error ? "unavailable" : "—"}</span>
                )}
              </span>
              <Button
                size="icon"
                variant="ghost"
                aria-label={t("cooling.sensorsSection.removeValue", { p0: name })}
                onClick={() => update(Object.fromEntries(Object.entries(graph.nodes).filter(([n]) => n !== name)))}
              >
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
              if (!newName || problem) return;
              update({ ...graph.nodes, [newName]: blank(newType, Object.keys(sources)[0] ?? "", names[0] ?? "") });
              setNewName("");
            }}
          >
            <input
              aria-label={t("cooling.sensorsSection.newSourceName")}
              placeholder={t("cooling.sensorsSection.newSourceNameAlternative")}
              value={newName}
              onChange={(e) => setNewName(e.target.value)}
              className={`${field} w-44 font-mono`}
            />
            <Select
              label={t("cooling.sensorsSection.type")}
              value={newType}
              onChange={setNewType}
              options={TYPES.map((t) => ({ value: t.type, label: t.label, disabled: t.type !== "input" && names.length === 0 }))}
            />
            <Button type="submit" size="sm" variant="secondary" disabled={!newName || !!problem}>
              <Plus /> {t("cooling.sensorsSection.add")} </Button>
          </form>
          {problem && <span className="text-caption text-danger-text">{problem}</span>}
        </li>
      </ul>
      {error && <p className="text-caption text-danger-text">{error}</p>}
    </Section>
  );
}
