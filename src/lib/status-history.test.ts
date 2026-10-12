import { describe, expect, it } from "vitest";

import { appendHistory, lastResultsAt, needsAttention } from "./status-history";

const event = (message: string, ts: number, state = "InMatch") => ({ detail: null, message, state, ts });

describe("appendHistory", () => {
  it("puts the newest first and drops an immediate repeat", () => {
    const one = appendHistory([], event("Results screen", 1, "Results"));
    const two = appendHistory(one, event("Results screen", 2, "Results"));
    expect(two).toHaveLength(1);
    const three = appendHistory(two, event("Battle Log saved", 3, "Saved"));
    expect(three.map((e) => e.message)).toEqual(["Battle Log saved", "Results screen"]);
  });

  it("caps the list", () => {
    let history = appendHistory([], event("a", 0));
    for (let i = 1; i < 80; i++) history = appendHistory(history, event(`m${i}`, i));
    expect(history).toHaveLength(50);
  });
});

describe("needsAttention", () => {
  it("flags an unresolved overlay error or clipboard pause", () => {
    expect(needsAttention([event("Overlay window error", 2)])).toBe(true);
    expect(needsAttention([event("Clipboard auto-save paused", 2)])).toBe(true);
  });

  it("clears once the newer event resolves it", () => {
    expect(needsAttention([event("Overlay windows recovered", 3), event("Overlay window error", 2)])).toBe(false);
    expect(needsAttention([event("Clipboard auto-save resumed", 3), event("Clipboard auto-save paused", 2)])).toBe(false);
    expect(needsAttention([event("Watching", 1)])).toBe(false);
  });
});

describe("lastResultsAt", () => {
  it("finds the newest results-screen event", () => {
    expect(lastResultsAt([event("x", 5), event("Results", 4, "Results"), event("Results", 1, "Results")])).toBe(4);
    expect(lastResultsAt([event("x", 5)])).toBeNull();
  });
});
