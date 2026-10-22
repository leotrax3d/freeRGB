import { describe, expect, it } from "vitest";
import { nameProblem } from "./name-form";

describe("profile/custom effect names", () => {
  it("accepts what the daemon accepts", () => {
    for (const ok of ["Gaming", "Abend 2", "work_late-v2", "Überstunden", "a".repeat(64)]) expect(nameProblem(ok)).toBeNull();
  });
  it("rejects what the daemon rejects", () => {
    for (const bad of [" lead", "trail ", "../up", "a/b", "dot.name", "a".repeat(65)]) expect(nameProblem(bad)).not.toBeNull();
  });
});
