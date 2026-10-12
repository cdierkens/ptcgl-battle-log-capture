import type { ComponentProps } from "react";

import { Button } from "@/components/ui/button";
import { Spinner } from "@/components/ui/spinner";

/** A Button that shows a spinner and disables itself while `loading`. */
export function LoadingButton({
  children,
  disabled,
  loading = false,
  ...props
}: ComponentProps<typeof Button> & { loading?: boolean }) {
  return (
    <Button aria-busy={loading || undefined} disabled={disabled || loading} {...props}>
      {loading ? <Spinner aria-hidden data-icon="inline-start" role={undefined} /> : null}
      {children}
    </Button>
  );
}
