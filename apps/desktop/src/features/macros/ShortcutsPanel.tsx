import { t } from "./messages";
import { AppWindow, ArrowRight, Bookmark, FlaskConical, Keyboard, Plus, Sparkles, Trash2 } from "lucide-react";
import { useState } from "react";
import { KeyRecorder } from "@/components/key-recorder";
import { Button } from "@/components/ui/button";
import { Select } from "@/components/ui/select";
import { EFFECTS } from "@/lib/api/effects";
import { useDaemon, useSlices } from "@/lib/store/daemon";
import { transport } from "@/lib/api/transport";
import { useAction } from "@/lib/store/useAction";

type ActionType = "macro" | "profile" | "effect" | "program";

interface Shortcut {
  id: number;
  keys: string[];
  action: ActionType;
  target: string;
}

const ACTIONS: { value: ActionType; label: string; icon: typeof Keyboard }[] = [
  { value: "macro", get label() { return t("macros.shortcutsPanel.runMacro"); }, icon: Keyboard },
  { value: "profile", get label() { return t("macros.shortcutsPanel.loadProfile"); }, icon: Bookmark },
  { value: "effect", get label() { return t("macros.shortcutsPanel.startEffect"); }, icon: Sparkles },
  { value: "program", get label() { return t("macros.shortcutsPanel.openProgram"); }, icon: AppWindow },
];

let nextId = 1;

/**
 * DESIGN PREVIEW. Global shortcuts currently run in freeRGB Classic (macros
 * only) and the service cannot open programs. Nothing here is saved or
 * registered; it shows the intended interaction until the backend exists.
 */
export function ShortcutsPanel() {
  const mode = useDaemon((s) => s.mode);
  const { pending, error, run } = useAction();
  if (mode === "mock") return <ShortcutDesignPreview />;
  return <div className="flex flex-col gap-3">
    <p className="text-small text-text-muted">{t("macros.shortcutsPanel.globalMacroShortcutsAreConfiguredAndRegistered")}</p>
    <Button variant="secondary" disabled={pending !== null} onClick={() => run("classic", async () => { const t = await transport(); await t.openClassic?.(); })}>{t("macros.shortcutsPanel.openClassicShortcuts")}</Button>
    {error && <p role="alert" className="text-small text-danger-text">{error}</p>}
  </div>;
}

function ShortcutDesignPreview() {
  useSlices("macros", "profiles");
  const macros = Object.keys(useDaemon((s) => s.macros) ?? {});
  const profiles = useDaemon((s) => s.profiles) ?? [];
  const [items, setItems] = useState<Shortcut[]>([
    { id: nextId++, keys: ["ctrl", "alt", "f6"], action: "profile", target: profiles[0] ?? "" },
  ]);
  const update = (id: number, patch: Partial<Shortcut>) => setItems((list) => list.map((s) => (s.id === id ? { ...s, ...patch } : s)));
  const taken = (s: Shortcut) => s.keys.length > 0 && items.some((o) => o.id !== s.id && o.keys.join("+") === s.keys.join("+"));

  const targets = (action: ActionType) =>
    action === "macro"
      ? macros.map((m) => ({ value: m, label: m }))
      : action === "profile"
        ? profiles.map((p) => ({ value: p, label: p }))
        : EFFECTS.filter((e) => e.name !== "layers").map((e) => ({ value: e.name as string, label: e.label }));

  return (
    <div className="flex flex-col gap-4">
      <div role="note" className="flex items-start gap-3 rounded-card border border-creative/40 bg-creative/10 px-4 py-3 text-small text-text-muted">
        <FlaskConical className="mt-0.5 size-4 shrink-0 text-creative-text" />
        <span>
          <span className="font-medium text-text">{t("macros.shortcutsPanel.designPreviewNotActiveYet")}</span> {t("macros.shortcutsPanel.shortcutsSetHereAreNotSavedOr")} </span>
      </div>

      <ul className="flex flex-col gap-2">
        {items.map((s) => {
          const action = ACTIONS.find((a) => a.value === s.action)!;
          const Icon = action.icon;
          return (
            <li key={s.id} className="flex flex-wrap items-center gap-3 rounded-card bg-surface-sunken p-3">
              <div className="flex flex-col gap-1">
                <KeyRecorder label={t("macros.shortcutsPanel.shortcut")} value={s.keys} onChange={(keys) => update(s.id, { keys })} placeholder={t("macros.shortcutsPanel.clickAndPressAShortcut")} />
                {taken(s) && <span className="text-caption text-danger-text">{t("macros.shortcutsPanel.alreadyUsedByAnotherShortcut")}</span>}
              </div>
              <ArrowRight className="size-4 text-text-subtle" aria-hidden />
              <div className="flex min-w-0 flex-1 flex-wrap items-center gap-2">
                <span className="grid size-8 place-items-center rounded-control bg-surface-raised text-text-muted">
                  <Icon className="size-4" />
                </span>
                <Select
                  label={t("macros.shortcutsPanel.action")}
                  className="w-36"
                  value={s.action}
                  onChange={(a) => update(s.id, { action: a, target: "" })}
                  options={ACTIONS.map((a) => ({ value: a.value, label: a.label }))}
                />
                {s.action === "program" ? (
                  <input
                    aria-label={t("macros.shortcutsPanel.program")}
                    value={s.target}
                    placeholder={t("macros.programExample")}
                    onChange={(e) => update(s.id, { target: e.target.value })}
                    className="h-8 min-w-0 flex-1 rounded-chip border border-border-strong bg-surface px-2.5 font-mono text-caption text-text outline-none placeholder:text-text-subtle focus-visible:border-info"
                  />
                ) : (
                  <Select
                    label={t("macros.shortcutsPanel.target")}
                    className="min-w-40 flex-1"
                    value={s.target}
                    placeholder={targets(s.action).length ? t("macros.shortcutsPanel.choose") : t("macros.shortcutsPanel.nothingSavedYet")}
                    onChange={(target) => update(s.id, { target })}
                    options={targets(s.action)}
                  />
                )}
              </div>
              <Button size="icon" variant="ghost" aria-label={t("macros.shortcutsPanel.removeShortcut")} onClick={() => setItems((list) => list.filter((x) => x.id !== s.id))}>
                <Trash2 />
              </Button>
            </li>
          );
        })}
      </ul>
      <Button
        variant="secondary"
        className="self-start"
        onClick={() => setItems((list) => [...list, { id: nextId++, keys: [], action: "macro", target: "" }])}
      >
        <Plus /> {t("macros.shortcutsPanel.addShortcut")} </Button>
    </div>
  );
}
