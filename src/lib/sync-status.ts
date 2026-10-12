import type { DeliveryStatus, Destination, SyncState, SyncStatus } from "./tauri";

export interface SyncAction {
  destinationId: string;
  label: string;
}

/** Why a destination is paused, or `null` when it isn't. */
export function attentionReason(status: null | SyncStatus, destinationId: string): null | string {
  return status?.attention.find((a) => a.destination === destinationId)?.reason ?? null;
}

/** A match's deliveries to destinations still configured, in destination order. */
export function deliveriesFor(status: null | SyncStatus, fileName: string, destinations: Destination[]): DeliveryStatus[] {
  const ids = new Set(destinations.map((d) => d.id));
  return (status?.deliveries ?? []).filter((d) => d.log === fileName && ids.has(d.destination));
}

/**
 * One state for a match across its destinations: any failure wins, then anything still
 * pending. `null` when it was never queued for a destination still configured.
 */
export function matchSyncState(status: null | SyncStatus, fileName: string, destinations: Destination[]): null | SyncState {
  const states = new Set(deliveriesFor(status, fileName, destinations).map((d) => d.state));
  if (states.has("failed")) return "failed";
  if (states.has("pending")) return "pending";
  return states.has("synced") ? "synced" : null;
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
