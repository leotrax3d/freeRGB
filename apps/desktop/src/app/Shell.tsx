import { Suspense, useEffect, useRef, useState } from "react";
import { Toaster } from "@/components/toast";
import { CommandPalette } from "./CommandPalette";
import { Tour, useFirstRunTour } from "./Tour";
import { BrandRing } from "@/components/brand";
import { Banner, EmptyState, InlineError } from "@/components/status";
import { Button } from "@/components/ui/button";
import { transport } from "@/lib/api/transport";
import { t, tn, useLocale } from "@/lib/i18n";
import { rpc, useDaemon } from "@/lib/store/daemon";
import { useNavigation } from "./navigation";
import { Sidebar } from "./Sidebar";
import { SetupWizard } from "@/features/setup/SetupWizard";
import { OnboardingWizard } from "@/features/onboarding/OnboardingWizard";
import { preloadViews, VIEWS } from "./views";

function ServiceUnavailable({ error }: { error: string | null }) {
  const [starting, setStarting] = useState(false);
  const [startError, setStartError] = useState<string | null>(null);
  const start = async () => {
    setStarting(true);
    setStartError(null);
    try {
      await (await transport()).startService();
    } catch (e) {
      setStartError(String(e));
    } finally {
      // The poller notices the daemon within a second; keep the hint briefly.
      setTimeout(() => setStarting(false), 2500);
    }
  };
  return (
    <EmptyState
      icon={<BrandRing size={48} className="opacity-40 grayscale" />}
      title={t("shell.offline.title")}
      action={
        <div className="flex flex-col items-center gap-3">
          <Button variant="primary" onClick={start} disabled={starting}>
            {starting ? t("shell.offline.starting") : t("shell.offline.start")}
          </Button>
          {startError && <InlineError>{startError}</InlineError>}
        </div>
      }
    >
      {t("shell.offline.text")} <code className="font-mono text-caption">freergbd</code>.
      {error && <span className="mt-3 block text-caption text-text-subtle">{error}</span>}
    </EmptyState>
  );
}

/** Daemon pseudo-conflict while the user paused lighting (engine::PAUSED_BY_USER). */
const PAUSED_BY_USER = "Paused by you";

function Conflicts() {
  const all = useDaemon((s) => s.diagnostics?.conflicts);
  const paused = all?.includes(PAUSED_BY_USER) ?? false;
  const conflicts = all?.filter((c) => c !== PAUSED_BY_USER);
  if (paused)
    return (
      <Banner tone="info">
        <span className="flex-1">{t("shell.paused")}</span>
        <Button size="sm" onClick={() => void rpc("lighting.pause", { paused: false })}>{t("shell.resume")}</Button>
      </Banner>
    );
  if (!conflicts?.length) return null;
  return (
    <Banner tone="warn">
      {tn("shell.conflict", conflicts.length, { names: conflicts.join(", ") })}
    </Banner>
  );
}

export function Shell() {
  const view = useNavigation((s) => s.view);
  const connection = useDaemon((s) => s.connection);
  const def = VIEWS.find((v) => v.id === view) ?? VIEWS[0]!;
  const offline = connection && !connection.connected;
  const onboardingDone = useDaemon((s) => s.onboarding?.completed === true);
  const View = def.component;
  // A language change remounts the whole tree so every plain `t()` call re-runs.
  const locale = useLocale((s) => s.locale);
  const main = useRef<HTMLElement>(null);
  const firstView = useRef(true);
  useEffect(preloadViews, []);
  useFirstRunTour();
  // After navigating, move focus to the new page so keyboard and screen-reader
  // users start at its top instead of in the sidebar. Not on the first render.
  useEffect(() => {
    if (firstView.current) {
      firstView.current = false;
      return;
    }
    main.current?.focus({ preventScroll: true });
  }, [view]);
  return (
    <div key={locale} className="flex h-full bg-bg">
      <a
        href="#main-content"
        onClick={(e) => {
          e.preventDefault();
          main.current?.focus();
        }}
        className="sr-only focus:not-sr-only focus:fixed focus:left-3 focus:top-3 focus:z-[100] focus:rounded-control focus:bg-text focus:px-4 focus:py-2 focus:text-black"
      >
        {t("a11y.skip")}
      </a>
      <Sidebar />
      <main
        id="main-content"
        ref={main}
        tabIndex={-1}
        aria-label={t(def.label)}
        className="my-2 mr-2 flex min-w-0 flex-1 flex-col overflow-hidden rounded-panel bg-surface outline-none"
      >
        <div className="flex-1 overflow-y-auto">
          <div className="mx-auto flex max-w-6xl flex-col gap-6 px-8 py-8">
            <Conflicts />
            {offline ? (
              <ServiceUnavailable error={connection.error} />
            ) : (
              <Suspense fallback={null}>
                <div key={def.id} className="flex flex-col gap-6 animate-[view-in_var(--duration-base)_var(--ease-out)]">
                  <View />
                </div>
              </Suspense>
            )}
          </div>
        </div>
      </main>
      <CommandPalette />
      <Toaster />
      <Tour />
      {!offline && <OnboardingWizard />}
      {/* New devices after the first-run wizard keep using the device setup. */}
      {!offline && onboardingDone && <SetupWizard />}
    </div>
  );
}
