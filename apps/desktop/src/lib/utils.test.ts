import { describe, expect, it } from "vitest";
import { cn } from "./utils";

describe("cn", () => {
  it("keeps a text colour next to a custom font size", () => {
    expect(cn("text-body text-black")).toBe("text-body text-black");
    expect(cn("text-body", "text-small")).toBe("text-small");
  });
});
