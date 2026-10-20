import { Keyboard, Play, Plus, Square, Trash2, Zap } from "lucide-react";
import { useState } from "react";
import { ConfirmButton } from "@/components/confirm-button";
import { NameForm } from "@/components/name-form";
import { PageHeader, Section } from "@/components/page";
import { Banner } from "@/components/status";
import { Button } from "@/components/ui/button";
import { macroProblem, type MacroDefinition, type MacroStep } from "@/lib/api/control";
import { approxEqual } from "@/lib/store/approxEqual";
import { rpc, useDaemon, useSlices } from "@/lib/store/daemon";
import { useAction } from "@/lib/store/useAction";
import { cn } from "@/lib/utils";
import { StepRow } from "./StepEditor";
import { STEP_TYPES, blankStep, moveStep } from "./steps";

const NEW: MacroDefinition = { repeat: 1, steps: [{ type: "brightness", value: 1 }] };

export default function MacrosView() {
  useSlices("macros", "macro_status");
  const macros = useDaemon((s) => s.macros);
  const status = useDaemon((s) => s.macro_status);
  const [name, setName] = useState<string | null>(null);
  const [draft, setDraft] = useState<MacroDefinition>(NEW);
  const [addType, setAddType] = useState<MacroStep["type"]>("color");
  const { pending, error, run } = useAction();

  const open = (n: string | null) => {
    setName(n);
    setDraft(n && macros?.[n] ? macros[n] : NEW);
  };
  const setSteps = (steps: MacroStep[]) => setDraft({ ...draft, steps });
  const problem = macroProblem(draft);
  const saved = name !== null && macros?.[name] !== undefined && approxEqual(macros[name], draft);
  const running = status?.running ?? null;

  return (
    <>
      <PageHeader title="Macros" description="Step sequences for lighting changes and, if enabled, keyboard output." />

      {running && (
        <Banner tone="info">
          <Zap className="size-4" /> Running “{running}”
          <Button size="sm" variant="secondary" className="ml-auto" onClick={() => run("cancel", () => rpc("macros.cancel"))}>
            <Square /> Stop
          </Button>
        </Banner>
      )}
      {status?.error && <Banner tone="error">Last macro stopped: {status.error}</Banner>}

      <div className="@container">
        <div className="grid items-start gap-4 @3xl:grid-cols-[220px_minmax(0,1fr)]">
          <ul className="flex flex-col gap-1">
            {Object.keys(macros ?? {}).map((n) => (
              <li key={n} className="flex items-center gap-1">
                <button
                  type="button"
                  onClick={() => open(n)}
                  className={cn(
                    "flex h-9 min-w-0 flex-1 items-center gap-2 rounded-control px-3 text-left text-body transition-colors",
                    name === n ? "bg-surface-raised" : "text-text-muted hover:bg-surface-raised hover:text-text",
                  )}
                >
                  <Keyboard className="size-4 shrink-0 text-text-subtle" />
                  <span className="truncate">{n}</span>
                </button>
                <Button
                  size="icon"
                  variant="ghost"
                  aria-label={`Run ${n}`}
                  disabled={running !== null || pending !== null}
                  onClick={() => run(`run:${n}`, () => rpc("macros.run", { name: n }))}
                >
                  <Play />
                </Button>
              </li>
            ))}
            <li>
              <Button variant="ghost" size="sm" className="w-full justify-start" onClick={() => open(null)}>
                <Plus /> New macro
              </Button>
            </li>
          </ul>

          <div className="flex flex-col gap-4 rounded-card bg-surface-sunken p-4">
            <div className="flex flex-wrap items-center justify-between gap-3">
              <h3 className="text-title font-semibold">{name ?? "New macro"}</h3>
              <label className="flex items-center gap-2 text-small">
                Repeat
                <input
                  type="number"
                  min={1}
                  max={20}
                  className="h-8 w-16 rounded-chip border border-border-strong bg-surface px-2 text-small tabular outline-none focus-visible:border-info"
                  value={draft.repeat}
                  onChange={(e) => setDraft({ ...draft, repeat: Math.round(Number(e.target.value)) })}
                />
                times
              </label>
            </div>
            <ol className="flex flex-col divide-y divide-border rounded-control border border-border">
              {draft.steps.map((step, i) => (
                <StepRow
                  key={i}
                  step={step}
                  index={i}
                  count={draft.steps.length}
                  inputEnabled={status?.input_enabled ?? false}
                  onChange={(s) => setSteps(draft.steps.map((x, j) => (j === i ? s : x)))}
                  onMove={(by) => setSteps(moveStep(draft.steps, i, by))}
                  onRemove={() => setSteps(draft.steps.filter((_, j) => j !== i))}
                />
              ))}
            </ol>
            <div className="flex items-center gap-2">
              <select
                aria-label="Step to add"
                className="h-8 rounded-chip border border-border-strong bg-surface px-2 text-small outline-none focus-visible:border-info"
                value={addType}
                onChange={(e) => setAddType(e.target.value as MacroStep["type"])}
              >
                {STEP_TYPES.map((t) => (
                  <option key={t.type} value={t.type}>
                    {t.label}
                  </option>
                ))}
              </select>
              <Button size="sm" variant="secondary" disabled={draft.steps.length >= 128} onClick={() => setSteps([...draft.steps, blankStep(addType)])}>
                <Plus /> Add step
              </Button>
            </div>
            {problem && <p className="text-caption text-danger-text">{problem}</p>}
            {error && <p className="text-caption text-danger-text">{error}</p>}
            <div className="flex flex-wrap items-start justify-between gap-3 border-t border-border pt-4">
              {name ? (
                <div className="flex items-center gap-2">
                  <Button variant="primary" disabled={!!problem || saved || pending !== null} onClick={() => run("save", () => rpc("macros.save", { name, definition: draft }))}>
                    {saved ? "Saved" : "Save changes"}
                  </Button>
                  <Button variant="secondary" disabled={!saved || running !== null || pending !== null} onClick={() => run("run", () => rpc("macros.run", { name }))}>
                    <Play /> Run
                  </Button>
                  <ConfirmButton
                    label={`Delete ${name}`}
                    icon={<Trash2 />}
                    disabled={pending !== null}
                    onConfirm={() => run("delete", () => rpc("macros.delete", { name })).then((ok) => ok && open(null))}
                  />
                </div>
              ) : (
                <div className="w-full max-w-sm">
                  <NameForm
                    label="Macro name"
                    placeholder="e.g. Movie mode"
                    submit="Save macro"
                    existing={Object.keys(macros ?? {})}
                    busy={pending !== null || !!problem}
                    onSubmit={async (n) => {
                      const ok = await run("save", () => rpc("macros.save", { name: n, definition: draft }));
                      if (ok) setName(n);
                      return ok;
                    }}
                  />
                </div>
              )}
            </div>
          </div>
        </div>
      </div>

      <Section title="Global shortcuts">
        <p className="rounded-card border border-border px-4 py-3 text-small text-text-muted">
          Keyboard shortcuts that start macros still run in freeRGB Classic. They move here once that logic is shared
          between both apps (M5).
        </p>
      </Section>
    </>
  );
}
