import { useEffect, useState } from "react";

import { StatusAlert } from "@/components/StatusAlert";

import type { OverlayStateView } from "../lib/use-overlay-state";

import { matchNeedsPaste } from "../lib/capture-status";
import { analyzeLog, fileNameOf, type MatchAnalysis } from "../lib/match-row";
import { readLog } from "../lib/tauri";
import { PasteForm } from "./PasteForm";

interface MatchCardProps {
  clipboardOn: boolean;
  focusPaste: boolean;
  /** Newest saved log; names the match when the save path wasn't seen by this window. */
  latestFileName: null | string;
  overlay: OverlayStateView;
  playerName: string;
}

/** The results-screen section above the tabs: save this match's Battle Log, or confirm it's saved. */
export function MatchCard({ clipboardOn, focusPaste, latestFileName, overlay, playerName }: MatchCardProps) {
  const [manualPath, setManualPath] = useState<null | string>(null);
  const [seenState, setSeenState] = useState(overlay.state);
  // A new results screen forgets the previous match's manual save.
  if (overlay.state !== seenState) {
    setSeenState(overlay.state);
    if (overlay.state === "Results") setManualPath(null);
  }
  const savedPath = (overlay.autosaved ? overlay.path : manualPath) ?? latestFileName;

  return (
    <section aria-label="This match" className="border-b border-border bg-muted p-4">
      {matchNeedsPaste(overlay.state) ? (
        <PasteForm clipboardOn={clipboardOn} focusPaste={focusPaste} matchState={overlay.state} onSaved={setManualPath} />
      ) : (
        <SavedLine autosaved={overlay.autosaved} path={savedPath} playerName={playerName} />
      )}
    </section>
  );
}

function SavedLine({ autosaved, path, playerName }: { autosaved: boolean; path: null | string; playerName: string }) {
  const [analysis, setAnalysis] = useState<MatchAnalysis | null>(null);
  const [error, setError] = useState<null | string>(null);

  useEffect(() => {
    if (path === null) return;
    let cancelled = false;
    readLog(fileNameOf(path))
      .then((raw) => {
        if (!cancelled) setAnalysis(analyzeLog(raw, playerName));
      })
      .catch((err: unknown) => {
        if (!cancelled) setError(String(err));
      });
    return () => {
      cancelled = true;
    };
  }, [path, playerName]);

  const parts = ["✓ Battle Log saved"];
  if (analysis !== null) {
    parts.push(`vs ${analysis.opponentName ?? "unknown opponent"}`);
    if (analysis.summary.winner !== null) parts.push(analysis.summary.winner === "self" ? "Win" : "Loss");
  }

  return (
    <div className="space-y-2">
      <div className="flex items-center justify-between gap-2">
        <p className="text-sm font-semibold text-foreground" role="status">
          {parts.join(" · ")}
        </p>
      </div>
      {autosaved ? <p className="text-xs text-muted-foreground">Saved from the clipboard.</p> : null}
      {error !== null ? <StatusAlert tone="critical">{error}</StatusAlert> : null}
    </div>
  );
}
