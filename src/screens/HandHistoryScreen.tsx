import { useEffect, useState } from "react";
import { History, Trophy } from "lucide-react";
import { Link } from "react-router-dom";
import { getTableView, type TableViewSnapshot } from "../api/desktop";
import { readPersistedHandHistory } from "../app/persistence";
import { SectionCard } from "../components/shared/SectionCard";
import { ScreenShell } from "./ScreenShell";
import type { ScreenProps } from "./types";

export function HandHistoryScreen({ bootstrap }: ScreenProps) {
  const [tableView, setTableView] = useState<TableViewSnapshot | null>(null);
  const [error, setError] = useState<string | null>(null);
  const persistedHistory = readPersistedHandHistory(bootstrap.storageNamespace);
  const historyEntries = tableView?.handHistory.length
    ? tableView.handHistory
    : (persistedHistory?.entries ?? []);

  useEffect(() => {
    let cancelled = false;
    void getTableView("local")
      .then((snapshot) => {
        if (!cancelled) {
          setTableView(snapshot);
        }
      })
      .catch((caughtError: unknown) => {
        if (!cancelled) {
          setError(caughtError instanceof Error ? caughtError.message : "Unknown history error");
        }
      });

    return () => {
      cancelled = true;
    };
  }, []);

  return (
    <ScreenShell
      title="Hand History"
      badges={[tableView?.blindLevelLabel ?? "History"]}
      className="support-screen-shell"
    >
      <div className="history-station-layout">
        <SectionCard kicker="History" title="Recent hands" className="support-card history-primary-card">
          {error ? <p>{error}</p> : null}
          {!tableView?.handHistory.length && persistedHistory?.entries.length ? (
            <p className="field-hint">
              Saved on this device.
            </p>
          ) : null}
          <div className="stacked-list scroll-list">
            {historyEntries.length ? (
              historyEntries.map((entry) => (
                <article key={entry.handNumber} className="list-panel history-row">
                  <div>
                    <strong>
                      <span className="button-content">
                        <Trophy className="button-icon" strokeWidth={1.9} />
                        <span>{entry.summary}</span>
                      </span>
                    </strong>
                    <p className="field-hint">
                      Hand {entry.handNumber} · Pot {entry.potTotal} · Winners: {entry.winningPlayers.join(", ")}
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
          <div className="button-row">
            <Link className="secondary-button compact-button" to="/table">
              <span className="button-content">
                <History className="button-icon" strokeWidth={1.9} />
                <span>Table</span>
              </span>
            </Link>
          </div>
        </SectionCard>

        <div className="history-side-stack">
          <SectionCard kicker="Standings" title="Order" className="support-card">
            <div className="stacked-list scroll-list">
              {tableView?.standings.map((entry) => (
                <article key={`${entry.rank}-${entry.displayName}`} className="list-panel history-row">
                  <div>
                    <strong>
                      #{entry.rank} {entry.displayName}
                    </strong>
                    <p className="field-hint">
                      {entry.chipCount ?? 0} chips · {entry.statusLabel}
                    </p>
                    {entry.note ? <p className="field-hint">{entry.note}</p> : null}
                  </div>
                </article>
              ))}
            </div>
          </SectionCard>

          <SectionCard kicker="Feed" title="Events" className="support-card">
            <div className="stacked-list scroll-list">
              {tableView?.eventFeed.map((event) => (
                <article key={event.sequence} className="list-panel history-row">
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
