import { Button, Callout } from "@pokedojo/ui";
import { useState } from "react";

import { dismissShield } from "../lib/tauri";

export function ContinueShield() {
  const [error, setError] = useState<null | string>(null);
  const [skipping, setSkipping] = useState(false);

  const skip = () => {
    setSkipping(true);
    dismissShield()
      .catch((err: unknown) => setError(String(err)))
      .finally(() => setSkipping(false));
  };

  return (
    <div className="flex h-screen w-screen items-center justify-between gap-2 rounded-md border border-border bg-surface-raised px-3">
      {error !== null ? (
        <Callout variant="critical">{error}</Callout>
      ) : (
        <p className="text-xs font-semibold text-content-primary">Save your Battle Log first</p>
      )}
      <Button isLoading={skipping} onClick={skip} size="sm" variant="ghost">
        Continue without saving
      </Button>
    </div>
  );
}
