import type { CaptureStatusEvent } from "./tauri";

const MAX_HISTORY = 50;

/** Problem message → the message that resolves it. */
const RESOLVED_BY: Record<string, string> = {
  "Clipboard auto-save paused": "Clipboard auto-save resumed",
  "Overlay window error": "Overlay windows recovered",
};

/** Newest first; an immediate repeat of the same message and state is dropped. */
export function appendHistory(history: CaptureStatusEvent[], event: CaptureStatusEvent): CaptureStatusEvent[] {
  const latest = history.at(0);
  if (latest?.message === event.message && latest.state === event.state && latest.detail === event.detail) {
    return history;
  }
  return [event, ...history].slice(0, MAX_HISTORY);
}

/** When the results screen was last seen, from the history. */
export function lastResultsAt(history: CaptureStatusEvent[]): null | number {
  return history.find((event) => event.state === "Results")?.ts ?? null;
}

/** True while the newest problem event hasn't been followed by its resolution. */
export function needsAttention(history: CaptureStatusEvent[]): boolean {
  const resolutions = new Set(Object.values(RESOLVED_BY));
  const latest = history.find((event) => Object.hasOwn(RESOLVED_BY, event.message) || resolutions.has(event.message));
  return latest !== undefined && Object.hasOwn(RESOLVED_BY, latest.message);
}
