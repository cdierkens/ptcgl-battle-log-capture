import { listen } from "@tauri-apps/api/event";
import { useCallback, useEffect, useMemo, useRef, useState } from "react";

import { logSavedState } from "./capture-status";
import { createAnalysisCache, type MatchAnalysis } from "./match-row";
import { listLogs, type LogSummary, OVERLAY_STATE_EVENT, type OverlayStateEvent, readLog } from "./tauri";

export interface Match {
  analysis: MatchAnalysis | null;
  log: LogSummary;
  raw: null | string;
}

export interface MatchesView {
  error: null | string;
  matches: Match[] | null;
  refresh: () => void;
  refreshing: boolean;
}

/** Saved logs with their parsed summaries. Each file is read once; rows re-derive when the name changes. */
export function useMatches(playerName: string): MatchesView {
  const [logs, setLogs] = useState<LogSummary[] | null>(null);
  const [raws, setRaws] = useState<ReadonlyMap<string, string>>(new Map());
  const [error, setError] = useState<null | string>(null);
  const [refreshing, setRefreshing] = useState(true);
  const requested = useRef(new Set<string>());
  // One cache per hook instance; repeat calls return the same analysis.
  const [analyze] = useState(createAnalysisCache);

  const load = useCallback(() => {
    listLogs()
      .then(setLogs)
      .catch((err: unknown) => setError(String(err)))
      .finally(() => setRefreshing(false));
  }, []);

  const refresh = useCallback(() => {
    setRefreshing(true);
    setError(null);
    load();
  }, [load]);

  useEffect(load, [load]);

  useEffect(() => {
    const unlisten = listen<OverlayStateEvent>(OVERLAY_STATE_EVENT, (event) => {
      if (logSavedState(event.payload.state) || event.payload.state === "Results") refresh();
    });
    unlisten.catch((err: unknown) => setError(String(err)));
    return () => {
      void unlisten.then((fn) => fn());
    };
  }, [refresh]);

  useEffect(() => {
    if (logs === null) return;
    for (const log of logs) {
      if (requested.current.has(log.fileName)) continue;
      requested.current.add(log.fileName);
      readLog(log.fileName)
        .then((raw) => setRaws((current) => new Map(current).set(log.fileName, raw)))
        .catch(() => requested.current.delete(log.fileName));
    }
  }, [logs]);

  const matches = useMemo(() => {
    if (logs === null) return null;
    return logs.map((log) => {
      const raw = raws.get(log.fileName) ?? null;
      return { analysis: raw === null ? null : analyze(log.fileName, raw, playerName), log, raw };
    });
  }, [analyze, logs, playerName, raws]);

  return { error, matches, refresh, refreshing };
}
