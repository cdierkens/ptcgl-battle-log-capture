import { useState } from "react";

import { LoadingButton } from "@/components/LoadingButton";
import { StatusAlert } from "@/components/StatusAlert";

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
    <div className="flex h-screen w-screen items-center justify-between gap-2 rounded-md border border-border bg-muted px-3">
      {error !== null ? (
        <StatusAlert tone="critical">{error}</StatusAlert>
      ) : (
        <p className="text-xs font-semibold text-foreground">Save your Battle Log first</p>
      )}
      <LoadingButton loading={skipping} onClick={skip} size="sm" variant="ghost">
        Continue without saving
      </LoadingButton>
    </div>
  );
}
