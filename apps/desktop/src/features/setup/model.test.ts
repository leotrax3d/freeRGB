import { describe, expect, it } from "vitest";
import type { SetupCard } from "@/lib/api/types";
import { defaultSelection, errorHint, newDevices, overallProgress, selectionSize } from "./model";

const card = (over: Partial<SetupCard>): SetupCard => ({
  key: "k",
  name: "Device",
  vendor: "Razer",
  kind: "mouse",
  phase: "detected",
  module: "razer-chroma",
  download_size: 1000,
  progress: null,
  error: null,
  configured: false,
  dismissed: false,
  builtin: false,
  installed: false,
  connected: false,
  engine_id: null,
  technical: "",
  ...over,
});

describe("setup model", () => {
  const cards = [
    card({ key: "a" }),
    card({ key: "b" }),
    card({ key: "c", module: "other", download_size: 500 }),
    card({ key: "builtin", builtin: true, module: null, phase: "ready" }),
    card({ key: "old", configured: true, phase: "ready" }),
    card({ key: "no", module: null, phase: "unsupported" }),
    card({ key: "skip", dismissed: true }),
  ];

  it("preselects only supported devices that are not set up", () => {
    expect(defaultSelection(cards)).toEqual(["a", "b", "c"]);
    expect(newDevices(cards).map((c) => c.key)).toEqual(["a", "b", "c"]);
  });

  it("counts a shared module once and skips installed ones", () => {
    expect(selectionSize(cards, ["a", "b"])).toEqual({ bytes: 1000, unknown: false });
    expect(selectionSize(cards, ["a", "b", "c"])).toEqual({ bytes: 1500, unknown: false });
    expect(selectionSize([card({ key: "x", installed: true })], ["x"])).toEqual({ bytes: 0, unknown: false });
    expect(selectionSize([card({ key: "x", download_size: null })], ["x"]).unknown).toBe(true);
  });

  it("reports progress only when it is measurable", () => {
    expect(overallProgress(null)).toBeNull();
    const job = { running: true, cancelled: false, bytes_done: 250, bytes_total: 1000, modules: ["m"] };
    expect(overallProgress({ job } as never)).toBe(0.25);
    expect(overallProgress({ job: { ...job, bytes_total: 0 } } as never)).toBeNull();
  });

  it("explains failures in plain words", () => {
    expect(errorHint(card({ error: { code: "offline", message: "dns" } }))).toMatch(/internet/);
    expect(errorHint(card({ error: { code: "initialization", message: "Refused: Busy or unavailable" } }))).toMatch(/Another program/);
    expect(errorHint(card({}))).toBeNull();
  });
});
