import { analyzeBattleLog, type GameSummary, isOk } from "@dierkens.dev/ptcgl-battle-log-parser";

import type { LogSummary } from "./tauri";

export interface MatchAnalysis {
  opponentName: null | string;
  summary: GameSummary;
}

export interface MatchRow {
  fileName: string;
  parsed: boolean;
  prizes: null | string;
  result: "—" | "L" | "W" | null;
  title: string;
  when: string;
}

const MINUTE = 60_000;
const HOUR = 60 * MINUTE;
const DAY = 24 * HOUR;

/** Parses a saved log for its row; `null` when it can't be analyzed or the player name is unset. */
export function analyzeLog(raw: string, playerName: string): MatchAnalysis | null {
  if (playerName.trim() === "") return null;
  const result = analyzeBattleLog(raw, { playerName: playerName.trim() });
  return isOk(result) ? { opponentName: result.value.opponentName, summary: result.value.summary } : null;
}

export function fileNameOf(path: string): string {
  return path.split(/[/\\]/).at(-1) ?? path;
}

export function matchRow(log: LogSummary, analysis: MatchAnalysis | null, nowMs: number): MatchRow {
  const when = relativeTime(log.createdMs, nowMs);
  if (analysis === null) {
    return {
      fileName: log.fileName,
      parsed: false,
      prizes: null,
      result: null,
      title: `${new Date(log.createdMs).toLocaleDateString()} · Battle Log`,
      when,
    };
  }
  const { opponentName, summary } = analysis;
  return {
    fileName: log.fileName,
    parsed: true,
    prizes: `${summary.prizesByPlayer.self}–${summary.prizesByPlayer.opponent}`,
    result: summary.winner === "self" ? "W" : summary.winner === "opponent" ? "L" : "—",
    title: `vs ${opponentName ?? "unknown opponent"}`,
    when,
  };
}

export function relativeTime(thenMs: number, nowMs: number): string {
  const elapsed = Math.max(0, nowMs - thenMs);
  if (elapsed < MINUTE) return "just now";
  if (elapsed < HOUR) return `${Math.floor(elapsed / MINUTE)}m ago`;
  if (elapsed < DAY) return `${Math.floor(elapsed / HOUR)}h ago`;
  return `${Math.floor(elapsed / DAY)}d ago`;
}
