import { useEffect } from "react";
import { create } from "zustand";
import type { OptionalSlice, Slices } from "../api/types";
import { transport, type Transport } from "../api/transport";
import { toast } from "@/components/toast";
import { actionFailure, incidentObserver } from "../notifications";

/**
 * Daemon state as pushed by the backend poller. Each slice is replaced only
 * when it changed, so components selecting one slice re-render only for it.
 */
export interface DaemonState extends Partial<Slices> {
  /** True until the first snapshot arrived. */
  loading: boolean;
  mode: Transport["kind"] | null;
}

export const useDaemon = create<DaemonState>(() => ({ loading: true, mode: null }));

const notices = incidentObserver((notice) => toast(notice.message, notice.tone, notice.details));
function apply(update: Partial<DaemonState>) {
  if (update.connection) notices.connection(update.connection);
  if (update.diagnostics) notices.diagnostics(update.diagnostics);
  useDaemon.setState(update);
}
let started = false;

/** Connects the store to the transport once per app lifetime. */
export async function startDaemonSync(): Promise<void> {
  if (started) return;
  started = true;
  let unsubscribe: (() => void) | undefined;
  try {
    const t = await transport();
    const changed = new Set<keyof Slices>();
    unsubscribe = await t.subscribe((slice, value) => {
      changed.add(slice);
      apply({ [slice]: value } as Partial<DaemonState>);
    });
    await t.wantSlices([...wanted.keys()]);
    const snapshot = await t.snapshot();
    for (const slice of changed) delete snapshot[slice];
    apply({ ...snapshot, loading: false, mode: t.kind });
  } catch (error) {
    unsubscribe?.();
    started = false;
    useDaemon.setState({ loading: false });
    const notice = actionFailure(error);
    if (notice) toast(notice.message, notice.tone, notice.details);
    throw error;
  }
}

const wanted = new Map<OptionalSlice, number>();

/**
 * Declares that a mounted view needs an optional slice. Reference-counted, so
 * the backend only polls what visible views use. Returns the release function.
 */
export function requireSlices(slices: OptionalSlice[]): () => void {
  for (const s of slices) wanted.set(s, (wanted.get(s) ?? 0) + 1);
  void sync();
  return () => {
    for (const s of slices) {
      const n = (wanted.get(s) ?? 1) - 1;
      if (n <= 0) {
        wanted.delete(s);
        // Remounts wait for a fresh poll instead of showing an old reading.
        useDaemon.setState({ [s]: null } as Partial<DaemonState>);
      }
      else wanted.set(s, n);
    }
    void sync();
  };
}

async function sync() {
  try { const t = await transport(); await t.wantSlices([...wanted.keys()]); }
  catch (error) { const notice = actionFailure(error); if (notice) toast(notice.message, notice.tone, notice.details); }
}

/** Keeps the given optional slices polled while the calling component is mounted. */
export function useSlices(...slices: OptionalSlice[]) {
  const key = slices.join(",");
  useEffect(() => requireSlices(key.split(",") as OptionalSlice[]), [key]);
}

export async function rpc<T = unknown>(method: string, params?: unknown): Promise<T> {
  try { return await (await transport()).rpc<T>(method, params); }
  catch (error) {
    const notice = actionFailure(error);
    if (notice) toast(notice.message, notice.tone, notice.details);
    throw error;
  }
}
