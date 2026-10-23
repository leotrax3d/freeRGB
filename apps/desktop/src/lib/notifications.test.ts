import { describe, expect, it } from "vitest";
import { actionFailure, incidentObserver, type Notice } from "./notifications";
import { RpcError } from "./api/transport";
import type { Diagnostics } from "./api/types";

const diagnostics = (ids: number[], uptime_s = 10, session = "session-a") => ({
  uptime_s, incident_session: session,
  incidents: ids.map((id) => ({ id, source: `device:${id}`, message: `Problem ${id}`, details: `Diagnostic ${id}` })),
}) as Diagnostics;

describe("service notifications", () => {
  it("delivers simultaneous failures once across snapshot and poll", () => {
    const out: Notice[] = [];
    const observer = incidentObserver((n) => out.push(n));
    observer.diagnostics(diagnostics([1, 2, 3]));
    observer.diagnostics(diagnostics([1, 2, 3], 11));
    expect(out.map((n) => n.message)).toEqual(["Problem 1", "Problem 2", "Problem 3"]);
    expect(out[0]?.details).toBe("Diagnostic 1");
  });
  it("summarizes offline backlogs while retaining every diagnostic", () => {
    const out: Notice[] = [];
    incidentObserver((n) => out.push(n)).diagnostics(diagnostics([1, 2, 3, 4, 5]));
    expect(out).toHaveLength(1);
    expect(out[0]?.message).toContain("5 problems");
    expect(out[0]?.details).toContain("Diagnostic 5");
  });
  it("detects a different service session even when its uptime is higher", () => {
    const out: Notice[] = [];
    const observer = incidentObserver((n) => out.push(n));
    observer.diagnostics(diagnostics([45]));
    observer.diagnostics(diagnostics([1], 200, "session-b"));
    expect(out.map((n) => n.message)).toEqual(["Problem 45", "Problem 1"]);
  });
  it("does not replay incidents after a temporary disconnection", () => {
    const out: Notice[] = [];
    const observer = incidentObserver((n) => out.push(n));
    observer.diagnostics(diagnostics([1]));
    const offline = { connected: false, error: "pipe unavailable", daemon_version: null, api_version: null };
    observer.connection(offline);
    observer.connection({ ...offline, error: "timeout" });
    observer.connection({ ...offline, connected: true, error: null });
    observer.diagnostics(diagnostics([1], 20));
    expect(out.map((n) => n.tone)).toEqual(["error", "warning", "success"]);
  });
  it("supports older diagnostics without incident fields", () => {
    const out: Notice[] = [];
    incidentObserver((n) => out.push(n)).diagnostics({ uptime_s: 20 } as Diagnostics);
    expect(out).toEqual([]);
  });
  it("silences superseded control updates and separates diagnostics from user text", () => {
    expect(actionFailure(new RpcError({ kind: "superseded" }))).toBeNull();
    expect(actionFailure(new Error("private technical detail"))).toMatchObject({
      tone: "error", details: "private technical detail",
    });
    expect(actionFailure(new Error("private technical detail"))?.message).not.toContain("private");
  });
});
