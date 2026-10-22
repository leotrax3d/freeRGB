import { describe, expect, it, vi } from "vitest";
import { mockTransport } from "./transport";

describe("public demo adapter", () => {
  it("uses neutral virtual hardware names", async () => {
    const state = JSON.stringify(await mockTransport().snapshot());
    expect(state).not.toMatch(/razer|corsair|icue|basilisk|chroma|k70/i);
  });

  it("cancels simulated downloads without subsequent installation", async () => {
    vi.useFakeTimers();
    try {
      const adapter = mockTransport();
      const key = (await adapter.snapshot()).modules_setup!.cards[0]!.key;
      await adapter.rpc("modules.setup.start", { keys: [key] });
      await vi.advanceTimersByTimeAsync(500);
      expect((await adapter.snapshot()).modules_setup?.job.bytes_done).toBeGreaterThan(0);
      await adapter.rpc("modules.setup.cancel");
      await vi.advanceTimersByTimeAsync(6000);
      const setup = (await adapter.snapshot()).modules_setup!;
      expect(setup.job.running).toBe(false);
      expect(setup.job.cancelled).toBe(true);
      expect(setup.installed).toEqual([]);
    } finally { vi.useRealTimers(); }
  });
  it("updates virtual devices and isolates separate sessions", async () => {
    const first = mockTransport();
    await first.rpc("devices.set_brightness", { id: "mock:keyboard", value: 0.2 });
    expect((await first.snapshot()).devices?.[0]?.brightness).toBe(0.2);
    expect((await mockTransport().snapshot()).devices?.[0]?.brightness).toBe(1);
  });

  it("publishes effects through the same subscription boundary as native IPC", async () => {
    const adapter = mockTransport();
    const changes: string[] = [];
    const stop = await adapter.subscribe((slice) => changes.push(slice));
    try {
      const effect = await adapter.rpc("effects.default", { name: "breathing" });
      await adapter.rpc("effects.set", effect);
      expect((await adapter.snapshot()).effect).toEqual(effect);
      expect(changes).toContain("effect");
      await adapter.wantSlices(["curves"]);
      expect(changes).toContain("curves");
    } finally { stop(); }
  });

  it("rejects unsupported operations instead of reporting fake success", async () => {
    const adapter = mockTransport();
    await expect(adapter.rpc("macros.run", { name: "Flash" })).rejects.toThrow("not simulated");
    const capture = await adapter.rpc("effects.default", { name: "audio_pulse" });
    await expect(adapter.rpc("effects.set", capture)).rejects.toThrow("unavailable in this demo");
    await expect(adapter.rpc("hardware.open")).rejects.toThrow("not mocked");
    await expect(adapter.exportDiagnostics()).rejects.toThrow("Not available");
    expect(adapter.openClassic).toBeUndefined();
  });
});
