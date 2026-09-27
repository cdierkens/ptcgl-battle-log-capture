export interface StateLabel {
  label: string;
  tone: Tone;
}

export type Tone = "brand" | "caution" | "critical" | "info" | "outline" | "positive" | "secondary";

const MAP: Record<string, StateLabel> = {
  AwaitingNextMatch: { label: "Resting — waiting for next match", tone: "secondary" },
  Captured: { label: "Captured", tone: "positive" },
  CloseDetected: { label: "Closing drawer", tone: "info" },
  ConfirmDetected: { label: "Confirming", tone: "info" },
  ConfirmPressed: { label: "Confirmed", tone: "info" },
  ExportDetected: { label: "Exporting", tone: "info" },
  ExportPressed: { label: "Export clicked", tone: "info" },
  Idle: { label: "Idle", tone: "secondary" },
  LogTabDetected: { label: "Results screen detected", tone: "info" },
  PanelOpened: { label: "Opening battle log", tone: "info" },
  PermissionRequired: { label: "Screen recording permission needed", tone: "critical" },
  Running: { label: "Running", tone: "positive" },
  Stopped: { label: "Stopped", tone: "secondary" },
  Watching: { label: "Watching for results", tone: "info" },
};

export function labelForState(state: string): StateLabel {
  return Object.hasOwn(MAP, state) ? MAP[state] : { label: state, tone: "secondary" };
}
