import { Check, Sun } from "lucide-react";
import { memo, useState } from "react";
import { PageHeader, Section } from "@/components/page";
import { Slider } from "@/components/ui/slider";
import { EFFECTS, effectLabel, effectPreview } from "@/lib/api/effects";
import fixtures from "@/lib/api/fixtures/effects.json";
import type { EffectName, EffectSpec } from "@/lib/api/types";
import { rpc, useDaemon } from "@/lib/store/daemon";
import { useDaemonValue } from "@/lib/store/useDaemonValue";
import { cn } from "@/lib/utils";

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
  const effect = useDaemon((s) => s.effect);
  const devices = useDaemon((s) => s.devices);
  const active = devices?.filter((d) => d.status === "Connected").length ?? 0;
  return (
    <div className="flex items-center gap-6 rounded-card bg-surface-sunken p-5">
      <div
        className="h-20 w-40 shrink-0 rounded-control"
        style={{ background: effect ? effectPreview(effect) : "#303030" }}
        aria-hidden
      />
      <div className="flex flex-col gap-1">
        <span className="text-caption uppercase tracking-wider text-text-subtle">Now running</span>
        <span className="text-section font-semibold">{effect ? effectLabel(effect.type) : "—"}</span>
        <span className="text-small text-text-muted">
          on {active} {active === 1 ? "device" : "devices"} · keeps running when this window is closed
        </span>
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
        "group flex flex-col gap-3 rounded-card p-2 pb-3 text-left transition-colors duration-(--duration-fast)",
        active ? "bg-surface-raised ring-1 ring-accent/60" : "hover:bg-surface-raised",
      )}
    >
      <div className="relative h-20 overflow-hidden rounded-control" style={{ background: effectPreview(spec) }}>
        {active && (
          <span className="absolute right-2 top-2 grid size-5 place-items-center rounded-full bg-accent text-black">
            <Check className="size-3.5" strokeWidth={3} />
          </span>
        )}
      </div>
      <div className="flex flex-col px-1">
        <span className="text-body font-medium">{label}</span>
        <span className="text-caption text-text-subtle">{HINTS[name] ?? " "}</span>
      </div>
    </button>
  );
});

export default function LightingView() {
  const effect = useDaemon((s) => s.effect);
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
      <PageHeader title="Lighting" description="Pick an effect for all devices." actions={<Brightness />} />
      <NowPlaying />
      <Section title="Effects" aside={error && <span className="text-caption text-danger-text">{error}</span>}>
        <div className="grid grid-cols-[repeat(auto-fill,minmax(168px,1fr))] gap-2">
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
        <p className="text-caption text-text-subtle">Effect parameters, custom layers and profiles follow in the next step (M2).</p>
      </Section>
    </>
  );
}
