import { isResultsState } from "./capture-status";

/**
 * Whether the drawer should be open after the overlay state changes: it opens
 * once on entering the results screen and closes after a save or on leaving the
 * results screen; otherwise the player's choice stands. With `prompts` off it never
 * auto-opens. After a clipboard auto-save it stays as it was so Undo is reachable.
 */
export function nextDrawerOpen(
  previousState: null | string,
  nextState: string,
  open: boolean,
  prompts: boolean,
  autosaved = false,
): boolean {
  if (nextState === previousState) return open;
  if (nextState === "Results") return prompts || open;
  if (nextState === "Saved") return autosaved && open;
  if (isResultsState(previousState) && !isResultsState(nextState)) return false;
  return open;
}
