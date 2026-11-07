import { t } from "./messages";
import { ChevronDown, ChevronUp, Trash2 } from "lucide-react";
import { Select } from "@/components/ui/select";
import { ColorField } from "@/components/color-picker";
import { Button } from "@/components/ui/button";
import { Slider } from "@/components/ui/slider";
import { chordProblem, SINGLE_KEYS, type MacroStep } from "@/lib/api/control";
import { KeyRecorder } from "@/components/key-recorder";
import { EFFECTS } from "@/lib/api/effects";
import fixtures from "@/lib/api/fixtures/effects.json";
import type { EffectSpec } from "@/lib/api/types";
import { STEP_TYPES, blankStep } from "./steps";

const field =
  "h-8 rounded-chip border border-border-strong bg-surface px-2 text-small text-text outline-none focus-visible:border-info";
const KEY_LABELS: Record<string, string> = { get media_play_pause() { return t("macros.stepEditor.playPause"); }, get volume_up() { return t("macros.stepEditor.volumeUp"); }, get volume_down() { return t("macros.stepEditor.volumeDown"); }, get volume_mute() { return t("macros.stepEditor.mute"); } };

function Fields({ step, onChange }: { step: MacroStep; onChange: (s: MacroStep) => void }) {
  switch (step.type) {
    case "color":
      return <ColorField label={t("macros.stepEditor.colour")} value={step.color} onChange={(color) => onChange({ ...step, color })} onCommit={(color) => onChange({ ...step, color })} />;
    case "brightness":
      return (
        <div className="flex items-center gap-3">
          <Slider aria-label={t("macros.stepEditor.brightness")} min={0} max={1} step={0.01} value={[step.value]} onValueChange={([v]) => onChange({ ...step, value: v! })} />
          <span className="w-10 text-end text-small text-text-muted tabular">{Math.round(step.value * 100)}%</span>
        </div>
      );
    case "effect":
      return (
        <Select
          label={t("macros.stepEditor.effect")}
          className="w-56"
          value={step.effect.type}
          onChange={(name) => onChange({ ...step, effect: (fixtures as Record<string, EffectSpec>)[name]! })}
          options={[
            ...EFFECTS.filter((e) => e.name !== "layers").map((e) => ({ value: e.name as string, label: e.label })),
            ...(step.effect.type === "layers" ? [{ value: "layers", label: t("macros.stepEditor.customLayers") }] : []),
          ]}
        />
      );
    case "delay":
      return (
        <div className="flex items-center gap-2">
          <input
            type="number"
            min={0}
            max={10}
            step={0.1}
            aria-label={t("macros.stepEditor.seconds")}
            className={`${field} w-24 tabular`}
            value={step.ms / 1000}
            onChange={(e) => onChange({ ...step, ms: Math.max(0, Math.round(Number(e.target.value) * 1000)) })}
          />
          <span className="text-caption text-text-subtle">{t("macros.stepEditor.secondsMax10")}</span>
        </div>
      );
    case "text":
      return (
        <input
          aria-label={t("macros.stepEditor.text")}
          className={`${field} w-full`}
          value={step.text}
          placeholder={t("macros.stepEditor.textToTypeIntoTheFocusedApp")}
          onChange={(e) => onChange({ ...step, text: e.target.value })}
        />
      );
    case "key_down":
    case "key_up":
      return <input aria-label={t("macros.stepEditor.recordedKey")} className={field} value={step.key} onChange={(e) => onChange({ ...step, key: e.target.value })} />;
    case "key":
      return (
        <Select label={t("macros.stepEditor.key")} className="w-40" value={step.key} onChange={(key) => onChange({ ...step, key })} options={SINGLE_KEYS.map((k) => ({ value: k as string, label: KEY_LABELS[k] ?? k }))} />
      );
    case "chord":
      return (
        <div className="flex flex-col gap-1">
          <KeyRecorder
            label={t("macros.stepEditor.shortcutToSend")}
            value={step.keys}
            accept={(k) => /^([a-z0-9]|enter|escape|tab|space|backspace|up|down|left|right)$/.test(k)}
            onChange={(keys) => onChange({ ...step, keys })}
          />
          {chordProblem(step.keys) && step.keys.length > 0 && <span className="text-caption text-danger-text">{chordProblem(step.keys)}</span>}
        </div>
      );
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
      <span className="mt-1.5 w-5 text-end text-caption text-text-subtle tabular">{index + 1}</span>
      <Select
        label={t("macros.stepEditor.stepType")}
        className="w-36 shrink-0"
        value={step.type}
        onChange={(type) => onChange(blankStep(type))}
        options={STEP_TYPES.map((t) => ({ value: t.type, label: t.label }))}
      />
      <div className="min-w-0 flex-1">
        <Fields step={step} onChange={onChange} />
        {def.input && !inputEnabled && (
          <p className="mt-1 text-caption text-[#fbbf24]">{t("macros.stepEditor.needsTheServiceStartedWithEnableInput")}</p>
        )}
      </div>
      <div className="flex shrink-0">
        <Button size="icon" variant="ghost" className="size-8" aria-label={t("macros.stepEditor.moveStepUp")} disabled={index === 0} onClick={() => onMove(-1)}>
          <ChevronUp />
        </Button>
        <Button size="icon" variant="ghost" className="size-8" aria-label={t("macros.stepEditor.moveStepDown")} disabled={index === count - 1} onClick={() => onMove(1)}>
          <ChevronDown />
        </Button>
        <Button size="icon" variant="ghost" className="size-8" aria-label={t("macros.stepEditor.removeStep")} onClick={onRemove}>
          <Trash2 />
        </Button>
      </div>
    </li>
  );
}
