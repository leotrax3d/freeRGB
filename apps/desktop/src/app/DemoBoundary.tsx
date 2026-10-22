import { Component, type ReactNode } from "react";

export function DemoFailure() {
  return <div role="alert" className="flex h-full flex-col items-center justify-center gap-4 bg-bg p-8 text-text">
    <h1 className="text-title">The browser demo could not start</h1>
    <p>Try a current browser and reload this page. No hardware connection is required.</p>
    <button className="rounded-control bg-surface-raised px-4 py-2" onClick={() => location.reload()}>Reload demo</button>
  </div>;
}

export class DemoBoundary extends Component<{ children: ReactNode }, { failed: boolean }> {
  state = { failed: false };
  static getDerivedStateFromError() { return { failed: true }; }
  render() { return this.state.failed ? <DemoFailure /> : this.props.children; }
}
