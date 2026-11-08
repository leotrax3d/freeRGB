// MOCK DATA — simulated device setup for `npm run dev` in a plain browser.
// Phases advance on timers; nothing is downloaded and no device is touched.
// The desktop app always uses the real service state instead.

import type { ModulesStatus, SetupCard } from "../api/types";

const base = (over: Partial<SetupCard>): SetupCard => ({
  key: "",
  name: "",
  vendor: "Demo Hardware",
  kind: "mouse",
  phase: "detected",
  module: "demo-rgb-mouse",
  download_size: 812_000,
  progress: null,
  error: null,
  configured: false,
  dismissed: false,
  builtin: false,
  installed: false,
  connected: false,
  engine_id: null,
  technical: "Virtual demo device; no USB connection",
  ...over,
});

export function mockSetup(onChange: () => void) {
  const pending = new Set<ReturnType<typeof setTimeout>>();
  let downloading: ReturnType<typeof setInterval> | undefined;
  const later = (callback: () => void, delay: number) => {
    const timer = setTimeout(() => { pending.delete(timer); callback(); }, delay);
    pending.add(timer);
  };
  let status: ModulesStatus = {
    first_run: true,
    cards: [
      base({ key: "demo:mouse:00a3:if0:0001:0002", name: "RGB Mouse One (virtual)" }),
      base({ key: "demo:mouse:0099:if0:0001:0002", name: "RGB Mouse Two (virtual)" }),
      base({ key: "builtin", name: "RGB Keyboard (virtual)", vendor: "Demo Hardware", kind: "keyboard", module: null, builtin: true, phase: "ready" }),
      base({ key: "demo:mouse:0f20", name: "Unconfigured RGB Device (virtual)", module: null, phase: "unsupported" }),
    ],
    catalog: { state: "ready", source: "mock", message: null, sequence: 1 },
    job: { running: false, cancelled: false, bytes_done: 0, bytes_total: 0, modules: [] },
    installed: [],
    trusted_release_key: false,
    light_test_remaining_s: null,
  };
  const set = (keys: string[], patch: Partial<SetupCard>) => {
    status = { ...status, cards: status.cards.map((c) => (keys.includes(c.key) ? { ...c, ...patch } : c)) };
    onChange();
  };
  return {
    get: () => status,
    start(keys: string[]) {
      if (status.job.running) return;
      const total = 812_000;
      status = { ...status, job: { running: true, cancelled: false, bytes_done: 0, bytes_total: total, modules: ["demo-rgb-mouse"] } };
      set(keys, { phase: "downloading", progress: { done: 0, total }, error: null });
      let done = 0;
      const tick = setInterval(() => {
        done = Math.min(total, done + 120_000);
        status = { ...status, job: { ...status.job, bytes_done: done } };
        set(keys, { progress: { done, total } });
        if (done < total) return;
        clearInterval(tick);
        downloading = undefined;
        set(keys, { phase: "verifying", progress: null });
        later(() => set(keys, { phase: "installing" }), 500);
        later(() => {
          status = { ...status, job: { ...status.job, running: false }, installed: [{ id: "demo-rgb-mouse", version: "0.1.0", previous: null, last_good: null, family: "demo-rgb-mouse", license: "GPL-2.0-or-later" }] };
          set(keys, { phase: "initializing", configured: true, installed: true });
        }, 1000);
        later(() => {
          const [first, ...rest] = keys;
          if (first) set([first], { phase: "ready", connected: true, engine_id: "mock:mouse-one" });
          if (rest.length) set(rest, { phase: "failed", error: { code: "busy", message: "Refused: Busy or unavailable (mock)" } });
        }, 1800);
      }, 250);
      downloading = tick;
    },
    cancel() {
      clearInterval(downloading);
      downloading = undefined;
      pending.forEach(clearTimeout);
      pending.clear();
      status = { ...status, job: { ...status.job, running: false, cancelled: true } };
      set(status.cards.filter((card) => ["downloading", "verifying", "installing", "initializing"].includes(card.phase)).map((card) => card.key), { phase: "detected", progress: null });
    },
    retry(keys: string[]) {
      set(keys, { phase: "initializing", error: null });
      later(() => set(keys, { phase: "ready", connected: true, engine_id: "mock:mouse-two" }), 900);
    },
    complete() {
      status = { ...status, first_run: false };
      onChange();
    },
    lightTest() {
      status = { ...status, light_test_remaining_s: 2.4 };
      onChange();
      setTimeout(() => {
        status = { ...status, light_test_remaining_s: null };
        onChange();
      }, 2400);
    },
    forget(keys: string[]) {
      set(keys.filter((key) => status.cards.some((c) => c.key === key && !c.builtin)), { configured: false });
    },
    dismiss(keys: string[]) {
      set(keys, { dismissed: true });
    },
  };
}
