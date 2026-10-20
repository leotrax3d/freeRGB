import { describe, expect, it } from "vitest";
import { hsvToRgb, parseHex, rgbToHsv, toHex } from "./color";

describe("colour conversions", () => {
  it("round-trips every primary and a few mixed colours", () => {
    for (const hex of ["#000000", "#ffffff", "#ff0000", "#00ff00", "#0000ff", "#10a37f", "#ef4444", "#3b82f6", "#a855f7", "#7f7f7f"]) {
      const rgb = parseHex(hex)!;
      expect(toHex(hsvToRgb(rgbToHsv(rgb)))).toBe(hex);
    }
  });

  it("parses short and unprefixed hex and rejects garbage", () => {
    expect(parseHex("f00")).toEqual({ r: 255, g: 0, b: 0 });
    expect(parseHex(" #10A37F ")).toEqual({ r: 16, g: 163, b: 127 });
    expect(parseHex("#12345")).toBeNull();
    expect(parseHex("zzzzzz")).toBeNull();
  });
});
