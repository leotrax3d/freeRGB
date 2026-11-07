import { Bookmark, CornerDownLeft, Search, Sparkles, Wand2 } from "lucide-react";
import { useEffect, useMemo, useRef, useState, type KeyboardEvent } from "react";
import { create } from "zustand";
import { EFFECTS, effectLabel } from "@/lib/api/effects";
import type { EffectSpec } from "@/lib/api/types";
import { useFocusTrap } from "@/lib/a11y";
import { t } from "@/lib/i18n";
import { requireSlices, rpc, useDaemon } from "@/lib/store/daemon";
import { toast } from "@/components/toast";
import { cn } from "@/lib/utils";
import { markLoaded } from "@/features/profiles/activeProfile";
import { useNavigation } from "./navigation";
import { VIEWS } from "./views";

export const usePalette = create<{ visible: boolean; returnTo: Element | null; open: () => void; close: () => void }>((set) => ({
  visible: false,
  returnTo: null,
  // Remember the focused element before the palette steals focus.
  open: () => set({ visible: true, returnTo: document.activeElement }),
  close: () => set({ visible: false }),
}));

interface Item {
  id: string;
  section: string;
  label: string;
  hint?: string;
  icon: typeof Search;
  run: () => Promise<unknown> | void;
}

/** Ranks by word-prefix match first, then substring; empty query keeps order. */
export function filterItems<T extends { label: string; hint?: string }>(items: T[], query: string): T[] {
  const q = query.trim().toLowerCase();
  if (!q) return items;
  const score = (item: T) => {
    const label = item.label.toLowerCase();
    if (label.startsWith(q)) return 0;
    if (label.split(/\s+/).some((w) => w.startsWith(q))) return 1;
    if (label.includes(q)) return 2;
    if (item.hint?.toLowerCase().includes(q)) return 3;
    return -1;
  };
  return items
    .map((item, index) => ({ item, index, s: score(item) }))
    .filter((x) => x.s >= 0)
    .sort((a, b) => a.s - b.s || a.index - b.index)
    .map((x) => x.item);
}

function useItems(): Item[] {
  const go = useNavigation((s) => s.go);
  const profiles = useDaemon((s) => s.profiles);
  const custom = useDaemon((s) => s.custom_effects);
  return useMemo(() => {
    const items: Item[] = VIEWS.map((v) => ({ id: `view:${v.id}`, section: t("palette.goTo"), label: t(v.label), hint: t(v.hint), icon: v.icon, run: () => go(v.id) }));
    for (const e of EFFECTS.filter((e) => e.name !== "layers")) {
      items.push({
        id: `effect:${e.name}`,
        section: t("palette.effects"),
        label: effectLabel(e.name),
        icon: Sparkles,
        run: async () => {
          await rpc("effects.set", await rpc<EffectSpec>("effects.default", { name: e.name }));
          toast(t("palette.running", { name: effectLabel(e.name) }));
        },
      });
    }
    for (const name of custom ?? []) {
      items.push({
        id: `custom:${name}`,
        section: t("palette.custom"),
        label: name,
        icon: Wand2,
        run: async () => {
          await rpc("effects.set", await rpc<EffectSpec>("custom_effects.get", { name }));
          toast(t("palette.running", { name }));
        },
      });
    }
    for (const name of profiles ?? []) {
      items.push({
        id: `profile:${name}`,
        section: t("palette.profiles"),
        label: name,
        icon: Bookmark,
        run: async () => {
          await rpc("profiles.load", { name });
          await markLoaded(name);
          toast(t("profiles.loaded", { name }));
        },
      });
    }
    return items;
  }, [go, profiles, custom]);
}

function Palette() {
  const close = usePalette((s) => s.close);
  const all = useItems();
  const [query, setQuery] = useState("");
  const [active, setActive] = useState(0);
  const input = useRef<HTMLInputElement>(null);
  const list = useRef<HTMLUListElement>(null);
  const dialog = useRef<HTMLDivElement>(null);
  useFocusTrap(dialog);
  const items = filterItems(all, query);

  useEffect(() => requireSlices(["profiles", "custom_effects"]), []);
  // Return focus to wherever it was when the palette opened.
  useEffect(() => {
    const previous = usePalette.getState().returnTo;
    return () => {
      if (previous instanceof HTMLElement) previous.focus();
    };
  }, []);
  useEffect(() => setActive(0), [query]);
  useEffect(() => {
    list.current?.querySelector(`[data-index="${active}"]`)?.scrollIntoView({ block: "nearest" });
  }, [active]);

  const choose = (item: Item | undefined) => {
    if (!item) return;
    close();
    Promise.resolve(item.run()).catch((e: unknown) => toast(e instanceof Error ? e.message : String(e), "error"));
  };
  const onKeyDown = (e: KeyboardEvent) => {
    if (e.key === "ArrowDown") setActive((a) => Math.min(items.length - 1, a + 1));
    else if (e.key === "ArrowUp") setActive((a) => Math.max(0, a - 1));
    else if (e.key === "Enter") choose(items[active]);
    else if (e.key === "Escape") close();
    else return;
    e.preventDefault();
  };

  let section = "";
  return (
    <div className="fixed inset-0 z-50 flex items-start justify-center bg-black/50 pt-[12vh] animate-[fade-in_var(--duration-fast)_var(--ease-out)]" onMouseDown={close}>
      <div
        ref={dialog}
        role="dialog"
        aria-modal="true"
        aria-label={t("palette.title")}
        onMouseDown={(e) => e.stopPropagation()}
        className="flex w-full max-w-lg flex-col overflow-hidden rounded-panel bg-surface-raised shadow-popover animate-[pop-in_var(--duration-base)_var(--ease-out)]"
      >
        <div className="flex items-center gap-3 border-b border-border px-4">
          <Search className="size-4 text-text-subtle" aria-hidden />
          <input
            ref={input}
            autoFocus
            value={query}
            onChange={(e) => setQuery(e.target.value)}
            onKeyDown={onKeyDown}
            placeholder={t("palette.placeholder")}
            role="combobox"
            aria-expanded="true"
            aria-autocomplete="list"
            aria-label={t("palette.placeholder")}
            aria-controls="palette-list"
            aria-activedescendant={items[active] ? `palette-${items[active].id}` : undefined}
            className="h-12 flex-1 bg-transparent text-body text-text outline-none placeholder:text-text-subtle"
          />
          <kbd className="rounded-chip bg-surface px-1.5 text-caption text-text-subtle" aria-hidden>Esc</kbd>
        </div>
        <ul ref={list} id="palette-list" role="listbox" aria-label={t("palette.title")} className="max-h-[50vh] overflow-y-auto p-2">
          {items.length === 0 && <li role="presentation" className="px-3 py-6 text-center text-small text-text-subtle">{t("palette.empty")}</li>}
          {items.map((item, i) => {
            const header = item.section !== section ? (section = item.section) : null;
            const Icon = item.icon;
            return (
              <li key={item.id} role="presentation">
                {header && <div className="px-3 pb-1 pt-2 text-caption font-medium text-text-subtle">{header}</div>}
                <div
                  id={`palette-${item.id}`}
                  role="option"
                  aria-selected={i === active}
                  data-index={i}
                  onMouseMove={() => setActive(i)}
                  onClick={() => choose(item)}
                  className={cn(
                    "flex h-10 cursor-pointer items-center gap-3 rounded-control px-3 text-body",
                    i === active ? "bg-white/8 text-text" : "text-text-muted",
                  )}
                >
                  <Icon className="size-4 shrink-0 text-text-subtle" />
                  <span className="truncate">{item.label}</span>
                  {item.hint && <span className="truncate text-caption text-text-subtle">{item.hint}</span>}
                  {i === active && <CornerDownLeft className="ms-auto size-3.5 shrink-0 text-text-subtle" />}
                </div>
              </li>
            );
          })}
        </ul>
      </div>
    </div>
  );
}

/** Mounted once; Ctrl+K (or Cmd+K) toggles the palette from anywhere. */
export function CommandPalette() {
  const visible = usePalette((s) => s.visible);
  useEffect(() => {
    const onKey = (e: globalThis.KeyboardEvent) => {
      if ((e.ctrlKey || e.metaKey) && e.key.toLowerCase() === "k") {
        e.preventDefault();
        const state = usePalette.getState();
        if (state.visible) state.close();
        else state.open();
      }
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, []);
  return visible ? <Palette /> : null;
}
