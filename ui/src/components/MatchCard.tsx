import { Button, Callout, Field, Textarea } from "@pokedojo/ui";
import { useEffect, useReducer, useRef, useState } from "react";

import type { OverlayStateView } from "../lib/use-overlay-state";

import { analyzeLog, fileNameOf, type MatchAnalysis } from "../lib/match-row";
import { actionForOutcome, initialPaste, isBlank, pasteReducer } from "../lib/paste-box";
import { readLog, submitBattleLog, undoLastSave } from "../lib/tauri";

const UNDO_MS = 8000;
const TEXTAREA_ID = "battle-log";

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
  const savedPath = (overlay.autosaved ? overlay.path : manualPath) ?? latestFileName;

  useEffect(() => {
    if (overlay.state === "Results") setManualPath(null);
  }, [overlay.state]);

  return (
    <section aria-label="This match" className="border-b border-border bg-surface-raised p-4">
      {overlay.state === "Saved" ? (
        <SavedLine autosaved={overlay.autosaved} path={savedPath} playerName={playerName} />
      ) : (
        <PasteForm clipboardOn={clipboardOn} focusPaste={focusPaste} onSaved={setManualPath} state={overlay.state} />
      )}
    </section>
  );
}

function PasteForm({
  clipboardOn,
  focusPaste,
  onSaved,
  state,
}: {
  clipboardOn: boolean;
  focusPaste: boolean;
  onSaved: (path: string) => void;
  state: null | string;
}) {
  const [model, dispatch] = useReducer(pasteReducer, initialPaste);
  const textarea = useRef<HTMLTextAreaElement>(null);
  // Classify outcomes against the state when the response arrives, not when Save was clicked.
  const stateRef = useRef(state);
  stateRef.current = state;

  useEffect(() => {
    if (focusPaste) textarea.current?.focus();
  }, [focusPaste]);

  const submit = () => {
    dispatch({ type: "submit" });
    if (isBlank(model.text)) return;
    submitBattleLog(model.text)
      .then((outcome) => {
        if (outcome.kind === "saved") onSaved(outcome.path);
        dispatch(actionForOutcome(outcome, stateRef.current));
      })
      .catch((err: unknown) => dispatch({ message: String(err), type: "failed" }));
  };

  return (
    <form
      className="flex flex-col gap-3"
      onSubmit={(event) => {
        event.preventDefault();
        submit();
      }}
    >
      <Field
        description={
          clipboardOn
            ? "We'll save it as soon as you Export. You can also paste it here (⌘V)."
            : "Then paste it here (⌘V) and press Save (⌘↵)."
        }
        error={model.error}
        htmlFor={TEXTAREA_ID}
        label="In the game: Battle Log → Export"
      >
        <Textarea
          className="h-24 font-mono text-xs"
          disabled={model.submitting}
          id={TEXTAREA_ID}
          onChange={(event) => dispatch({ text: event.target.value, type: "edit" })}
          onKeyDown={(event) => {
            if (event.key === "Enter" && (event.metaKey || event.ctrlKey)) {
              event.preventDefault();
              submit();
            }
          }}
          placeholder="Paste the exported Battle Log"
          ref={textarea}
          value={model.text}
        />
      </Field>
      <Button isLoading={model.submitting} size="sm" type="submit">
        Save Battle Log
      </Button>
    </form>
  );
}

function SavedLine({ autosaved, path, playerName }: { autosaved: boolean; path: null | string; playerName: string }) {
  const [analysis, setAnalysis] = useState<MatchAnalysis | null>(null);
  const [undoOpen, setUndoOpen] = useState(autosaved && path !== null);
  const [undoing, setUndoing] = useState(false);
  const [error, setError] = useState<null | string>(null);

  useEffect(() => {
    if (path === null) return;
    let cancelled = false;
    readLog(fileNameOf(path))
      .then((raw) => {
        if (!cancelled) setAnalysis(analyzeLog(raw, playerName));
      })
      .catch(() => undefined);
    return () => {
      cancelled = true;
    };
  }, [path, playerName]);

  useEffect(() => {
    if (!undoOpen) return;
    const timer = window.setTimeout(() => setUndoOpen(false), UNDO_MS);
    return () => window.clearTimeout(timer);
  }, [undoOpen]);

  const undo = () => {
    if (path === null) return;
    setUndoing(true);
    setError(null);
    undoLastSave(path)
      .catch((err: unknown) => setError(String(err)))
      .finally(() => setUndoing(false));
  };

  const parts = ["✓ Battle Log saved"];
  if (analysis !== null) {
    parts.push(`vs ${analysis.opponentName ?? "unknown opponent"}`);
    if (analysis.summary.winner !== null) parts.push(analysis.summary.winner === "self" ? "Win" : "Loss");
  }

  return (
    <div className="space-y-2">
      <div className="flex items-center justify-between gap-2">
        <p className="text-sm font-semibold text-content-primary" role="status">
          {parts.join(" · ")}
        </p>
        {undoOpen ? (
          <Button isLoading={undoing} onClick={undo} size="sm" variant="outline">
            Undo
          </Button>
        ) : null}
      </div>
      {autosaved ? <p className="text-xs text-content-subtle">Saved from the clipboard.</p> : null}
      {error !== null ? <Callout variant="critical">{error}</Callout> : null}
    </div>
  );
}
