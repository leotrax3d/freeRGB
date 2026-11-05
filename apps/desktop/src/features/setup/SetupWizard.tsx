import { LoaderCircle, Sparkles } from "lucide-react";
import { useEffect, useMemo, useRef, useState } from "react";
import { useNavigation } from "@/app/navigation";
import { Banner, InlineError } from "@/components/status";
import { t, tn } from "./messages";
import { Button } from "@/components/ui/button";
import { Checkbox } from "@/components/ui/checkbox";
import { useDaemon } from "@/lib/store/daemon";
import type { SetupCard } from "@/lib/api/types";
import { bytes, defaultSelection, isWorking, overallProgress, selectable, selectionSize, summary, supported } from "./model";
import { SetupCardView } from "./SetupCard";
import { useSetup } from "./useSetup";

/** Overall bar: measured bytes while downloading, otherwise an activity line. */
function Overall({ cards }: { cards: SetupCard[] }) {
  const status = useDaemon((s) => s.modules_setup);
  const progress = overallProgress(status);
  const s = summary(cards);
  const downloading = status?.job.running && progress !== null && progress < 1;
  return (
    <div className="flex flex-col gap-2" aria-live="polite">
      <div className="flex items-baseline justify-between text-small">
        <span className="text-text">
          {s.working > 0 ? t("setup.wizard.working", { working: s.working, total: s.total }) : `${t("setup.wizard.ready", { ready: s.ready })}${s.failed ? t("setup.wizard.failed", { failed: s.failed }) : ""}`}
        </span>
        {downloading && status && (
          <span className="text-text-subtle tabular">
            {t("setup.wizard.progress", { done: bytes(status.job.bytes_done), total: bytes(status.job.bytes_total) })}
          </span>
        )}
      </div>
      <div className="h-1.5 overflow-hidden rounded-full bg-surface-raised">
        {downloading ? (
          <div className="h-full rounded-full bg-accent transition-[width] duration-(--duration-base)" style={{ width: `${Math.round(progress! * 100)}%` }} />
        ) : s.working > 0 ? (
          // Not measurable (verifying, connecting): activity, not a percentage.
          <div className="h-full w-1/3 rounded-full bg-accent/70 animate-[setup-activity_1.4s_var(--ease-out)_infinite]" />
        ) : (
          <div className="h-full rounded-full bg-accent" style={{ width: s.total ? `${(s.ready / s.total) * 100}%` : 0 }} />
        )}
      </div>
    </div>
  );
}

function LightTest({ ready }: { ready: SetupCard[] }) {
  const simulated = useDaemon((s) => s.mode === "mock");
  const setup = useSetup();
  const remaining = setup.status?.light_test_remaining_s ?? null;
  const ids = ready.map((c) => c.engine_id).filter((id): id is string => !!id);
  const [chosen, setChosen] = useState<string[]>(ids);
  if (ids.length === 0) return null;
  return (
    <section className="flex flex-col gap-3 rounded-card border border-border p-4">
      <div className="flex flex-col gap-1">
        <h3 className="text-body font-medium">{t("setup.wizard.test")}</h3>
        <p className="text-small text-text-muted">
          {simulated ? t("setup.wizard.testSim") : t("setup.wizard.testReal")}
        </p>
      </div>
      <div className="flex flex-wrap gap-x-5 gap-y-2">
        {ready.filter((c) => c.engine_id).map((c) => (
          <Checkbox
            key={c.key}
            checked={chosen.includes(c.engine_id!)}
            onChange={(on) => setChosen(on ? [...chosen, c.engine_id!] : chosen.filter((x) => x !== c.engine_id))}
          >
            {c.name}
          </Checkbox>
        ))}
      </div>
      <div>
        <Button onClick={() => void setup.lightTest(chosen)} disabled={chosen.length === 0 || remaining !== null}>
          {remaining !== null ? <LoaderCircle className="animate-spin" aria-hidden /> : <Sparkles aria-hidden />}
          {remaining !== null ? t("setup.wizard.testing") : t("setup.wizard.runTest")}
        </Button>
      </div>
      {setup.error && <InlineError>{setup.error}</InlineError>}
    </section>
  );
}

/**
 * First-start setup. Shown while the service reports `first_run`; all work
 * runs in the service, so closing the window does not interrupt it and the
 * current phase reappears on the next start.
 */
export function SetupWizard({ embedded = false, onDone }: { embedded?: boolean; onDone?: () => void } = {}) {
  const simulated = useDaemon((s) => s.mode === "mock");
  const setup = useSetup();
  const status = setup.status;
  const go = useNavigation((s) => s.go);
  const heading = useRef<HTMLHeadingElement>(null);
  const [runKeys, setRunKeys] = useState<string[]>([]);
  const [selection, setSelection] = useState<Set<string> | null>(null);
  const [closed, setClosed] = useState(false);
  const cards = useMemo(() => status?.cards ?? [], [status?.cards]);
  const devices = cards.filter((c) => c.phase !== "unsupported");
  const unsupported = cards.filter((c) => c.phase === "unsupported");

  useEffect(() => heading.current?.focus(), []);
  // Preselect once cards arrive; later arrivals are added, user choices stay.
  const defaults = defaultSelection(cards);
  const selected = selection ?? new Set(defaults);

  // Embedded in the first-run wizard it always shows; standalone only on first run.
  if (!status || (!embedded && (!status.first_run || closed))) return null;

  // After a GUI restart the run is recovered from the service's phases.
  const active = runKeys.length ? runKeys : cards.filter(isWorking).map((c) => c.key);
  const runCards = cards.filter((c) => active.includes(c.key));
  const stage = active.length === 0 ? "select" : runCards.some(isWorking) || status.job.running ? "working" : "done";
  const searching = status.catalog.state === "loading";
  const chosen = [...selected].filter((k) => cards.some((c) => c.key === k && selectable(c)));
  const size = selectionSize(cards, chosen);
  const ready = cards.filter((c) => c.phase === "ready" && !c.builtin);

  const finish = async () => {
    if (!await setup.complete()) return;
    setClosed(true);
    if (onDone) onDone();
    else go("lighting");
  };
  const start = async (keys: string[]) => {
    setRunKeys(keys);
    await setup.start(keys);
  };

  return (
    <div
      role={embedded ? undefined : "dialog"}
      aria-modal={embedded ? undefined : true}
      aria-labelledby="setup-title"
      className={embedded ? "flex justify-center" : "fixed inset-0 z-40 flex justify-center overflow-y-auto bg-bg animate-[fade-in_var(--duration-base)_var(--ease-out)]"}
    >
      <div className={embedded ? "flex w-full flex-col gap-6" : "flex w-full max-w-4xl flex-col gap-6 px-8 py-10"}>
        <header className="flex flex-col gap-1">
          <h1 id="setup-title" ref={heading} tabIndex={-1} className="text-page font-semibold tracking-tight outline-none">
            {stage === "done" ? t("setup.wizard.titleDone") : t("setup.wizard.title")}
          </h1>
          <p className="text-body text-text-muted">
            {stage === "select" && t("setup.wizard.introSelect")}
            {stage === "working" && (simulated ? t("setup.wizard.introSim") : t("setup.wizard.introWorking"))}
            {stage === "done" && t("setup.wizard.introDone")}
          </p>
        </header>

        {status.catalog.state === "error" && (
          <Banner tone="warn">
            <span className="flex-1">{t("setup.wizard.catalog")}</span>
            <Button size="sm" onClick={() => void setup.refreshCatalog()}>
              {t("setup.card.retry")}
            </Button>
          </Banner>
        )}

        {(searching || devices.length === 0) && stage === "select" && (
          <div className="flex items-center gap-3 text-small text-text-muted" role="status">
            {searching ? <LoaderCircle className="size-4 animate-spin" aria-hidden /> : null}
            {searching ? t("setup.wizard.searching") : t("setup.wizard.none")}
          </div>
        )}

        {stage !== "select" && <Overall cards={runCards} />}

        <div className="grid grid-cols-[repeat(auto-fill,minmax(260px,1fr))] gap-3">
          {devices.map((c, i) => (
            <SetupCardView
              key={c.key}
              card={c}
              index={i}
              selected={stage === "select" && selectable(c) ? selected.has(c.key) : undefined}
              onSelect={(on) => {
                const next = new Set(selected);
                if (on) next.add(c.key);
                else next.delete(c.key);
                setSelection(next);
              }}
              onRetry={c.phase === "failed" && supported(c) ? () => void setup.retry(c.key) : undefined}
            />
          ))}
        </div>

        {unsupported.length > 0 && (
          <details className="flex flex-col gap-3">
            <summary className="cursor-pointer text-small text-text-subtle">
              {tn("setup.wizard.unsupported", unsupported.length)}
            </summary>
            <div className="mt-3 grid grid-cols-[repeat(auto-fill,minmax(260px,1fr))] gap-3">
              {unsupported.map((c, i) => (
                <SetupCardView key={c.key} card={c} index={i} compact />
              ))}
            </div>
          </details>
        )}

        {stage === "done" && <LightTest ready={ready} />}
        {setup.error && <InlineError>{setup.error}</InlineError>}

        <footer className="sticky bottom-0 flex flex-wrap items-center justify-between gap-3 border-t border-border bg-bg py-4">
          {stage === "select" ? (
            <>
              <Button variant="ghost" onClick={() => void finish()}>
                {embedded ? t("setup.wizard.skipEmbedded") : t("setup.wizard.skip")}
              </Button>
              <div className="flex items-center gap-3">
                {chosen.length > 0 && (
                  <span className="text-small text-text-subtle tabular">
                    {size.bytes > 0 ? t("setup.wizard.download", { size: bytes(size.bytes) }) : t("setup.wizard.noDownload")}
                    {size.unknown && t("setup.wizard.partly")}
                  </span>
                )}
                <Button variant="primary" disabled={chosen.length === 0 || setup.pending === "start"} onClick={() => void start(chosen)}>
                  {chosen.length > 0 ? tn("setup.wizard.setUpN", chosen.length) : t("setup.wizard.setUp")}
                </Button>
              </div>
            </>
          ) : stage === "working" ? (
            <>
              <Button variant="ghost" onClick={() => void setup.cancel()} disabled={!status.job.running}>
                {t("setup.wizard.cancel")}
              </Button>
              <Button variant="secondary" onClick={() => void finish()}>
                {t("setup.wizard.continueWorking")}
              </Button>
            </>
          ) : (
            <>
              <span className="text-small text-text-subtle">
                {summary(runCards).failed > 0 && t("setup.wizard.retryLater")}
              </span>
              <Button variant="primary" onClick={() => void finish()}>
                {embedded ? t("setup.wizard.continue") : t("setup.wizard.dashboard")}
              </Button>
            </>
          )}
        </footer>
      </div>
    </div>
  );
}
