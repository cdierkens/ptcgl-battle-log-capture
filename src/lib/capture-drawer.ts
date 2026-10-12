import { isResultsState } from "./capture-status";

/**
 * Whether the drawer should be open after the overlay state changes: it opens
 * once on entering the results screen and closes as soon as the log is saved —
 * by hand or from the clipboard — or on leaving the results screen; otherwise the
 * player's choice stands. With `prompts` off it never auto-opens.
 *
 * The close belongs here, on the state transition, rather than in a predicate
 * over whether the drawer is currently open: a predicate also fires when the
 * player reopens the drawer on the same saved screen, which made the handle
 * unusable for the rest of the match.
 *
 * Note the `undo_last_save` Tauri command and the `SaveUndone` transition remain
 * registered in Rust but have no caller: the Undo button that used them was
 * removed, since a saved log can always be deleted from the matches list. Kept
 * rather than deleted so re-adding Undo is a UI-only change.
 */
export function nextDrawerOpen(
  previousState: null | string,
  nextState: string,
  open: boolean,
  prompts: boolean,
): boolean {
  if (nextState === previousState) return open;
  if (nextState === "Results") return prompts || open;
  if (nextState === "Saved") return false;
  if (isResultsState(previousState) && !isResultsState(nextState)) return false;
  return open;
}
