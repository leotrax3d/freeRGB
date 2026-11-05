import { Slider } from "@/components/ui/slider";
import type { Package } from "@/features/marketplace/types";
import { NameForm } from "@/components/name-form";
import { InlineError } from "@/components/status";
import { t } from "./messages";
import { effectLabel } from "@/lib/api/effects";
import { rpc, useDaemon } from "@/lib/store/daemon";
import { useAction } from "@/lib/store/useAction";
import { EffectFields } from "./EffectFields";
import { LayerStack } from "./LayerStack";
import { useEffectDraft } from "./useEffectDraft";

function SaveCustom() {
  const effect = useDaemon((s) => s.effect);
  const names = useDaemon((s) => s.custom_effects);
  const { pending, error, run } = useAction();
  if (names === null) return null; // --no-state: no storage
  return (
    <div className="flex flex-col gap-2 border-t border-border pt-4">
      <span className="text-small font-medium text-text-subtle">{t("lighting.custom.save")}</span>
      <NameForm
        label={t("lighting.custom.name")}
        placeholder={t("lighting.custom.namePlaceholder")}
        submit={t("lighting.custom.submit")}
        existing={names ?? []}
        busy={pending !== null}
        onSubmit={(name) => run("save", () => rpc("custom_effects.save", { name, effect }), t("lighting.custom.saved", { name }))}
      />
      {error && <InlineError>{error}</InlineError>}
    </div>
  );
}

export function Inspector({packages=[]}:{packages?:Package[]}) {
  const { effect, set, error } = useEffectDraft();
  if (!effect) return null;
  const pkg=effect.type==="marketplace" ? packages.find(p=>p.id===effect.id && p.version===effect.version) : undefined;

  return (
    <aside data-tour="inspector" aria-label={t("lighting.settings")} className="flex flex-col gap-4 rounded-card bg-surface-sunken p-4">
      <header className="flex flex-col gap-0.5 px-2">
        <h2 className="text-title font-semibold">{pkg?.name ?? effectLabel(effect.type)}</h2>
      </header>
      {effect.type === "marketplace" ? (
        <div className="flex flex-col gap-3 px-2">
          <p className="text-small text-text-muted">{effect.id} · {effect.version}</p>
          {pkg?.parameters.map(p=><div key={p.key} className="flex flex-col gap-2"><label className="flex justify-between gap-3 text-small" htmlFor={`effect-${p.key}`}>{p.label}<output>{(effect.parameters[p.key] ?? p.default).toFixed(2)}</output></label><Slider id={`effect-${p.key}`} aria-label={p.label} min={p.min} max={p.max} step={p.step} value={[effect.parameters[p.key] ?? p.default]} onValueChange={([value])=>set("parameters",{...effect.parameters,[p.key]:value!},false)} onValueCommit={([value])=>set("parameters",{...effect.parameters,[p.key]:value!},true)} /></div>)}
        </div>
      ) : effect.type === "layers" ? (
        <LayerStack layers={effect.layers} onChange={(layers, commit) => set("layers", layers, commit)} />
      ) : (
        <EffectFields spec={effect} onChange={set} />
      )}
      {error && <InlineError className="px-2">{error}</InlineError>}
      <SaveCustom />
    </aside>
  );
}
