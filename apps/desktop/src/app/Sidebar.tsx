import { useRef, type KeyboardEvent } from "react";
import { BrandTiles } from "@/components/brand";
import { StatusDot } from "@/components/status";
import { useDaemon } from "@/lib/store/daemon";
import { cn } from "@/lib/utils";
import { useNavigation } from "./navigation";
import { VIEWS, type ViewDef } from "./views";

function NavItem({ view, active, onSelect }: { view: ViewDef; active: boolean; onSelect: () => void }) {
  const Icon = view.icon;
  return (
    <button
      type="button"
      data-nav
      aria-current={active ? "page" : undefined}
      onClick={onSelect}
      className={cn(
        "group flex h-9 w-full items-center gap-3 rounded-control px-3 text-body transition-colors duration-(--duration-fast)",
        active ? "bg-surface-raised text-text" : "text-text-muted hover:bg-surface hover:text-text",
      )}
    >
      <Icon className={cn("size-4", active ? "text-text" : "text-text-subtle group-hover:text-text-muted")} />
      <span className="flex-1 text-left">{view.label}</span>
      {view.planned && <span className="text-caption text-text-disabled">{view.planned}</span>}
    </button>
  );
}

function ServiceStatus() {
  const connection = useDaemon((s) => s.connection);
  const fps = useDaemon((s) => s.diagnostics?.measured_fps);
  const mode = useDaemon((s) => s.mode);
  if (mode === "mock") return <StatusDot tone="warn" label="Mock data" />;
  if (!connection) return <StatusDot tone="idle" label="Connecting…" />;
  if (!connection.connected) return <StatusDot tone="error" label="Service offline" />;
  return (
    <StatusDot
      tone="ok"
      label={fps === undefined ? "Service running" : `Running · ${fps.toFixed(0)} fps`}
      className="tabular"
    />
  );
}

export function Sidebar() {
  const { view, go } = useNavigation();
  const nav = useRef<HTMLElement>(null);

  // Arrow keys move focus between items; Enter/Space select (native buttons).
  const onKeyDown = (event: KeyboardEvent) => {
    if (event.key !== "ArrowDown" && event.key !== "ArrowUp") return;
    const items = [...(nav.current?.querySelectorAll<HTMLButtonElement>("[data-nav]") ?? [])];
    const index = items.indexOf(document.activeElement as HTMLButtonElement);
    const next = items[(index + (event.key === "ArrowDown" ? 1 : -1) + items.length) % items.length];
    next?.focus();
    event.preventDefault();
  };

  const main = VIEWS.filter((v) => !v.footer);
  const footer = VIEWS.filter((v) => v.footer);
  return (
    <nav ref={nav} onKeyDown={onKeyDown} aria-label="Main" className="flex w-60 shrink-0 flex-col gap-1 bg-bg px-3 pb-3 pt-5">
      <div className="mb-6 flex items-center gap-3 px-3">
        <BrandTiles size={9} />
        <span className="text-title font-semibold tracking-tight">freeRGB</span>
      </div>
      {main.map((v) => (
        <NavItem key={v.id} view={v} active={view === v.id} onSelect={() => go(v.id)} />
      ))}
      <div className="flex-1" />
      {footer.map((v) => (
        <NavItem key={v.id} view={v} active={view === v.id} onSelect={() => go(v.id)} />
      ))}
      <div className="mt-2 border-t border-border px-3 pt-3">
        <ServiceStatus />
      </div>
    </nav>
  );
}
