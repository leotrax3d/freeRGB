import { RpcError } from "./api/transport";
import type { Diagnostics, Connection } from "./api/types";

export interface Notice { message: string; details?: string; tone: "error" | "warning" | "success" | "info" }
export function actionFailure(error: unknown): Notice | null {
  if (error instanceof RpcError && error.detail.kind === "superseded") return null;
  return { message: error instanceof RpcError && error.detail.kind === "transport"
    ? "The service could not be reached. Check the connection and try again."
    : "The action could not be completed. Please check your settings and try again.",
    details: error instanceof Error ? error.message : String(error), tone: "error" };
}

/** Per-window cursor; reads do not consume incidents for other GUI clients. */
export function incidentObserver(send: (notice: Notice) => void) {
  let cursor = 0;
  let uptime = 0;
  let session: string | undefined;
  let connected: boolean | undefined;
  return {
    connection(value: Connection) {
      if (value.connected === connected) return;
      const previous = connected;
      connected = value.connected;
      if (!value.connected) send({ tone: "warning", message: "The freeRGB service is unavailable. Reconnecting automatically.", details: value.error ?? undefined });
      else if (previous === false) send({ tone: "success", message: "Connected to the freeRGB service." });
    },
    diagnostics(value: Diagnostics) {
      if (value.incident_session !== session || value.uptime_s < uptime) cursor = 0;
      session = value.incident_session;
      uptime = value.uptime_s;
      const fresh = (value.incidents ?? []).filter((event) => event.id > cursor);
      if (!fresh.length) return;
      cursor = fresh[fresh.length - 1]!.id;
      // Summarize a backlog instead of replaying dozens of old failures.
      if (fresh.length > 3) send({ tone: "warning", message: `freeRGB recorded ${fresh.length} problems.`, details: fresh.map((e) => `${e.message}\n${e.details}`).join("\n\n") });
      else fresh.forEach((event) => send({ tone: "error", message: event.message, details: event.details }));
    },
  };
}
