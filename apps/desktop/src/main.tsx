import { StrictMode } from "react";
import { createRoot } from "react-dom/client";
import { Shell } from "./app/Shell";
import { DemoBoundary, DemoFailure } from "./app/DemoBoundary";
import { startDaemonSync } from "./lib/store/daemon";
import "./styles/globals.css";

const root = createRoot(document.getElementById("root")!);
void startDaemonSync().catch((error: unknown) => {
  if (__WEB_DEMO__) root.render(<DemoFailure />);
  else console.error("Could not connect to freeRGB", error);
});

root.render(
  <StrictMode>
    {__WEB_DEMO__ ? <DemoBoundary><div className="flex h-full flex-col overflow-x-auto overflow-y-hidden">
      <div role="note" className="relative z-[100] shrink-0 border-b border-border bg-surface px-4 py-2 text-small text-text-muted">
        Browser demo · Virtual devices only · Downloads and readings are simulated.
        No hardware, microphone or screen access. Changes reset when reloaded.
      </div>
      <div className="min-h-0 min-w-[960px] flex-1"><Shell /></div>
    </div></DemoBoundary> : <Shell />}
  </StrictMode>,
);
