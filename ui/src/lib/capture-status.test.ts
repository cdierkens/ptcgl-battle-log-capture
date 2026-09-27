import { describe, expect, it } from "vitest";

import { labelForState } from "./capture-status";

describe("labelForState", () => {
  it("maps key engine states to friendly labels", () => {
    expect(labelForState("Watching").label).toBe("Watching for results");
    expect(labelForState("AwaitingNextMatch").label).toContain("Resting");
    expect(labelForState("PermissionRequired").tone).toBe("critical");
    expect(labelForState("Captured").tone).toBe("positive");
  });
  it("maps running/stopped pseudo-states", () => {
    // tone pins the Running entry (fallback tone is "secondary")
    expect(labelForState("Running").tone).toBe("positive");
    expect(labelForState("Stopped").label).toBe("Stopped");
    // Stopped's values equal the fallback, so pin the exact shape
    expect(labelForState("Stopped")).toEqual({ label: "Stopped", tone: "secondary" });
  });
  it("falls back to the raw state for unknown values", () => {
    expect(labelForState("SomethingNew").label).toBe("SomethingNew");
  });
  it("falls back for Object.prototype keys", () => {
    expect(labelForState("toString").label).toBe("toString");
    expect(labelForState("toString").tone).toBe("secondary");
  });
});
