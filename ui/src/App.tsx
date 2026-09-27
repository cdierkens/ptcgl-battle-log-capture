import { Button, Callout } from "@pokedojo/ui";
import { useEffect, useState } from "react";

import { LogsBrowser } from "./components/LogsBrowser";
import { SettingsForm } from "./components/SettingsForm";
import { StatusPanel } from "./components/StatusPanel";
import { getCaptureRunning, setCaptureRunning } from "./lib/tauri";

export function App() {
  const [error, setError] = useState<null | string>(null);
  const [running, setRunning] = useState<boolean | null>(null);
  const [toggling, setToggling] = useState(false);

  useEffect(() => {
    void getCaptureRunning()
      .then(setRunning)
      .catch((err: unknown) => setError(String(err)));
  }, []);

  const toggle = () => {
    if (running === null) return;
    const next = !running;
    const previous = running;
    setRunning(next);
    setToggling(true);
    void setCaptureRunning(next)
      .then(() => setError(null))
      .catch((err: unknown) => {
        setRunning(previous);
        setError(String(err));
      })
      .finally(() => setToggling(false));
  };

  return (
    <main className="mx-auto max-w-3xl space-y-4 p-6">
      <header className="flex items-center justify-between">
        <h1 className="text-lg font-semibold text-content-primary">Battle Log Capture</h1>
        <Button
          disabled={running === null}
          isLoading={toggling}
          onClick={toggle}
          variant={running ? "outline" : "brand"}
        >
          {running === null ? "Loading…" : running ? "Stop capture" : "Start capture"}
        </Button>
      </header>

      {error !== null ? (
        <Callout variant="critical">Capture failed: {error}</Callout>
      ) : null}

      <StatusPanel />

      <LogsBrowser />

      <SettingsForm />
    </main>
  );
}