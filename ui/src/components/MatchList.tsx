import { Badge, Button, Callout, ConfirmDialog, EmptyState, Menu, MenuItem, Skeleton } from "@pokedojo/ui";
import { useState } from "react";

import type { Match, MatchesView } from "../lib/use-matches";

import { matchRow, type MatchRow } from "../lib/match-row";
import { deleteLog, revealLog } from "../lib/tauri";

const RESULT_TONE = { "—": "secondary", L: "critical", W: "positive" } as const;

export function MatchList({ matches, playerName }: { matches: MatchesView; playerName: string }) {
  const [selected, setSelected] = useState<null | string>(null);
  const [pendingDelete, setPendingDelete] = useState<Match | null>(null);
  const [actionError, setActionError] = useState<null | string>(null);
  const now = Date.now();

  if (matches.matches === null) {
    return matches.error === null ? (
      <div className="space-y-2 p-4">
        <Skeleton className="h-10 w-full" />
        <Skeleton className="h-10 w-full" />
        <Skeleton className="h-10 w-full" />
      </div>
    ) : (
      <Callout
        actions={
          <Button isLoading={matches.refreshing} onClick={matches.refresh} size="sm" variant="outline">
            Try again
          </Button>
        }
        className="m-4"
        variant="critical"
      >
        Couldn't load saved Battle Logs: {matches.error}
      </Callout>
    );
  }

  const copy = (match: Match) => {
    setActionError(null);
    if (match.raw === null) {
      setActionError("That Battle Log is still loading.");
      return;
    }
    navigator.clipboard.writeText(match.raw).catch((err: unknown) => setActionError(String(err)));
  };

  const reveal = (match: Match) => {
    setActionError(null);
    revealLog(match.log.fileName).catch((err: unknown) => setActionError(String(err)));
  };

  const confirmDelete = () => {
    if (pendingDelete === null) return;
    const fileName = pendingDelete.log.fileName;
    setPendingDelete(null);
    deleteLog(fileName)
      .then(() => {
        if (selected === fileName) setSelected(null);
        matches.refresh();
      })
      .catch((err: unknown) => setActionError(String(err)));
  };

  const actions = (match: Match) => (
    <Menu label="Battle Log actions">
      <MenuItem onAction={() => copy(match)}>Copy Battle Log</MenuItem>
      <MenuItem onAction={() => reveal(match)}>Show in Finder</MenuItem>
      <MenuItem onAction={() => setPendingDelete(match)} tone="critical">
        Delete…
      </MenuItem>
    </Menu>
  );

  const dialog = (
    <ConfirmDialog
      confirmLabel="Move to Trash"
      description="The file moves to the Trash, so you can put it back from there."
      destructive
      onConfirm={confirmDelete}
      onOpenChange={(open) => {
        if (!open) setPendingDelete(null);
      }}
      open={pendingDelete !== null}
      title="Delete this Battle Log?"
    />
  );

  const detail = matches.matches.find((match) => match.log.fileName === selected);
  if (detail !== undefined) {
    return (
      <>
        <MatchDetail actions={actions(detail)} match={detail} onBack={() => setSelected(null)} row={matchRow(detail.log, detail.analysis, now)} />
        {actionError !== null ? <Callout className="m-4" variant="critical">{actionError}</Callout> : null}
        {dialog}
      </>
    );
  }

  if (matches.matches.length === 0) {
    return (
      <EmptyState
        className="m-4"
        description="After a match, open Battle Log → Export in the game. Your saved matches show up here."
        title="No Battle Logs yet"
      />
    );
  }

  const nameMissing = playerName.trim() === "";
  return (
    <div className="space-y-3 p-4">
      {nameMissing ? (
        <Callout variant="info">Add your player name in Settings to see wins and losses.</Callout>
      ) : null}
      {actionError !== null ? <Callout variant="critical">{actionError}</Callout> : null}
      {matches.error !== null ? <Callout variant="critical">{matches.error}</Callout> : null}
      <ul className="divide-y divide-border rounded-md border border-border">
        {matches.matches.map((match) => {
          const row = matchRow(match.log, match.analysis, now);
          return (
            <li className="flex items-center gap-1 pr-1" key={row.fileName}>
              <Button
                className="h-auto min-w-0 flex-1 justify-start gap-3 rounded-none px-3 py-2 text-left"
                onClick={() => setSelected(row.fileName)}
                variant="ghost"
              >
                <RowContent row={row} />
              </Button>
              {actions(match)}
            </li>
          );
        })}
      </ul>
      {dialog}
    </div>
  );
}

function MatchDetail({ actions, match, onBack, row }: { actions: React.ReactNode; match: Match; onBack: () => void; row: MatchRow }) {
  const [showRaw, setShowRaw] = useState(false);
  const summary = match.analysis?.summary;
  return (
    <div className="space-y-3 p-4">
      <div className="flex items-center justify-between">
        <Button onClick={onBack} size="sm" variant="ghost">
          ← All matches
        </Button>
        {actions}
      </div>
      <div className="flex items-center gap-3">
        <RowContent row={row} />
      </div>
      {summary === undefined ? (
        <p className="text-sm text-content-secondary">This Battle Log couldn't be summarized.</p>
      ) : (
        <dl className="grid grid-cols-2 gap-2 text-sm">
          <dt className="text-content-subtle">Prizes taken</dt>
          <dd className="text-content-primary">{row.prizes}</dd>
          <dt className="text-content-subtle">Knockouts</dt>
          <dd className="text-content-primary">
            {summary.knockoutsByPlayer.self}–{summary.knockoutsByPlayer.opponent}
          </dd>
          <dt className="text-content-subtle">Turns</dt>
          <dd className="text-content-primary">{summary.turnCount}</dd>
        </dl>
      )}
      <Button aria-expanded={showRaw} onClick={() => setShowRaw((open) => !open)} size="sm" variant="outline">
        {showRaw ? "Hide raw log" : "View raw log"}
      </Button>
      {showRaw ? (
        <pre className="max-h-72 overflow-y-auto whitespace-pre-wrap rounded-md border border-border p-2 font-mono text-xs text-content-secondary">
          {match.raw ?? "Loading…"}
        </pre>
      ) : null}
    </div>
  );
}

function RowContent({ row }: { row: MatchRow }) {
  return (
    <>
      <Badge className="w-7 shrink-0 justify-center" variant={row.result === null ? "outline" : RESULT_TONE[row.result]}>
        {row.result ?? "?"}
      </Badge>
      <span className="min-w-0 flex-1">
        <span className="block truncate text-sm font-medium text-content-primary">{row.title}</span>
        <span className="block text-xs text-content-subtle">
          {row.prizes === null ? row.when : `Prizes ${row.prizes} · ${row.when}`}
        </span>
      </span>
    </>
  );
}
