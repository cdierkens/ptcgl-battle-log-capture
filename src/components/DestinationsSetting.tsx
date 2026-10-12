import { useState } from "react";

import { LoadingButton } from "@/components/LoadingButton";
import { StatusAlert } from "@/components/StatusAlert";
import {
  AlertDialog,
  AlertDialogAction,
  AlertDialogCancel,
  AlertDialogContent,
  AlertDialogDescription,
  AlertDialogFooter,
  AlertDialogHeader,
  AlertDialogTitle,
} from "@/components/ui/alert-dialog";
import { Button } from "@/components/ui/button";
import { Field, FieldDescription, FieldLabel, FieldTitle } from "@/components/ui/field";
import { Input } from "@/components/ui/input";
import { Switch } from "@/components/ui/switch";
import { Textarea } from "@/components/ui/textarea";

import type { SettingsView } from "../lib/use-settings";

import { canConfirm, hostOf, tokenToSend } from "../lib/destinations";
import {
  type Destination,
  importDestination,
  type ImportPreview,
  openTokenPage,
  previewDestination,
  removeDestination,
  setDestinationEnabled,
} from "../lib/tauri";

type Step =
  | { config: string; preview: ImportPreview; token: string; type: "confirming" }
  | { config: string; type: "pasting" }
  | { type: "idle" };

/** Sync destinations: paste a site's config, confirm it, then enable, disable, or remove. */
export function DestinationsSetting({ settings }: { settings: SettingsView }) {
  const destinations = settings.settings?.destinations ?? [];
  const [step, setStep] = useState<Step>({ type: "idle" });
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<null | string>(null);
  const [pendingRemove, setPendingRemove] = useState<Destination | null>(null);

  const run = (action: () => Promise<void>) => {
    setBusy(true);
    setError(null);
    action()
      .catch((err: unknown) => setError(String(err)))
      .finally(() => setBusy(false));
  };

  const cancel = () => {
    setStep({ type: "idle" });
    setError(null);
  };

  const review = (config: string) =>
    run(async () => {
      const preview = await previewDestination(config);
      setStep({ config, preview, token: "", type: "confirming" });
    });

  const save = (config: string, preview: ImportPreview, token: string) =>
    run(async () => {
      await importDestination(config, tokenToSend(preview, token));
      setStep({ type: "idle" });
    });

  const openPage = (url: string) => {
    setError(null);
    openTokenPage(url).catch((err: unknown) => setError(String(err)));
  };

  return (
    <Field>
      <FieldTitle>Sync</FieldTitle>
      <FieldDescription>Sends each saved Battle Log to a site you use. Copy the destination config from the site and paste it here.</FieldDescription>

      {destinations.map((destination) => (
        <div className="flex items-center gap-3 py-1" key={destination.id}>
          <Switch
            checked={destination.enabled}
            disabled={busy}
            id={`destination-${destination.id}`}
            onCheckedChange={(enabled) => run(() => setDestinationEnabled(destination.id, enabled))}
          />
          <FieldLabel className="min-w-0 flex-1 flex-col items-start gap-0" htmlFor={`destination-${destination.id}`}>
            <span>{destination.name}</span>
            <span className="text-xs font-normal text-muted-foreground">{hostOf(destination.url)}</span>
          </FieldLabel>
          <Button disabled={busy} onClick={() => setPendingRemove(destination)} size="sm" variant="ghost">
            Remove…
          </Button>
        </div>
      ))}

      {step.type === "idle" ? (
        <div className="pt-1">
          <Button onClick={() => setStep({ config: "", type: "pasting" })} size="sm" variant="outline">
            Paste config…
          </Button>
        </div>
      ) : null}

      {step.type === "pasting" ? (
        <div className="space-y-2">
          <Textarea
            aria-label="Destination config"
            autoFocus
            className="h-24 font-mono text-xs"
            onChange={(event) => setStep({ config: event.target.value, type: "pasting" })}
            placeholder='{"ptcglTrackerDestination": 1, …}'
            value={step.config}
          />
          <div className="flex gap-2">
            <LoadingButton disabled={step.config.trim() === ""} loading={busy} onClick={() => review(step.config)} size="sm">
              Continue
            </LoadingButton>
            <Button onClick={cancel} size="sm" variant="ghost">
              Cancel
            </Button>
          </div>
        </div>
      ) : null}

      {step.type === "confirming" ? (
        <div className="space-y-3 rounded-md border p-3">
          <p className="text-sm">
            Sync saved Battle Logs to <strong>{step.preview.name}</strong> at{" "}
            <span className="font-mono">{step.preview.host}</span>?
          </p>
          {step.preview.replaces === null ? null : (
            <p className="text-xs text-muted-foreground">This updates {step.preview.replaces}.</p>
          )}
          {step.preview.token === "embedded" ? null : (
            <Field>
              <FieldLabel htmlFor="destination-token">
                {step.preview.token === "required" ? "Token" : "New token (optional)"}
              </FieldLabel>
              <Input
                autoComplete="off"
                id="destination-token"
                onChange={(event) => setStep({ ...step, token: event.target.value })}
                type="password"
                value={step.token}
              />
              <FieldDescription>
                {step.preview.token === "optional" ? "Leave blank to keep the current token. " : null}
                {step.preview.tokenUrl === null ? null : (
                  <Button className="h-auto p-0" onClick={() => openPage(step.preview.tokenUrl ?? "")} size="sm" variant="link">
                    Get a token from {step.preview.host}
                  </Button>
                )}
              </FieldDescription>
            </Field>
          )}
          <div className="flex gap-2">
            <LoadingButton
              disabled={!canConfirm(step.preview, step.token)}
              loading={busy}
              onClick={() => save(step.config, step.preview, step.token)}
              size="sm"
            >
              {step.preview.replaces === null ? "Add" : "Update"}
            </LoadingButton>
            <Button onClick={cancel} size="sm" variant="ghost">
              Cancel
            </Button>
          </div>
        </div>
      ) : null}

      {error !== null ? <StatusAlert tone="critical">{error}</StatusAlert> : null}

      <AlertDialog
        onOpenChange={(open) => {
          if (!open) setPendingRemove(null);
        }}
        open={pendingRemove !== null}
      >
        <AlertDialogContent>
          <AlertDialogHeader>
            <AlertDialogTitle>Remove {pendingRemove?.name}?</AlertDialogTitle>
            <AlertDialogDescription>
              Battle Logs waiting to sync won't be sent, and its token is deleted from this Mac. Matches already synced
              stay on {pendingRemove === null ? "the site" : hostOf(pendingRemove.url)}.
            </AlertDialogDescription>
          </AlertDialogHeader>
          <AlertDialogFooter>
            <AlertDialogCancel>Cancel</AlertDialogCancel>
            <AlertDialogAction
              onClick={() => {
                const target = pendingRemove;
                if (target !== null) run(() => removeDestination(target.id));
              }}
              variant="destructive"
            >
              Remove
            </AlertDialogAction>
          </AlertDialogFooter>
        </AlertDialogContent>
      </AlertDialog>
    </Field>
  );
}
