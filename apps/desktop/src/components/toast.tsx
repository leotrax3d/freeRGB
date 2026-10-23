import { CircleAlert, CircleCheck, Info, X } from "lucide-react";
import { useEffect, useState } from "react";
import { create } from "zustand";
import { cn } from "@/lib/utils";
import type { Notice } from "@/lib/notifications";

type Tone = Notice["tone"];
interface Toast { id: number; message: string; tone: Tone; details?: string }
const useToasts = create<{ toasts: Toast[] }>(() => ({ toasts: [] }));
let next = 0;
const recent = new Map<string, number>();
function dismiss(id: number) {
  useToasts.setState((s) => ({ toasts: s.toasts.filter((t) => t.id !== id) }));
}

/** Queue a bounded burst; visible messages get their full reading time. */
export function toast(message: string, tone: Tone = "success", details?: string) {
  const key = `${tone}:${message}`;
  const now = Date.now();
  for (const [k, at] of recent) if (now - at >= 30000) recent.delete(k);
  if (recent.has(key)) return;
  recent.set(key, now);
  const id = ++next;
  useToasts.setState((s) => ({ toasts: [...s.toasts, { id, message, tone, details }].slice(-64) }));
}

function Item({ t }: { t: Toast }) {
  const [hovered, setHovered] = useState(false);
  const [focused, setFocused] = useState(false);
  const [expanded, setExpanded] = useState(false);
  const Icon = t.tone === "success" ? CircleCheck : t.tone === "info" ? Info : CircleAlert;
  useEffect(() => {
    if (hovered || focused || expanded) return;
    const timer = setTimeout(() => dismiss(t.id), t.tone === "success" ? 3500 : 10000);
    return () => clearTimeout(timer);
  }, [t.id, t.tone, hovered, focused, expanded]);
  return (
    <div onMouseEnter={() => setHovered(true)} onMouseLeave={() => setHovered(false)}
      onFocusCapture={() => setFocused(true)} onBlurCapture={(e) => { if (!e.currentTarget.contains(e.relatedTarget)) setFocused(false); }}
      onKeyDown={(e) => { if (e.key === "Escape") { e.stopPropagation(); dismiss(t.id); } }}
      className="pointer-events-auto w-full rounded-control border border-border bg-surface-raised px-4 py-3 text-small text-text shadow-popover motion-safe:animate-[toast-in_var(--duration-base)_var(--ease-out)]">
      <div className="flex items-start gap-2.5">
        <Icon aria-hidden className={cn("mt-0.5 size-4 shrink-0", t.tone === "error" || t.tone === "warning" ? "text-danger-text" : "text-accent")} />
        <span className="min-w-0 flex-1 break-words">{t.message}</span>
        <button aria-label="Dismiss notification" onClick={() => dismiss(t.id)} className="rounded-control p-0.5 text-text-muted hover:text-text focus-visible:outline-2 focus-visible:outline-accent"><X className="size-4" /></button>
      </div>
      {t.details && <details className="mt-2 pl-6 text-text-muted" onToggle={(e) => setExpanded(e.currentTarget.open)}>
        <summary className="cursor-pointer">Technical details</summary>
        <pre className="mt-2 max-h-48 overflow-auto whitespace-pre-wrap break-all text-xs">{t.details}</pre>
      </details>}
    </div>
  );
}

export function Toaster() {
  const toasts = useToasts((s) => s.toasts);
  return <div aria-live="polite" aria-relevant="additions" role="status" aria-label="Notifications"
    className="pointer-events-none fixed bottom-5 right-5 z-50 flex w-[min(26rem,calc(100vw-2.5rem))] flex-col gap-2">
    {toasts.slice(0, 3).map((t) => <Item key={t.id} t={t} />)}
  </div>;
}
