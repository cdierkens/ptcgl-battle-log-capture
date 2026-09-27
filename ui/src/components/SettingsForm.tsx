import { Button, Callout, Field, Input, Panel } from "@pokedojo/ui";
import { useEffect, useState } from "react";

import { getSettings, saveSettings } from "../lib/tauri";

export function SettingsForm() {
  const [playerName, setPlayerName] = useState("");
  const [outputDir, setOutputDir] = useState("");
  const [saving, setSaving] = useState(false);
  const [message, setMessage] = useState<null | string>(null);
  const [error, setError] = useState<null | string>(null);

  useEffect(() => {
    void getSettings()
      .then((s) => {
        setPlayerName(s.player_name);
        setOutputDir(s.output_dir);
      })
      .catch((err: unknown) => setError(String(err)));
  }, []);

  const save = () => {
    setSaving(true);
    setMessage(null);
    setError(null);
    void saveSettings({ output_dir: outputDir, player_name: playerName })
      .then(() => setMessage("Settings saved."))
      .catch((err: unknown) => setError(String(err)))
      .finally(() => setSaving(false));
  };

  return (
    <Panel className="space-y-3 p-4">
      <h2 className="text-sm font-semibold text-content-secondary">Settings</h2>
      {error !== null ? <Callout variant="critical">{error}</Callout> : null}
      {message !== null ? <Callout variant="positive">{message}</Callout> : null}
      <Field htmlFor="player-name" label="Player name">
        <Input
          id="player-name"
          onChange={(e) => setPlayerName(e.target.value)}
          value={playerName}
        />
      </Field>
      <Field htmlFor="output-dir" label="Output directory">
        <Input
          id="output-dir"
          onChange={(e) => setOutputDir(e.target.value)}
          value={outputDir}
        />
      </Field>
      <Button isLoading={saving} onClick={save}>
        Save settings
      </Button>
    </Panel>
  );
}