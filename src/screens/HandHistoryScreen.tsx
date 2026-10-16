import { useEffect, useState } from "react";
import { History, Trophy } from "lucide-react";
import { Link } from "react-router";
import {
  getTableView,
  type TableViewSnapshot,
} from "../api/desktop";
import {
  readDurableHandHistoryV2,
  type PersistedHandHistoryV2,
  type PersistedHistoryEntryV2,
} from "../app/persistence";
import { SectionCard } from "../components/shared/SectionCard";
import { ScreenShell } from "./ScreenShell";
import type { ScreenProps } from "./types";

const HISTORY_PAGE_SIZE = 100;

type HistoryGroup = {
  id: string;
  label: string;
  epoch: number;
  entries: PersistedHistoryEntryV2[];
};

function buildGroups(
  persisted: PersistedHandHistoryV2 | null,
  current: TableViewSnapshot | null,
): HistoryGroup[] {
  const entries = [...(persisted?.entries ?? [])];
  const keyFor = (entry: PersistedHistoryEntryV2) =>
    entry.provenance === "live"
      ? `live:${entry.tableId}:${entry.sessionEpoch}`
      : `${entry.provenance}:${entry.legacyOriginId}`;
  const known = new Set(
    entries.map((entry) => `${keyFor(entry)}:${entry.handNumber}`),
  );
  for (const hand of current?.handHistory ?? []) {
    const entry: PersistedHistoryEntryV2 = {
      ...hand,
      provenance: "live",
      tableId: current?.tableId,
      sessionEpoch: current?.sessionEpoch,
      tournamentName: current?.tournamentName,
    };
    if (!known.has(`${keyFor(entry)}:${hand.handNumber}`)) {
      entries.push(entry);
    }
  }
  const groups = new Map<string, HistoryGroup>();
  for (const entry of entries) {
    const id = keyFor(entry);
    const epoch = entry.provenance === "live" ? (entry.sessionEpoch ?? 0) : 0;
    const label =
      entry.provenance === "live"
        ? entry.tournamentName || entry.tableId || "Tournament"
        : entry.provenance === "importedDurableV1"
          ? "Legacy saved history (unattributed)"
          : "Legacy browser history (unattributed)";
    if (!groups.has(id)) {
      groups.set(id, { id, label, epoch, entries: [] });
    }
    groups.get(id)?.entries.push(entry);
  }
  return [...groups.values()]
    .map((group) => ({
      ...group,
      entries: group.entries.sort(
        (left, right) => right.handNumber - left.handNumber,
      ),
    }))
    .sort(
      (left, right) =>
        right.epoch - left.epoch || left.id.localeCompare(right.id),
    );
}

export function HandHistoryScreen({ bootstrap }: ScreenProps) {
  const [tableView, setTableView] = useState<TableViewSnapshot | null>(null);
  const [persistedHistory, setPersistedHistory] =
    useState<PersistedHandHistoryV2 | null>(null);
  const [selectedGroupId, setSelectedGroupId] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [visibleHistoryCount, setVisibleHistoryCount] =
    useState(HISTORY_PAGE_SIZE);
  const groups = buildGroups(persistedHistory, tableView);
  const activeGroup =
    groups.find((group) => group.id === selectedGroupId) ?? groups[0];
  const historyEntries = activeGroup?.entries ?? [];
  const visibleHistoryEntries = historyEntries.slice(0, visibleHistoryCount);
  const hiddenHistoryCount = Math.max(
    0,
    historyEntries.length - visibleHistoryCount,
  );
  const nextHistoryBatchSize = Math.min(HISTORY_PAGE_SIZE, hiddenHistoryCount);

  useEffect(() => {
    let cancelled = false;

    void readDurableHandHistoryV2(bootstrap.storageNamespace)
      .then((history) => {
        if (!cancelled) {
          setPersistedHistory(history);
          if (history?.entries.length) {
            setError(null);
          }
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
        if (!cancelled) {
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
  }, [bootstrap.storageNamespace]);

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
          {groups.length > 1 ? (
            <div>
              <label htmlFor="hand-history-tournament">Tournament</label>
              <select
                id="hand-history-tournament"
                value={activeGroup?.id ?? ""}
                onChange={(event) => {
                  setSelectedGroupId(event.target.value);
                  setVisibleHistoryCount(HISTORY_PAGE_SIZE);
                }}
              >
                {groups.map((group) => (
                  <option key={group.id} value={group.id}>
                    {group.label} ({group.entries.length} hands)
                  </option>
                ))}
              </select>
            </div>
          ) : null}
          {!tableView?.handHistory.length && historyEntries.length ? (
            <p className="field-hint">Saved on this device.</p>
          ) : null}
          {historyEntries.length ? (
            <p aria-live="polite" className="field-hint" role="status">
              Showing {visibleHistoryEntries.length} of {historyEntries.length}{" "}
              settled hands, newest first in the selected tournament.
            </p>
          ) : null}
          <div className="stacked-list scroll-list">
            {visibleHistoryEntries.length ? (
              visibleHistoryEntries.map((entry) => (
                <article
                  key={`${activeGroup?.id}:${entry.handNumber}`}
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
