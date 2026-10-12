import { describe, expect, it } from "vitest";

import { chipForState, handleForState, historyText, isResultsState, logSavedState, matchNeedsPaste } from "./capture-status";

describe("chipForState", () => {
  it("maps overlay states to the header chip", () => {
    expect(chipForState("NoWindow")).toEqual({ label: "Waiting for the game", variant: "secondary" });
    expect(chipForState("WarmingUp")).toEqual({ label: "Starting up", variant: "secondary" });
    expect(chipForState("InMatch")).toEqual({ label: "Watching", variant: "secondary" });
    expect(chipForState("Results")).toEqual({ label: "Ready to save", variant: "default" });
    expect(chipForState("Dismissed")).toEqual({ label: "Ready to save", variant: "default" });
    expect(chipForState("Saved")).toEqual({ label: "Saved", variant: "default" });
    expect(chipForState("PermissionRequired")).toEqual({ label: "Permission needed", variant: "destructive" });
  });
  it("falls back for unknown and prototype keys", () => {
    expect(chipForState("SomethingNew").label).toBe("SomethingNew");
    expect(chipForState("toString").variant).toBe("secondary");
  });
});

describe("handleForState", () => {
  it("is quiet mid-match", () => {
    expect(handleForState("InMatch")).toEqual({ label: "Battle Log", pulse: false, variant: "outline" });
  });
  it("says it is starting up during warm-up", () => {
    expect(handleForState("WarmingUp")).toEqual({ label: "Starting up…", pulse: false, variant: "outline" });
  });
  it("asks to save on results, pulsing only when the shield is up", () => {
    expect(handleForState("Results")).toEqual({ label: "Save Battle Log", pulse: true, variant: "default" });
    expect(handleForState("Dismissed")).toEqual({ label: "Save Battle Log", pulse: false, variant: "default" });
  });
  it("confirms a save with text, not colour alone", () => {
    expect(handleForState("Saved")).toEqual({ label: "✓ Saved", pulse: false, variant: "default" });
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

describe("matchNeedsPaste", () => {
  it("is true only while this match's Battle Log is unsaved", () => {
    expect(["Results", "Dismissed"].every(matchNeedsPaste)).toBe(true);
    expect(["Saved", "InMatch", null].some(matchNeedsPaste)).toBe(false);
  });
});
