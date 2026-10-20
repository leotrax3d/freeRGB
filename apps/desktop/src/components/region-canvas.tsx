import { useRef, type KeyboardEvent, type PointerEvent } from "react";
import type { Region } from "@/lib/api/types";
import { moved, nudged, resized } from "@/lib/region";
import { cn } from "@/lib/utils";

export interface CanvasItem {
  id: string;
  label: string;
  /** CSS colour used for the outline and fill tint. */
  color: string;
  region: Region;
  muted?: boolean;
}

type Gesture = { id: string; mode: "move" | "resize"; start: Region; px: number; py: number };

/**
 * The normalised effect canvas with draggable rectangles. Drag to move, drag
 * the corner to resize (Alt disables grid snapping). Focused items accept
 * arrow keys (Shift = larger steps, Alt = resize). `onChange(..., true)`
 * marks the end of a gesture.
 */
export function RegionCanvas({
  items,
  selected,
  background,
  aspect = 16 / 9,
  compact = false,
  onSelect,
  onChange,
}: {
  items: CanvasItem[];
  selected?: string | null;
  background?: string;
  aspect?: number;
  compact?: boolean;
  onSelect?: (id: string) => void;
  onChange: (id: string, region: Region, commit: boolean) => void;
}) {
  const canvas = useRef<HTMLDivElement>(null);
  const gesture = useRef<Gesture | null>(null);

  const delta = (e: PointerEvent) => {
    const g = gesture.current!;
    const rect = canvas.current!.getBoundingClientRect();
    return [(e.clientX - g.px) / rect.width, (e.clientY - g.py) / rect.height] as const;
  };
  const apply = (e: PointerEvent, commit: boolean) => {
    const g = gesture.current;
    if (!g) return;
    const [dx, dy] = delta(e);
    const next = g.mode === "move" ? moved(g.start, dx, dy, !e.altKey) : resized(g.start, dx, dy, !e.altKey);
    onChange(g.id, next, commit);
  };
  const begin = (e: PointerEvent<HTMLElement>, item: CanvasItem, mode: Gesture["mode"]) => {
    if (e.button !== 0) return;
    e.stopPropagation();
    e.currentTarget.setPointerCapture(e.pointerId);
    gesture.current = { id: item.id, mode, start: item.region, px: e.clientX, py: e.clientY };
    onSelect?.(item.id);
  };
  const end = (e: PointerEvent) => {
    apply(e, true);
    gesture.current = null;
  };
  const key = (e: KeyboardEvent, item: CanvasItem) => {
    const next = nudged(item.region, e.key, e.shiftKey ? 0.05 : 0.01, e.altKey);
    if (!next) return;
    e.preventDefault();
    onChange(item.id, next, true);
  };

  return (
    <div
      ref={canvas}
      className="relative w-full select-none overflow-hidden rounded-card border border-border bg-surface-sunken"
      style={{ aspectRatio: aspect }}
    >
      {background && <div className="absolute inset-0 opacity-25" style={{ background }} aria-hidden />}
      {/* 10 % grid: orientation without competing with the devices. */}
      <div
        className="absolute inset-0"
        aria-hidden
        style={{
          backgroundImage:
            "linear-gradient(rgb(255 255 255 / 0.05) 1px, transparent 1px), linear-gradient(90deg, rgb(255 255 255 / 0.05) 1px, transparent 1px)",
          backgroundSize: "10% 10%",
        }}
      />
      {items.map((item) => {
        const active = item.id === selected;
        const { x, y, w, h } = item.region;
        return (
          <div
            key={item.id}
            role="button"
            tabIndex={0}
            aria-label={`${item.label}: ${Math.round(x * 100)}% from left, ${Math.round(y * 100)}% from top, ${Math.round(w * 100)}% wide, ${Math.round(h * 100)}% high. Arrow keys move, Alt with arrows resizes.`}
            aria-pressed={active}
            onPointerDown={(e) => begin(e, item, "move")}
            onPointerMove={(e) => gesture.current?.mode === "move" && apply(e, false)}
            onPointerUp={end}
            onKeyDown={(e) => key(e, item)}
            onFocus={() => onSelect?.(item.id)}
            className={cn(
              "absolute cursor-grab touch-none rounded-control border-2 active:cursor-grabbing",
              active ? "z-10" : "z-0",
              item.muted && "opacity-50",
            )}
            style={{
              left: `${x * 100}%`,
              top: `${y * 100}%`,
              width: `${w * 100}%`,
              height: `${h * 100}%`,
              borderColor: item.color,
              background: `color-mix(in srgb, ${item.color} ${active ? 28 : 16}%, transparent)`,
              boxShadow: active ? `0 0 0 3px color-mix(in srgb, ${item.color} 35%, transparent)` : undefined,
            }}
          >
            {!compact && (
              <span className="pointer-events-none absolute left-2 top-1.5 right-2 truncate text-caption font-medium text-text">
                {item.label}
              </span>
            )}
            <span
              aria-hidden
              onPointerDown={(e) => begin(e, item, "resize")}
              onPointerMove={(e) => gesture.current?.mode === "resize" && apply(e, false)}
              onPointerUp={end}
              className="absolute -bottom-1 -right-1 size-3.5 cursor-nwse-resize rounded-full border-2 border-black"
              style={{ background: item.color }}
            />
          </div>
        );
      })}
    </div>
  );
}
