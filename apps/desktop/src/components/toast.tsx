import { CircleAlert, CircleCheck } from "lucide-react";
import { useEffect } from "react";
import { create } from "zustand";
import { cn } from "@/lib/utils";

type Tone = "success" | "error";
interface Toast {
  id: number;
  message: string;
  tone: Tone;
}

const useToasts = create<{ toasts: Toast[] }>(() => ({ toasts: [] }));
let next = 0;

/** Brief confirmation for an action; errors stay a little longer. Max three at once. */
export function toast(message: string, tone: Tone = "success") {
  const id = ++next;
  useToasts.setState((s) => ({ toasts: [...s.toasts.slice(-2), { id, message, tone }] }));
  setTimeout(() => useToasts.setState((s) => ({ toasts: s.toasts.filter((t) => t.id !== id) })), tone === "error" ? 6000 : 2500);
}

function Item({ t }: { t: Toast }) {
  const Icon = t.tone === "error" ? CircleAlert : CircleCheck;
  // Dismiss on Escape for keyboard users.
  useEffect(() => {
    const onKey = (e: KeyboardEvent) => e.key === "Escape" && useToasts.setState((s) => ({ toasts: s.toasts.filter((x) => x.id !== t.id) }));
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [t.id]);
  return (
    <div className="pointer-events-auto flex items-center gap-2.5 rounded-control bg-surface-raised px-4 py-2.5 text-small text-text shadow-popover animate-[toast-in_var(--duration-base)_var(--ease-out)]">
      <Icon className={cn("size-4 shrink-0", t.tone === "error" ? "text-danger-text" : "text-accent")} />
      {t.message}
    </div>
  );
}

export function Toaster() {
  const toasts = useToasts((s) => s.toasts);
  return (
    <div aria-live="polite" role="status" className="pointer-events-none fixed bottom-5 left-1/2 z-50 flex -translate-x-1/2 flex-col items-center gap-2">
      {toasts.map((t) => (
        <Item key={t.id} t={t} />
      ))}
    </div>
  );
}
