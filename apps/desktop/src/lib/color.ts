import type { Rgb } from "./api/types";

/** Hue 0..360, saturation and value 0..1. */
export interface Hsv {
  h: number;
  s: number;
  v: number;
}

export function rgbToHsv({ r, g, b }: Rgb): Hsv {
  const [rn, gn, bn] = [r / 255, g / 255, b / 255];
  const max = Math.max(rn, gn, bn);
  const d = max - Math.min(rn, gn, bn);
  let h = 0;
  if (d > 0) {
    if (max === rn) h = ((gn - bn) / d) % 6;
    else if (max === gn) h = (bn - rn) / d + 2;
    else h = (rn - gn) / d + 4;
    h = (h * 60 + 360) % 360;
  }
  return { h, s: max === 0 ? 0 : d / max, v: max };
}

export function hsvToRgb({ h, s, v }: Hsv): Rgb {
  const c = v * s;
  const x = c * (1 - Math.abs(((h / 60) % 2) - 1));
  const m = v - c;
  const [r, g, b] =
    h < 60 ? [c, x, 0] : h < 120 ? [x, c, 0] : h < 180 ? [0, c, x] : h < 240 ? [0, x, c] : h < 300 ? [x, 0, c] : [c, 0, x];
  const to = (n: number) => Math.round((n + m) * 255);
  return { r: to(r), g: to(g), b: to(b) };
}

export function toHex({ r, g, b }: Rgb): string {
  return "#" + [r, g, b].map((n) => n.toString(16).padStart(2, "0")).join("");
}

/** Accepts `#rgb`, `#rrggbb`, with or without `#`; returns null otherwise. */
export function parseHex(text: string): Rgb | null {
  let hex = text.trim().replace(/^#/, "");
  if (/^[0-9a-f]{3}$/i.test(hex)) hex = [...hex].map((c) => c + c).join("");
  if (!/^[0-9a-f]{6}$/i.test(hex)) return null;
  const n = parseInt(hex, 16);
  return { r: (n >> 16) & 255, g: (n >> 8) & 255, b: n & 255 };
}

export function sameRgb(a: Rgb, b: Rgb): boolean {
  return a.r === b.r && a.g === b.g && a.b === b.b;
}

/** Quick picks: the logo palette plus the extremes Classic offers. */
export const PRESETS: readonly { name: string; rgb: Rgb }[] = [
  { name: "Red", rgb: { r: 239, g: 68, b: 68 } },
  { name: "Amber", rgb: { r: 245, g: 158, b: 11 } },
  { name: "Green", rgb: { r: 16, g: 163, b: 127 } },
  { name: "Blue", rgb: { r: 59, g: 130, b: 246 } },
  { name: "Violet", rgb: { r: 168, g: 85, b: 247 } },
  { name: "White", rgb: { r: 255, g: 255, b: 255 } },
  { name: "Off", rgb: { r: 0, g: 0, b: 0 } },
];
