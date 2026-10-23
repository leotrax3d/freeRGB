import { describe, expect, it } from "vitest";
import { externalUrl } from "./transport";
describe("reviewed external destinations", () => {
  it("keeps contact links fixed", () => {
    expect(externalUrl({ kind: "email" })).toBe("mailto:freergbteam@gmail.com");
    expect(externalUrl({ kind: "youtube" })).toBe("https://www.youtube.com/channel/UCh-B-9nUgKeCNJm3ESrpR8w");
  });
  it("encodes a reviewed report inside the official GitHub issue URL", () => {
    const url = new URL(externalUrl({ kind: "issue", title: "FX_TEST &redirect=evil#x", body: "Device info\n<test>" }));
    expect(url.origin).toBe("https://github.com");
    expect(url.pathname).toBe("/leotrax3d/FreeRGB/issues/new");
    expect(url.searchParams.get("body")).toBe("Device info\n<test>");
    expect(url.hash).toBe("");
    expect([...url.searchParams]).toHaveLength(2);
  });
});
