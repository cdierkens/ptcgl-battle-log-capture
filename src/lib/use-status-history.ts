import { listen } from "@tauri-apps/api/event";
import { useEffect, useState } from "react";

import { appendHistory } from "./status-history";
import { CAPTURE_STATUS_EVENT, type CaptureStatusEvent } from "./tauri";

/** Status events since this window opened, newest first. Mount once per window so tabs share it. */
export function useStatusHistory(): { error: null | string; history: CaptureStatusEvent[] } {
  const [history, setHistory] = useState<CaptureStatusEvent[]>([]);
  const [error, setError] = useState<null | string>(null);

  useEffect(() => {
    const unlisten = listen<CaptureStatusEvent>(CAPTURE_STATUS_EVENT, (event) =>
      setHistory((current) => appendHistory(current, event.payload)),
    );
    unlisten.catch((err: unknown) => setError(String(err)));
    return () => {
      void unlisten.then((fn) => fn());
    };
  }, []);

  return { error, history };
}
