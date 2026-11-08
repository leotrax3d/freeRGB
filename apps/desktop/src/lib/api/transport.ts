import type { CallError, OptionalSlice, Slices, UpdateInfo } from "./types";

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
  openClassic?(): Promise<void>;
  /** Verified release updates (desktop only). */
  update?(action: "check" | "download" | "install"): Promise<UpdateInfo | null>;
  /** Saves the redacted diagnostics report; resolves to the file path. */
  exportDiagnostics(includeCoolingHardware?: boolean): Promise<string>;
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

/** Explicit Tauri entry points for the public session API. */
export function coolingSessionCommand(method: string): string | null {
  const prefix = "cooling.sessions.";
  const action = method.startsWith(prefix) ? method.slice(prefix.length) : "";
  return ["status", "start", "stop", "list", "stop_channel", "stop_all"].includes(action)
    ? `cooling_sessions_${action}` : null;
}

async function tauriTransport(): Promise<Transport> {
  const { invoke } = await import("@tauri-apps/api/core");
  const { listen } = await import("@tauri-apps/api/event");
  return {
    kind: "tauri",
    async rpc<T>(method: string, params?: unknown) {
      try {
        const sessionCommand = coolingSessionCommand(method);
        return (await (sessionCommand
          ? invoke(sessionCommand, { params: params ?? null })
          : invoke("rpc", { method, params: params ?? null }))) as T;
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
    openClassic: () => invoke("open_classic"),
    update: (action) => invoke<UpdateInfo | null>("update_action", { action }),
    exportDiagnostics: (includeCoolingHardware) => invoke<string>("export_diagnostics", { includeCoolingHardware: includeCoolingHardware ?? false }),
  };
}

let current: Promise<Transport> | undefined;

export function transport(): Promise<Transport> {
  current ??= !__WEB_DEMO__ && inTauri()
    ? tauriTransport()
    : import("../mock/transport").then((m) => m.mockTransport());
  return current;
}


export type ExternalDestination = { kind: "email" | "youtube" | "publisher" } | { kind: "issue"; title: string; body: string };
export function externalUrl(destination: ExternalDestination): string {
  switch (destination.kind) {
    case "email": return "mailto:freergbteam@gmail.com";
    case "youtube": return "https://www.youtube.com/channel/UCh-B-9nUgKeCNJm3ESrpR8w";
    case "publisher": return "https://github.com/leotrax3d/freeRGB";
    case "issue": {
      if (!destination.title || [...destination.title].length > 160 || new TextEncoder().encode(destination.body).length > 6000) throw new Error("Shorten the report before opening GitHub.");
      const url = new URL("https://github.com/leotrax3d/freeRGB/issues/new");
      url.searchParams.set("title", destination.title); url.searchParams.set("body", destination.body);
      if (url.href.length > 16000) throw new Error("Report is too long for a browser link. Copy it instead.");
      return url.href;
    }
  }
}
export async function openExternal(destination: ExternalDestination): Promise<void> {
  const url = externalUrl(destination);
  if (!__WEB_DEMO__ && inTauri()) {
    const { invoke } = await import("@tauri-apps/api/core");
    await invoke("open_external", { destination });
  } else {
    window.open(url, "_blank", "noopener,noreferrer");
  }
}
