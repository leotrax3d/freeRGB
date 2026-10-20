import { StrictMode } from "react";
import { createRoot } from "react-dom/client";
import { Shell } from "./app/Shell";
import { startDaemonSync } from "./lib/store/daemon";
import "./styles/globals.css";

void startDaemonSync();

createRoot(document.getElementById("root")!).render(
  <StrictMode>
    <Shell />
  </StrictMode>,
);
