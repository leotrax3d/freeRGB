import {
  Cable,
  Check,
  CircleDot,
  CircleAlert,
  CircleSlash,
  Copy,
  Cpu,
  Download,
  Headphones,
  Keyboard,
  Laptop,
  LoaderCircle,
  Mouse,
  RotateCw,
  ShieldCheck,
  Speaker,
  Unplug,
  type LucideIcon,
} from "lucide-react";
import { memo } from "react";
import { Checkbox } from "@/components/ui/checkbox";
import { Button } from "@/components/ui/button";
import { toast } from "@/components/toast";
import type { ModuleDeviceKind, SetupCard, SetupPhase } from "@/lib/api/types";
import { cn } from "@/lib/utils";
import { t } from "./messages";
import { bytes, errorHint, PHASES } from "./model";

const KIND_ICONS: Partial<Record<ModuleDeviceKind, LucideIcon>> = {
  keyboard: Keyboard,
  keypad: Keyboard,
  mouse: Mouse,
  headset: Headphones,
  headset_stand: Headphones,
  speaker: Speaker,
  laptop: Laptop,
  lighting_controller: Cable,
};

const PHASE_ICONS: Partial<Record<SetupPhase, LucideIcon>> = {
  detected: CircleDot,
  ready: Check,
  failed: CircleAlert,
  downloading: Download,
  verifying: ShieldCheck,
  not_connected: Unplug,
  unsupported: CircleSlash,
};

const TONE_TEXT = {
  ok: "text-accent",
  warn: "text-text",
  error: "text-danger-text",
  idle: "text-text-subtle",
} as const;

/** Phase line: icon + text, never colour alone. Busy phases spin. */
function PhaseLine({ card }: { card: SetupCard }) {
  const info = PHASES[card.phase];
  const Icon = info.busy ? LoaderCircle : (PHASE_ICONS[card.phase] ?? Check);
  return (
    <span
      // Re-keyed per phase so each change gets one short, calm transition.
      key={card.phase}
      className={cn("inline-flex items-center gap-1.5 text-small animate-[fade-in_var(--duration-base)_var(--ease-out)]", TONE_TEXT[info.tone])}
    >
      <Icon
        className={cn(
          "size-3.5",
          info.busy && "animate-spin",
          card.phase === "ready" && "animate-[pop-in_var(--duration-slow)_var(--ease-out)]",
        )}
        aria-hidden
      />
      {card.builtin && card.phase === "ready" ? t("setup.readyBuiltin") : t(info.label)}
    </span>
  );
}

function Progress({ card }: { card: SetupCard }) {
  // Only measured values: no bar without a known total.
  if (card.phase !== "downloading" || !card.progress || card.progress.total <= 0) return null;
  const { done, total } = card.progress;
  const pct = Math.round((done / total) * 100);
  return (
    <div className="flex flex-col gap-1">
      <div
        className="h-1.5 overflow-hidden rounded-full bg-surface-raised"
        role="progressbar"
        aria-label={t("setup.card.download", { name: card.name })}
        aria-valuemin={0}
        aria-valuemax={total}
        aria-valuenow={done}
      >
        <div className="h-full rounded-full bg-accent transition-[width] duration-(--duration-base)" style={{ width: `${pct}%` }} />
      </div>
      <span className="text-caption text-text-subtle tabular">
        {t("setup.card.of", { done: bytes(done), total: bytes(total) })}
        {card.module && t("setup.card.shared")}
      </span>
    </div>
  );
}

async function copyInfo(card: SetupCard) {
  try {
    await navigator.clipboard.writeText(card.technical);
    toast(t("setup.card.copied"));
  } catch {
    toast(t("setup.card.clipboardFailed"), "error");
  }
}

export const SetupCardView = memo(function SetupCardView({
  card,
  index = 0,
  selected,
  onSelect,
  onRetry,
  compact = false,
}: {
  card: SetupCard;
  /** Position for the staggered entrance. */
  index?: number;
  /** Shown as a checkbox when defined. */
  selected?: boolean;
  onSelect?: (selected: boolean) => void;
  onRetry?: () => void;
  compact?: boolean;
}) {
  const Icon = KIND_ICONS[card.kind] ?? Cpu;
  const hint = errorHint(card);
  return (
    <article
      aria-label={`${card.name}: ${t(PHASES[card.phase].label)}`}
      className={cn(
        "flex flex-col gap-3 rounded-card border bg-surface-sunken animate-[pop-in_var(--duration-slow)_var(--ease-out)_both]",
        compact ? "p-3" : "p-4",
        card.phase === "ready" && !card.builtin ? "border-accent/40" : "border-border",
        "transition-[border-color] duration-(--duration-base)",
      )}
      style={{ animationDelay: `${Math.min(index, 8) * 60}ms` }}
    >
      <header className="flex items-start gap-3">
        <div className="grid size-10 shrink-0 place-items-center rounded-control bg-surface-raised text-text-muted">
          <Icon className="size-5" aria-hidden />
        </div>
        <div className="flex min-w-0 flex-1 flex-col gap-0.5">
          <h3 className="truncate text-body font-medium" title={card.name}>
            {card.name}
          </h3>
          <PhaseLine card={card} />
        </div>
        {selected !== undefined && onSelect && (
          <Checkbox checked={selected} onChange={onSelect} className="mt-1">
            <span className="sr-only">{t("setup.card.setUp", { name: card.name })}</span>
          </Checkbox>
        )}
      </header>
      <Progress card={card} />
      {card.phase === "detected" && card.download_size !== null && !card.installed && selected !== undefined && (
        <p className="text-caption text-text-subtle">{t("setup.card.needs", { size: bytes(card.download_size) })}</p>
      )}
      {card.phase === "detected" && card.installed && <p className="text-caption text-text-subtle">{t("setup.card.installed")}</p>}
      {card.phase === "failed" && (
        <div className="flex flex-col gap-2">
          <p className="rounded-chip bg-danger/10 px-2.5 py-1.5 text-caption text-danger-text">{hint}</p>
          <div className="flex items-center gap-2">
            {onRetry && (
              <Button size="sm" onClick={onRetry}>
                <RotateCw aria-hidden /> {t("setup.card.retry")}
              </Button>
            )}
            <details className="text-caption text-text-subtle">
              <summary className="cursor-pointer select-none">{t("setup.card.details")}</summary>
              <p className="mt-1 select-text break-words font-mono">{card.error?.message}</p>
            </details>
          </div>
        </div>
      )}
      {card.phase === "unsupported" && (
        <div className="flex items-center justify-between gap-2">
          <p className="text-caption text-text-subtle">{t("setup.card.cannot")}</p>
          <Button size="sm" variant="ghost" onClick={() => void copyInfo(card)}>
            <Copy aria-hidden /> {t("setup.card.copy")}
          </Button>
        </div>
      )}
    </article>
  );
});
