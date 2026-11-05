import * as Popover from "@radix-ui/react-popover";
import { useEffect, useRef, useState, type KeyboardEvent, type PointerEvent } from "react";
import type { Rgb } from "@/lib/api/types";
import { hasMessage, t, type MessageKey } from "@/lib/i18n";
import { hsvToRgb, parseHex, PRESETS, rgbToHsv, sameRgb, toHex, type Hsv } from "@/lib/color";
import { cn } from "@/lib/utils";

const clamp = (v: number, lo = 0, hi = 1) => Math.min(hi, Math.max(lo, v));

/** Preset names come from lib/color (English); show them in the UI language. */
const presetName = (name: string) => {
  const key = `color.preset.${name.toLowerCase()}` as MessageKey;
  return hasMessage(key) ? t(key) : name;
};

/** Pointer drag helper: reports the position inside `el` as 0..1 fractions. */
function useDrag(onMove: (x: number, y: number) => void, onEnd: () => void) {
  const ref = useRef<HTMLDivElement>(null);
  const move = (e: PointerEvent) => {
    const r = ref.current!.getBoundingClientRect();
    onMove(clamp((e.clientX - r.left) / r.width), clamp((e.clientY - r.top) / r.height));
  };
  return {
    ref,
    onPointerDown: (e: PointerEvent<HTMLDivElement>) => {
      e.currentTarget.setPointerCapture(e.pointerId);
      move(e);
    },
    onPointerMove: (e: PointerEvent<HTMLDivElement>) => {
      if (e.currentTarget.hasPointerCapture(e.pointerId)) move(e);
    },
    onPointerUp: onEnd,
  };
}

/**
 * HSV picker. `onChange` fires continuously while dragging, `onCommit` once
 * when a gesture ends — callers throttle the first and flush on the second.
 */
export function ColorPicker({
  value,
  onChange,
  onCommit,
}: {
  value: Rgb;
  onChange: (rgb: Rgb) => void;
  onCommit: (rgb: Rgb) => void;
}) {
  // Keep hue/saturation while value is 0 (black loses them in RGB).
  const [hsv, setHsv] = useState<Hsv>(() => rgbToHsv(value));
  const [hex, setHex] = useState(toHex(value));
  const last = useRef(value);

  useEffect(() => {
    if (!sameRgb(value, last.current)) {
      last.current = value;
      setHsv(rgbToHsv(value));
      setHex(toHex(value));
    }
  }, [value]);

  const apply = (next: Hsv, commit = false) => {
    setHsv(next);
    const rgb = hsvToRgb(next);
    last.current = rgb;
    setHex(toHex(rgb));
    (commit ? onCommit : onChange)(rgb);
  };
  const commitCurrent = () => onCommit(hsvToRgb(hsv));

  const area = useDrag((x, y) => apply({ ...hsv, s: x, v: 1 - y }), commitCurrent);
  const hue = useDrag((x) => apply({ ...hsv, h: x * 359.9 }), commitCurrent);

  const areaKeys = (e: KeyboardEvent) => {
    const step = e.shiftKey ? 0.1 : 0.02;
    const d = { ArrowLeft: [-step, 0], ArrowRight: [step, 0], ArrowUp: [0, step], ArrowDown: [0, -step] }[e.key];
    if (!d) return;
    e.preventDefault();
    apply({ ...hsv, s: clamp(hsv.s + d[0]!), v: clamp(hsv.v + d[1]!) }, true);
  };
  const hueKeys = (e: KeyboardEvent) => {
    const d = { ArrowLeft: -1, ArrowDown: -1, ArrowRight: 1, ArrowUp: 1 }[e.key];
    if (!d) return;
    e.preventDefault();
    apply({ ...hsv, h: (hsv.h + d * (e.shiftKey ? 15 : 3) + 360) % 360 }, true);
  };

  const pure = toHex(hsvToRgb({ h: hsv.h, s: 1, v: 1 }));
  return (
    <div className="flex w-60 flex-col gap-3">
      <div
        {...area}
        role="slider"
        tabIndex={0}
        aria-label={t("color.area")}
        aria-valuemin={0}
        aria-valuemax={100}
        aria-valuenow={Math.round(hsv.s * 100)}
        aria-valuetext={t("color.areaValue", { s: Math.round(hsv.s * 100), v: Math.round(hsv.v * 100) })}
        onKeyDown={areaKeys}
        className="relative h-36 cursor-crosshair touch-none rounded-control"
        style={{ background: `linear-gradient(to top, #000, transparent), linear-gradient(to right, #fff, ${pure})` }}
      >
        <span
          className="pointer-events-none absolute size-4 -translate-x-1/2 -translate-y-1/2 rounded-full border-2 border-white shadow-[0_0_0_1px_rgb(0_0_0/0.4)]"
          style={{ left: `${hsv.s * 100}%`, top: `${(1 - hsv.v) * 100}%`, background: hex }}
        />
      </div>
      <div
        {...hue}
        role="slider"
        tabIndex={0}
        aria-label={t("color.hue")}
        aria-valuemin={0}
        aria-valuemax={360}
        aria-valuenow={Math.round(hsv.h)}
        onKeyDown={hueKeys}
        className="relative h-3 cursor-pointer touch-none rounded-full"
        style={{ background: "linear-gradient(to right, #f00, #ff0, #0f0, #0ff, #00f, #f0f, #f00)" }}
      >
        <span
          className="pointer-events-none absolute top-1/2 size-4 -translate-x-1/2 -translate-y-1/2 rounded-full border-2 border-white shadow-[0_0_0_1px_rgb(0_0_0/0.4)]"
          style={{ left: `${(hsv.h / 360) * 100}%`, background: pure }}
        />
      </div>
      <div className="flex items-center gap-2">
        <span className="size-8 shrink-0 rounded-chip border border-border-strong" style={{ background: hex }} />
        <input
          aria-label={t("color.hex")}
          value={hex}
          spellCheck={false}
          onChange={(e) => {
            setHex(e.target.value);
            const rgb = parseHex(e.target.value);
            if (rgb) {
              last.current = rgb;
              setHsv(rgbToHsv(rgb));
              onCommit(rgb);
            }
          }}
          className="h-8 min-w-0 flex-1 rounded-chip border border-border-strong bg-surface-sunken px-2 font-mono text-small uppercase text-text outline-none focus-visible:border-info"
        />
      </div>
      <div className="flex gap-1.5" role="group" aria-label={t("color.presets")}>
        {PRESETS.map((p) => (
          <button
            key={p.name}
            type="button"
            title={presetName(p.name)}
            aria-label={presetName(p.name)}
            aria-pressed={sameRgb(p.rgb, hsvToRgb(hsv))}
            onClick={() => apply(rgbToHsv(p.rgb), true)}
            className={cn(
              "size-6 rounded-chip border border-border-strong transition-transform duration-(--duration-fast) hover:scale-110",
              sameRgb(p.rgb, hsvToRgb(hsv)) && "ring-2 ring-text ring-offset-2 ring-offset-surface-raised",
            )}
            style={{ background: toHex(p.rgb) }}
          />
        ))}
      </div>
    </div>
  );
}

/** Labelled swatch that opens the picker in a popover. */
export function ColorField({
  label,
  value,
  onChange,
  onCommit,
}: {
  label: string;
  value: Rgb;
  onChange: (rgb: Rgb) => void;
  onCommit: (rgb: Rgb) => void;
}) {
  return (
    <Popover.Root>
      <Popover.Trigger asChild>
        <button
          type="button"
          aria-label={`${label}: ${toHex(value)}`}
          className="flex h-10 w-full items-center gap-3 rounded-control px-2 text-left transition-colors duration-(--duration-fast) hover:bg-surface-raised data-[state=open]:bg-surface-raised"
        >
          <span className="size-6 shrink-0 rounded-chip border border-border-strong" style={{ background: toHex(value) }} />
          <span className="flex-1 text-body">{label}</span>
          <span className="font-mono text-caption uppercase text-text-subtle">{toHex(value)}</span>
        </button>
      </Popover.Trigger>
      <Popover.Portal>
        <Popover.Content
          side="left"
          align="start"
          sideOffset={12}
          collisionPadding={12}
          className="z-50 rounded-card bg-surface-raised p-3 shadow-popover outline-none data-[state=open]:animate-[pop-in_var(--duration-base)_var(--ease-out)]"
        >
          <ColorPicker value={value} onChange={onChange} onCommit={onCommit} />
        </Popover.Content>
      </Popover.Portal>
    </Popover.Root>
  );
}
