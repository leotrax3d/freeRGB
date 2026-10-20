import type { CallError, OptionalSlice, Slices } from "./types";

export type SliceListener = (slice: keyof Slices, value: unknown) => void;

/**
 * The only boundary between the UI and the daemon. In the desktop app this is
 * Tauri IPC; in a plain browser (`npm run dev`) a clearly labelled mock.
 */
export interface Transport {
  readonly kind: "tauri" | "mock";
  rpc<T = unknown>(method: string, params?: unknown): Promise<T>;
  snapshot(): Promise<Partial<Slices>>;
  subscribe(listener: SliceListener): Promise<() => void>;
  wantSlices(slices: OptionalSlice[]): Promise<void>;
  startService(): Promise<void>;
}

export class RpcError extends Error {
  constructor(readonly detail: CallError) {
    super(
      detail.kind === "rpc" || detail.kind === "transport"
        ? detail.message
        : detail.kind === "forbidden"
          ? `Method not allowed: ${detail.method}`
          : "Superseded by a newer value",
    );
  }
}

const inTauri = () => typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;

async function tauriTransport(): Promise<Transport> {
  const { invoke } = await import("@tauri-apps/api/core");
  const { listen } = await import("@tauri-apps/api/event");
  return {
    kind: "tauri",
    async rpc<T>(method: string, params?: unknown) {
      try {
        return (await invoke("rpc", { method, params: params ?? null })) as T;
      } catch (error) {
        throw new RpcError(error as CallError);
      }
    },
    snapshot: () => invoke("snapshot"),
    async subscribe(listener) {
      return listen<{ slice: keyof Slices; value: unknown }>("state", (event) =>
        listener(event.payload.slice, event.payload.value),
      );
    },
    wantSlices: (slices) => invoke("want_slices", { slices }),
    startService: () => invoke("start_service"),
  };
}

let current: Promise<Transport> | undefined;

export function transport(): Promise<Transport> {
  current ??= inTauri()
    ? tauriTransport()
    : import("../mock/transport").then((m) => m.mockTransport());
  return current;
}
