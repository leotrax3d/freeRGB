import { t } from "./messages";
/** Output shaping sent with live sessions (`freergb_control::shaping::OutputShaping`). */
export interface AvoidRange {
  from: number;
  to: number;
  unit?: "percent" | "rpm";
}

export interface OutputShaping {
  offset_percent?: number;
  min_percent?: number;
  max_percent?: number;
  response_delay_ms?: number;
  hysteresis_up_percent?: number;
  hysteresis_down_percent?: number;
  avoid?: AvoidRange[];
  stop_below_percent?: number;
  spin_up?: { percent: number; ms: number };
}

/** Editor state: plain numbers and the avoided ranges as text ("50-60, 1200-1300 rpm"). */
export interface ShapingDraft {
  offset: number;
  min: number;
  max: number;
  delayS: number;
  upPercent: number;
  downPercent: number;
  avoid: string;
  stopBelow: number;
  spinUpPercent: number;
  spinUpMs: number;
}

export const EMPTY_SHAPING: ShapingDraft = { offset: 0, min: 0, max: 100, delayS: 0, upPercent: 0, downPercent: 0, avoid: "", stopBelow: 0, spinUpPercent: 0, spinUpMs: 0 };

/** Parses "a-b" items separated by commas; an "rpm" suffix makes a range RPM-based. */
export function parseAvoid(text: string): AvoidRange[] | string {
  const ranges: AvoidRange[] = [];
  for (const item of text.split(",").map((s) => s.trim()).filter(Boolean)) {
    const m = /^(\d+(?:\.\d+)?)\s*-\s*(\d+(?:\.\d+)?)\s*(rpm|%)?$/i.exec(item);
    if (!m) return t("cooling.shapingModel.cannotReadValueUse5060Or", { p0: item });
    const [from, to] = [Number(m[1]), Number(m[2])];
    const rpm = m[3]?.toLowerCase() === "rpm";
    if (!(from < to) || (!rpm && to > 100)) return t("cooling.shapingModel.valueNeedsFromToValue", { p0: item, p1: rpm ? "" : " ≤ 100 %" });
    ranges.push(rpm ? { from, to, unit: "rpm" } : { from, to });
  }
  if (ranges.length > 8) return t("cooling.shapingModel.atMost8AvoidedRanges");
  return ranges;
}

/** Request payload, or a message when the draft is invalid; `null` when nothing is set. */
export function shapingPayload(d: ShapingDraft, allowStop: boolean): OutputShaping | null | string {
  const avoid = parseAvoid(d.avoid);
  if (typeof avoid === "string") return avoid;
  if (!(d.min >= 0 && d.min <= d.max && d.max <= 100)) return t("cooling.shapingModel.need0MinMax100");
  if (Math.abs(d.offset) > 100) return t("cooling.shapingModel.offsetMustBe100100");
  if (!(d.delayS >= 0 && d.delayS <= 60)) return t("cooling.shapingModel.delayMustBe060S");
  if (!(d.upPercent >= 0 && d.upPercent <= 50 && d.downPercent >= 0 && d.downPercent <= 50)) return t("cooling.shapingModel.hysteresisMustBe050");
  if (d.stopBelow > 0 && !allowStop) return t("cooling.shapingModel.fanStopNeedsAnUnrestrictedFanChannel");
  if (d.stopBelow > 50) return t("cooling.shapingModel.fanStopThresholdMustBe150");
  if (d.spinUpMs > 10000 || d.spinUpPercent > 100) return t("cooling.shapingModel.spinUpIsAtMost100For");
  const out: OutputShaping = {};
  if (d.offset) out.offset_percent = d.offset;
  if (d.min) out.min_percent = d.min;
  if (d.max !== 100) out.max_percent = d.max;
  if (d.delayS) out.response_delay_ms = Math.round(d.delayS * 1000);
  if (d.upPercent) out.hysteresis_up_percent = d.upPercent;
  if (d.downPercent) out.hysteresis_down_percent = d.downPercent;
  if (avoid.length) out.avoid = avoid;
  if (d.stopBelow > 0) out.stop_below_percent = d.stopBelow;
  if (d.spinUpMs > 0 && d.spinUpPercent > 0) out.spin_up = { percent: d.spinUpPercent, ms: Math.round(d.spinUpMs) };
  return Object.keys(out).length ? out : null;
}
