import { Button, Callout, Field, Spinner, Textarea } from "@pokedojo/ui";
import { useEffect, useReducer, useRef, useState } from "react";

import type { OverlayStateView } from "../lib/use-overlay-state";

import { analyzeLog, fileNameOf, type MatchAnalysis } from "../lib/match-row";
import { actionForOutcome, initialPaste, isBlank, pasteReducer } from "../lib/paste-box";
import { readLog, submitBattleLog } from "../lib/tauri";

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
      {clipboardOn ? <SavingLine /> : null}
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
        <p className="text-sm font-semibold text-content-primary" role="status">
          {parts.join(" · ")}
        </p>
      </div>
      {autosaved ? <p className="text-xs text-content-subtle">Saved from the clipboard.</p> : null}
      {error !== null ? <Callout variant="critical">{error}</Callout> : null}
    </div>
  );
}

/**
 * The clipboard watcher is working. Shown beside the paste form rather than in
 * place of it: the watcher legitimately doesn't fire when the log was already on
 * the clipboard, when the copy isn't recognised as a battle log, or when the game
 * wasn't focused, so the manual path has to stay reachable. The drawer closes the
 * moment the log lands, so this is the whole of the waiting.
 */
function SavingLine() {
  return (
    <p className="flex items-center gap-2 text-xs text-content-subtle" role="status">
      <Spinner label="" size="sm" />
      Saving your Battle Log as soon as you Export…
    </p>
  );
}
