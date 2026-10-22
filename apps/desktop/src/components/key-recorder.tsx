import { Keyboard, X } from "lucide-react";
import { useEffect, useRef, useState, type KeyboardEvent } from "react";
import { cn } from "@/lib/utils";

const MODIFIER_ORDER = ["ctrl", "alt", "shift", "meta"] as const;

const LABELS: Record<string, string> = {
  ctrl: "Ctrl",
  alt: "Alt",
  shift: "Shift",
  meta: "Win",
  enter: "Enter",
  escape: "Esc",
  tab: "Tab",
  space: "Space",
  backspace: "⌫",
  up: "↑",
  down: "↓",
  left: "←",
  right: "→",
};

/** Browser key → freeRGB key name (lowercase, as used by macros/hotkeys). */
export function keyName(e: Pick<KeyboardEvent, "key" | "code">): string | null {
  if (/^Key[A-Z]$/.test(e.code)) return e.code.slice(3).toLowerCase();
  if (/^Digit[0-9]$/.test(e.code)) return e.code.slice(5);
  if (/^F([1-9]|1[0-9]|2[0-4])$/.test(e.key)) return e.key.toLowerCase();
  const named: Record<string, string> = {
    Enter: "enter",
    Escape: "escape",
    Tab: "tab",
    " ": "space",
    Backspace: "backspace",
    ArrowUp: "up",
    ArrowDown: "down",
    ArrowLeft: "left",
    ArrowRight: "right",
  };
  return named[e.key] ?? null;
}

export function KeyCap({ k, className }: { k: string; className?: string }) {
  return (
    <kbd
      className={cn(
        "inline-grid h-7 min-w-7 place-items-center rounded-chip border border-border-strong border-b-2 bg-surface-raised px-2 font-sans text-small font-medium text-text shadow-[0_1px_0_rgb(0_0_0/0.4)]",
        className,
      )}
    >
      {LABELS[k] ?? k.toUpperCase()}
    </kbd>
  );
}

/**
 * Click, then press a combination: modifiers plus one key, shown as key caps.
 * `accept` decides which keys are allowed (e.g. F6–F10 for global shortcuts).
 * Escape alone cancels recording; Backspace alone clears.
 */
export function KeyRecorder({
  value,
  onChange,
  accept,
  label,
  placeholder = "Click and press keys",
}: {
  value: string[];
  onChange: (keys: string[]) => void;
  accept?: (key: string) => boolean;
  label: string;
  placeholder?: string;
}) {
  const [recording, setRecording] = useState(false);
  const [live, setLive] = useState<string[]>([]);
  const box = useRef<HTMLButtonElement>(null);

  useEffect(() => {
    if (!recording) setLive([]);
  }, [recording]);

  const onKeyDown = (e: KeyboardEvent) => {
    if (!recording) {
      if (e.key === "Enter" || e.key === " ") {
        e.preventDefault();
        setRecording(true);
      }
      return;
    }
    e.preventDefault();
    e.stopPropagation();
    const mods = MODIFIER_ORDER.filter((m) => (m === "ctrl" ? e.ctrlKey : m === "alt" ? e.altKey : m === "shift" ? e.shiftKey : e.metaKey));
    const key = keyName(e);
    if (!key) return setLive([...mods]); // only modifiers held so far
    if (key === "escape" && mods.length === 0) return setRecording(false);
    if (key === "backspace" && mods.length === 0) {
      onChange([]);
      return setRecording(false);
    }
    if (accept && !accept(key)) return setLive([...mods, key]);
    onChange([...mods, key]);
    setRecording(false);
  };

  const shown = recording ? live : value;
  const rejected = recording && live.length > 0 && accept && !accept(live[live.length - 1]!) && !MODIFIER_ORDER.includes(live[live.length - 1] as never);
  return (
    <div className="inline-flex items-center gap-1">
      <button
        ref={box}
        type="button"
        aria-label={`${label}: ${value.length ? value.join(" + ") : "none"}. Press Enter, then the keys.`}
        onClick={() => setRecording(true)}
        onBlur={() => setRecording(false)}
        onKeyDown={onKeyDown}
        className={cn(
          "flex h-10 min-w-44 items-center gap-1.5 rounded-control border px-2 text-left outline-none transition-colors duration-(--duration-fast)",
          recording ? "border-info bg-info/10" : "border-dashed border-border-strong hover:border-text-subtle",
        )}
      >
        {shown.length ? (
          shown.map((k, i) => (
            <span key={k} className="flex items-center gap-1.5">
              {i > 0 && <span className="text-caption text-text-subtle">+</span>}
              <KeyCap k={k} className={rejected && i === shown.length - 1 ? "border-danger text-danger-text" : undefined} />
            </span>
          ))
        ) : (
          <span className="flex items-center gap-2 px-1 text-small text-text-subtle">
            <Keyboard className="size-4" />
            {recording ? "Press keys…" : placeholder}
          </span>
        )}
        {recording && shown.length > 0 && <span className="ml-1 text-caption text-info-text">…</span>}
      </button>
      {value.length > 0 && !recording && (
        <button type="button" aria-label={`Clear ${label}`} onClick={() => onChange([])} className="rounded-chip p-1 text-text-subtle hover:text-text">
          <X className="size-3.5" />
        </button>
      )}
    </div>
  );
}
