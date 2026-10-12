import { describe, expect, it } from "vitest";

import { nextDrawerOpen } from "./capture-drawer";

describe("nextDrawerOpen", () => {
  it("opens on entering the results screen", () => {
    expect(nextDrawerOpen("InMatch", "Results", false, true)).toBe(true);
  });
  it("opens when the app starts on the results screen", () => {
    expect(nextDrawerOpen(null, "Results", false, true)).toBe(true);
  });
  it("stays closed if the player closed it on the same results screen", () => {
    expect(nextDrawerOpen("Results", "Results", false, true)).toBe(false);
  });
  it("closes after a save", () => {
    expect(nextDrawerOpen("Results", "Saved", true, true)).toBe(false);
  });
  it("closes on leaving the results screen", () => {
    expect(nextDrawerOpen("Dismissed", "InMatch", true, true)).toBe(false);
    expect(nextDrawerOpen("Results", "NoWindow", true, true)).toBe(false);
    expect(nextDrawerOpen("Dismissed", "NoWindow", true, true)).toBe(false);
    expect(nextDrawerOpen("Saved", "InMatch", true, true)).toBe(false);
  });
  it("keeps the player's choice otherwise", () => {
    expect(nextDrawerOpen("Results", "Dismissed", true, true)).toBe(true);
    expect(nextDrawerOpen("Saved", "InMatch", false, true)).toBe(false);
    expect(nextDrawerOpen("InMatch", "InMatch", true, true)).toBe(true);
  });
  it("does not auto-open when prompts are off", () => {
    expect(nextDrawerOpen("InMatch", "Results", false, false)).toBe(false);
    expect(nextDrawerOpen("InMatch", "Results", true, false)).toBe(true);
  });
  it("closes on save whether it came from the clipboard or a paste", () => {
    // A saved log can always be deleted from the matches list, so nothing needs
    // holding the drawer open, and the wait only kept it over the board.
    expect(nextDrawerOpen("Results", "Saved", true, true)).toBe(false);
  });
  it("can be reopened after a save", () => {
    // The close fires on the state transition, so a player who reopens the
    // drawer on the same saved screen keeps it open. A predicate over `open`
    // would slam it shut again immediately.
    expect(nextDrawerOpen("Saved", "Saved", false, true)).toBe(false);
    expect(nextDrawerOpen("Saved", "Saved", true, true)).toBe(true);
  });
});


