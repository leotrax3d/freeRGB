import { describe, expect, it } from "vitest";
import { mockTransport } from "@/lib/mock/transport";
import { advanceSetup } from "./presentation";

async function download(done: number) {
  const initial = (await mockTransport().snapshot()).modules_setup!;
  return { ...initial, job: { ...initial.job, running: true, bytes_total: 1000, bytes_done: done }, cards: initial.cards.map((card, index) => index === 0 ? { ...card, phase: "downloading" as const, progress: { done, total: 1000 } } : card) };
}

describe("download presentation", () => {
  it("moves continuously towards measured progress without predicting bytes", async () => {
    const initial = await download(0);
    const measured = await download(500);
    const first = advanceSetup(initial, measured, 16);
    const second = advanceSetup(first, measured, 16);
    expect(first.job.bytes_done).toBeGreaterThan(0);
    expect(first.job.bytes_done).toBeLessThan(500);
    expect(second.job.bytes_done).toBeGreaterThan(first.job.bytes_done);
    expect(second.job.bytes_done).toBeLessThanOrEqual(500);
    expect(second.cards[0]!.progress!.done).toBe(second.job.bytes_done);
  });

  it("finishes the bar before showing a completed phase, even for an instant download", async () => {
    let shown = await download(0);
    const measured = await download(1000);
    measured.job.running = false;
    measured.cards[0] = { ...measured.cards[0]!, phase: "ready", progress: null };
    shown = advanceSetup(shown, measured, 16);
    expect(shown.cards[0]!.phase).toBe("downloading");
    expect(shown.job.running).toBe(true);
    for (let frame = 0; frame < 100; frame++) shown = advanceSetup(shown, measured, 16);
    expect(shown).toBe(measured);
    expect(shown.job.bytes_done).toBe(1000);
  });

  it("shows cancellation immediately and does not animate a false completion", async () => {
    const shown = await download(100);
    const cancelled = await download(300);
    cancelled.job.cancelled = true;
    cancelled.job.running = false;
    expect(advanceSetup(shown, cancelled, 16)).toBe(cancelled);
  });
});
