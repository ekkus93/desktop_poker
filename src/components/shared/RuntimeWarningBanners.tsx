import { useEffect, useState } from "react";
import { getRuntimeWarnings } from "../../api/runtimeWarnings";

const RUNTIME_WARNING_POLL_MS = 5_000;

function errorMessage(error: unknown) {
  return error instanceof Error ? error.message : String(error);
}

export function RuntimeWarningBanners() {
  const [warnings, setWarnings] = useState<string[]>([]);
  const [statusError, setStatusError] = useState<string | null>(null);

  useEffect(() => {
    let cancelled = false;

    const refresh = async () => {
      try {
        const nextWarnings = await getRuntimeWarnings();
        if (!cancelled) {
          setWarnings(nextWarnings);
          setStatusError(null);
        }
      } catch (error) {
        if (!cancelled) {
          setWarnings([]);
          setStatusError(
            `Runtime health status is unavailable: ${errorMessage(error)}`,
          );
        }
      }
    };

    void refresh();
    const intervalId = window.setInterval(() => {
      void refresh();
    }, RUNTIME_WARNING_POLL_MS);

    return () => {
      cancelled = true;
      window.clearInterval(intervalId);
    };
  }, []);

  return (
    <>
      {statusError ? (
        <div className="inline-banner error" role="alert">
          {statusError}
        </div>
      ) : null}
      {warnings.map((warning) => (
        <div className="inline-banner info" key={warning} role="status">
          {warning}
        </div>
      ))}
    </>
  );
}
