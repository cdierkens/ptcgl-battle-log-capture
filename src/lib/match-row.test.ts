import type { GameSummary } from "@dierkens.dev/ptcgl-battle-log-parser";

import { describe, expect, it } from "vitest";

import { analyzeLog, createAnalysisCache, fileNameOf, matchRow, relativeTime } from "./match-row";

const NOW = Date.UTC(2026, 9, 7, 12, 0, 0);
const LOG = { createdMs: NOW - 12 * 60_000, fileName: "ptcgl-1.txt", sizeBytes: 100 };

function summary(overrides: Partial<GameSummary> = {}): GameSummary {
  return {
    firstKnockoutBy: null,
    knockoutsByPlayer: { opponent: 2, self: 4 },
    prizesByPlayer: { opponent: 3, self: 6 },
    totalEntries: 10,
    turnCount: 9,
    winner: "self",
    ...overrides,
  };
}

describe("matchRow", () => {
  it("shows the result, opponent, prizes, and time for a parsed log", () => {
    const row = matchRow(LOG, { opponentName: "Ash", summary: summary() }, NOW);
    expect(row).toMatchObject({ parsed: true, prizes: "6–3", result: "W", title: "vs Ash", when: "12m ago" });
  });

  it("marks losses and unfinished games", () => {
    expect(matchRow(LOG, { opponentName: "Ash", summary: summary({ winner: "opponent" }) }, NOW).result).toBe("L");
    expect(matchRow(LOG, { opponentName: "Ash", summary: summary({ winner: null }) }, NOW).result).toBe("—");
  });

  it("falls back when the opponent is unknown", () => {
    expect(matchRow(LOG, { opponentName: null, summary: summary() }, NOW).title).toBe("vs unknown opponent");
  });

  it("falls back to the date when the log can't be analyzed", () => {
    const row = matchRow(LOG, null, NOW);
    expect(row).toMatchObject({ parsed: false, prizes: null, result: null });
    expect(row.title).toMatch(/· Battle Log$/);
  });
});

describe("relativeTime", () => {
  it("buckets by minute, hour, and day", () => {
    expect(relativeTime(NOW - 10_000, NOW)).toBe("just now");
    expect(relativeTime(NOW - 5 * 60_000, NOW)).toBe("5m ago");
    expect(relativeTime(NOW - 3 * 3_600_000, NOW)).toBe("3h ago");
    expect(relativeTime(NOW - 2 * 86_400_000, NOW)).toBe("2d ago");
  });
});

describe("fileNameOf", () => {
  it("takes the last path segment", () => {
    expect(fileNameOf("/Users/me/.ptcgl-battle-logs/ptcgl-1.txt")).toBe("ptcgl-1.txt");
    expect(fileNameOf("C:\\logs\\ptcgl-1.txt")).toBe("ptcgl-1.txt");
  });
});

describe("analyzeLog", () => {
  it("analyzes a battle log from the player's side", () => {
    const raw = "Setup\nAlice drew 7 cards for the opening hand.\n\nAlice's Turn\nAlice ended their turn.";
    expect(analyzeLog(raw, " Alice ")?.summary.turnCount).toBe(1);
  });

  it("returns null without a player name", () => {
    expect(analyzeLog("Setup\n- x", "  ")).toBeNull();
  });

  it("returns null for text that isn't a battle log", () => {
    expect(analyzeLog("hello world", "Ash")).toBeNull();
  });
});

describe("createAnalysisCache", () => {
  const raw = "Setup\nAlice drew 7 cards for the opening hand.\n\nAlice's Turn\nAlice ended their turn.";

  it("parses each saved log once per player name", () => {
    const analyze = createAnalysisCache();
    const first = analyze("ptcgl-1.txt", raw, "Alice");
    expect(first?.summary.turnCount).toBe(1);
    expect(analyze("ptcgl-1.txt", raw, "Alice")).toBe(first);
  });

  it("re-parses when the player name changes", () => {
    const analyze = createAnalysisCache();
    expect(analyze("ptcgl-1.txt", raw, "Alice")).not.toBeNull();
    expect(analyze("ptcgl-1.txt", raw, "")).toBeNull();
  });
});
