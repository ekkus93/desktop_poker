import { useEffect, useState } from "react";
import { Link } from "react-router";
import { getTableView, type TableViewSnapshot } from "../api/desktop";
import { mergePersistedHandHistory } from "../app/persistence";
import { useDesktopShell } from "../app/useDesktopShell";
import { SectionCard } from "../components/shared/SectionCard";
import { ScreenShell } from "./ScreenShell";

const FINAL_HISTORY_SYNC_ATTEMPTS = 20;
const FINAL_HISTORY_SYNC_DELAY_MS = import.meta.env.MODE === "test" ? 0 : 250;

function containsCurrentHand(snapshot: TableViewSnapshot) {
  return (
    snapshot.currentHandNumber === null ||
    snapshot.handHistory.some(
      (entry) => entry.handNumber === snapshot.currentHandNumber,
    )
  );
}

function waitForNextHistoryAttempt() {
  return new Promise<void>((resolve) => {
    window.setTimeout(resolve, FINAL_HISTORY_SYNC_DELAY_MS);
  });
}

export function TournamentCompleteScreen() {
  const {
    bootstrap,
    hostDraft,
    persistedHandHistoryCount,
    wasHost,
  } = useDesktopShell();
  const [tableView, setTableView] = useState<TableViewSnapshot | null>(null);
  const [savedHandHistoryCount, setSavedHandHistoryCount] = useState(
    persistedHandHistoryCount,
  );
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    let cancelled = false;

    const loadFinalSnapshot = async () => {
      let loadedSnapshot = false;

      for (let attempt = 0; attempt < FINAL_HISTORY_SYNC_ATTEMPTS; attempt += 1) {
        try {
          const snapshot = await getTableView("local");
          if (cancelled) {
            return;
          }

          loadedSnapshot = true;
          setTableView(snapshot);
          setError(null);

          if (snapshot.handHistory.length > 0) {
            const mergedHistory = mergePersistedHandHistory(
              bootstrap.storageNamespace,
              snapshot.handHistory,
            );
            setSavedHandHistoryCount(mergedHistory.entries.length);
          }

          if (
            snapshot.tournamentPhase !== "complete" ||
            containsCurrentHand(snapshot)
          ) {
            return;
          }
        } catch (caughtError: unknown) {
          if (cancelled) {
            return;
          }

          if (attempt === FINAL_HISTORY_SYNC_ATTEMPTS - 1) {
            setError(
              loadedSnapshot
                ? caughtError instanceof Error
                  ? `Final standings loaded, but hand history could not be synchronized: ${caughtError.message}`
                  : "Final standings loaded, but hand history could not be synchronized."
                : caughtError instanceof Error
                  ? caughtError.message
                  : "Unable to load final standings.",
            );
            return;
          }
        }

        await waitForNextHistoryAttempt();
        if (cancelled) {
          return;
        }
      }

      if (!cancelled) {
        setError(
          "Final standings loaded, but the last hand has not arrived for saved history.",
        );
      }
    };

    void loadFinalSnapshot();

    return () => {
      cancelled = true;
    };
  }, [bootstrap.storageNamespace]);

  return (
    <ScreenShell
      title="Tournament Complete"
      badges={[hostDraft.tournamentName]}
      className="support-screen-shell"
    >
      <div className="complete-grid">
        <SectionCard
          kicker="Result"
          title="Final result"
          className="support-card history-primary-card"
        >
          {error ? <p className="inline-banner error">{error}</p> : null}
          <p>
            {tableView?.tournamentPhase === "complete"
              ? tableView.standings.length > 0
                ? `${tableView.standings[0].displayName} wins ${hostDraft.tournamentName}.`
                : "No result available."
              : "Final order appears here when the tournament closes."}
          </p>
          {tableView ? (
            <p className="field-hint">
              {tableView.standings.length} players
              {tableView.currentHandNumber !== null
                ? ` · ${tableView.currentHandNumber} hands played`
                : ""}
            </p>
          ) : null}
          <div className="stacked-list scroll-list">
            {(tableView?.standings.length ? tableView.standings : []).map(
              (entry) => (
                <article
                  key={`${entry.rank}-${entry.displayName}`}
                  className="list-panel history-row"
                >
                  <div>
                    <strong>
                      #{entry.rank} {entry.displayName}
                    </strong>
                    <p className="field-hint">
                      {entry.isObserver
                        ? entry.statusLabel
                        : `${entry.chipCount ?? 0} chips`}
                    </p>
                    {entry.note ? (
                      <p className="field-hint">{entry.note}</p>
                    ) : null}
                  </div>
                </article>
              ),
            )}
          </div>
        </SectionCard>

        <div className="history-side-stack">
          <SectionCard title="Next" className="support-card">
            <p>{savedHandHistoryCount} hand summaries saved.</p>
            <div className="button-row">
              {wasHost ? (
                <Link className="primary-button" to="/host">
                  Host another table
                </Link>
              ) : (
                <Link className="primary-button" to="/join">
                  Join another table
                </Link>
              )}
              <Link className="secondary-button" to="/history">
                Review history
              </Link>
              <Link className="secondary-button" to="/">
                Return home
              </Link>
            </div>
          </SectionCard>

          <SectionCard title="Final hands" className="support-card">
            <div className="stacked-list scroll-list">
              {tableView?.handHistory.length ? (
                tableView.handHistory.slice(0, 3).map((entry) => (
                  <article
                    key={entry.handNumber}
                    className="list-panel history-row"
                  >
                    <div>
                      <strong>{entry.summary}</strong>
                      <p className="field-hint">Hand {entry.handNumber}</p>
                    </div>
                  </article>
                ))
              ) : (
                <p className="field-hint">No final hands yet.</p>
              )}
            </div>
          </SectionCard>
        </div>
      </div>
    </ScreenShell>
  );
}
