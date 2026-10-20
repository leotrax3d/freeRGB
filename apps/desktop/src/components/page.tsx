import type { ReactNode } from "react";

export function PageHeader({ title, description, actions }: { title: string; description?: ReactNode; actions?: ReactNode }) {
  return (
    <header className="flex items-end justify-between gap-6">
      <div className="flex flex-col gap-1">
        <h1 className="text-page font-semibold tracking-tight">{title}</h1>
        {description && <p className="text-body text-text-muted">{description}</p>}
      </div>
      {actions}
    </header>
  );
}

export function Section({ title, aside, children }: { title: string; aside?: ReactNode; children: ReactNode }) {
  return (
    <section className="flex flex-col gap-3">
      <div className="flex items-baseline justify-between">
        <h2 className="text-small font-medium text-text-subtle">{title}</h2>
        {aside}
      </div>
      {children}
    </section>
  );
}
