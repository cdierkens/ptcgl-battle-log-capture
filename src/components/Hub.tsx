import { StatusAlert } from "@/components/StatusAlert";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Tabs, TabsContent, TabsList, TabsTrigger } from "@/components/ui/tabs";

import type { CaptureStatusEvent } from "../lib/tauri";

import { chipForState, isResultsState } from "../lib/capture-status";
import { needsAttention } from "../lib/status-history";
import { useMatches } from "../lib/use-matches";
import { useOverlayState } from "../lib/use-overlay-state";
import { useSettings } from "../lib/use-settings";
import { HelpTab } from "./HelpTab";
import { MatchCard } from "./MatchCard";
import { MatchList } from "./MatchList";
import { SettingsTab } from "./SettingsTab";

interface HubProps {
  focusPaste?: boolean;
  history: CaptureStatusEvent[];
  historyError?: null | string;
  onClose?: () => void;
}

/** The one place for this match, past matches, settings and help. Shared by the drawer and the main window. */
export function Hub({ focusPaste = false, history, historyError = null, onClose }: HubProps) {
  const overlay = useOverlayState();
  const settings = useSettings();
  const playerName = settings.settings?.player_name ?? "";
  const matches = useMatches(playerName);
  const chip = overlay.state === null ? null : chipForState(overlay.state);
  const attention = needsAttention(history);
  const errors = [overlay.error, settings.error, historyError].filter((e): e is string => e !== null);

  return (
    <div className="flex h-full flex-col bg-background text-foreground">
      <header className="flex items-center justify-between gap-2 border-b border-border px-4 py-3">
        <div className="flex items-center gap-2">
          <h1 className="text-sm font-semibold">Battle Log Capture</h1>
          {chip !== null ? <Badge variant={chip.variant}>{chip.label}</Badge> : null}
        </div>
        {onClose !== undefined ? (
          <Button aria-label="Close drawer" onClick={onClose} size="sm" variant="ghost">
            Close
          </Button>
        ) : null}
      </header>
      {errors.map((message) => (
        <StatusAlert className="m-3" key={message} tone="critical">
          {message}
        </StatusAlert>
      ))}
      {isResultsState(overlay.state) ? (
        <MatchCard
          clipboardOn={settings.settings?.clipboard_autosave ?? false}
          focusPaste={focusPaste}
          latestFileName={matches.matches?.[0]?.log.fileName ?? null}
          overlay={overlay}
          playerName={playerName}
        />
      ) : null}
      <Tabs className="flex min-h-0 flex-1 flex-col" defaultValue="matches">
        <TabsList aria-label="Sections" className="mx-4 mt-2" variant="line">
          <TabsTrigger value="matches">Matches</TabsTrigger>
          <TabsTrigger value="settings">Settings</TabsTrigger>
          <TabsTrigger value="help">
            Help
            {attention ? (
              <>
                <span aria-hidden className="ml-1 inline-block size-2 rounded-full bg-destructive" />
                <span className="sr-only">(needs attention)</span>
              </>
            ) : null}
          </TabsTrigger>
        </TabsList>
        <TabsContent className="min-h-0 overflow-y-auto" value="matches">
          <MatchList matches={matches} playerName={playerName} />
        </TabsContent>
        <TabsContent className="min-h-0 overflow-y-auto" value="settings">
          <SettingsTab settings={settings} />
        </TabsContent>
        <TabsContent className="min-h-0 overflow-y-auto" value="help">
          <HelpTab history={history} state={overlay.state} />
        </TabsContent>
      </Tabs>
    </div>
  );
}
