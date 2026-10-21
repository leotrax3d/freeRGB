import { create } from "zustand";
import type { EffectSpec } from "@/lib/api/types";
import { approxEqual } from "@/lib/store/approxEqual";
import { rpc, useDaemon } from "@/lib/store/daemon";

/**
 * The daemon does not report which profile is active, so the app remembers
 * the profile it loaded last and the lighting that resulted from it. If the
 * lighting changes afterwards, the profile shows as modified.
 */
const KEY = "freergb.activeProfile";

interface Baseline {
  name: string;
  effect: EffectSpec | null;
  brightness: number | null;
}

function load(): Baseline | null {
  try {
    return JSON.parse(localStorage.getItem(KEY) ?? "null") as Baseline | null;
  } catch {
    return null;
  }
}

const useBaseline = create<{ baseline: Baseline | null }>(() => ({ baseline: load() }));

function save(baseline: Baseline | null) {
  useBaseline.setState({ baseline });
  try {
    localStorage.setItem(KEY, JSON.stringify(baseline));
  } catch {
    // Only a convenience.
  }
}

/** Call after a successful `profiles.load`: records what the service now runs. */
export async function markLoaded(name: string) {
  save({ name, effect: null, brightness: null });
  // Best effort: without a baseline the profile simply shows as active.
  const fetched = await Promise.all([rpc<EffectSpec>("effects.get"), rpc<{ brightness: number }>("diagnostics.get")]).catch(() => null);
  if (!fetched) return;
  const [effect, diagnostics] = fetched;
  if (useBaseline.getState().baseline?.name === name) save({ name, effect, brightness: diagnostics.brightness });
}

/** Call when a profile is deleted. */
export function forget(name: string) {
  if (useBaseline.getState().baseline?.name === name) save(null);
}

export type ProfileState = "active" | "modified" | null;

export function useProfileState(name: string): ProfileState {
  const baseline = useBaseline((s) => s.baseline);
  const effect = useDaemon((s) => s.effect);
  const brightness = useDaemon((s) => s.diagnostics?.brightness);

  if (!baseline || baseline.name !== name) return null;
  if (baseline.effect === null) return "active";
  return approxEqual(baseline.effect, effect) && approxEqual(baseline.brightness, brightness) ? "active" : "modified";
}
