import { Button, Callout, CopyButton } from "@pokedojo/ui";
import { useEffect, useState } from "react";

import { historyText } from "../lib/capture-status";
import { relativeTime } from "../lib/match-row";
import { lastResultsAt } from "../lib/status-history";
import { type CaptureStatusEvent, getDiagnostics, openScreenRecordingSettings, relaunchApp } from "../lib/tauri";

export function HelpTab({ history, state }: { history: CaptureStatusEvent[]; state: null | string }) {
  const [diagnostics, setDiagnostics] = useState<null | string>(null);
  const [error, setError] = useState<null | string>(null);

  useEffect(() => {
    getDiagnostics()
      .then(setDiagnostics)
      .catch((err: unknown) => setError(String(err)));
  }, [state]);

  const lines = history.map((event) => `${new Date(event.ts).toLocaleTimeString()} ${historyText(event)}`);
  const report = diagnostics === null ? "" : [diagnostics, "", "Recent events:", ...lines].join("\n");
  const lastResults = lastResultsAt(history);

  return (
    <div className="space-y-5 p-4">
      {error !== null ? <Callout variant="critical">{error}</Callout> : null}

      <section className="space-y-2">
        <h3 className="text-sm font-semibold text-content-primary">Screen Recording</h3>
        {state === "PermissionRequired" ? (
          <Callout
            actions={
              <div className="flex gap-2">
                <Button onClick={() => void openScreenRecordingSettings().catch((err: unknown) => setError(String(err)))} size="sm">
                  Open System Settings
                </Button>
                <Button onClick={() => void relaunchApp()} size="sm" variant="outline">
                  Quit &amp; reopen
                </Button>
              </div>
            }
            variant="critical"
          >
            Needed to spot the results screen. Turn on Battle Log Capture in Privacy &amp; Security → Screen Recording,
            then quit and reopen.
          </Callout>
        ) : (
          <p className="text-sm text-content-secondary">{state === null ? "Checking…" : "Allowed."}</p>
        )}
      </section>

      <section className="space-y-1">
        <h3 className="text-sm font-semibold text-content-primary">Detection</h3>
        <p className="text-sm text-content-secondary">
          {lastResults === null
            ? "No results screen seen since the app opened."
            : `Last results screen ${relativeTime(lastResults, Date.now())}.`}
        </p>
      </section>

      <section className="space-y-2">
        <h3 className="text-sm font-semibold text-content-primary">Recent activity</h3>
        {lines.length === 0 ? (
          <p className="text-sm text-content-subtle">Nothing yet.</p>
        ) : (
          <ul className="max-h-48 space-y-1 overflow-y-auto rounded-md border border-border p-2 text-xs text-content-secondary">
            {lines.map((line, index) => (
              <li key={`${history[index].ts}-${index}`}>{line}</li>
            ))}
          </ul>
        )}
      </section>

      <section className="space-y-1">
        <CopyButton disabled={diagnostics === null} label="Copy diagnostic report" size="sm" value={report} variant="outline" />
        <p className="text-xs text-content-subtle">App state and recent activity only — never your Battle Logs or clipboard.</p>
      </section>
    </div>
  );
}
