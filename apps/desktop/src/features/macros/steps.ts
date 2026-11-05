import type { MacroStep } from "@/lib/api/control";
import fixtures from "@/lib/api/fixtures/effects.json";
import type { EffectSpec } from "@/lib/api/types";

export const STEP_TYPES: { type: MacroStep["type"]; label: string; input?: boolean }[] = [
  { type: "color", label: "Set colour" },
  { type: "brightness", label: "Set brightness" },
  { type: "effect", label: "Set effect" },
  { type: "delay", label: "Wait" },
  { type: "text", label: "Type text", input: true },
  { type: "key", label: "Press key", input: true },
  { type: "key_down", label: "Key down", input: true },
  { type: "key_up", label: "Key up", input: true },
  { type: "chord", label: "Shortcut", input: true },
];

export function blankStep(type: MacroStep["type"]): MacroStep {
  switch (type) {
    case "color":
      return { type, color: { r: 16, g: 163, b: 127 } };
    case "brightness":
      return { type, value: 1 };
    case "effect":
      return { type, effect: (fixtures as Record<string, EffectSpec>).rainbow_wave! };
    case "delay":
      return { type, ms: 1000 };
    case "text":
      return { type, text: "" };
    case "key_down":
    case "key_up":
      return { type, key: "a" };
    case "key":
      return { type, key: "enter" };
    case "chord":
      return { type, keys: ["ctrl", "c"] };
  }
}

export function moveStep(steps: MacroStep[], index: number, by: number): MacroStep[] {
  const target = index + by;
  if (target < 0 || target >= steps.length) return steps;
  const next = [...steps];
  const [step] = next.splice(index, 1);
  next.splice(target, 0, step!);
  return next;
}
