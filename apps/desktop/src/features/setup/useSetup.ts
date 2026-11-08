import { useAction } from "@/lib/store/useAction";
import { rpc, useDaemon } from "@/lib/store/daemon";

/** Setup state from the service plus its commands. All work runs in the service. */
export function useSetup() {
  const status = useDaemon((s) => s.modules_setup);
  const action = useAction();
  const call = (key: string, method: string, params?: unknown) => action.run(key, () => rpc(method, params));
  return {
    status,
    ...action,
    start: (keys: string[]) => call("start", "modules.setup.start", { keys }),
    retry: (key: string) => call(`retry:${key}`, "modules.setup.retry", { keys: [key] }),
    cancel: () => call("cancel", "modules.setup.cancel"),
    dismiss: (keys: string[]) => call("dismiss", "modules.setup.dismiss", { keys }),
    forget: (keys: string[]) => call("forget", "modules.forget", { keys }),
    complete: () => call("complete", "modules.setup.complete"),
    refreshCatalog: () => call("catalog", "modules.catalog.refresh"),
    lightTest: (devices: string[]) => call("light", "modules.light_test", { devices }),
    rollback: (id: string) => call(`rollback:${id}`, "modules.rollback", { id }),
  };
}
