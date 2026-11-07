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
    {__WEB_DEMO__ ? <DemoBoundary><div className="flex h-full min-w-[960px] flex-col">
      <div className="min-h-0 flex-1"><Shell /></div>
    </div></DemoBoundary> : <Shell />}
  </StrictMode>,
);
