import { describe, expect, it } from "vitest";

import { chipForState, handleForState, historyText, isResultsState, logSavedState } from "./capture-status";

describe("chipForState", () => {
  it("maps overlay states to the header chip", () => {
    expect(chipForState("NoWindow")).toEqual({ label: "Waiting for the game", tone: "secondary" });
    expect(chipForState("InMatch")).toEqual({ label: "Watching", tone: "info" });
    expect(chipForState("Results")).toEqual({ label: "Ready to save", tone: "brand" });
    expect(chipForState("Dismissed")).toEqual({ label: "Ready to save", tone: "brand" });
    expect(chipForState("Saved")).toEqual({ label: "Saved", tone: "positive" });
    expect(chipForState("PermissionRequired")).toEqual({ label: "Permission needed", tone: "critical" });
  });
  it("falls back for unknown and prototype keys", () => {
    expect(chipForState("SomethingNew").label).toBe("SomethingNew");
    expect(chipForState("toString").tone).toBe("secondary");
  });
});

describe("handleForState", () => {
  it("is quiet mid-match", () => {
    expect(handleForState("InMatch")).toEqual({ label: "Battle Log", pulse: false, variant: "outline" });
  });
  it("asks to save on results, pulsing only when the shield is up", () => {
    expect(handleForState("Results")).toEqual({ label: "Save Battle Log", pulse: true, variant: "brand" });
    expect(handleForState("Dismissed")).toEqual({ label: "Save Battle Log", pulse: false, variant: "brand" });
  });
  it("confirms a save with text, not colour alone", () => {
    expect(handleForState("Saved")).toEqual({ label: "✓ Saved", pulse: false, variant: "positive" });
  });
});

describe("isResultsState", () => {
  it("covers every results-screen state", () => {
    expect(["Results", "Dismissed", "Saved"].every(isResultsState)).toBe(true);
    expect(isResultsState("InMatch")).toBe(false);
    expect(isResultsState(null)).toBe(false);
  });
});

describe("historyText", () => {
  it("appends the detail when present", () => {
    expect(historyText({ detail: "window unavailable", message: "Overlay window error" })).toBe(
      "Overlay window error — window unavailable",
    );
  });
  it("is just the message without a detail", () => {
    expect(historyText({ detail: null, message: "Battle Log saved" })).toBe("Battle Log saved");
  });
});

describe("logSavedState", () => {
  it("is true only for the saved overlay state", () => {
    expect(logSavedState("Saved")).toBe(true);
    expect(logSavedState("Results")).toBe(false);
  });
});
