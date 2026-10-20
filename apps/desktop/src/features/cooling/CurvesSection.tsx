import { Plus, Thermometer, Trash2 } from "lucide-react";
import { useEffect, useState } from "react";
import { ConfirmButton } from "@/components/confirm-button";
import { NameForm } from "@/components/name-form";
import { Section } from "@/components/page";
import { Button } from "@/components/ui/button";
import { Slider } from "@/components/ui/slider";
import { curveProblem, DEFAULT_CURVE, type FanCurve } from "@/lib/api/control";
import { num } from "@/lib/format";
import { approxEqual } from "@/lib/store/approxEqual";
import { rpc, useDaemon } from "@/lib/store/daemon";
import { useAction } from "@/lib/store/useAction";
import { cn } from "@/lib/utils";
import { addPoint, removePoint, withMinimum } from "./curve";
import { CurveEditor } from "./CurveEditor";

function LimitSlider({
  label,
  unit,
  value,
  min,
  max,
  step = 1,
  onChange,
}: {
  label: string;
  unit: string;
  value: number;
  min: number;
  max: number;
  step?: number;
  onChange: (v: number) => void;
}) {
  return (
    <div className="flex flex-col gap-1.5">
      <div className="flex justify-between text-small">
        <span>{label}</span>
        <span className="text-text-muted tabular">
          {num(value, 1)} {unit}
        </span>
      </div>
      <Slider aria-label={label} min={min} max={max} step={step} value={[value]} onValueChange={([v]) => onChange(v!)} />
    </div>
  );
}

/** Simulated duty for a temperature, asked from the daemon (no hardware write). */
function usePreview(curve: FanCurve, temperature: number) {
  const [duty, setDuty] = useState<number | null>(null);
  useEffect(() => {
    if (curveProblem(curve)) return setDuty(null);
    let live = true;
    const timer = setTimeout(() => {
      rpc<{ duty_percent: number }>("cooling.preview", { curve, temperature_c: temperature, age_ms: 0 }).then(
        (r) => live && setDuty(r.duty_percent),
        () => live && setDuty(null),
      );
    }, 120);
    return () => {
      live = false;
      clearTimeout(timer);
    };
  }, [curve, temperature]);
  return duty;
}

export function CurvesSection() {
  const curves = useDaemon((s) => s.curves);
  const [name, setName] = useState<string | null>(null);
  const [draft, setDraft] = useState<FanCurve>(DEFAULT_CURVE);
  const [point, setPoint] = useState<number | null>(null);
  const [temperature, setTemperature] = useState(45);
  const duty = usePreview(draft, temperature);
  const { pending, error, run } = useAction();

  const open = (n: string | null) => {
    setName(n);
    setDraft(n && curves?.[n] ? curves[n] : DEFAULT_CURVE);
    setPoint(null);
  };
  const problem = curveProblem(draft);
  const saved = name !== null && curves?.[name] !== undefined && approxEqual(curves[name], draft);
  const lastTemp = draft.points[draft.points.length - 1]!.temperature_c;

  return (
    <Section title="Fan curves" aside={error && <span className="text-caption text-danger-text">{error}</span>}>
      <div className="grid items-start gap-4 @3xl:grid-cols-[200px_minmax(0,1fr)]">
        <ul className="flex flex-col gap-1">
          {Object.keys(curves ?? {}).map((n) => (
            <li key={n}>
              <button
                type="button"
                onClick={() => open(n)}
                className={cn(
                  "flex h-9 w-full items-center gap-2 rounded-control px-3 text-left text-body transition-colors",
                  name === n ? "bg-surface-raised" : "text-text-muted hover:bg-surface-raised hover:text-text",
                )}
              >
                <Thermometer className="size-4 text-text-subtle" />
                <span className="truncate">{n}</span>
              </button>
            </li>
          ))}
          <li>
            <Button variant="ghost" size="sm" className="w-full justify-start" onClick={() => open(null)}>
              <Plus /> New curve
            </Button>
          </li>
        </ul>

        <div className="flex flex-col gap-4 rounded-card bg-surface-sunken p-4">
          <div className="flex items-center justify-between">
            <h3 className="text-title font-semibold">{name ?? "New curve"}</h3>
            <div className="flex items-center gap-1">
              <Button size="sm" variant="ghost" disabled={draft.points.length >= 16} onClick={() => setDraft(addPoint(draft))}>
                <Plus /> Point
              </Button>
              <Button
                size="sm"
                variant="ghost"
                disabled={point === null || draft.points.length <= 2}
                onClick={() => {
                  setDraft(removePoint(draft, point!));
                  setPoint(null);
                }}
              >
                <Trash2 /> Point
              </Button>
            </div>
          </div>
          <CurveEditor
            curve={draft}
            selected={point}
            preview={{ temperature_c: temperature, duty_percent: duty }}
            onSelect={setPoint}
            onChange={setDraft}
          />
          {point !== null && draft.points[point] && (
            <p className="text-caption text-text-muted tabular">
              Point {point + 1}: {num(draft.points[point]!.temperature_c, 1)} °C → {num(draft.points[point]!.duty_percent, 1)} %
              · arrow keys adjust, Shift for bigger steps
            </p>
          )}
          <div className="grid gap-4 @xl:grid-cols-3">
            <LimitSlider label="Minimum duty" unit="%" value={draft.minimum_percent} min={20} max={100} onChange={(v) => setDraft(withMinimum(draft, v))} />
            <LimitSlider
              label="Critical"
              unit="°C"
              value={draft.critical_c}
              min={Math.max(40, Math.ceil(lastTemp))}
              max={110}
              onChange={(v) => setDraft({ ...draft, critical_c: v })}
            />
            <LimitSlider label="Hysteresis" unit="°C" value={draft.hysteresis_c} min={0} max={10} step={0.5} onChange={(v) => setDraft({ ...draft, hysteresis_c: v })} />
          </div>
          <div className="flex flex-col gap-1.5 rounded-control bg-surface p-3">
            <div className="flex justify-between text-small">
              <span>Try a temperature</span>
              <span className="text-info-text tabular">
                {temperature} °C → {duty === null ? "—" : `${num(duty, 1)} %`} <span className="text-text-subtle">(simulated)</span>
              </span>
            </div>
            <Slider aria-label="Preview temperature" min={0} max={110} step={1} value={[temperature]} onValueChange={([v]) => setTemperature(v!)} />
          </div>
          {problem && <p className="text-caption text-danger-text">{problem}</p>}
          <div className="flex flex-wrap items-start justify-between gap-3 border-t border-border pt-4">
            {name ? (
              <div className="flex items-center gap-2">
                <Button variant="primary" disabled={!!problem || saved || pending !== null} onClick={() => run("save", () => rpc("cooling.curves.save", { name, curve: draft }))}>
                  {saved ? "Saved" : "Save changes"}
                </Button>
                <ConfirmButton
                  label={`Delete ${name}`}
                  icon={<Trash2 />}
                  disabled={pending !== null}
                  onConfirm={() => run("delete", () => rpc("cooling.curves.delete", { name })).then((ok) => ok && open(null))}
                />
              </div>
            ) : (
              <div className="w-full max-w-sm">
                <NameForm
                  label="Curve name"
                  placeholder="e.g. Quiet"
                  submit="Save curve"
                  existing={Object.keys(curves ?? {})}
                  busy={pending !== null || !!problem}
                  onSubmit={async (n) => {
                    const ok = await run("save", () => rpc("cooling.curves.save", { name: n, curve: draft }));
                    if (ok) setName(n);
                    return ok;
                  }}
                />
              </div>
            )}
          </div>
        </div>
      </div>
    </Section>
  );
}
