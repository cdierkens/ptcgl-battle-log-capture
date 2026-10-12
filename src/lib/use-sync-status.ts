import { listen } from "@tauri-apps/api/event";
import { useCallback, useEffect, useState } from "react";

import { logSavedState } from "./capture-status";
import { getSyncStatus, OVERLAY_STATE_EVENT, type OverlayStateEvent, SYNC_CHANGED_EVENT, type SyncStatus } from "./tauri";

export interface SyncStatusView {
  error: null | string;
  refresh: () => void;
  status: null | SyncStatus;
}

/** Live sync status: reloaded on `sync://changed` and after each save, which queues deliveries. */
export function useSyncStatus(): SyncStatusView {
  const [status, setStatus] = useState<null | SyncStatus>(null);
  const [error, setError] = useState<null | string>(null);

  const refresh = useCallback(() => {
    getSyncStatus()
      .then((next) => {
        setStatus(next);
        setError(null);
      })
      .catch((err: unknown) => setError(String(err)));
  }, []);

  useEffect(() => {
    refresh();
    const unlistenSync = listen(SYNC_CHANGED_EVENT, refresh);
    const unlistenOverlay = listen<OverlayStateEvent>(OVERLAY_STATE_EVENT, (event) => {
      if (logSavedState(event.payload.state)) refresh();
    });
    for (const unlisten of [unlistenSync, unlistenOverlay]) unlisten.catch((err: unknown) => setError(String(err)));
    return () => {
      void unlistenSync.then((fn) => fn());
      void unlistenOverlay.then((fn) => fn());
    };
  }, [refresh]);

  return { error, refresh, status };
}
