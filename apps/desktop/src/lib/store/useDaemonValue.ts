import { useEffect, useMemo, useRef, useState } from "react";
import { RpcError } from "../api/transport";
import { frameThrottle } from "../utils";
import { rpc } from "./daemon";

/**
 * Local value for a continuous control backed by a daemon setter. While the
 * user drags, the local value wins and writes are throttled per frame; after
 * release the daemon's value takes over again on the next poll.
 */
export function useDaemonValue(remote: number | undefined, write: (value: number) => [string, unknown]) {
  const [local, setLocal] = useState<number | null>(null);
  const [error, setError] = useState<string | null>(null);
  const writeRef = useRef(write);
  writeRef.current = write;

  const sender = useMemo(
    () =>
      frameThrottle((value: number) => {
        const [method, params] = writeRef.current(value);
        rpc(method, params).then(
          () => setError(null),
          (e: unknown) => {
            if (e instanceof RpcError && e.detail.kind === "superseded") return;
            setLocal(null);
            setError(e instanceof Error ? e.message : String(e));
          },
        );
      }),
    [],
  );

  // Once the daemon confirms the committed value, drop the local override.
  useEffect(() => {
    if (local !== null && remote !== undefined && Math.abs(remote - local) < 1e-3) setLocal(null);
  }, [remote, local]);

  return {
    value: local ?? remote ?? 0,
    error,
    change: (value: number) => {
      setLocal(value);
      sender.push(value);
    },
    commit: (value: number) => {
      setLocal(value);
      sender.flush(value);
    },
  };
}
