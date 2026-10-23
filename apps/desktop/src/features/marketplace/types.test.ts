import { describe, expect, it } from "vitest";
import catalog from "../../../../../effects/catalog.json";
import { defaults, matches, type Package } from "./types";
import { mockTransport } from "@/lib/mock/transport";
describe("marketplace contracts", () => {
  const raw = catalog.effects[0]!;
  const pkg: Package = { ...raw, preview: [raw.preview[0]!, raw.preview[1]!] };
  it("uses the packaged Rust effect's parameter defaults", () => {
    expect(defaults(pkg)).toEqual({ speed: 1, brightness: 1 });
  });
  it("searches publisher names", () => { expect(matches(pkg, "freeRGB Team", false, false)).toBe(true); });
  it("filters by text and installed state", () => {
    expect(matches(pkg, " AURORA ", true, true)).toBe(true);
    expect(matches(pkg, "aurora", false, true)).toBe(false);
    expect(matches(pkg, "no match", true, false)).toBe(false);
  });
  it("browser previews expose official metadata but cannot install effects", async () => {
    const mock = mockTransport();
    const status = await mock.rpc<{packages: {catalog: Package[]; installed: Package[]}}>("marketplace.status");
    expect(status.packages.catalog).toHaveLength(3);
    expect(status.packages.installed).toEqual([]);
    await expect(mock.rpc("marketplace.install", { id: pkg.id })).rejects.toThrow();
  });
});
