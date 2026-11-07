import { ChevronDown, ChevronUp, Plus, Trash2 } from "lucide-react";
import { Checkbox } from "@/components/ui/checkbox";
import { Select } from "@/components/ui/select";
import { useState, type ReactNode } from "react";
import { RegionCanvas } from "@/components/region-canvas";
import { Button } from "@/components/ui/button";
import { Slider } from "@/components/ui/slider";
import { EFFECTS, effectLabel, effectPreview } from "@/lib/api/effects";
import type { EffectName, Layer } from "@/lib/api/types";
import { FULL } from "@/lib/region";
import { cn } from "@/lib/utils";
import { EffectFields } from "./EffectFields";
import { t } from "./messages";
import { addLayer, BLENDS, MAX_LAYERS, moveLayer, removeLayer, setLayerEffect, updateLayer } from "./layers";

type Change = (layers: Layer[], commit: boolean) => void;

function Labelled({ label, children }: { label: string; children: ReactNode }) {
  return (
    <label className="flex flex-col gap-1">
      <span className="text-caption text-text-subtle">{label}</span>
      {children}
    </label>
  );
}

function LayerCard({
  layers,
  index,
  open,
  onToggle,
  onChange,
  onMove,
  onRemove,
}: {
  layers: Layer[];
  index: number;
  open: boolean;
  onToggle: () => void;
  onChange: Change;
  onMove: (by: number) => void;
  onRemove: () => void;
}) {
  const layer = layers[index]!;
  const set = (patch: Partial<Layer>, commit = true) => onChange(updateLayer(layers, index, patch), commit);
  const isTop = index === layers.length - 1;
  const isBottom = index === 0;

  return (
    <li className={cn("rounded-control border transition-colors", open ? "border-border-strong bg-surface" : "border-border")}>
      <div className="flex items-center gap-2 p-1.5">
        <button
          type="button"
          onClick={onToggle}
          aria-expanded={open}
          className="flex min-w-0 flex-1 items-center gap-2.5 rounded-chip px-1 py-1 text-start"
        >
          <span className="h-6 w-9 shrink-0 rounded-chip" style={{ background: effectPreview(layer.effect) }} />
          <span className="flex min-w-0 flex-col">
            <span className="truncate text-small font-medium">{effectLabel(layer.effect.type)}</span>
            <span className="text-caption text-text-subtle tabular">
              {t(BLENDS.find((b) => b.mode === layer.blend)!.label)} · {Math.round(layer.opacity * 100)}%
              {layer.region ? t("lighting.layer.part") : ""}
            </span>
          </span>
        </button>
        <div className="flex">
          <Button size="icon" variant="ghost" className="size-7" aria-label={t("lighting.layer.up")} disabled={isTop} onClick={() => onMove(1)}>
            <ChevronUp />
          </Button>
          <Button size="icon" variant="ghost" className="size-7" aria-label={t("lighting.layer.down")} disabled={isBottom} onClick={() => onMove(-1)}>
            <ChevronDown />
          </Button>
          <Button size="icon" variant="ghost" className="size-7" aria-label={t("lighting.layer.remove")} disabled={layers.length <= 1} onClick={onRemove}>
            <Trash2 />
          </Button>
        </div>
      </div>

      {open && (
        <div className="flex flex-col gap-3 border-t border-border px-2 pb-3 pt-3">
          <div className="grid grid-cols-2 gap-2 px-2">
            <Labelled label={t("lighting.layer.effect")}>
              <Select
                label={t("lighting.layer.effect")}
                className="w-full"
                value={layer.effect.type}
                onChange={(name) => onChange(setLayerEffect(layers, index, name), true)}
                options={EFFECTS.filter((e) => e.name !== "layers").map((e) => ({ value: e.name as Exclude<EffectName, "layers">, label: effectLabel(e.name) }))}
              />
            </Labelled>
            <Labelled label={t("lighting.layer.blend")}>
              <Select label={t("lighting.layer.blend")} className="w-full" value={layer.blend} onChange={(blend) => set({ blend })} options={BLENDS.map((b) => ({ value: b.mode, label: t(b.label) }))} />
            </Labelled>
          </div>
          <div className="flex flex-col gap-2 px-2">
            <div className="flex justify-between text-small">
              <span>{t("lighting.layer.opacity")}</span>
              <span className="text-text-muted tabular">{Math.round(layer.opacity * 100)}%</span>
            </div>
            <Slider
              aria-label={t("lighting.layer.opacity")}
              min={0}
              max={1}
              step={0.01}
              value={[layer.opacity]}
              onValueChange={([v]) => set({ opacity: v! }, false)}
              onValueCommit={([v]) => set({ opacity: v! }, true)}
            />
          </div>
          <EffectFields
            spec={layer.effect}
            onChange={(key, value, commit) => set({ effect: { ...layer.effect, [key]: value } as Layer["effect"] }, commit)}
          />
          <div className="flex flex-col gap-2 px-2">
            <Checkbox checked={!!layer.region} onChange={(on) => set({ region: on ? { x: 0, y: 0, w: 0.5, h: 1 } : null })}>
              {t("lighting.layer.region")}
            </Checkbox>
            {layer.region && (
              <RegionCanvas
                compact
                items={[{ id: "layer", label: t("lighting.layer.area"), color: "#a855f7", region: layer.region ?? FULL }]}
                selected="layer"
                background={effectPreview(layer.effect)}
                onChange={(_, region, commit) => set({ region }, commit)}
              />
            )}
          </div>
        </div>
      )}
    </li>
  );
}

/** Editor for a custom layered effect. Shows the top layer first, like design tools. */
export function LayerStack({ layers, onChange }: { layers: Layer[]; onChange: Change }) {
  const [open, setOpen] = useState<number | null>(layers.length - 1);
  const move = (index: number, by: number) => {
    const next = moveLayer(layers, index, by);
    if (next !== layers && open === index) setOpen(index + by);
    onChange(next, true);
  };
  const remove = (index: number) => {
    setOpen(null);
    onChange(removeLayer(layers, index), true);
  };
  const add = () => {
    const next = addLayer(layers);
    setOpen(next.length - 1);
    onChange(next, true);
  };
  const order = layers.map((_, i) => i).reverse();
  return (
    <div className="flex flex-col gap-2">
      <ol className="flex flex-col gap-1.5" aria-label={t("lighting.layer.list")}>
        {order.map((i) => (
          <LayerCard key={i} layers={layers} index={i} open={open === i} onToggle={() => setOpen(open === i ? null : i)} onChange={onChange} onMove={(by) => move(i, by)} onRemove={() => remove(i)} />
        ))}
      </ol>
      <Button variant="ghost" size="sm" disabled={layers.length >= MAX_LAYERS} onClick={add}>
        <Plus aria-hidden /> {t("lighting.layer.add")} {layers.length >= MAX_LAYERS && t("lighting.layer.max", { max: MAX_LAYERS })}
      </Button>
      <p className="px-2 text-caption text-text-subtle">{t("lighting.layer.note")}</p>
    </div>
  );
}
