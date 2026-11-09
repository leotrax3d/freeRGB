import { describe, expect, it, vi } from "vitest";
import { mockTransport } from "./transport";

describe("public demo adapter", () => {
  it("pauses and resumes demo fans with daemon replies and background updates", async () => {
    const adapter = mockTransport();
    const updates: unknown[] = [];
    const stop = await adapter.subscribe((slice, value) => {
      if (slice === "cooling_background") updates.push(value);
    });
    try {
      expect((await adapter.snapshot()).cooling_background?.service?.paused).toBe(false);
      for (const paused of [true, false]) {
        expect(await adapter.rpc("cooling.fans.pause", { paused })).toEqual({ paused });
        const background = (await adapter.snapshot()).cooling_background;
        expect(background?.service?.paused).toBe(paused);
        expect(await adapter.rpc("cooling.background.get")).toMatchObject({ service: { paused } });
        expect(updates.at(-1)).toMatchObject({ service: { paused } });
      }
      expect(updates).toHaveLength(2);
      await adapter.rpc("cooling.fans.pause", { paused: true });
      expect((await mockTransport().snapshot()).cooling_background?.service?.paused).toBe(false);
      expect((await adapter.snapshot()).cooling_status?.hardware_control).toBe(false);
      expect((await adapter.snapshot()).onboarding?.safety_acknowledged).toBe(false);
      await expect(adapter.rpc("cooling.sessions.start", {
        mode: "manual", control_id: "/lpc/nct6701d/0/control/2", duty_percent: 100,
      })).rejects.toThrow("safety notice");
    } finally { stop(); }
  });

  it("refuses invalid fan pause parameters without changing state or publishing updates", async () => {
    const adapter = mockTransport();
    await adapter.rpc("cooling.fans.pause", { paused: true });
    const listener = vi.fn();
    const stop = await adapter.subscribe(listener);
    try {
      const before = (await adapter.snapshot()).cooling_session;
      for (const params of [undefined, null, {}, { paused: null }, { paused: "false" }, { paused: 0 }, { paused: [] }, true]) {
        await expect(adapter.rpc("cooling.fans.pause", params)).rejects.toMatchObject({
          detail: { kind: "rpc", code: -32602, message: "paused:boolean required" },
        });
        expect((await adapter.snapshot()).cooling_background?.service?.paused).toBe(true);
        expect((await adapter.snapshot()).cooling_session).toEqual(before);
      }
      expect(listener).not.toHaveBeenCalled();
    } finally { stop(); }
  });

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

  it("runs and stops independent live cooling channels without hardware", async () => {
    const adapter = mockTransport();
    const controls = (await adapter.snapshot()).cooling_channels!.filter((c) => c.kind === "control" && !c.support?.software_mode).slice(0, 2);
    expect(controls).toHaveLength(2);
    await adapter.rpc("safety.acknowledge", { accepted: true, version: 3 });
    await adapter.rpc("cooling.sessions.stop_all");
    for (const control of controls) {
      await adapter.rpc("cooling.sessions.start", { mode: "manual", control_id: control.reading.id, duty_percent: 100 });
    }
    const list = () => adapter.rpc<import("@/features/cooling/liveModel").SessionList>("cooling.sessions.list");
    expect((await list()).active).toHaveLength(2);
    await adapter.rpc("cooling.sessions.stop_channel", { control_id: controls[0]!.reading.id });
    expect((await list()).active.map((j) => j.control_id)).toEqual([controls[1]!.reading.id]);
    await adapter.rpc("cooling.sessions.stop_all");
    expect((await list()).active).toEqual([]);
    expect((await adapter.snapshot()).cooling_session?.jobs?.every((j) => j.state === "stopped")).toBe(true);
  });
});
