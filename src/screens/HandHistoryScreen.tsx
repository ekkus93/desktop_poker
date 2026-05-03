import { useEffect, useState } from "react";
import { Link } from "react-router-dom";
import { getTableView, type TableViewSnapshot } from "../api/desktop";
import { SectionCard } from "../components/shared/SectionCard";
import { ScreenShell } from "./ScreenShell";
import type { ScreenProps } from "./types";

export function HandHistoryScreen({ bootstrap }: ScreenProps) {
  const [tableView, setTableView] = useState<TableViewSnapshot | null>(null);
  const [error, setError] = useState<string | null>(null);

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
      lead="Settled hands, table events, and current standings now come from the same Rust-backed table shell that powers the main table route."
      badges={[bootstrap.serializationStrategy, tableView?.blindLevelLabel ?? "History"]}
    >
      <div className="content-grid wide-grid">
        <SectionCard kicker="History" title="Settled hands">
          {error ? <p>{error}</p> : null}
          <div className="stacked-list">
            {tableView?.handHistory.length ? (
              tableView.handHistory.map((entry) => (
                <article key={entry.handNumber} className="list-panel history-row">
                  <div>
                    <strong>Hand {entry.handNumber}</strong>
                    <p className="field-hint">{entry.summary}</p>
                    <p className="field-hint">
                      Pot {entry.potTotal} · Winners: {entry.winningPlayers.join(", ")}
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
              <p className="field-hint">No hands have settled yet. Use the main table to progress the current hand.</p>
            )}
          </div>
        </SectionCard>

        <SectionCard kicker="Standings" title="Current order">
          <div className="stacked-list">
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

        <SectionCard kicker="Feed" title="Recent public events">
          <div className="stacked-list">
            {tableView?.eventFeed.map((event) => (
              <article key={event.sequence} className="list-panel history-row">
                <div>
                  <strong>
                    Seq {event.sequence} · {event.kind}
                  </strong>
                  <p className="field-hint">{event.message}</p>
                </div>
              </article>
            ))}
          </div>
          <div className="button-row">
            <Link className="secondary-button compact-button" to="/table">
              Back to table
            </Link>
          </div>
        </SectionCard>
      </div>
    </ScreenShell>
  );
}
