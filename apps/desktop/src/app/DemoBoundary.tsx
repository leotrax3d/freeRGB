import { Component, type ReactNode } from "react";
import { t } from "@/lib/i18n";

export function DemoFailure() {
  return <div role="alert" className="flex h-full flex-col items-center justify-center gap-4 bg-bg p-8 text-text">
    <h1 className="text-title">{t("demo.failedTitle")}</h1>
    <p>{t("demo.failedText")}</p>
    <button className="rounded-control bg-surface-raised px-4 py-2" onClick={() => location.reload()}>{t("demo.reload")}</button>
  </div>;
}

export class DemoBoundary extends Component<{ children: ReactNode }, { failed: boolean }> {
  state = { failed: false };
  static getDerivedStateFromError() { return { failed: true }; }
  render() { return this.state.failed ? <DemoFailure /> : this.props.children; }
}
