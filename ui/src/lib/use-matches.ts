import { listen } from "@tauri-apps/api/event";
import { useCallback, useEffect, useMemo, useRef, useState } from "react";

import { logSavedState } from "./capture-status";
import { analyzeLog, type MatchAnalysis } from "./match-row";
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
  const [refreshing, setRefreshing] = useState(false);
  const requested = useRef(new Set<string>());

  const refresh = useCallback(() => {
    setRefreshing(true);
    setError(null);
    listLogs()
      .then(setLogs)
      .catch((err: unknown) => setError(String(err)))
      .finally(() => setRefreshing(false));
  }, []);

  useEffect(refresh, [refresh]);

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

  // Keyed by player name + file so each log is parsed once per name.
  const analyses = useRef(new Map<string, MatchAnalysis | null>());
  const matches = useMemo(() => {
    if (logs === null) return null;
    return logs.map((log) => {
      const raw = raws.get(log.fileName) ?? null;
      if (raw === null) return { analysis: null, log, raw };
      const key = `${playerName}\u0000${log.fileName}`;
      if (!analyses.current.has(key)) analyses.current.set(key, analyzeLog(raw, playerName));
      return { analysis: analyses.current.get(key) ?? null, log, raw };
    });
  }, [logs, playerName, raws]);

  return { error, matches, refresh, refreshing };
}
