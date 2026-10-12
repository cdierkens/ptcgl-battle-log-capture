import { listen } from "@tauri-apps/api/event";
import { useCallback, useEffect, useState } from "react";

import { getSettings, saveSettings, type Settings, SETTINGS_CHANGED_EVENT } from "./tauri";

export interface SettingsView {
  error: null | string;
  saving: boolean;
  settings: null | Settings;
  /** Persists a partial change on top of the latest settings. Resolves false on failure. */
  update: (patch: Partial<Settings>) => Promise<boolean>;
}

/** Live settings: loaded once, then kept in sync with `settings://changed` from any window. */
export function useSettings(): SettingsView {
  const [settings, setSettings] = useState<null | Settings>(null);
  const [error, setError] = useState<null | string>(null);
  const [saving, setSaving] = useState(false);

  useEffect(() => {
    getSettings()
      // An event may already have delivered newer settings.
      .then((initial) => setSettings((current) => current ?? initial))
      .catch((err: unknown) => setError(String(err)));
    const unlisten = listen<Settings>(SETTINGS_CHANGED_EVENT, (event) => setSettings(event.payload));
    unlisten.catch((err: unknown) => setError(String(err)));
    return () => {
      void unlisten.then((fn) => fn());
    };
  }, []);

  const update = useCallback(
    async (patch: Partial<Settings>) => {
      const base = settings ?? (await getSettings().catch(() => null));
      if (base === null) {
        setError("Settings haven't loaded yet.");
        return false;
      }
      const next = { ...base, ...patch };
      setSaving(true);
      setError(null);
      return saveSettings(next)
        .then((saved) => {
          setSettings(saved);
          return true;
        })
        .catch((err: unknown) => {
          setError(String(err));
          return false;
        })
        .finally(() => setSaving(false));
    },
    [settings],
  );

  return { error, saving, settings, update };
}
