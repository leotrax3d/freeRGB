import type { FanCurve } from "@/lib/api/control";

const clamp = (v: number, lo: number, hi: number) => Math.min(hi, Math.max(lo, v));
const round1 = (v: number) => Math.round(v * 10) / 10;

/**
 * Moves point `index` while keeping the curve valid: temperatures stay
 * strictly increasing (1 °C apart), duty never decreases and stays within
 * minimum..100 %, and the last point never passes the critical temperature.
 */
export function movePoint(curve: FanCurve, index: number, temperature: number, duty: number): FanCurve {
  const prev = curve.points[index - 1];
  const next = curve.points[index + 1];
  const tMin = prev ? prev.temperature_c + 1 : -20;
  const tMax = next ? next.temperature_c - 1 : Math.min(110, curve.critical_c);
  const dMin = Math.max(curve.minimum_percent, prev?.duty_percent ?? 0);
  const dMax = next ? next.duty_percent : 100;
  const points = curve.points.map((p, i) =>
    i === index
      ? { temperature_c: round1(clamp(temperature, tMin, tMax)), duty_percent: round1(clamp(duty, dMin, dMax)) }
      : p,
  );
  return { ...curve, points };
}

/** Inserts a point in the middle of the widest temperature gap (max 64 points). */
export function addPoint(curve: FanCurve): FanCurve {
  if (curve.points.length >= 64) return curve;
  let at = 1;
  let widest = -1;
  for (let i = 1; i < curve.points.length; i++) {
    const gap = curve.points[i]!.temperature_c - curve.points[i - 1]!.temperature_c;
    if (gap > widest) [widest, at] = [gap, i];
  }
  const a = curve.points[at - 1]!;
  const b = curve.points[at]!;
  if (widest < 2) return curve;
  const point = {
    temperature_c: round1((a.temperature_c + b.temperature_c) / 2),
    duty_percent: round1((a.duty_percent + b.duty_percent) / 2),
  };
  return { ...curve, points: [...curve.points.slice(0, at), point, ...curve.points.slice(at)] };
}

export function removePoint(curve: FanCurve, index: number): FanCurve {
  if (curve.points.length <= 2) return curve;
  return { ...curve, points: curve.points.filter((_, i) => i !== index) };
}

/**
 * Raises point duties to a new minimum so a changed limit cannot leave the
 * curve invalid; lowering the minimum keeps the points as they are.
 */
export function withMinimum(curve: FanCurve, minimum: number): FanCurve {
  return {
    ...curve,
    minimum_percent: minimum,
    points: curve.points.map((p) => ({ ...p, duty_percent: Math.max(p.duty_percent, minimum) })),
  };
}
