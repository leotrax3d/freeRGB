import { create } from "zustand";
import type { OptionalSlice, Slices } from "../api/types";
import { transport, type Transport } from "../api/transport";

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

let started = false;

/** Connects the store to the transport once per app lifetime. */
export async function startDaemonSync(): Promise<void> {
  if (started) return;
  started = true;
  const t = await transport();
  // Subscribe first so no change between snapshot and listen gets lost.
  await t.subscribe((slice, value) => useDaemon.setState({ [slice]: value } as Partial<DaemonState>));
  const snapshot = await t.snapshot();
  useDaemon.setState({ ...snapshot, loading: false, mode: t.kind });
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
      if (n <= 0) wanted.delete(s);
      else wanted.set(s, n);
    }
    void sync();
  };
}

async function sync() {
  const t = await transport();
  await t.wantSlices([...wanted.keys()]);
}

export async function rpc<T = unknown>(method: string, params?: unknown): Promise<T> {
  return (await transport()).rpc<T>(method, params);
}
