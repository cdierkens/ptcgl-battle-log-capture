import { describe, expect, it } from "vitest";

import type { Destination, SyncStatus } from "./tauri";

import { attentionReason, deliveriesFor, matchSyncState, syncActions, syncNeedsAttention } from "./sync-status";

function destination(id: string, name: string, enabled = true): Destination {
  return { enabled, headers: {}, id, name, token_url: null, url: `https://${id}.example/api` };
}

const dojo = destination("dojo", "PokeDojo");
const court = destination("court", "TrainingCourt");

function status(deliveries: [string, string, "failed" | "pending" | "synced"][], attention: string[] = []): SyncStatus {
  return {
    attention: attention.map((d) => ({ destination: d, reason: "The site rejected the token (HTTP 401)." })),
    deliveries: deliveries.map(([log, destination, state]) => ({ destination, lastError: null, log, state })),
  };
}

describe("matchSyncState", () => {
  it("is null for a match never queued", () => {
    expect(matchSyncState(status([]), "a.txt", [dojo])).toBeNull();
    expect(matchSyncState(null, "a.txt", [dojo])).toBeNull();
  });

  it("lets a failure win, then pending", () => {
    const s = status([
      ["a.txt", "dojo", "synced"],
      ["a.txt", "court", "failed"],
      ["b.txt", "dojo", "synced"],
      ["b.txt", "court", "pending"],
      ["c.txt", "dojo", "synced"],
    ]);
    expect(matchSyncState(s, "a.txt", [dojo, court])).toBe("failed");
    expect(matchSyncState(s, "b.txt", [dojo, court])).toBe("pending");
    expect(matchSyncState(s, "c.txt", [dojo, court])).toBe("synced");
  });

  it("ignores destinations no longer configured", () => {
    const s = status([
      ["a.txt", "dojo", "synced"],
      ["a.txt", "gone", "failed"],
    ]);
    expect(matchSyncState(s, "a.txt", [dojo])).toBe("synced");
  });
});

describe("syncActions", () => {
  it("offers each enabled destination, including for an older match", () => {
    expect(syncActions(status([]), "old.txt", [dojo, court, destination("off", "Off", false)])).toEqual([
      { destinationId: "dojo", label: "Sync to PokeDojo" },
      { destinationId: "court", label: "Sync to TrainingCourt" },
    ]);
  });

  it("reads Retry sync when that delivery failed", () => {
    const s = status([["a.txt", "dojo", "failed"]]);
    expect(syncActions(s, "a.txt", [dojo])).toEqual([{ destinationId: "dojo", label: "Retry sync" }]);
    expect(syncActions(s, "a.txt", [dojo, court])[0]?.label).toBe("Retry sync to PokeDojo");
  });
});

describe("attention", () => {
  it("reports why a destination is paused", () => {
    const s = status([], ["dojo"]);
    expect(attentionReason(s, "dojo")).toMatch(/rejected/);
    expect(attentionReason(s, "court")).toBeNull();
  });

  it("badges only for a configured destination", () => {
    expect(syncNeedsAttention(status([], ["dojo"]), [dojo])).toBe(true);
    expect(syncNeedsAttention(status([], ["gone"]), [dojo])).toBe(false);
    expect(syncNeedsAttention(null, [dojo])).toBe(false);
  });
});

describe("deliveriesFor", () => {
  it("keeps only this match's deliveries to configured destinations", () => {
    const s = status([
      ["a.txt", "dojo", "synced"],
      ["a.txt", "gone", "failed"],
      ["b.txt", "dojo", "pending"],
    ]);
    expect(deliveriesFor(s, "a.txt", [dojo]).map((d) => d.destination)).toEqual(["dojo"]);
  });
});
