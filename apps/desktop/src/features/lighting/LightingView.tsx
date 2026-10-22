import { Check, Sun } from "lucide-react";
import { memo, useState } from "react";
import { PageHeader, Section } from "@/components/page";
import { Slider } from "@/components/ui/slider";
import { EFFECTS, effectLabel, effectPreview } from "@/lib/api/effects";
import fixtures from "@/lib/api/fixtures/effects.json";
import type { EffectName, EffectSpec } from "@/lib/api/types";
import { rpc, useDaemon, useSlices } from "@/lib/store/daemon";
import { useDaemonValue } from "@/lib/store/useDaemonValue";
import { cn } from "@/lib/utils";
import { CustomEffects } from "./CustomEffects";
import { Inspector } from "./Inspector";
import { useCurrentEffect } from "./useEffectDraft";

const DEFAULTS = fixtures as Record<EffectName, EffectSpec>;

const HINTS: Partial<Record<EffectName, string>> = {
  audio_pulse: "Reacts to sound",
  audio_spectrum: "Reacts to sound",
  screen_ambience: "Mirrors your screen",
  layers: "Your own mix",
};

function Brightness() {
  const remote = useDaemon((s) => s.diagnostics?.brightness);
  const { value, change, commit, error } = useDaemonValue(remote, (v) => ["brightness.set", { value: v }]);
  return (
    <div className="flex w-72 flex-col gap-1">
      <div className="flex items-center gap-3">
        <Sun className="size-4 text-text-subtle" aria-hidden />
        <Slider
          aria-label="Global brightness"
          min={0}
          max={1}
          step={0.01}
          value={[value]}
          onValueChange={([v]) => change(v!)}
          onValueCommit={([v]) => commit(v!)}
        />
        <span className="w-10 text-right text-small text-text-muted tabular">{Math.round(value * 100)}%</span>
      </div>
      {error && <p className="text-caption text-danger-text">{error}</p>}
    </div>
  );
}

function NowPlaying() {
  const effect = useCurrentEffect();
  return (
    <div data-tour="now" className="relative overflow-hidden rounded-card border border-border">
      <div className="absolute inset-0 opacity-35" style={{ background: effect ? effectPreview(effect) : "#303030" }} aria-hidden />
      <div className="absolute inset-0 bg-gradient-to-r from-black/80 via-black/50 to-black/20" aria-hidden />
      <div className="relative flex flex-wrap items-end justify-between gap-4 p-5">
        <div className="flex flex-col gap-1">
          <span className="text-caption font-medium uppercase tracking-wider text-text-muted">Now running</span>
          <span className="text-section font-semibold">{effect ? effectLabel(effect.type) : "—"}</span>
        </div>
        <Brightness />
      </div>
    </div>
  );
}

const EffectTile = memo(function EffectTile({
  name,
  label,
  spec,
  active,
  busy,
  onSelect,
}: {
  name: EffectName;
  label: string;
  spec: EffectSpec;
  active: boolean;
  busy: boolean;
  onSelect: (name: EffectName) => void;
}) {
  return (
    <button
      type="button"
      aria-pressed={active}
      disabled={busy}
      onClick={() => onSelect(name)}
      className={cn(
        "group flex flex-col gap-2 rounded-card p-1.5 pb-2.5 text-left transition-colors duration-(--duration-fast)",
        active ? "bg-surface-raised ring-1 ring-accent/60" : "hover:bg-surface-raised",
      )}
    >
      <div className="relative h-14 overflow-hidden rounded-control opacity-90 transition-opacity group-hover:opacity-100" style={{ background: effectPreview(spec) }}>
        {active && (
          <span className="absolute right-2 top-2 grid size-5 place-items-center rounded-full bg-accent text-black">
            <Check className="size-3.5" strokeWidth={3} />
          </span>
        )}
      </div>
      <div className="flex flex-col px-1">
        <span className="text-small font-medium">{label}</span>
        <span className="text-caption text-text-subtle">{HINTS[name] ?? " "}</span>
      </div>
    </button>
  );
});

export default function LightingView() {
  useSlices("custom_effects");
  const effect = useCurrentEffect();
  const [busy, setBusy] = useState<EffectName | null>(null);
  const [error, setError] = useState<string | null>(null);

  const select = async (name: EffectName) => {
    if (effect?.type === name) return;
    setBusy(name);
    setError(null);
    try {
      // Same behaviour as Classic: switching starts from the effect's defaults.
      const spec = await rpc<EffectSpec>("effects.default", { name });
      await rpc("effects.set", spec);
    } catch (e) {
      setError(e instanceof Error ? e.message : String(e));
    } finally {
      setBusy(null);
    }
  };

  return (
    <>
      <PageHeader title="Lighting" />
      {/* Wide: settings beside the effects. Narrow: settings under "Now running". */}
      <div className="@container">
        <div className="grid items-start gap-6 [grid-template-areas:'now'_'settings'_'effects'] @3xl:grid-cols-[minmax(0,1fr)_300px] @3xl:[grid-template-areas:'now_settings'_'effects_settings']">
          <div className="[grid-area:now]">
            <NowPlaying />
          </div>
          <div className="flex min-w-0 flex-col gap-6 [grid-area:effects]">
            <Section title="Effects" aside={error && <span className="text-caption text-danger-text">{error}</span>}>
              <div data-tour="effects" className="grid grid-cols-[repeat(auto-fill,minmax(128px,1fr))] gap-2">
                {EFFECTS.map(({ name, label }) => (
                  <EffectTile
                    key={name}
                    name={name}
                    label={label}
                    spec={effect?.type === name ? effect : DEFAULTS[name]}
                    active={effect?.type === name}
                    busy={busy !== null}
                    onSelect={select}
                  />
                ))}
              </div>
            </Section>
            <CustomEffects />
          </div>
          <div className="[grid-area:settings] @3xl:sticky @3xl:top-0">
            <Inspector />
          </div>
        </div>
      </div>
    </>
  );
}
