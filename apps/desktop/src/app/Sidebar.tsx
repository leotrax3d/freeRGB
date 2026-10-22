import { useRef, type KeyboardEvent } from "react";
import { BrandWordmark } from "@/components/brand";
import { StatusDot } from "@/components/status";
import { useDaemon } from "@/lib/store/daemon";
import { cn } from "@/lib/utils";
import { useNavigation } from "./navigation";
import { Search } from "lucide-react";
import { usePalette } from "./CommandPalette";
import { GROUP_LABELS, VIEWS, type ViewDef } from "./views";

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

  const footer = VIEWS.filter((v) => v.group === "footer");
  const openPalette = usePalette((s) => s.open);
  return (
    <nav ref={nav} onKeyDown={onKeyDown} aria-label="Main" className="flex w-60 shrink-0 flex-col gap-1 bg-bg px-3 pb-3 pt-5">
      <div className="mb-5 flex items-center gap-3 px-3">
        <BrandWordmark />
      </div>
      <button
        type="button"
        onClick={openPalette}
        data-tour="search"
        className="mb-4 flex h-9 items-center gap-2.5 rounded-control border border-border px-3 text-small text-text-subtle transition-colors hover:border-border-strong hover:text-text-muted"
      >
        <Search className="size-4" />
        <span className="flex-1 text-left">Search</span>
        <kbd className="rounded-chip bg-surface px-1.5 font-sans text-caption">Ctrl K</kbd>
      </button>
      <div data-tour="nav" className="flex flex-col">
      {(Object.keys(GROUP_LABELS) as (keyof typeof GROUP_LABELS)[]).map((group) => (
        <div key={group} className="mb-3 flex flex-col gap-0.5">
          <span className="px-3 pb-1 text-caption font-medium text-text-subtle">{GROUP_LABELS[group]}</span>
          {VIEWS.filter((v) => v.group === group).map((v) => (
            <NavItem key={v.id} view={v} active={view === v.id} onSelect={() => go(v.id)} />
          ))}
        </div>
      ))}
      </div>
      <div className="flex-1" />
      {footer.map((v) => (
        <NavItem key={v.id} view={v} active={view === v.id} onSelect={() => go(v.id)} />
      ))}
      <div data-tour="status" className="mt-2 border-t border-border px-3 pt-3">
        <ServiceStatus />
      </div>
    </nav>
  );
}
