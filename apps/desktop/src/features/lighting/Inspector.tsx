import { Info, Layers } from "lucide-react";
import { ColorField } from "@/components/color-picker";
import { NameForm } from "@/components/name-form";
import { Slider } from "@/components/ui/slider";
import { effectLabel } from "@/lib/api/effects";
import type { Rgb } from "@/lib/api/types";
import { rpc, useDaemon } from "@/lib/store/daemon";
import { useAction } from "@/lib/store/useAction";
import { SCHEMA, type Field } from "./schema";
import { useEffectDraft } from "./useEffectDraft";

function RangeField({
  field,
  value,
  onChange,
}: {
  field: Extract<Field, { kind: "range" }>;
  value: number;
  onChange: (value: number, commit: boolean) => void;
}) {
  return (
    <div className="flex flex-col gap-2 px-2 py-1.5">
      <div className="flex items-baseline justify-between">
        <span className="text-body">{field.label}</span>
        <span className="text-small text-text-muted tabular">{field.format(value)}</span>
      </div>
      <Slider
        aria-label={field.label}
        min={field.min}
        max={field.max}
        step={field.step}
        // Values set elsewhere (CLI, profile) may lie outside the slider range.
        value={[Math.min(field.max, Math.max(field.min, value))]}
        onValueChange={([v]) => onChange(v!, false)}
        onValueCommit={([v]) => onChange(v!, true)}
      />
      {field.hint && <span className="text-caption text-text-subtle">{field.hint}</span>}
    </div>
  );
}

function SaveCustom() {
  const effect = useDaemon((s) => s.effect);
  const names = useDaemon((s) => s.custom_effects);
  const { pending, error, run } = useAction();
  if (names === null) return null; // --no-state: no storage
  return (
    <div className="flex flex-col gap-2 border-t border-border pt-4">
      <span className="text-small font-medium text-text-subtle">Save as custom effect</span>
      <NameForm
        label="Custom effect name"
        placeholder="Name"
        submit="Save"
        existing={names ?? []}
        busy={pending !== null}
        onSubmit={(name) => run("save", () => rpc("custom_effects.save", { name, effect }))}
      />
      {error && <p className="text-caption text-danger-text">{error}</p>}
    </div>
  );
}

export function Inspector() {
  const { effect, set, error } = useEffectDraft();
  if (!effect) return null;
  const values = effect as unknown as Record<string, unknown>;

  return (
    <aside aria-label="Effect settings" className="flex flex-col gap-4 rounded-card bg-surface-sunken p-4">
      <header className="flex flex-col gap-0.5 px-2">
        <span className="text-caption uppercase tracking-wider text-text-subtle">Settings</span>
        <h2 className="text-title font-semibold">{effectLabel(effect.type)}</h2>
      </header>

      {effect.type === "layers" ? (
        <div className="flex gap-3 rounded-control bg-surface px-3 py-3 text-small text-text-muted">
          <Layers className="mt-0.5 size-4 shrink-0 text-creative-text" />
          <span>
            {effect.layers.length} {effect.layers.length === 1 ? "layer" : "layers"}. The layer editor arrives in the
            next step (M3); until then edit layers in freeRGB Classic.
          </span>
        </div>
      ) : (
        <div className="flex flex-col gap-1">
          {SCHEMA[effect.type].fields.map((field) =>
            field.kind === "color" ? (
              <ColorField
                key={field.key}
                label={field.label}
                value={values[field.key] as Rgb}
                onChange={(rgb) => set(field.key, rgb, false)}
                onCommit={(rgb) => set(field.key, rgb, true)}
              />
            ) : (
              <RangeField
                key={field.key}
                field={field}
                value={values[field.key] as number}
                onChange={(v, commit) => set(field.key, field.key === "period_ms" ? Math.round(v) : v, commit)}
              />
            ),
          )}
          {SCHEMA[effect.type].note && (
            <p className="flex gap-2 px-2 pt-2 text-caption text-text-subtle">
              <Info className="mt-px size-3.5 shrink-0" />
              {SCHEMA[effect.type].note}
            </p>
          )}
        </div>
      )}
      {error && <p className="px-2 text-caption text-danger-text">{error}</p>}
      <SaveCustom />
    </aside>
  );
}
