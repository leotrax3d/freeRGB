import { CircleAlert, TriangleAlert } from "lucide-react";
import type { ReactNode } from "react";
import { t } from "@/lib/i18n";
import { cn } from "@/lib/utils";

export type Tone = "ok" | "warn" | "error" | "idle";

const DOT: Record<Tone, string> = {
  ok: "bg-accent",
  warn: "bg-warning",
  error: "bg-danger",
  idle: "bg-text-disabled",
};

/** Colour is never the only signal: the dot always comes with a label. */
export function StatusDot({ tone, label, className }: { tone: Tone; label: string; className?: string }) {
  return (
    <span className={cn("inline-flex items-center gap-2 text-small text-text-muted", className)}>
      <span className={cn("size-2 shrink-0 rounded-full", DOT[tone])} aria-hidden />
      {label}
    </span>
  );
}

export function EmptyState({
  icon,
  title,
  children,
  action,
}: {
  icon?: ReactNode;
  title: string;
  children?: ReactNode;
  action?: ReactNode;
}) {
  return (
    <div className="mx-auto flex max-w-sm flex-col items-center gap-3 py-16 text-center">
      {icon && <div className="mb-1 text-text-subtle [&_svg]:size-8">{icon}</div>}
      <h2 className="text-title font-semibold">{title}</h2>
      {children && <p className="text-small text-text-muted">{children}</p>}
      {action && <div className="mt-2">{action}</div>}
    </div>
  );
}

export function Banner({ tone, children }: { tone: "warn" | "error" | "info"; children: ReactNode }) {
  const styles = {
    warn: "border-warning/30 bg-warning/10 text-[#fbbf24]",
    error: "border-danger/30 bg-danger/10 text-danger-text",
    info: "border-info/30 bg-info/10 text-info-text",
  }[tone];
  return (
    <div role={tone === "error" ? "alert" : "status"} className={cn("flex items-center gap-3 rounded-control border px-4 py-2.5 text-small", styles)}>
      {children}
    </div>
  );
}

/**
 * Inline error next to the control that failed. Announced to screen readers
 * (role="alert") and never colour-only: the icon and text carry the meaning.
 */
export function InlineError({ children, className }: { children: ReactNode; className?: string }) {
  return (
    <p role="alert" className={cn("flex items-start gap-1.5 text-caption text-danger-text", className)}>
      <CircleAlert className="mt-px size-3.5 shrink-0" aria-hidden />
      <span className="min-w-0 break-words">{children}</span>
    </p>
  );
}

/** Block-level error with an optional action (retry), for a section that could not load or save. */
export function ErrorState({ title, children, action }: { title?: string; children: ReactNode; action?: ReactNode }) {
  return (
    <div role="alert" className="flex flex-col gap-2 rounded-control border border-danger/30 bg-danger/10 p-3 text-small text-danger-text">
      <p className="flex items-start gap-2 font-medium">
        <TriangleAlert className="mt-0.5 size-4 shrink-0" aria-hidden />
        <span>{title ?? t("common.errorTitle")}</span>
      </p>
      <div className="break-words ps-6 text-text-muted">{children}</div>
      {action && <div className="ps-6">{action}</div>}
    </div>
  );
}
