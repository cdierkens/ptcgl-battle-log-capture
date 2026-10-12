import { Check, Copy } from "lucide-react";
import { type ComponentProps, useEffect, useRef, useState } from "react";

import { LoadingButton } from "./LoadingButton";

const COPIED_MS = 2000;

/** Copies `value` to the clipboard and shows "Copied" briefly. */
export function CopyButton({
  label,
  value,
  ...props
}: Omit<ComponentProps<typeof LoadingButton>, "children" | "loading" | "onClick"> & { label: string; value: string }) {
  const [copied, setCopied] = useState(false);
  const [copying, setCopying] = useState(false);
  const timer = useRef<null | ReturnType<typeof setTimeout>>(null);

  useEffect(
    () => () => {
      if (timer.current !== null) clearTimeout(timer.current);
    },
    [],
  );

  const copy = () => {
    setCopying(true);
    navigator.clipboard
      .writeText(value)
      .then(() => {
        setCopied(true);
        if (timer.current !== null) clearTimeout(timer.current);
        timer.current = setTimeout(() => setCopied(false), COPIED_MS);
      })
      .catch(() => undefined)
      .finally(() => setCopying(false));
  };

  return (
    <LoadingButton loading={copying} onClick={copy} type="button" {...props}>
      {copying ? null : copied ? <Check aria-hidden data-icon="inline-start" /> : <Copy aria-hidden data-icon="inline-start" />}
      {copied ? "Copied" : label}
    </LoadingButton>
  );
}
