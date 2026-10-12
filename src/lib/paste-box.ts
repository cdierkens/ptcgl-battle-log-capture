import type { SubmitOutcome } from "./tauri";

import { isResultsState } from "./capture-status";

export type PasteAction =
  | { message: string; type: "failed" }
  | { text: string; type: "edit" }
  | { type: "saved" }
  | { type: "submit" };

export interface PasteModel {
  error: null | string;
  submitting: boolean;
  text: string;
}

export const ALREADY_SAVED_MESSAGE = "That Battle Log was already saved. Export this match's Battle Log and paste it.";
export const ALREADY_SAVED_ANYTIME_MESSAGE = "That Battle Log is already saved.";

export const initialPaste: PasteModel = { error: null, submitting: false, text: "" };

/** On the results screen a re-paste only counts once this match's log is saved; elsewhere it's just a duplicate. */
export function actionForOutcome(outcome: SubmitOutcome, overlayState: null | string): PasteAction {
  if (outcome.kind !== "alreadySaved" || overlayState === "Saved") return { type: "saved" };
  const message = isResultsState(overlayState) ? ALREADY_SAVED_MESSAGE : ALREADY_SAVED_ANYTIME_MESSAGE;
  return { message, type: "failed" };
}

export function isBlank(text: string): boolean {
  return text.trim() === "";
}

export function pasteReducer(model: PasteModel, action: PasteAction): PasteModel {
  switch (action.type) {
    case "edit":
      return { ...model, error: null, text: action.text };
    case "failed":
      return { ...model, error: action.message, submitting: false };
    case "saved":
      return initialPaste;
    case "submit":
      return isBlank(model.text)
        ? { ...model, error: "Paste your Battle Log first." }
        : { ...model, error: null, submitting: true };
  }
}
