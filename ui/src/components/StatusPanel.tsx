import { Badge, Callout, Panel } from "@pokedojo/ui";
import { listen } from "@tauri-apps/api/event";
import { useEffect, useState } from "react";

import { labelForState } from "../lib/capture-status";
import { type CaptureStatusEvent, getCaptureState } from "../lib/tauri";

const POLL_INTERVAL_MS = 2000;

export function StatusPanel() {
  const [currentState, setCurrentState] = useState<null | string>(null);
  const [error, setError] = useState<null | string>(null);
  const [history, setHistory] = useState<CaptureStatusEvent[]>([]);

  useEffect(() => {
    let cancelled = false;
    getCaptureState()
      .then((state) => {
        if (!cancelled) {
          setCurrentState((prev) => prev ?? state);
        }
      })
      .catch((err: unknown) => setError(String(err)));

    const unlisten = listen<CaptureStatusEvent>("capture-status", (event) => {
      const status = event.payload;
      setCurrentState(status.state);
      setHistory((prev) => [status, ...prev].slice(0, 50));
    });
    unlisten.catch((err: unknown) => setError(String(err)));

    const pollId = setInterval(() => {
      getCaptureState()
        .then((state) => {
          if (!cancelled) {
            setCurrentState((prev) => (prev === state ? prev : state));
          }
        })
        .catch(() => undefined);
    }, POLL_INTERVAL_MS);

    return () => {
      cancelled = true;
      clearInterval(pollId);
      void unlisten.then((fn) => fn());
    };
  }, []);

  const info = currentState === null ? null : labelForState(currentState);

  return (
    <Panel className="space-y-3 p-4">
      <div className="flex items-center gap-3">
        <h2 className="text-sm font-semibold text-content-secondary">Capture status</h2>
        {info === null ? (
          <Badge variant="secondary">Connecting…</Badge>
        ) : (
          <Badge variant={info.tone}>{info.label}</Badge>
        )}
      </div>

      {error !== null ? (
        <Callout variant="critical">
          Failed to connect to capture status: {error}
        </Callout>
      ) : null}

      {currentState === "PermissionRequired" ? (
        <Callout variant="critical">
          Screen Recording access is required. Grant it in System Settings → Privacy
          &amp; Security → Screen Recording, then restart the app.
        </Callout>
      ) : null}

      <div className="max-h-48 overflow-y-auto rounded border border-border p-2 font-mono text-xs">
        {history.length === 0 ? (
          <p className="text-content-subtle">No capture events yet.</p>
        ) : (
          history.map((h, i) => (
            <p className="text-content-secondary" key={`${h.ts}-${i}`}>
              {new Date(h.ts).toLocaleTimeString()} [{labelForState(h.state).label}]{" "}
              {h.message}
            </p>
          ))
        )}
      </div>
    </Panel>
  );
}