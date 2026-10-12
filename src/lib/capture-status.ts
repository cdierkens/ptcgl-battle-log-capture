/** shadcn badge variants; status tones collapse onto these (ADR 0003). */
export type BadgeVariant = "default" | "destructive" | "outline" | "secondary";

export interface HandleLook {
  label: string;
  pulse: boolean;
  variant: "default" | "outline";
}

export interface StateLabel {
  label: string;
  variant: BadgeVariant;
}

const CHIPS: Record<string, StateLabel> = {
  Dismissed: { label: "Ready to save", variant: "default" },
  InMatch: { label: "Watching", variant: "secondary" },
  NoWindow: { label: "Waiting for the game", variant: "secondary" },
  PermissionRequired: { label: "Permission needed", variant: "destructive" },
  Results: { label: "Ready to save", variant: "default" },
  Saved: { label: "Saved", variant: "default" },
  WarmingUp: { label: "Starting up", variant: "secondary" },
};

const HANDLES: Record<string, HandleLook> = {
  Dismissed: { label: "Save Battle Log", pulse: false, variant: "default" },
  Results: { label: "Save Battle Log", pulse: true, variant: "default" },
  Saved: { label: "✓ Saved", pulse: false, variant: "default" },
  WarmingUp: { label: "Starting up…", pulse: false, variant: "outline" },
};

const QUIET_HANDLE: HandleLook = { label: "Battle Log", pulse: false, variant: "outline" };

const RESULTS_STATES = new Set(["Dismissed", "Results", "Saved"]);

/** The header status chip for an overlay state. */
export function chipForState(state: string): StateLabel {
  return Object.hasOwn(CHIPS, state) ? CHIPS[state] : { label: state, variant: "secondary" };
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
