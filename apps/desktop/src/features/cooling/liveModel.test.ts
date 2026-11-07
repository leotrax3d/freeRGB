import { describe, expect, it } from "vitest";
import type { CoolingSession } from "@/lib/api/types";
import { activeCount, clampDuty, liveChannels, liveConfirmation, type SessionList } from "./liveModel";

const job = (job_id: number, control_id: string, state: string, mode = "curve", error: string | null = null): CoolingSession =>
  ({ available: true, job_id, state, mode, control_id, report: null, error, tier: "verified" });

describe("live channel model", () => {
  it("shows every channel once with its newest job", () => {
    const jobs = [job(4, "/a/control/1", "executing", "manual"), job(3, "/a/control/2", "faulted", "curve", "sensor lost"), job(2, "/a/control/1", "stopped"), job(1, "/a/control/3", "completed")];
    const session = { ...jobs[0]!, jobs };
    const list: SessionList = { active: [], max_active: 16, assignments: { resume_pending: false, assignments: [
      { control_id: "/a/control/4", mode: "curve", temperature_id: "t", rpm_id: "r", duty_percent: null, needs_attention: "binding changed" },
      { control_id: "/a/control/1", mode: "manual", temperature_id: "t", rpm_id: "r", duty_percent: 50, needs_attention: null },
    ] } };
    const channels = liveChannels(session, list);
    expect(channels.map((c) => [c.control_id, c.state])).toEqual([
      ["/a/control/1", "manual"], ["/a/control/2", "fault"], ["/a/control/4", "needs_attention"],
    ]);
    expect(channels[1]!.error).toBe("sensor lost");
    expect(activeCount(session)).toBe(1);
  });

  it("a stopped job clears an older fault on the same channel", () => {
    const jobs = [job(2, "/a/control/1", "stopped"), job(1, "/a/control/1", "faulted")];
    expect(liveChannels({ ...jobs[0]!, jobs }, null)).toEqual([]);
    expect(liveChannels(null, null)).toEqual([]);
  });

  it("supports the Classic single-job status and keeps channel faults separate", () => {
    const legacy = job(1, "/a/control/1", "authorizing");
    expect(activeCount(legacy)).toBe(1);
    expect(liveChannels(legacy, null)[0]?.state).toBe("starting");
    const jobs = [job(3, "/a/control/2", "stopping"), job(2, "/a/control/1", "failed", "calibration", "sensor lost")];
    expect(liveChannels({ ...jobs[0]!, jobs }, null).map((c) => [c.control_id, c.state, c.error])).toEqual([
      ["/a/control/1", "fault", "sensor lost"], ["/a/control/2", "stopping", null],
    ]);
    expect(activeCount({ ...jobs[0]!, jobs })).toBe(1);
  });

  it("clamps manual duty to the tier floor and pumps to 100 %", () => {
    expect(clampDuty(20, 40)).toBe(40);
    expect(clampDuty(55.4, 40)).toBe(55);
    expect(clampDuty(30, 100)).toBe(100);
    expect(clampDuty(150, 0)).toBe(100);
    expect(clampDuty(Number.NaN, 40)).toBe(100);
  });

  it("asks for confirmation once per app session", () => {
    liveConfirmation.reset();
    expect(liveConfirmation.needed()).toBe(true);
    liveConfirmation.confirm();
    expect(liveConfirmation.needed()).toBe(false);
    liveConfirmation.reset();
  });
});
