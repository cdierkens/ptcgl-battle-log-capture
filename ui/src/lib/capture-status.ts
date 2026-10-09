export interface HandleLook {
  label: string;
  pulse: boolean;
  variant: "brand" | "outline" | "positive";
}

export interface StateLabel {
  label: string;
  tone: Tone;
}

export type Tone = "brand" | "caution" | "critical" | "info" | "outline" | "positive" | "secondary";

const CHIPS: Record<string, StateLabel> = {
  Dismissed: { label: "Ready to save", tone: "brand" },
  InMatch: { label: "Watching", tone: "info" },
  NoWindow: { label: "Waiting for the game", tone: "secondary" },
  PermissionRequired: { label: "Permission needed", tone: "critical" },
  Results: { label: "Ready to save", tone: "brand" },
  Saved: { label: "Saved", tone: "positive" },
};

const HANDLES: Record<string, HandleLook> = {
  Dismissed: { label: "Save Battle Log", pulse: false, variant: "brand" },
  Results: { label: "Save Battle Log", pulse: true, variant: "brand" },
  Saved: { label: "✓ Saved", pulse: false, variant: "positive" },
};

const QUIET_HANDLE: HandleLook = { label: "Battle Log", pulse: false, variant: "outline" };

const RESULTS_STATES = new Set(["Dismissed", "Results", "Saved"]);

/** The header status chip for an overlay state. */
export function chipForState(state: string): StateLabel {
  return Object.hasOwn(CHIPS, state) ? CHIPS[state] : { label: state, tone: "secondary" };
}

/** How the drawer handle looks in an overlay state. */
export function handleForState(state: null | string): HandleLook {
  return state !== null && Object.hasOwn(HANDLES, state) ? HANDLES[state] : QUIET_HANDLE;
}

/** One status-history line: the message, plus its detail (e.g. an overlay error reason). */
export function historyText(event: { detail: null | string; message: string }): string {
  return event.detail === null ? event.message : `${event.message} — ${event.detail}`;
}

export function isResultsState(state: null | string): boolean {
  return state !== null && RESULTS_STATES.has(state);
}

/** The overlay state a successful save produces. */
export function logSavedState(state: null | string): boolean {
  return state === "Saved";
}
