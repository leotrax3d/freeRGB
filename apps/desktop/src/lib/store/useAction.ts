import { useCallback, useState } from "react";

/**
 * Runs one user action at a time and keeps its error for inline display.
 * `pending` holds a key (e.g. the profile name) so only that row shows busy.
 */
export function useAction() {
  const [pending, setPending] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);
  const run = useCallback(async (key: string, action: () => Promise<unknown>): Promise<boolean> => {
    setPending(key);
    setError(null);
    try {
      await action();
      return true;
    } catch (e) {
      setError(e instanceof Error ? e.message : String(e));
      return false;
    } finally {
      setPending(null);
    }
  }, []);
  return { pending, error, run, clearError: () => setError(null) };
}
