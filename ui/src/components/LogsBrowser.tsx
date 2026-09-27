import { Button, Callout, CopyButton, EmptyState, Panel, Skeleton } from "@pokedojo/ui";
import { useEffect, useRef, useState } from "react";

import { listLogs, type LogSummary, readLog } from "../lib/tauri";

export function LogsBrowser() {
  const [logs, setLogs] = useState<LogSummary[] | null>(null);
  const [error, setError] = useState<null | string>(null);
  const [isRefreshing, setIsRefreshing] = useState(false);
  const [selected, setSelected] = useState<LogSummary | null>(null);
  const [content, setContent] = useState<null | string>(null);
  const [loadingLog, setLoadingLog] = useState(false);

  const reqId = useRef(0);

  const refresh = () => {
    setError(null);
    setIsRefreshing(true);
    void listLogs()
      .then(setLogs)
      .catch((err: unknown) => setError(String(err)))
      .finally(() => setIsRefreshing(false));
  };

  useEffect(refresh, []);

  const open = (log: LogSummary) => {
    const id = ++reqId.current;
    setSelected(log);
    setContent(null);
    setLoadingLog(true);
    setError(null);
    void readLog(log.fileName)
      .then((text) => {
        if (id !== reqId.current) return;
        setContent(text);
      })
      .catch((err: unknown) => {
        if (id !== reqId.current) return;
        setError(String(err));
      })
      .finally(() => {
        if (id !== reqId.current) return;
        setLoadingLog(false);
      });
  };

  return (
    <Panel className="space-y-3 p-4">
      <div className="flex items-center justify-between">
        <h2 className="text-sm font-semibold text-content-secondary">Captured logs</h2>
        <Button isLoading={isRefreshing} onClick={refresh} size="sm" variant="outline">
          Refresh
        </Button>
      </div>

      {error !== null ? <Callout variant="critical">{error}</Callout> : null}

      {logs === null ? (
        <div className="space-y-2">
          <Skeleton className="h-6 w-full" />
          <Skeleton className="h-6 w-full" />
        </div>
      ) : logs.length === 0 ? (
        <EmptyState
          description="Play a match — the log is saved automatically when a match ends."
          title="No captured logs yet"
        />
      ) : (
        <>
          <p className="text-xs text-content-subtle">
            {logs.length} captured {logs.length === 1 ? "log" : "logs"}
          </p>
          <ul className="divide-y divide-border rounded border border-border">
            {logs.map((log) => (
              <li className="flex items-center justify-between gap-2 p-2" key={log.fileName}>
                <Button
                  onClick={() => open(log)}
                  size="sm"
                  variant={selected?.fileName === log.fileName ? "brand" : "ghost"}
                >
                  {new Date(log.createdMs).toLocaleString()} · {formatSize(log.sizeBytes)}
                </Button>
                <CopyButton label="Copy name" size="sm" value={log.fileName} />
              </li>
            ))}
          </ul>
        </>
      )}

      {selected !== null ? (
        <div className="rounded border border-border">
          <div className="border-b border-border p-2 text-xs text-content-subtle">
            {selected.fileName}
          </div>
          {loadingLog ? (
            <div className="space-y-2 p-2">
              <Skeleton className="h-4 w-full" />
              <Skeleton className="h-4 w-3/4" />
            </div>
          ) : (
            <pre className="max-h-72 overflow-y-auto whitespace-pre-wrap p-2 font-mono text-xs">
              {content}
            </pre>
          )}
        </div>
      ) : null}
    </Panel>
  );
}

function formatSize(bytes: number): string {
  if (bytes < 1024) return `${bytes} B`;
  return `${(bytes / 1024).toFixed(1)} KB`;
}