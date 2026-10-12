import { Callout, PageSpinner } from "@pokedojo/ui";
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

  if (error !== null) return <Callout className="m-4" variant="critical">{error}</Callout>;
  if (firstRun === null) return <PageSpinner />;
  if (firstRun) return <SetupFlow onDone={() => setFirstRun(false)} />;
  return (
    <div className="h-screen">
      <Hub history={history} historyError={historyError} />
    </div>
  );
}
