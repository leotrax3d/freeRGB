import { describe, expect, it, vi } from "vitest";
import { RecordingSession, recordedKey } from "./recording";
import { macroProblem } from "@/lib/api/control";
import { MockMacroSupport } from "./mockRecording";

describe("focused macro recording with mocked RPC", () => {
  it("orders events before stop and never saves the returned draft", async () => {
    const calls: string[] = [];
    const call = vi.fn(async <T,>(method: string): Promise<T> => { calls.push(method); return { repeat: 1, steps: [] } as T; });
    const session = new RecordingSession(call);
    await session.start();
    const first = session.event({ type: "key_down", key: "a" });
    const second = session.event({ type: "key_up", key: "a" });
    await session.stop(false); await first; await second;
    expect(calls).toEqual(["macros.record.start", "macros.record.event", "macros.record.event", "macros.record.stop"]);
    await expect(session.event({ type: "text", text: "late" })).rejects.toThrow("ended");
  });
  it("discards when transport rejects an event", async () => {
    const call = vi.fn(async <T,>(method: string): Promise<T> => {
      if (method === "macros.record.event") throw new Error("disconnected");
      return true as T;
    });
    const session = new RecordingSession(call); await session.start();
    await expect(session.event({ type: "privacy_stop" })).rejects.toThrow("disconnected");
    await expect(session.stop(false)).rejects.toThrow("disconnected");
    expect(call.mock.calls.at(-1)?.[0]).toBe("macros.record.stop");
  });
  it("normalizes only portable keys and validates held pairs", () => {
    expect(recordedKey("Control")).toBe("ctrl"); expect(recordedKey("ArrowLeft")).toBe("left");
    expect(recordedKey("F11")).toBeNull();
    expect(macroProblem({repeat:1,steps:[{type:"key_down",key:"a"}]})).toBe("Release every held key.");
    expect(macroProblem({repeat:1,steps:[{type:"key_down",key:"a"},{type:"key_up",key:"a"}]})).toBeNull();
  });
  it("simulates unsaved drafts and rejects duplicate device assignments", () => {
    const mock = new MockMacroSupport();
    mock.call("macros.record.start", {session:"fixture"}, []);
    mock.call("macros.record.event", {session:"fixture",event:{type:"text",text:"hello"}}, []);
    expect(mock.call("macros.record.status", {}, [])).toMatchObject({recording:true});
    expect(mock.call("macros.record.stop", {session:"fixture",discard:false}, [])).toEqual({repeat:1,steps:[{type:"text",text:"hello"}]});
    expect(mock.call("macros.record.status", {}, [])).toMatchObject({recording:false,steps:0});
    const binding = {device_id:"mock:keyboard",usage:63,macro_name:"Demo"};
    expect(() => mock.call("macros.device_keys.save", {bindings:[binding,binding]}, ["Demo"])).toThrow("duplicate");
  });
});
