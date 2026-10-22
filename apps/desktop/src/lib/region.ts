import type { Region } from "./api/types";

/**
 * Editing rules for canvas regions. The daemon accepts x, y in 0..1 and
 * w, h in 0.01..1 (`Region::is_valid`); the editor additionally keeps the
 * rectangle inside the canvas and above a size that is still grabbable.
 */
export const MIN_SIZE = 0.05;
export const GRID = 0.05;

const clamp = (v: number, lo: number, hi: number) => Math.min(hi, Math.max(lo, v));
const snap = (v: number, step: number) => Math.round(v / step) * step;
/** Removes float noise such as 0.30000000000000004. */
const tidy = (v: number) => Math.round(v * 10_000) / 10_000;

export function normalize(r: Region): Region {
  const w = tidy(clamp(r.w, MIN_SIZE, 1));
  const h = tidy(clamp(r.h, MIN_SIZE, 1));
  return { x: tidy(clamp(r.x, 0, 1 - w)), y: tidy(clamp(r.y, 0, 1 - h)), w, h };
}

/** Moves `start` by a pointer delta (canvas fractions); snaps the corner. */
export function moved(start: Region, dx: number, dy: number, snapping = true): Region {
  let x = start.x + dx;
  let y = start.y + dy;
  if (snapping) {
    x = snap(x, GRID);
    y = snap(y, GRID);
  }
  return normalize({ ...start, x, y });
}

/** Resizes from the bottom-right corner; the top-left corner stays put. */
export function resized(start: Region, dx: number, dy: number, snapping = true): Region {
  let right = start.x + start.w + dx;
  let bottom = start.y + start.h + dy;
  if (snapping) {
    right = snap(right, GRID);
    bottom = snap(bottom, GRID);
  }
  const w = clamp(right, start.x + MIN_SIZE, 1) - start.x;
  const h = clamp(bottom, start.y + MIN_SIZE, 1) - start.y;
  return normalize({ ...start, w, h });
}

/** Keyboard editing: arrows move, with `resize` they change width/height. */
export function nudged(r: Region, key: string, step: number, resize: boolean): Region | null {
  const d = { ArrowLeft: [-step, 0], ArrowRight: [step, 0], ArrowUp: [0, -step], ArrowDown: [0, step] }[key];
  if (!d) return null;
  const [dx, dy] = d as [number, number];
  return resize ? normalize({ ...r, w: r.w + dx, h: r.h + dy }) : normalize({ ...r, x: r.x + dx, y: r.y + dy });
}

export const FULL: Region = { x: 0, y: 0, w: 1, h: 1 };
