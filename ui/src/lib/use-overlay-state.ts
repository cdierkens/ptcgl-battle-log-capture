import { listen } from "@tauri-apps/api/event";
import { useEffect, useState } from "react";

import { getCaptureState, OVERLAY_STATE_EVENT, type OverlayStateEvent } from "./tauri";

export interface OverlayStateView {
  /** True when the latest save came from the clipboard watcher. */
  autosaved: boolean;
  error: null | string;
  /** The auto-saved file, while `autosaved`. */
  path: null | string;
  state: null | string;
}

/** The engine's overlay state, kept live from `overlay://state`. */
export function useOverlayState(): OverlayStateView {
  const [event, setEvent] = useState<null | OverlayStateEvent>(null);
  const [error, setError] = useState<null | string>(null);

  useEffect(() => {
    getCaptureState()
      // An event may already have delivered a newer state.
      .then((initial) => setEvent((current) => current ?? { autosaved: false, path: null, state: initial }))
      .catch((err: unknown) => setError(String(err)));
    const unlisten = listen<OverlayStateEvent>(OVERLAY_STATE_EVENT, (next) => setEvent(next.payload));
    unlisten.catch((err: unknown) => setError(String(err)));
    return () => {
      void unlisten.then((fn) => fn());
    };
  }, []);

  return { autosaved: event?.autosaved ?? false, error, path: event?.path ?? null, state: event?.state ?? null };
}
