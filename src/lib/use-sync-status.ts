import { listen } from "@tauri-apps/api/event";
import { useEffect, useState } from "react";

import { getSyncStatus, SYNC_CHANGED_EVENT, type SyncStatus } from "./tauri";

export interface SyncStatusView {
  error: null | string;
  status: null | SyncStatus;
}

/** Live sync status: loaded once, then reloaded on every `sync://changed`. */
export function useSyncStatus(): SyncStatusView {
  const [status, setStatus] = useState<null | SyncStatus>(null);
  const [error, setError] = useState<null | string>(null);

  useEffect(() => {
    const load = () => {
      getSyncStatus()
        .then((next) => {
          setStatus(next);
          setError(null);
        })
        .catch((err: unknown) => setError(String(err)));
    };
    load();
    const unlisten = listen(SYNC_CHANGED_EVENT, load);
    unlisten.catch((err: unknown) => setError(String(err)));
    return () => {
      void unlisten.then((fn) => fn());
    };
  }, []);

  return { error, status };
}
