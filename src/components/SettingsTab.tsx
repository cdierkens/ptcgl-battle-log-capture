import { Button, Callout, Field, Input, Switch } from "@pokedojo/ui";
import { useEffect, useState } from "react";

import type { SettingsView } from "../lib/use-settings";

import { openLogsFolder, pickOutputDir } from "../lib/tauri";
import { ClipboardSetting } from "./ClipboardSetting";

export function SettingsTab({ settings }: { settings: SettingsView }) {
  const current = settings.settings;
  const [name, setName] = useState("");
  const [nameSaved, setNameSaved] = useState(false);
  const [picking, setPicking] = useState(false);
  const [actionError, setActionError] = useState<null | string>(null);

  useEffect(() => {
    if (current !== null) setName(current.player_name);
  }, [current]);

  if (current === null) {
    return settings.error === null ? (
      <p className="p-4 text-sm text-content-secondary">Loading settings…</p>
    ) : (
      <Callout className="m-4" variant="critical">
        {settings.error}
      </Callout>
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
      {settings.error !== null ? <Callout variant="critical">{settings.error}</Callout> : null}
      {actionError !== null ? <Callout variant="critical">{actionError}</Callout> : null}

      <Field
        description={nameSaved ? "Saved." : "As shown in PTCG Live. Used to tell your wins from losses."}
        htmlFor="player-name"
        label="Player name"
      >
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
      </Field>

      <Field label="Save folder">
        <p className="break-all font-mono text-xs text-content-secondary">{current.output_dir}</p>
        <div className="flex gap-2 pt-1">
          <Button isLoading={picking} onClick={pickFolder} size="sm" variant="outline">
            Change…
          </Button>
          <Button onClick={showFolder} size="sm" variant="ghost">
            Show in Finder
          </Button>
        </div>
      </Field>

      <div className="space-y-1">
        <Switch
          isDisabled={settings.saving}
          isSelected={current.prompt_after_match}
          onChange={(next) => void settings.update({ prompt_after_match: next })}
        >
          Prompt to save after each match
        </Switch>
        <p className="text-xs text-content-subtle">
          Opens this drawer on the results screen and covers Continue until the Battle Log is saved.
        </p>
      </div>

      <ClipboardSetting settings={settings} />
    </div>
  );
}
