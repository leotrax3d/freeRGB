import { describe, expect, it } from "vitest";
import { mockTransport } from "@/lib/mock/transport";
import type { CoolingProfileImport, CoolingServiceState } from "@/lib/api/types";
import { profileDocument } from "./profileServiceModel";

describe("cooling profile transfer", () => {
  it("rejects malformed JSON and non-object documents", () => {
    for (const text of ["{", "null", "[]", '"profile"', "42"]) expect(() => profileDocument(text)).toThrow();
    expect(profileDocument('{"schema":"freergb.cooling-profile"}')).toEqual({ schema: "freergb.cooling-profile" });
  });

  it("round trips a complete profile without activating or altering current cooling", async () => {
    const transport = mockTransport();
    const before = await transport.snapshot();
    const profile = { channels: [], assignments: [], curve_set: { nodes: {} }, sensors: { nodes: {} } };
    const document = { schema: "freergb.cooling-profile", version: 2, name: "Quiet", source: { board: "mock", devices: [] }, profile };
    const imported = await transport.rpc<CoolingProfileImport>("cooling.profiles.import", { document: JSON.stringify(document), name: "Copy" });
    expect(imported.name).toBe("Copy");
    expect(imported.report.needs_confirmation).toBe(false);
    expect(await transport.rpc("cooling.profiles.export", { name: "Copy" })).toEqual({ ...document, name: "Copy" });
    const after = await transport.snapshot();
    expect(after.cooling_profiles?.active).toBe(before.cooling_profiles?.active);
    expect(after.cooling_background?.saved).toEqual(before.cooling_background?.saved);
    expect(after.cooling_background?.service?.channels).toEqual(before.cooling_background?.service?.channels);
    expect(after.cooling_session).toEqual(before.cooling_session);
  });

  it("clears foreign-board fan confirmations and reports the requirement", async () => {
    const transport = mockTransport();
    const result = await transport.rpc<CoolingProfileImport>("cooling.profiles.import", { document: {
      schema: "freergb.cooling-profile", version: 2, name: "Foreign", source: { board: "other", devices: [] },
      profile: { channels: [], assignments: [{ confirmed_fan: true }] },
    } });
    expect(result.report.needs_confirmation).toBe(true);
    const exported = await transport.rpc<{ profile: { assignments: { confirmed_fan: boolean }[] } }>("cooling.profiles.export", { name: result.name });
    expect(exported.profile.assignments[0]?.confirmed_fan).toBe(false);
  });

  it("persists startup configuration without starting a service or activating a profile", async () => {
    const transport = mockTransport();
    const before = await transport.snapshot();
    await transport.rpc("cooling.service.set", { mode: "service", boot_profile: "Quiet" });
    expect(await transport.rpc<CoolingServiceState>("cooling.service.get")).toMatchObject({ mode: "service", boot_profile: "Quiet", headless: false, boot: { state: "unavailable" } });
    const after = await transport.snapshot();
    expect(after.cooling_profiles).toEqual(before.cooling_profiles);
    expect(after.cooling_background?.saved).toEqual(before.cooling_background?.saved);
    expect(after.cooling_session).toEqual(before.cooling_session);
    await transport.rpc("cooling.service.set", { mode: "login", boot_profile: null });
    expect(await transport.rpc("cooling.service.get")).toMatchObject({ mode: "login", boot_profile: null });
    await expect(transport.rpc("cooling.service.set", { mode: "invalid" })).rejects.toThrow();
  });
});
