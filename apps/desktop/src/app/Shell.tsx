import { Suspense, useEffect, useState } from "react";
import { Toaster } from "@/components/toast";
import { CommandPalette } from "./CommandPalette";
import { Tour, useFirstRunTour } from "./Tour";
import { BrandTiles } from "@/components/brand";
import { Banner, EmptyState } from "@/components/status";
import { Button } from "@/components/ui/button";
import { transport } from "@/lib/api/transport";
import { useDaemon } from "@/lib/store/daemon";
import { useNavigation } from "./navigation";
import { Sidebar } from "./Sidebar";
import { SetupWizard } from "@/features/setup/SetupWizard";
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
      icon={<BrandTiles size={18} letters className="opacity-40 grayscale" />}
      title="The freeRGB service is not running"
      action={
        <div className="flex flex-col items-center gap-3">
          <Button variant="primary" onClick={start} disabled={starting}>
            {starting ? "Starting…" : "Start service"}
          </Button>
          {startError && <p className="text-small text-danger-text">{startError}</p>}
        </div>
      }
    >
      Lighting keeps running in the background service, independent of this window. Start it here, with the
      freeRGB launcher, or with <code className="font-mono text-caption">freergbd</code>.
      {error && <span className="mt-3 block text-caption text-text-subtle">{error}</span>}
    </EmptyState>
  );
}

function Conflicts() {
  const conflicts = useDaemon((s) => s.diagnostics?.conflicts);
  if (!conflicts?.length) return null;
  return (
    <Banner tone="warn">
      {conflicts.join(", ")} {conflicts.length === 1 ? "is" : "are"} running — freeRGB leaves your hardware alone
      until {conflicts.length === 1 ? "it is" : "they are"} closed.
    </Banner>
  );
}

export function Shell() {
  const view = useNavigation((s) => s.view);
  const connection = useDaemon((s) => s.connection);
  const def = VIEWS.find((v) => v.id === view) ?? VIEWS[0]!;
  const offline = connection && !connection.connected;
  const View = def.component;
  useEffect(preloadViews, []);
  useFirstRunTour();
  return (
    <div className="relative flex h-full min-h-0 overflow-hidden bg-bg">
      <Sidebar />
      <main className="my-2 mr-2 flex min-w-0 flex-1 flex-col overflow-hidden rounded-panel bg-surface">
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
      {!offline && <SetupWizard />}
    </div>
  );
}
