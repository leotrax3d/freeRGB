import { useEffect, useMemo, useState } from "react";
import { create } from "zustand";
import { RpcError } from "@/lib/api/transport";
import type { EffectSpec } from "@/lib/api/types";
import { approxEqual } from "@/lib/store/approxEqual";
import { rpc, useDaemon } from "@/lib/store/daemon";
import { frameThrottle } from "@/lib/utils";

/** After this long without confirmation the daemon's value wins again. */
const SETTLE_MS = 2000;

/** Unconfirmed local edit of the running effect, shared by every preview. */
const useDraft = create<{ draft: EffectSpec | null }>(() => ({ draft: null }));
let settle: ReturnType<typeof setTimeout> | undefined;

/** The effect to display: the local edit while one is pending, else the daemon's. */
export function useCurrentEffect(): EffectSpec | undefined {
  const draft = useDraft((s) => s.draft);
  const remote = useDaemon((s) => s.effect);
  return draft ?? remote;
}

/**
 * Edits the running effect. Changes are shown immediately, sent at most once
 * per frame while dragging, and the draft is dropped once the daemon reports
 * the same effect (or after a short timeout).
 */
export function useEffectDraft() {
  const remote = useDaemon((s) => s.effect);
  const draft = useDraft((s) => s.draft);
  const [error, setError] = useState<string | null>(null);

  const sender = useMemo(
    () =>
      frameThrottle((spec: EffectSpec) => {
        rpc("effects.set", spec).then(
          () => setError(null),
          (e: unknown) => {
            if (e instanceof RpcError && e.detail.kind === "superseded") return;
            setError(e instanceof Error ? e.message : String(e));
            useDraft.setState({ draft: null });
          },
        );
      }),
    [],
  );

  useEffect(() => {
    if (draft && remote && approxEqual(draft, remote)) useDraft.setState({ draft: null });
  }, [draft, remote]);

  const current = draft ?? remote;
  const set = (key: string, value: unknown, commit: boolean) => {
    if (!current) return;
    const next = { ...current, [key]: value } as EffectSpec;
    useDraft.setState({ draft: next });
    clearTimeout(settle);
    settle = setTimeout(() => useDraft.setState({ draft: null }), SETTLE_MS);
    if (commit) sender.flush(next);
    else sender.push(next);
  };

  return { effect: current, set, error };
}
