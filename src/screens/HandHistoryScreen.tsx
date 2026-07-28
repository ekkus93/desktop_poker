import { useEffect, useState } from "react";
import { History, Trophy } from "lucide-react";
import { Link } from "react-router";
import { getTableView, type TableViewSnapshot } from "../api/desktop";
import {
  readDurableHandHistory,
  readPersistedHandHistory,
  type PersistedHandHistory,
} from "../app/persistence";
import { SectionCard } from "../components/shared/SectionCard";
import { ScreenShell } from "./ScreenShell";
import type { ScreenProps } from "./types";

const HISTORY_PAGE_SIZE = 100;

export function HandHistoryScreen({ bootstrap }: ScreenProps) {
  const [tableView, setTableView] = useState<TableViewSnapshot | null>(null);
  const [persistedHistory, setPersistedHistory] =
    useState<PersistedHandHistory | null>(() =>
      readPersistedHandHistory(bootstrap.storageNamespace),
    );
  const [error, setError] = useState<string | null>(null);
  const [visibleHistoryCount, setVisibleHistoryCount] =
    useState(HISTORY_PAGE_SIZE);
  const historyEntries = tableView?.handHistory.length
    ? tableView.handHistory
    : (persistedHistory?.entries ?? []);
  const firstVisibleHistoryIndex = Math.max(
    0,
    historyEntries.length - visibleHistoryCount,
  );
  const visibleHistoryEntries = historyEntries.slice(firstVisibleHistoryIndex);
  const hiddenHistoryCount = firstVisibleHistoryIndex;
  const nextHistoryBatchSize = Math.min(HISTORY_PAGE_SIZE, hiddenHistoryCount);

  useEffect(() => {
    let cancelled = false;

    void readDurableHandHistory(bootstrap.storageNamespace)
      .then((history) => {
        if (!cancelled) {
          setPersistedHistory(history);
        }
      })
      .catch((caughtError: unknown) => {
        if (!cancelled) {
          setError(
            caughtError instanceof Error
              ? caughtError.message
              : "Saved history could not be loaded.",
          );
        }
      });

    void getTableView("local")
      .then((snapshot) => {
        if (!cancelled) {
          setTableView(snapshot);
        }
      })
      .catch((caughtError: unknown) => {
        if (!cancelled && !persistedHistory?.entries.length) {
          setError(
            caughtError instanceof Error
              ? caughtError.message
              : "Unknown history error",
          );
        }
      });

    return () => {
      cancelled = true;
    };
  }, [bootstrap.storageNamespace, persistedHistory?.entries.length]);

  return (
    <ScreenShell
      title="Hand History"
      badges={[tableView?.blindLevelLabel ?? "History"]}
      className="support-screen-shell"
    >
      <div className="history-station-layout">
        <SectionCard
          kicker="History"
          title="Recent hands"
          className="support-card history-primary-card"
        >
          {error ? <p role="alert">{error}</p> : null}
          {!tableView?.handHistory.length && historyEntries.length ? (
            <p className="field-hint">Saved on this device.</p>
          ) : null}
          {historyEntries.length ? (
            <p aria-live="polite" className="field-hint" role="status">
              Showing {visibleHistoryEntries.length} of {historyEntries.length}{" "}
              settled hands, newest first in the retained range.
            </p>
          ) : null}
          <div className="stacked-list scroll-list">
            {visibleHistoryEntries.length ? (
              visibleHistoryEntries.map((entry) => (
                <article
                  key={entry.handNumber}
                  className="list-panel history-row"
                >
                  <div>
                    <strong>
                      <span className="button-content">
                        <Trophy
                          aria-hidden="true"
                          className="button-icon"
                          strokeWidth={1.9}
                        />
                        <span>{entry.summary}</span>
                      </span>
                    </strong>
                    <p className="field-hint">
                      Hand {entry.handNumber} · Pot {entry.potTotal} · Winners:{" "}
                      {entry.winningPlayers.join(", ")}
                    </p>
                    {entry.eliminatedPlayers.length ? (
                      <p className="field-hint">
                        Eliminated: {entry.eliminatedPlayers.join(", ")}
                      </p>
                    ) : null}
                  </div>
                </article>
              ))
            ) : (
              <p className="field-hint">No settled hands yet.</p>
            )}
          </div>
          {hiddenHistoryCount > 0 ? (
            <div className="button-row">
              <button
                className="secondary-button compact-button"
                onClick={() => {
                  setVisibleHistoryCount(
                    (current) => current + HISTORY_PAGE_SIZE,
                  );
                }}
                type="button"
              >
                Show {nextHistoryBatchSize} older hand
                {nextHistoryBatchSize === 1 ? "" : "s"}
              </button>
            </div>
          ) : null}
          <div className="button-row">
            <Link className="secondary-button compact-button" to="/table">
              <span className="button-content">
                <History
                  aria-hidden="true"
                  className="button-icon"
                  strokeWidth={1.9}
                />
                <span>Table</span>
              </span>
            </Link>
          </div>
        </SectionCard>

        <div className="history-side-stack">
          <SectionCard
            kicker="Standings"
            title="Order"
            className="support-card"
          >
            <div className="stacked-list scroll-list">
              {tableView?.standings.map((entry) => (
                <article
                  key={`${entry.rank}-${entry.displayName}`}
                  className="list-panel history-row"
                >
                  <div>
                    <strong>
                      #{entry.rank} {entry.displayName}
                    </strong>
                    <p className="field-hint">
                      {entry.chipCount ?? 0} chips · {entry.statusLabel}
                    </p>
                    {entry.note ? (
                      <p className="field-hint">{entry.note}</p>
                    ) : null}
                  </div>
                </article>
              ))}
            </div>
          </SectionCard>

          <SectionCard kicker="Feed" title="Events" className="support-card">
            <div className="stacked-list scroll-list">
              {tableView?.eventFeed.map((event) => (
                <article
                  key={event.sequence}
                  className="list-panel history-row"
                >
                  <div>
                    <strong>{event.message}</strong>
                    <p className="field-hint">{event.kind}</p>
                  </div>
                </article>
              ))}
            </div>
          </SectionCard>
        </div>
      </div>
    </ScreenShell>
  );
}
