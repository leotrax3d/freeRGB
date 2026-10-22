import { LoaderCircle, Sparkles } from "lucide-react";
import { useEffect, useMemo, useRef, useState } from "react";
import { useNavigation } from "@/app/navigation";
import { Banner } from "@/components/status";
import { Button } from "@/components/ui/button";
import { Checkbox } from "@/components/ui/checkbox";
import { useDaemon } from "@/lib/store/daemon";
import type { ModulesStatus, SetupCard } from "@/lib/api/types";
import { defaultSelection, isWorking, overallProgress, selectable, summary, supported } from "./model";
import { SetupCardView } from "./SetupCard";
import { useSetup } from "./useSetup";

/** Overall bar: measured bytes while downloading, otherwise an activity line. */
function Overall({ cards, status }: { cards: SetupCard[]; status: ModulesStatus }) {
  const progress = overallProgress(status);
  const s = summary(cards);
  return (
    <div className="flex flex-col gap-2" aria-live="polite">
      <div className="flex items-baseline justify-between text-small">
        <span className="text-text">
          {s.working > 0 ? `Setting up ${s.working} of ${s.total}` : `${s.ready} ready${s.failed ? ` · ${s.failed} failed` : ""}`}
        </span>
        {progress !== null && (
          <span className="text-text-subtle tabular">
            {Math.floor(progress * 100)}%
          </span>
        )}
      </div>
      <div className="h-1.5 overflow-hidden rounded-full bg-surface-raised">
        {progress !== null ? (
          <div className="h-full rounded-full bg-accent" style={{ width: `${progress * 100}%` }} />
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
        <h3 className="text-body font-medium">Optional light test</h3>
        <p className="text-small text-text-muted">
          {simulated ? "This simulates a two-second light test. No physical device will light up." : "The chosen devices glow once in soft white for about two seconds, without flashing, and then return to the lighting freeRGB is showing now."}
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
          {remaining !== null ? <LoaderCircle className="animate-spin" /> : <Sparkles />}
          {remaining !== null ? "Testing…" : "Run light test"}
        </Button>
      </div>
      {setup.error && <p className="text-caption text-danger-text">{setup.error}</p>}
    </section>
  );
}

/**
 * First-start setup. Shown while the service reports `first_run`; all work
 * runs in the service, so closing the window does not interrupt it and the
 * current phase reappears on the next start.
 */
export function SetupWizard() {
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

  useEffect(() => heading.current?.focus(), []);
  // Preselect once cards arrive; later arrivals are added, user choices stay.
  const defaults = defaultSelection(cards);
  const selected = selection ?? new Set(defaults);

  if (!status || !status.first_run || closed) return null;

  // After a GUI restart the run is recovered from the service's phases.
  const active = runKeys.length ? runKeys : cards.filter(isWorking).map((c) => c.key);
  const runCards = cards.filter((c) => active.includes(c.key));
  const stage = active.length === 0 ? "select" : runCards.some(isWorking) || status.job.running ? "working" : "done";
  const searching = status.catalog.state === "loading";
  const chosen = [...selected].filter((k) => cards.some((c) => c.key === k && selectable(c)));
  const ready = cards.filter((c) => c.phase === "ready" && !c.builtin);

  const finish = async () => {
    await setup.complete();
    setClosed(true);
    go("lighting");
  };
  const start = async (keys: string[]) => {
    setRunKeys(keys);
    await setup.start(keys);
  };

  return (
    <div
      role="dialog"
      aria-modal="true"
      aria-labelledby="setup-title"
      className="absolute inset-0 z-40 flex justify-center overflow-y-auto bg-bg animate-[fade-in_var(--duration-base)_var(--ease-out)]"
    >
      <div className="flex w-full max-w-4xl flex-col gap-6 px-8 py-10">
        <header className="flex flex-col gap-1">
          <h1 id="setup-title" ref={heading} tabIndex={-1} className="text-page font-semibold tracking-tight outline-none">
            {stage === "done" ? "Your devices are set up" : "Set up your devices"}
          </h1>
          <p className="text-body text-text-muted">
            {stage === "select" &&
              "Choose the devices you want to set up."}
            {stage === "working" && (simulated ? "Simulated setup — keep this tab open. Nothing is downloaded or installed." : "You can close this window — setup continues in the background service.")}
            {stage === "done" && "Everything that could be set up is ready. You can change devices later under Devices."}
          </p>
        </header>

        {status.catalog.state === "error" && (
          <Banner tone="warn">
            <span className="flex-1">
              The module catalog is unavailable, so new devices cannot be set up right now. Installed modules keep
              working offline.
            </span>
            <Button size="sm" onClick={() => void setup.refreshCatalog()}>
              Try again
            </Button>
          </Banner>
        )}

        {(searching || devices.length === 0) && stage === "select" && (
          <div className="flex items-center gap-3 text-small text-text-muted" role="status">
            {searching ? <LoaderCircle className="size-4 animate-spin" aria-hidden /> : null}
            {searching ? "Looking for devices…" : "No lighting devices that need setup were found."}
          </div>
        )}

        {stage !== "select" && <Overall cards={runCards} status={status} />}

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

        {stage === "done" && <LightTest ready={ready} />}
        {setup.error && stage !== "done" && <p className="text-small text-danger-text">{setup.error}</p>}

        <footer className="sticky bottom-0 flex flex-wrap items-center justify-between gap-3 border-t border-border bg-bg py-4">
          {stage === "select" ? (
            <>
              <Button variant="ghost" onClick={() => void finish()}>
                Skip for now
              </Button>
              <div className="flex items-center gap-3">
                <Button variant="primary" disabled={chosen.length === 0 || setup.pending === "start"} onClick={() => void start(chosen)}>
                  Set up{chosen.length > 0 ? ` ${chosen.length} device${chosen.length === 1 ? "" : "s"}` : ""}
                </Button>
              </div>
            </>
          ) : stage === "working" ? (
            <>
              <Button variant="ghost" onClick={() => void setup.cancel()} disabled={!status.job.running}>
                Cancel
              </Button>
              <Button variant="secondary" onClick={() => void finish()}>
                Continue with working devices
              </Button>
            </>
          ) : (
            <>
              <span className="text-small text-text-subtle">
                {summary(runCards).failed > 0 && "Failed devices can be retried here or later under Devices."}
              </span>
              <Button variant="primary" onClick={() => void finish()}>
                Go to dashboard
              </Button>
            </>
          )}
        </footer>
      </div>
    </div>
  );
}
