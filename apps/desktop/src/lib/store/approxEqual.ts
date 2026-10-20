/**
 * Structural equality that tolerates f32 round-trips: the daemon stores
 * floats as f32, so 0.05 comes back as 0.05000000074505806.
 */
export function approxEqual(a: unknown, b: unknown, epsilon = 1e-4): boolean {
  if (typeof a === "number" && typeof b === "number") return Math.abs(a - b) <= epsilon * Math.max(1, Math.abs(a));
  if (a === b) return true;
  if (typeof a !== "object" || typeof b !== "object" || a === null || b === null) return false;
  if (Array.isArray(a) !== Array.isArray(b)) return false;
  const ka = Object.keys(a);
  const kb = Object.keys(b);
  if (ka.length !== kb.length) return false;
  return ka.every((k) => approxEqual((a as Record<string, unknown>)[k], (b as Record<string, unknown>)[k], epsilon));
}
