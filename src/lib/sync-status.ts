import type { DeliveryStatus, Destination, SyncState, SyncStatus } from "./tauri";

export interface MatchSync {
  destination: Destination;
  lastError: null | string;
  state: MatchSyncState;
}

/** `unsynced`: the match was never queued for that destination. */
export type MatchSyncState = "unsynced" | SyncState;

export interface SyncAction {
  destinationId: string;
  label: string;
}

/** Why a destination is paused, or `null` when it isn't. */
export function attentionReason(status: null | SyncStatus, destinationId: string): null | string {
  return status?.attention.find((a) => a.destination === destinationId)?.reason ?? null;
}

/** A match's sync state for every configured destination, in Settings order. */
export function matchSyncs(status: null | SyncStatus, fileName: string, destinations: Destination[]): MatchSync[] {
  const deliveries = new Map<string, DeliveryStatus>();
  for (const d of status?.deliveries ?? []) {
    if (d.log === fileName) deliveries.set(d.destination, d);
  }
  return destinations.map((destination) => {
    const delivery = deliveries.get(destination.id);
    return delivery === undefined
      ? { destination, lastError: null, state: "unsynced" }
      : { destination, lastError: delivery.lastError, state: delivery.state };
  });
}

/** The `⋯` menu items: one per enabled destination, reading Retry when that delivery failed. */
export function syncActions(status: null | SyncStatus, fileName: string, destinations: Destination[]): SyncAction[] {
  const enabled = destinations.filter((d) => d.enabled);
  return enabled.map((destination) => {
    const failed = (status?.deliveries ?? []).some(
      (d) => d.log === fileName && d.destination === destination.id && d.state === "failed",
    );
    const retry = enabled.length === 1 ? "Retry sync" : `Retry sync to ${destination.name}`;
    return { destinationId: destination.id, label: failed ? retry : `Sync to ${destination.name}` };
  });
}

/** Whether any configured destination needs attention; drives the attention badges. */
export function syncNeedsAttention(status: null | SyncStatus, destinations: Destination[]): boolean {
  return destinations.some((d) => attentionReason(status, d.id) !== null);
}
