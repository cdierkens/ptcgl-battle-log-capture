import { PageSpinner } from "@/components/PageSpinner";
import { StatusAlert } from "@/components/StatusAlert";
import { useEffect, useState } from "react";

import { Hub } from "./components/Hub";
import { SetupFlow } from "./components/SetupFlow";
import { isFirstRun } from "./lib/tauri";
import { useStatusHistory } from "./lib/use-status-history";

export function MainWindow() {
  const [firstRun, setFirstRun] = useState<boolean | null>(null);
  const [error, setError] = useState<null | string>(null);
  const { error: historyError, history } = useStatusHistory();

  useEffect(() => {
    isFirstRun()
      .then(setFirstRun)
      .catch((err: unknown) => setError(String(err)));
  }, []);

  if (error !== null) return <StatusAlert className="m-4" tone="critical">{error}</StatusAlert>;
  if (firstRun === null) return <PageSpinner />;
  if (firstRun) return <SetupFlow onDone={() => setFirstRun(false)} />;
  return (
    <div className="h-screen">
      <Hub history={history} historyError={historyError} />
    </div>
  );
}
