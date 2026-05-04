import { useEffect, useState } from "react";
import { Link } from "react-router-dom";
import { getTableView, type TableViewSnapshot } from "../api/desktop";
import { useDesktopShell } from "../app/useDesktopShell";
import { SectionCard } from "../components/shared/SectionCard";
import { ScreenShell } from "./ScreenShell";
import type { ScreenProps } from "./types";

export function TournamentCompleteScreen({ bootstrap }: ScreenProps) {
  const { displayName, hostDraft, recentJoinPayloads, persistedHandHistoryCount } = useDesktopShell();
  const [tableView, setTableView] = useState<TableViewSnapshot | null>(null);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    let cancelled = false;

    void getTableView("local")
      .then((snapshot) => {
        if (!cancelled) {
          setTableView(snapshot);
          setError(null);
        }
      })
      .catch((caughtError: unknown) => {
        if (!cancelled) {
          setError(caughtError instanceof Error ? caughtError.message : "Unable to load final standings.");
        }
      });

    return () => {
      cancelled = true;
    };
  }, []);

  return (
    <ScreenShell
      title="Tournament Complete"
      lead="Wrap up the session, check the final order, then decide whether to host again, review history, or return home."
      badges={[hostDraft.tournamentName, `Profile ${bootstrap.instanceId}`]}
    >
      <div className="content-grid">
        <SectionCard kicker="Result" title="Session summary">
          {error ? <p className="inline-banner error">{error}</p> : null}
          <p>
            {tableView?.phaseLabel.toLowerCase().includes("complete")
              ? "The tournament is complete. Review the final order below."
              : `${displayName}, this screen is ready to show the final order when the backend marks the tournament complete.`}
          </p>
          <div className="stacked-list">
            {(tableView?.standings.length ? tableView.standings : []).map((entry) => (
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

        <SectionCard title="What stays on this device">
          <ul>
            <li>Last host settings for {hostDraft.tournamentName}</li>
            <li>{recentJoinPayloads.length} remembered join payloads</li>
            <li>{persistedHandHistoryCount} saved hand summaries</li>
            <li>Per-instance profile isolation for reconnect-safe state</li>
          </ul>
          <div className="button-row">
            <Link className="primary-button" to="/host">
              Host again
            </Link>
            <Link className="secondary-button" to="/history">
              Review history
            </Link>
            <Link className="secondary-button" to="/">
              Return home
            </Link>
          </div>
        </SectionCard>

        <SectionCard title="Recent deciding hands">
          <div className="stacked-list">
            {tableView?.handHistory.length ? (
              tableView.handHistory.slice(0, 3).map((entry) => (
                <article key={entry.handNumber} className="list-panel history-row">
                  <div>
                    <strong>Hand {entry.handNumber}</strong>
                    <p className="field-hint">{entry.summary}</p>
                  </div>
                </article>
              ))
            ) : (
              <p className="field-hint">The final hand list will appear here when the backend publishes it.</p>
            )}
          </div>
        </SectionCard>
      </div>
    </ScreenShell>
  );
}
