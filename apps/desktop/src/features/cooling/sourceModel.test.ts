import { describe, expect, it } from "vitest";
import { fileSensorProblem, groupSources, type CatalogEntry } from "./sourceModel";

const entry = (id: string, category: CatalogEntry["category"], label = id): CatalogEntry => ({
  id, label, category, provider: "x", unit: "°C", value: 40, age_ms: 0, error: null,
});

describe("groupSources", () => {
  it("orders categories, sorts labels and drops empty groups", () => {
    const groups = groupSources([entry("b", "gpu", "B"), entry("a", "cpu"), entry("c", "gpu", "A"), entry("f", "file")]);
    expect(groups.map((g) => g.category)).toEqual(["cpu", "gpu", "file"]);
    expect(groups[1]!.entries.map((e) => e.id)).toEqual(["c", "b"]);
  });
  it("puts unknown categories under other", () => {
    expect(groupSources([entry("z", "weird" as CatalogEntry["category"])])[0]!.category).toBe("other");
  });
});

describe("fileSensorProblem", () => {
  it("accepts absolute local paths", () => {
    expect(fileSensorProblem("gpu", "C:\\temps\\gpu.txt", [])).toBeNull();
    expect(fileSensorProblem("gpu", "/run/gpu", [])).toBeNull();
  });
  it("rejects bad names, duplicates, relative and network paths", () => {
    expect(fileSensorProblem("bad name", "/t", [])).toMatch(/Name/);
    expect(fileSensorProblem("gpu", "/t", ["gpu"])).toMatch(/already/);
    expect(fileSensorProblem("gpu", "temp.txt", [])).toMatch(/absolute/);
    expect(fileSensorProblem("gpu", "\\\\srv\\share\\t", [])).toMatch(/Network/);
  });
});
