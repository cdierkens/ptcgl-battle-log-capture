import { Badge, Button, Callout, Tab, TabList, TabPanel, Tabs } from "@pokedojo/ui";

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
    <div className="flex h-full flex-col bg-background text-content-primary">
      <header className="flex items-center justify-between gap-2 border-b border-border px-4 py-3">
        <div className="flex items-center gap-2">
          <h1 className="text-sm font-semibold">Battle Log Capture</h1>
          {chip !== null ? <Badge variant={chip.tone}>{chip.label}</Badge> : null}
        </div>
        {onClose !== undefined ? (
          <Button aria-label="Close drawer" onClick={onClose} size="sm" variant="ghost">
            Close
          </Button>
        ) : null}
      </header>
      {errors.map((message) => (
        <Callout className="m-3" key={message} variant="critical">
          {message}
        </Callout>
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
      <Tabs className="flex min-h-0 flex-1 flex-col" defaultSelectedKey="matches">
        <TabList aria-label="Sections" className="px-4">
          <Tab id="matches">Matches</Tab>
          <Tab id="settings">Settings</Tab>
          <Tab id="help">
            Help
            {attention ? (
              <>
                <span aria-hidden className="ml-1 inline-block size-2 rounded-full bg-status-critical" />
                <span className="sr-only">(needs attention)</span>
              </>
            ) : null}
          </Tab>
        </TabList>
        <TabPanel id="matches">
          <MatchList matches={matches} playerName={playerName} />
        </TabPanel>
        <TabPanel id="settings">
          <SettingsTab settings={settings} />
        </TabPanel>
        <TabPanel id="help">
          <HelpTab history={history} state={overlay.state} />
        </TabPanel>
      </Tabs>
    </div>
  );
}
