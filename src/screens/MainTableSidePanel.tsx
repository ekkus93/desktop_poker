import { SectionCard } from "../components/shared/SectionCard";
import type { TableViewSnapshot } from "../api/desktop";

const EVENT_FEED_CAP = 50;

export function TableSidePanel({
  tableView,
  displayName,
}: {
  tableView: TableViewSnapshot;
  displayName: string;
}) {
  return (
    <aside className="table-side-panel">
      <SectionCard kicker="Standings" title="Chip order">
        <div className="stacked-list" id="standings-panel">
          {tableView.standings.map((entry) => (
            <article
              key={`${entry.rank}-${entry.displayName}`}
              className="list-panel standings-row"
            >
              <div>
                <strong>
                  #{entry.rank}{" "}
                  {entry.isLocal ? `${displayName} (you)` : entry.displayName}
                </strong>
                <p className="field-hint">
                  {entry.isObserver
                    ? entry.statusLabel
                    : `${entry.chipCount ?? 0} chips`}
                </p>
                {entry.note ? <p className="field-hint">{entry.note}</p> : null}
              </div>
            </article>
          ))}
        </div>
      </SectionCard>

      <SectionCard kicker="Table feed" title="Latest public events">
        <div className="stacked-list event-feed-list">
          {tableView.eventFeed.slice(-EVENT_FEED_CAP).map((event) => (
            <article key={event.sequence} className="list-panel history-row">
              <div>
                <strong>{event.kind}</strong>
                <p className="field-hint">{event.message}</p>
              </div>
            </article>
          ))}
        </div>
        {tableView.eventFeed.length > EVENT_FEED_CAP ? (
          <p className="field-hint">Showing last {EVENT_FEED_CAP} events.</p>
        ) : null}
      </SectionCard>

      <SectionCard kicker="History" title="Latest settled hands">
        <div className="stacked-list">
          {tableView.handHistory.length > 0 ? (
            tableView.handHistory.map((entry) => (
              <article
                key={entry.handNumber}
                className="list-panel history-row"
              >
                <div>
                  <strong>Hand {entry.handNumber}</strong>
                  <p className="field-hint">{entry.summary}</p>
                  <p className="field-hint">
                    Pot {entry.potTotal} · Winners:{" "}
                    {entry.winningPlayers.join(", ")}
                  </p>
                </div>
              </article>
            ))
          ) : (
            <p className="field-hint">No settled hands yet.</p>
          )}
        </div>
      </SectionCard>
    </aside>
  );
}
