import type { ReactNode } from "react";

import { CircleCheck, CircleX, Info, TriangleAlert } from "lucide-react";

import { Alert, AlertDescription, AlertTitle } from "@/components/ui/alert";

export type StatusTone = "caution" | "critical" | "info" | "positive";

const ICONS = { caution: TriangleAlert, critical: CircleX, info: Info, positive: CircleCheck } as const;

/** An Alert whose tone is carried by an icon, not colour alone (ADR 0003). */
export function StatusAlert({
  actions,
  children,
  className,
  title,
  tone,
}: {
  actions?: ReactNode;
  children: ReactNode;
  className?: string;
  title?: string;
  tone: StatusTone;
}) {
  const Icon = ICONS[tone];
  return (
    <Alert className={className} variant={tone === "critical" ? "destructive" : "default"}>
      <Icon aria-hidden />
      {title === undefined ? null : <AlertTitle>{title}</AlertTitle>}
      <AlertDescription>
        {children}
        {actions === undefined ? null : <div className="mt-2 flex flex-wrap gap-2">{actions}</div>}
      </AlertDescription>
    </Alert>
  );
}
