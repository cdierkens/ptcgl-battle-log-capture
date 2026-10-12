import { useEffect, useReducer, useRef } from "react";

import { LoadingButton } from "@/components/LoadingButton";
import { Field, FieldDescription, FieldError, FieldLabel } from "@/components/ui/field";
import { Spinner } from "@/components/ui/spinner";
import { Textarea } from "@/components/ui/textarea";

import { actionForOutcome, initialPaste, isBlank, pasteReducer } from "../lib/paste-box";
import { submitBattleLog } from "../lib/tauri";

const TEXTAREA_ID = "battle-log";

/** Save a Battle Log by pasting it: this match's (`matchState` is its overlay state), or any log (`null`). */
export function PasteForm({
  clipboardOn = false,
  focusPaste = false,
  matchState,
  onSaved,
}: {
  clipboardOn?: boolean;
  focusPaste?: boolean;
  matchState: null | string;
  onSaved: (path: string) => void;
}) {
  const [model, dispatch] = useReducer(pasteReducer, initialPaste);
  const textarea = useRef<HTMLTextAreaElement>(null);
  // Classify outcomes against the state when the response arrives, not when Save was clicked.
  const stateRef = useRef(matchState);
  useEffect(() => {
    stateRef.current = matchState;
  }, [matchState]);

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
      <Field data-invalid={model.error !== null || undefined}>
        <FieldLabel htmlFor={TEXTAREA_ID}>
          {matchState === null ? "Save a Battle Log" : "In the game: Battle Log → Export"}
        </FieldLabel>
        <Textarea
          aria-invalid={model.error !== null}
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
        <FieldDescription>
          {clipboardOn
            ? "We'll save it as soon as you Export. You can also paste it here (⌘V)."
            : "Then paste it here (⌘V) and press Save (⌘↵)."}
        </FieldDescription>
        <FieldError>{model.error}</FieldError>
      </Field>
      <LoadingButton loading={model.submitting} size="sm" type="submit">
        Save Battle Log
      </LoadingButton>
    </form>
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
    <p className="flex items-center gap-2 text-xs text-muted-foreground" role="status">
      <Spinner aria-hidden className="size-3" role={undefined} />
      Saving your Battle Log as soon as you Export…
    </p>
  );
}
