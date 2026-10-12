import { Button, Callout, cn } from "@pokedojo/ui";
import { useEffect, useRef, useState } from "react";

import { Hub } from "../components/Hub";
import { nextDrawerOpen } from "../lib/capture-drawer";
import { handleForState, isResultsState } from "../lib/capture-status";
import { setDrawerOpen } from "../lib/tauri";
import { useOverlayState } from "../lib/use-overlay-state";
import { useSettings } from "../lib/use-settings";
import { useStatusHistory } from "../lib/use-status-history";

/** Matches the panel's `duration-200` slide. */
const SLIDE_MS = 200;

export function CaptureDrawer() {
  const { error: bridgeError, state } = useOverlayState();
  const prompts = useSettings().settings?.prompt_after_match ?? true;
  const { error: historyError, history } = useStatusHistory();
  const [open, setOpen] = useState(false);
  const [focusPaste, setFocusPaste] = useState(false);
  // `mounted` renders the panel and `shown` slides it in. The window grows before the
  // slide-in and shrinks after the slide-out, so no transparent area blocks the game.
  const [mounted, setMounted] = useState(false);
  const [shown, setShown] = useState(false);
  const [resizeError, setResizeError] = useState<null | string>(null);
  const previousState = useRef<null | string>(null);

  useEffect(() => {
    if (state === null) return;
    const previous = previousState.current;
    previousState.current = state;
    setOpen((current) => nextDrawerOpen(previous, state, current, prompts));
  }, [prompts, state]);

  useEffect(() => {
    if (!open) setFocusPaste(false);
  }, [open]);

  useEffect(() => {
    if (open) {
      let cancelled = false;
      setMounted(true);
      setDrawerOpen(true)
        .then(() => {
          setResizeError(null);
          if (!cancelled) requestAnimationFrame(() => setShown(true));
        })
        .catch((err: unknown) => setResizeError(String(err)));
      return () => {
        cancelled = true;
      };
    }
    setShown(false);
    const timer = window.setTimeout(() => {
      setMounted(false);
      setDrawerOpen(false)
        .then(() => setResizeError(null))
        .catch((err: unknown) => setResizeError(String(err)));
    }, SLIDE_MS);
    return () => window.clearTimeout(timer);
  }, [open]);

  useEffect(() => {
    const onKey = (event: KeyboardEvent) => {
      if (event.key === "Escape") setOpen(false);
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, []);

  if (!mounted) {
    const handle = handleForState(state);
    return (
      <div className="flex h-screen w-screen">
        <Button
          aria-label={`${handle.label}: open the Battle Log Capture drawer`}
          className={cn(
            "h-full w-full",
            handle.pulse && "animate-pulse [animation-iteration-count:3] motion-reduce:animate-none",
          )}
          onClick={() => {
            setFocusPaste(state === "Results" || state === "Dismissed");
            setOpen(true);
          }}
          size="sm"
          variant={handle.variant}
        >
          {handle.label}
        </Button>
      </div>
    );
  }

  return (
    <aside
      aria-label="Battle Log Capture"
      className={cn(
        "flex h-screen w-screen flex-col border-r border-border bg-background transition-transform duration-200 motion-reduce:transition-none",
        // The click that opened the drawer must not land on whatever renders under the cursor.
        shown ? "translate-x-0" : "pointer-events-none -translate-x-full",
      )}

    >
      {resizeError !== null ? <Callout variant="critical">{resizeError}</Callout> : null}
      <div className="min-h-0 flex-1">
        <Hub
          focusPaste={focusPaste && isResultsState(state)}
          history={history}
          historyError={historyError ?? bridgeError}
          onClose={() => setOpen(false)}
        />
      </div>
    </aside>
  );
}
