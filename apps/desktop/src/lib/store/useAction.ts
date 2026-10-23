import { useCallback, useState } from "react";
import { actionFailure } from "../notifications";
import { toast } from "@/components/toast";

/**
 * Runs one user action at a time, keeps its error for inline display and
 * optionally confirms success with a toast.
 * `pending` holds a key (e.g. the profile name) so only that row shows busy.
 */
export function useAction() {
  const [pending, setPending] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);
  const run = useCallback(async (key: string, action: () => Promise<unknown>, success?: string): Promise<boolean> => {
    setPending(key);
    setError(null);
    try {
      await action();
      if (success) toast(success);
      return true;
    } catch (e) {
      const notice = actionFailure(e);
      if (notice) toast(notice.message, notice.tone, notice.details);
      setError(e instanceof Error ? e.message : String(e));
      return false;
    } finally {
      setPending(null);
    }
  }, []);
  return { pending, error, run, clearError: () => setError(null) };
}
