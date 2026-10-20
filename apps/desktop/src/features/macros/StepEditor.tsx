import { ChevronDown, ChevronUp, Trash2 } from "lucide-react";
import { ColorField } from "@/components/color-picker";
import { Button } from "@/components/ui/button";
import { Slider } from "@/components/ui/slider";
import { MODIFIERS, SINGLE_KEYS, type MacroStep } from "@/lib/api/control";
import { EFFECTS } from "@/lib/api/effects";
import fixtures from "@/lib/api/fixtures/effects.json";
import type { EffectSpec } from "@/lib/api/types";
import { STEP_TYPES, blankStep } from "./steps";

const field =
  "h-8 rounded-chip border border-border-strong bg-surface px-2 text-small text-text outline-none focus-visible:border-info";
const KEY_LABELS: Record<string, string> = { media_play_pause: "Play/Pause", volume_up: "Volume up", volume_down: "Volume down", volume_mute: "Mute" };

function Fields({ step, onChange }: { step: MacroStep; onChange: (s: MacroStep) => void }) {
  switch (step.type) {
    case "color":
      return <ColorField label="Colour" value={step.color} onChange={(color) => onChange({ ...step, color })} onCommit={(color) => onChange({ ...step, color })} />;
    case "brightness":
      return (
        <div className="flex items-center gap-3">
          <Slider aria-label="Brightness" min={0} max={1} step={0.01} value={[step.value]} onValueChange={([v]) => onChange({ ...step, value: v! })} />
          <span className="w-10 text-right text-small text-text-muted tabular">{Math.round(step.value * 100)}%</span>
        </div>
      );
    case "effect":
      return (
        <select
          className={field}
          aria-label="Effect"
          value={step.effect.type}
          onChange={(e) => onChange({ ...step, effect: (fixtures as Record<string, EffectSpec>)[e.target.value]! })}
        >
          {EFFECTS.filter((e) => e.name !== "layers").map((e) => (
            <option key={e.name} value={e.name}>
              {e.label} (default settings)
            </option>
          ))}
          {step.effect.type === "layers" && <option value="layers">Custom layers</option>}
        </select>
      );
    case "delay":
      return (
        <div className="flex items-center gap-2">
          <input
            type="number"
            min={0}
            max={10}
            step={0.1}
            aria-label="Seconds"
            className={`${field} w-24 tabular`}
            value={step.ms / 1000}
            onChange={(e) => onChange({ ...step, ms: Math.max(0, Math.round(Number(e.target.value) * 1000)) })}
          />
          <span className="text-caption text-text-subtle">seconds (max 10)</span>
        </div>
      );
    case "text":
      return (
        <input
          aria-label="Text"
          className={`${field} w-full`}
          value={step.text}
          placeholder="Text to type into the focused app"
          onChange={(e) => onChange({ ...step, text: e.target.value })}
        />
      );
    case "key":
      return (
        <select className={field} aria-label="Key" value={step.key} onChange={(e) => onChange({ ...step, key: e.target.value })}>
          {SINGLE_KEYS.map((k) => (
            <option key={k} value={k}>
              {KEY_LABELS[k] ?? k}
            </option>
          ))}
        </select>
      );
    case "chord": {
      const mods = step.keys.filter((k) => (MODIFIERS as readonly string[]).includes(k));
      const key = step.keys.find((k) => !(MODIFIERS as readonly string[]).includes(k)) ?? "";
      const set = (m: string[], k: string) => onChange({ ...step, keys: [...MODIFIERS.filter((x) => m.includes(x)), ...(k ? [k] : [])] });
      return (
        <div className="flex flex-wrap items-center gap-3">
          {MODIFIERS.map((m) => (
            <label key={m} className="flex items-center gap-1 text-small capitalize">
              <input
                type="checkbox"
                className="accent-[#10a37f]"
                checked={mods.includes(m)}
                onChange={(e) => set(e.target.checked ? [...mods, m] : mods.filter((x) => x !== m), key)}
              />
              {m === "meta" ? "Win" : m}
            </label>
          ))}
          <span className="text-caption text-text-subtle">+</span>
          <input
            aria-label="Key"
            className={`${field} w-24 font-mono`}
            value={key}
            maxLength={9}
            placeholder="a–z, 0–9"
            onChange={(e) => set(mods, e.target.value.trim().toLowerCase())}
          />
        </div>
      );
    }
  }
}

export function StepRow({
  step,
  index,
  count,
  inputEnabled,
  onChange,
  onMove,
  onRemove,
}: {
  step: MacroStep;
  index: number;
  count: number;
  inputEnabled: boolean;
  onChange: (s: MacroStep) => void;
  onMove: (by: number) => void;
  onRemove: () => void;
}) {
  const def = STEP_TYPES.find((t) => t.type === step.type)!;
  return (
    <li className="flex items-start gap-3 px-3 py-2.5">
      <span className="mt-1.5 w-5 text-right text-caption text-text-subtle tabular">{index + 1}</span>
      <select
        aria-label="Step type"
        className={`${field} w-36 shrink-0`}
        value={step.type}
        onChange={(e) => onChange(blankStep(e.target.value as MacroStep["type"]))}
      >
        {STEP_TYPES.map((t) => (
          <option key={t.type} value={t.type}>
            {t.label}
          </option>
        ))}
      </select>
      <div className="min-w-0 flex-1">
        <Fields step={step} onChange={onChange} />
        {def.input && !inputEnabled && (
          <p className="mt-1 text-caption text-[#fbbf24]">Needs the service started with --enable-input-macros.</p>
        )}
      </div>
      <div className="flex shrink-0">
        <Button size="icon" variant="ghost" className="size-8" aria-label="Move step up" disabled={index === 0} onClick={() => onMove(-1)}>
          <ChevronUp />
        </Button>
        <Button size="icon" variant="ghost" className="size-8" aria-label="Move step down" disabled={index === count - 1} onClick={() => onMove(1)}>
          <ChevronDown />
        </Button>
        <Button size="icon" variant="ghost" className="size-8" aria-label="Remove step" onClick={onRemove}>
          <Trash2 />
        </Button>
      </div>
    </li>
  );
}
