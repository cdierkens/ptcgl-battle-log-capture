import { useState } from "react";

import { LoadingButton } from "@/components/LoadingButton";
import { StatusAlert } from "@/components/StatusAlert";
import { Button } from "@/components/ui/button";
import { Field, FieldDescription, FieldLabel, FieldTitle } from "@/components/ui/field";
import { Input } from "@/components/ui/input";
import { Switch } from "@/components/ui/switch";

import type { SettingsView } from "../lib/use-settings";

import { openLogsFolder, pickOutputDir } from "../lib/tauri";
import { ClipboardSetting } from "./ClipboardSetting";

export function SettingsTab({ settings }: { settings: SettingsView }) {
  const current = settings.settings;
  const [name, setName] = useState(current?.player_name ?? "");
  const [syncedFrom, setSyncedFrom] = useState(current);
  const [nameSaved, setNameSaved] = useState(false);
  const [picking, setPicking] = useState(false);
  const [actionError, setActionError] = useState<null | string>(null);

  // Saved settings win over the draft name whenever they change.
  if (current !== syncedFrom) {
    setSyncedFrom(current);
    if (current !== null) setName(current.player_name);
  }

  if (current === null) {
    return settings.error === null ? (
      <p className="p-4 text-sm text-muted-foreground">Loading settings…</p>
    ) : (
      <StatusAlert className="m-4" tone="critical">
        {settings.error}
      </StatusAlert>
    );
  }

  const commitName = () => {
    const trimmed = name.trim();
    if (trimmed === current.player_name) return;
    void settings.update({ player_name: trimmed }).then((ok) => setNameSaved(ok));
  };

  const pickFolder = () => {
    setPicking(true);
    setActionError(null);
    pickOutputDir()
      .then((dir) => (dir === null ? undefined : settings.update({ output_dir: dir })))
      .catch((err: unknown) => setActionError(String(err)))
      .finally(() => setPicking(false));
  };

  const showFolder = () => {
    setActionError(null);
    openLogsFolder().catch((err: unknown) => setActionError(String(err)));
  };

  return (
    <div className="space-y-5 p-4">
      {settings.error !== null ? <StatusAlert tone="critical">{settings.error}</StatusAlert> : null}
      {actionError !== null ? <StatusAlert tone="critical">{actionError}</StatusAlert> : null}

      <Field>
        <FieldLabel htmlFor="player-name">Player name</FieldLabel>
        <Input
          id="player-name"
          onBlur={commitName}
          onChange={(event) => {
            setName(event.target.value);
            setNameSaved(false);
          }}
          onKeyDown={(event) => {
            if (event.key === "Enter") commitName();
          }}
          value={name}
        />
        <FieldDescription>{nameSaved ? "Saved." : "As shown in PTCG Live. Used to tell your wins from losses."}</FieldDescription>
      </Field>

      <Field>
        <FieldTitle>Save folder</FieldTitle>
        <p className="break-all font-mono text-xs text-muted-foreground">{current.output_dir}</p>
        <div className="flex gap-2 pt-1">
          <LoadingButton loading={picking} onClick={pickFolder} size="sm" variant="outline">
            Change…
          </LoadingButton>
          <Button onClick={showFolder} size="sm" variant="ghost">
            Show in Finder
          </Button>
        </div>
      </Field>

      <Field>
        <Field orientation="horizontal">
          <Switch
            checked={current.prompt_after_match}
            disabled={settings.saving}
            id="prompt-after-match"
            onCheckedChange={(next) => void settings.update({ prompt_after_match: next })}
          />
          <FieldLabel htmlFor="prompt-after-match">Prompt to save after each match</FieldLabel>
        </Field>
        <FieldDescription>Opens this drawer on the results screen and covers Continue until the Battle Log is saved.</FieldDescription>
      </Field>

      <ClipboardSetting settings={settings} />
    </div>
  );
}
