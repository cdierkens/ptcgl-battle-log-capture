import { StatusAlert } from "@/components/StatusAlert";
import { Field, FieldLabel } from "@/components/ui/field";
import { Switch } from "@/components/ui/switch";
import { useEffect, useState } from "react";

import type { SettingsView } from "../lib/use-settings";

import { type ClipboardAccess, getClipboardAccess, requestClipboardAccess } from "../lib/tauri";

const PRIVACY_NOTE =
  "The app checks the clipboard only on the results screen, only when it changes, and never keeps anything that isn't a Battle Log.";

/** The clipboard auto-save switch. Turning it on reads once so macOS asks now, not mid-match. */
export function ClipboardSetting({ settings }: { settings: SettingsView }) {
  const enabled = settings.settings?.clipboard_autosave ?? false;
  const [access, setAccess] = useState<ClipboardAccess | null>(null);
  const [requesting, setRequesting] = useState(false);
  const [error, setError] = useState<null | string>(null);

  useEffect(() => {
    if (!enabled) return;
    getClipboardAccess()
      .then(setAccess)
      .catch((err: unknown) => setError(String(err)));
  }, [enabled]);

  const toggle = (next: boolean) => {
    setError(null);
    if (!next) {
      void settings.update({ clipboard_autosave: false });
      return;
    }
    setRequesting(true);
    requestClipboardAccess()
      .then((granted) => {
        setAccess(granted);
        return settings.update({ clipboard_autosave: granted !== "denied" });
      })
      .catch((err: unknown) => setError(String(err)))
      .finally(() => setRequesting(false));
  };

  return (
    <div className="space-y-2">
      <Field orientation="horizontal">
        <Switch
          checked={enabled}
          disabled={settings.settings === null || settings.saving || requesting}
          id="clipboard-autosave"
          onCheckedChange={toggle}
        />
        <FieldLabel htmlFor="clipboard-autosave">Save Battle Logs from the clipboard automatically</FieldLabel>
      </Field>
      <p className="text-xs text-muted-foreground">{PRIVACY_NOTE}</p>
      {requesting ? <p className="text-xs text-muted-foreground">Waiting for macOS to allow paste access…</p> : null}
      {access === "denied" ? (
        <StatusAlert tone="critical">
          Clipboard access is denied. Allow it in System Settings → Privacy &amp; Security → Paste from Other Apps,
          then turn this on again. Pasting with ⌘V still works.
        </StatusAlert>
      ) : null}
      {enabled && access === "ask" ? (
        <StatusAlert tone="caution">
          macOS will ask before each save. Choose Always Allow in System Settings → Privacy &amp; Security → Paste
          from Other Apps to save without the prompt.
        </StatusAlert>
      ) : null}
      {error !== null ? <StatusAlert tone="critical">{error}</StatusAlert> : null}
    </div>
  );
}
