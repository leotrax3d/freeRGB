import { Info } from "lucide-react";
import { ColorField } from "@/components/color-picker";
import { Slider } from "@/components/ui/slider";
import type { EffectSpec, Rgb } from "@/lib/api/types";
import { SCHEMA, type Field } from "./schema";

export type FieldChange = (key: string, value: unknown, commit: boolean) => void;

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

/** Schema-driven controls for one (non-layered) effect. */
export function EffectFields({ spec, onChange }: { spec: Exclude<EffectSpec, { type: "layers" }>; onChange: FieldChange }) {
  const values = spec as unknown as Record<string, unknown>;
  const { fields, note } = SCHEMA[spec.type];
  return (
    <div className="flex flex-col gap-1">
      {fields.map((field) =>
        field.kind === "color" ? (
          <ColorField
            key={field.key}
            label={field.label}
            value={values[field.key] as Rgb}
            onChange={(rgb) => onChange(field.key, rgb, false)}
            onCommit={(rgb) => onChange(field.key, rgb, true)}
          />
        ) : (
          <RangeField
            key={field.key}
            field={field}
            value={values[field.key] as number}
            onChange={(v, commit) => onChange(field.key, field.key === "period_ms" ? Math.round(v) : v, commit)}
          />
        ),
      )}
      {note && (
        <p className="flex gap-2 px-2 pt-2 text-caption text-text-subtle">
          <Info className="mt-px size-3.5 shrink-0" />
          {note}
        </p>
      )}
    </div>
  );
}
