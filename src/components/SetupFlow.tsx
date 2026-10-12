import { LoadingButton } from "@/components/LoadingButton";
import { PageSpinner } from "@/components/PageSpinner";
import { StatusAlert } from "@/components/StatusAlert";
import { Button } from "@/components/ui/button";
import { Field, FieldDescription, FieldError, FieldLabel } from "@/components/ui/field";
import { Input } from "@/components/ui/input";
import { useState } from "react";

import { openScreenRecordingSettings, relaunchApp } from "../lib/tauri";
import { useOverlayState } from "../lib/use-overlay-state";
import { useSettings } from "../lib/use-settings";
import { ClipboardSetting } from "./ClipboardSetting";

type Step = "clipboard" | "done" | "name" | "screen";
const STEPS: Step[] = ["screen", "name", "clipboard", "done"];

/** First run: Screen Recording, player name, then the optional clipboard auto-save. */
export function SetupFlow({ onDone }: { onDone: () => void }) {
  const [step, setStep] = useState<Step>("screen");
  const settings = useSettings();
  const index = STEPS.indexOf(step);
  const next = () => setStep(STEPS[Math.min(index + 1, STEPS.length - 1)]);

  return (
    <main className="mx-auto flex h-screen max-w-md flex-col justify-center gap-6 bg-background p-6 text-foreground">
      <header className="space-y-1">
        <p className="text-xs text-muted-foreground">
          {step === "done" ? "All set" : `Step ${index + 1} of ${STEPS.length - 1}`}
        </p>
        <h1 className="text-lg font-semibold">Set up Battle Log Capture</h1>
      </header>
      {settings.error !== null ? <StatusAlert tone="critical">{settings.error}</StatusAlert> : null}
      {step === "screen" ? <ScreenStep onNext={next} /> : null}
      {step === "name" ? <NameStep onNext={next} settings={settings} /> : null}
      {step === "clipboard" ? (
        <section className="space-y-4">
          <ClipboardSetting settings={settings} />
          <Button onClick={next}>Continue</Button>
        </section>
      ) : null}
      {step === "done" ? (
        <section className="space-y-4">
          <p className="text-sm text-muted-foreground">
            You&apos;re set. Start a match — we&apos;ll show up on the results screen.
          </p>
          <Button onClick={onDone}>Open Battle Log Capture</Button>
        </section>
      ) : null}
    </main>
  );
}

function NameStep({ onNext, settings }: { onNext: () => void; settings: ReturnType<typeof useSettings> }) {
  const [name, setName] = useState(settings.settings?.player_name ?? "");
  const [error, setError] = useState<null | string>(null);

  if (settings.settings === null && settings.error === null) return <PageSpinner />;

  const save = () => {
    const trimmed = name.trim();
    if (trimmed === "") {
      setError("Enter your PTCG Live name.");
      return;
    }
    setError(null);
    void settings.update({ player_name: trimmed }).then((ok) => {
      if (ok) onNext();
    });
  };

  return (
    <form
      className="space-y-4"
      onSubmit={(event) => {
        event.preventDefault();
        save();
      }}
    >
      <Field data-invalid={error !== null || undefined}>
        <FieldLabel htmlFor="setup-player-name">Player name</FieldLabel>
        <Input aria-invalid={error !== null} autoFocus id="setup-player-name" onChange={(event) => setName(event.target.value)} value={name} />
        <FieldDescription>As shown in PTCG Live. Used to tell your wins from losses.</FieldDescription>
        <FieldError>{error}</FieldError>
      </Field>
      <LoadingButton loading={settings.saving} type="submit">
        Continue
      </LoadingButton>
    </form>
  );
}

function ScreenStep({ onNext }: { onNext: () => void }) {
  const { state } = useOverlayState();
  const [error, setError] = useState<null | string>(null);

  if (state === null) return <PageSpinner />;

  if (state !== "PermissionRequired") {
    return (
      <section className="space-y-4">
        <StatusAlert title="Screen Recording allowed" tone="positive">
          The app can spot the results screen.
        </StatusAlert>
        <Button onClick={onNext}>Continue</Button>
      </section>
    );
  }

  return (
    <section className="space-y-4">
      <p className="text-sm text-muted-foreground">
        Battle Log Capture looks at the game window to spot the results screen. Turn it on in System Settings → Privacy
        &amp; Security → Screen Recording, then quit and reopen.
      </p>
      {error !== null ? <StatusAlert tone="critical">{error}</StatusAlert> : null}
      <div className="flex flex-wrap gap-2">
        <Button onClick={() => void openScreenRecordingSettings().catch((err: unknown) => setError(String(err)))}>
          Open System Settings
        </Button>
        <Button onClick={() => void relaunchApp().catch((err: unknown) => setError(String(err)))} variant="outline">
          Quit &amp; reopen
        </Button>
        <Button onClick={onNext} variant="ghost">
          Not now
        </Button>
      </div>
    </section>
  );
}
