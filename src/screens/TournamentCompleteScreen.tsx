import { useEffect, useState } from "react";
import { Link } from "react-router-dom";
import { getTableView, type TableViewSnapshot } from "../api/desktop";
import { useDesktopShell } from "../app/useDesktopShell";
import { SectionCard } from "../components/shared/SectionCard";
import { ScreenShell } from "./ScreenShell";

export function TournamentCompleteScreen() {
  const { displayName, hostDraft, persistedHandHistoryCount } = useDesktopShell();
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
      lead="Review the finish, check the final order, then choose what to do next."
      badges={[hostDraft.tournamentName]}
    >
      <div className="content-grid">
        <SectionCard kicker="Result" title="Final result">
          {error ? <p className="inline-banner error">{error}</p> : null}
          <p>
            {tableView?.phaseLabel.toLowerCase().includes("complete")
              ? `${tableView.standings[0]?.displayName ?? "The winner"} wins ${hostDraft.tournamentName}.`
              : `${displayName}, the final order will appear here when the tournament closes.`}
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

        <SectionCard title="Next">
          <p>{persistedHandHistoryCount} settled hand summaries are ready to review.</p>
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

        <SectionCard title="Final hands">
          <div className="stacked-list">
            {tableView?.handHistory.length ? (
              tableView.handHistory.slice(0, 3).map((entry) => (
                <article key={entry.handNumber} className="list-panel history-row">
                  <div>
                    <strong>{entry.summary}</strong>
                    <p className="field-hint">Hand {entry.handNumber}</p>
                  </div>
                </article>
              ))
            ) : (
              <p className="field-hint">Final hands will appear here when they are available.</p>
            )}
          </div>
        </SectionCard>
      </div>
    </ScreenShell>
  );
}
