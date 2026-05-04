import { useEffect, useMemo, useState } from "react";
import { History, PanelRight, Trophy } from "lucide-react";
import { Link } from "react-router-dom";
import {
  getTableView,
  submitTableAction,
  type DesktopTableActionKind,
  type TableCardView,
  type TableViewSnapshot,
  type TableViewerMode,
} from "../api/desktop";
import { useDesktopShell } from "../app/useDesktopShell";
import { SectionCard } from "../components/shared/SectionCard";
import { StatusBadge } from "../components/shared/StatusBadge";
import { ScreenShell } from "./ScreenShell";
import type { ScreenProps } from "./types";

const BOARD_SLOT_COUNT = 5;

export function MainTableScreen({ bootstrap }: ScreenProps) {
  const { displayName, persistHandHistory } = useDesktopShell();
  const [viewerMode, setViewerMode] = useState<TableViewerMode>("local");
  const [tableView, setTableView] = useState<TableViewSnapshot | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [actionError, setActionError] = useState<string | null>(null);
  const [submitting, setSubmitting] = useState(false);
  const [showSidePanel, setShowSidePanel] = useState(false);
  const [openSeatIndex, setOpenSeatIndex] = useState<number | null>(null);
  const [raiseAmount, setRaiseAmount] = useState<number | null>(null);
  const [confirmation, setConfirmation] = useState<
    { actionKind: "betOrRaise" | "allIn"; label: string } | null
  >(null);

  useEffect(() => {
    let cancelled = false;
    setLoading(true);
    setError(null);
    setActionError(null);
    setConfirmation(null);

    void getTableView(viewerMode)
      .then((snapshot) => {
        if (!cancelled) {
          setTableView(snapshot);
        }
      })
      .catch((caughtError: unknown) => {
        if (!cancelled) {
          setError(getErrorMessage(caughtError));
          setTableView(null);
        }
      })
      .finally(() => {
        if (!cancelled) {
          setLoading(false);
        }
      });

    return () => {
      cancelled = true;
    };
  }, [viewerMode]);

  useEffect(() => {
    const actionTray = tableView?.actionTray;
    if (!actionTray) {
      setRaiseAmount(null);
      return;
    }

    setRaiseAmount((currentAmount) => {
      if (currentAmount !== null && isWithinRaiseBounds(currentAmount, actionTray)) {
        return currentAmount;
      }

      return defaultRaiseAmount(actionTray);
    });
  }, [tableView]);

  useEffect(() => {
    if (!tableView?.handHistory.length) {
      return;
    }

    persistHandHistory(tableView.handHistory);
  }, [persistHandHistory, tableView]);

  const boardCards = useMemo(() => {
    const visibleCards = tableView?.boardCards ?? [];
    return Array.from({ length: BOARD_SLOT_COUNT }, (_, index) => visibleCards[index] ?? null);
  }, [tableView]);

  const quickSizes = useMemo(() => buildQuickSizes(tableView?.actionTray), [tableView]);
  const activeViewerMode = tableView?.viewerMode ?? viewerMode;

  const submitAction = async (
    actionKind: DesktopTableActionKind,
    nextRaiseAmount?: number,
  ) => {
    setSubmitting(true);
    setActionError(null);
    try {
      const snapshot = await submitTableAction(viewerMode, actionKind, nextRaiseAmount);
      setTableView(snapshot);
      setConfirmation(null);
    } catch (caughtError) {
      setActionError(getErrorMessage(caughtError));
    } finally {
      setSubmitting(false);
    }
  };

  const queueConfirmation = (actionKind: "betOrRaise" | "allIn", label: string) => {
    setConfirmation({ actionKind, label });
    setActionError(null);
  };

  const confirmQueuedAction = async () => {
    if (!confirmation || !tableView?.actionTray) {
      return;
    }

    const nextRaiseAmount =
      confirmation.actionKind === "betOrRaise" ? raiseAmount ?? undefined : undefined;
    await submitAction(confirmation.actionKind, nextRaiseAmount);
  };

  const handlePrimaryAction = async (actionKind: DesktopTableActionKind) => {
    if (actionKind === "betOrRaise") {
      if (!tableView?.actionTray) {
        return;
      }

      await submitAction(actionKind, raiseAmount ?? defaultRaiseAmount(tableView.actionTray));
      return;
    }

    if (actionKind === "allIn") {
      queueConfirmation("allIn", "Confirm all-in");
      return;
    }

    await submitAction(actionKind);
  };

  return (
    <ScreenShell
      title="Main Table"
      lead="See the board, the pot, and whose turn it is at a glance."
      badges={[displayName, tableView?.blindLevelLabel ?? "Loading table"]}
    >
      <div className="table-screen-shell">
        <div className="table-top-row">
          <div className="button-row">
            {bootstrap.debugToolsEnabled ? (
              <>
                <button
                  className={viewerMode === "local" ? "primary-button compact-button" : "secondary-button compact-button"}
                  onClick={() => setViewerMode("local")}
                  type="button"
                >
                  Player preview
                </button>
                <button
                  className={viewerMode === "observer" ? "primary-button compact-button" : "secondary-button compact-button"}
                  onClick={() => setViewerMode("observer")}
                  type="button"
                >
                  Observer preview
                </button>
              </>
            ) : null}
            <button
              className="secondary-button compact-button"
              onClick={() => setShowSidePanel((current) => !current)}
              type="button"
            >
              <span className="button-content">
                <PanelRight className="button-icon" strokeWidth={1.9} />
                <span>{showSidePanel ? "Hide table details" : "Table details"}</span>
              </span>
            </button>
          </div>
          <div className="button-row">
            <Link className="secondary-button compact-button" to="/history">
              <span className="button-content">
                <History className="button-icon" strokeWidth={1.9} />
                <span>Hand history</span>
              </span>
            </Link>
            {tableView?.phaseLabel.toLowerCase().includes("complete") ? (
              <Link className="secondary-button compact-button" to="/complete">
                <span className="button-content">
                  <Trophy className="button-icon" strokeWidth={1.9} />
                  <span>Tournament complete</span>
                </span>
              </Link>
            ) : null}
          </div>
        </div>

        {loading ? (
          <SectionCard title="Loading table state">
            <p className="field-hint">Getting the latest table state.</p>
          </SectionCard>
        ) : null}

        {error ? (
          <SectionCard title="Table unavailable">
            <p>{error}</p>
            <div className="button-row">
              <Link className="primary-button" to="/lobby">
                Return to lobby
              </Link>
              <Link className="secondary-button" to="/history">
                Open history
              </Link>
            </div>
          </SectionCard>
        ) : null}

        {tableView ? (
          <div className={`desktop-table-layout ${showSidePanel ? "with-side-panel" : ""}`}>
            <section className="table-surface enhanced-table-surface">
              <div className={`table-headline-card ${tableView.actionTray ? "is-active-turn" : "is-waiting"}`}>
                <p className="kicker">{activeViewerMode === "observer" ? "Observing" : tableView.actionTray ? "Your move" : "Waiting"}</p>
                <h3>
                  {activeViewerMode === "observer"
                    ? "Watching the public table."
                    : tableView.actionTray
                      ? `${tableView.actionTray.ownerLabel} to act.`
                      : `${tableView.actionOwnerLabel} to act.`}
                </h3>
                <p className="field-hint">
                  {activeViewerMode === "observer"
                    ? "Private cards and action controls stay hidden while you watch."
                    : tableView.actionTray
                      ? `Call ${tableView.actionTray.callAmount} chips, or choose another legal action below.`
                      : "The table stays live while you wait."}
                </p>
              </div>

              <header className="table-surface-header">
                <div>
                  <p className="kicker">{tableView.tableName}</p>
                  <h3>{tableView.tournamentName}</h3>
                  <p className="field-hint">
                    Hand {tableView.currentHandNumber ?? "—"} · {tableView.phaseLabel}
                  </p>
                </div>
                <div className="badge-row">
                  <StatusBadge tone="success">{tableView.streetLabel}</StatusBadge>
                  <StatusBadge tone="info">{tableView.blindLevelLabel}</StatusBadge>
                  <StatusBadge tone="warning">Pot {tableView.potTotal}</StatusBadge>
                </div>
              </header>

              {tableView.observerBanner ? (
                <div className="inline-banner info observer-banner">
                  <strong>Observer mode</strong>
                  <span>{tableView.observerBanner}</span>
                </div>
              ) : null}

              <div className="table-summary-row">
                <div className="pot-summary-card">
                  <span className="surface-label">Pot</span>
                  <strong>{tableView.potTotal} chips</strong>
                </div>
                <div className="pot-summary-card action-owner-card">
                  <span className="surface-label">Acting player</span>
                  <strong>{tableView.actionOwnerLabel}</strong>
                </div>
                <div className="pot-summary-card elimination-card">
                  <span className="surface-label">Latest outcome</span>
                  <strong>{tableView.eliminationSummary}</strong>
                </div>
              </div>

              <div className="community-cards centered-community-cards" aria-label="Community cards">
                {boardCards.map((card, index) => (
                  <PlayingCard key={index} card={card} placeholderLabel={`Board ${index + 1}`} size="board" />
                ))}
              </div>

              <div className="seat-grid enhanced-seat-grid">
                {tableView.seats.map((seat) => {
                  const seatLabel = seat.isLocal && activeViewerMode === "local" ? `${displayName} (you)` : seat.displayName;
                  const isOpen = openSeatIndex === seat.seatIndex;
                  return (
                    <article
                      key={seat.seatIndex}
                      className={`seat-card enhanced-seat-card ${seat.isLocal ? "local-seat" : ""} ${seat.isActing ? "action-seat" : ""} ${seat.isObserver ? "observer-seat" : ""} ${seat.isEliminated ? "eliminated-seat" : ""} ${seat.isCompact ? "compact-seat" : ""}`}
                    >
                      <div className="seat-card-header">
                        <div>
                          <strong>Seat {seat.seatIndex}</strong>
                          <span>{seatLabel}</span>
                        </div>
                        {seat.markerLabel ? <StatusBadge>{seat.markerLabel}</StatusBadge> : null}
                      </div>
                      <p className="seat-status-line">
                        <span>{seat.statusLabel}</span>
                        {seat.chipCount !== null ? <span>{seat.chipCount} chips</span> : null}
                      </p>
                      <p className="seat-detail">
                        {seat.isObserver ? "Watching public action" : seat.isEliminated ? "Eliminated from this hand" : `Contribution ${seat.contribution}`}
                      </p>
                      <div className={`seat-card-grid ${seat.isLocal ? "show-local-cards" : ""}`}>
                        {seat.cardsHidden ? (
                          <div className="hidden-card-row" aria-label={`Seat ${seat.seatIndex} hidden cards`}>
                            <HiddenCard />
                            <HiddenCard />
                          </div>
                        ) : (
                          <div className="seat-hole-cards" aria-label={`Seat ${seat.seatIndex} hole cards`}>
                            {seat.holeCards.map((card) => (
                              <PlayingCard key={card.compactLabel} card={card} size={seat.isLocal ? "local" : "compact"} />
                            ))}
                          </div>
                        )}
                      </div>
                      <button
                        className="secondary-button compact-button"
                        onClick={() => setOpenSeatIndex(isOpen ? null : seat.seatIndex)}
                        type="button"
                      >
                        {isOpen ? "Hide" : "Seat details"}
                      </button>
                      {isOpen ? (
                        <div className="seat-popover" role="note">
                          {seat.detailLines.map((line) => (
                            <p key={line} className="field-hint">
                              {line}
                            </p>
                          ))}
                        </div>
                      ) : null}
                    </article>
                  );
                })}
              </div>
            </section>

            {showSidePanel ? (
              <aside className="table-side-panel">
                <SectionCard kicker="Standings" title="Current order">
                  <div className="stacked-list" id="standings-panel">
                    {tableView.standings.map((entry) => (
                      <article key={`${entry.rank}-${entry.displayName}`} className="list-panel standings-row">
                        <div>
                          <strong>
                            #{entry.rank} {entry.isLocal && activeViewerMode === "local" ? `${displayName} (you)` : entry.displayName}
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

                <SectionCard kicker="Table events" title="Recent public events">
                  <div className="stacked-list event-feed-list">
                    {tableView.eventFeed.map((event) => (
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
                </SectionCard>

                <SectionCard kicker="History" title="Latest settled hands">
                  <div className="stacked-list">
                    {tableView.handHistory.length > 0 ? (
                      tableView.handHistory.map((entry) => (
                        <article key={entry.handNumber} className="list-panel history-row">
                          <div>
                            <strong>Hand {entry.handNumber}</strong>
                            <p className="field-hint">{entry.summary}</p>
                            <p className="field-hint">Pot {entry.potTotal} · Winners: {entry.winningPlayers.join(", ")}</p>
                          </div>
                        </article>
                      ))
                    ) : (
                      <p className="field-hint">No hands have settled yet.</p>
                    )}
                  </div>
                </SectionCard>
              </aside>
            ) : null}
          </div>
        ) : null}

        {tableView?.actionTray ? (
          <SectionCard kicker="Action" title="Act from your seat">
            <div className="action-owner-banner">
              <strong>{tableView.actionTray.ownerLabel}</strong>
              <span className="field-hint">Choose one action now.</span>
            </div>
            <div className="button-row action-tray-row">
              <button
                className="secondary-button"
                disabled={submitting}
                onClick={() => void handlePrimaryAction("fold")}
                type="button"
              >
                Fold
              </button>
              <button
                className="primary-button"
                disabled={submitting}
                onClick={() => void handlePrimaryAction("checkOrCall")}
                type="button"
              >
                {tableView.actionTray.checkOrCallLabel}
              </button>
              <button
                className="secondary-button"
                disabled={submitting || tableView.actionTray.maxRaiseTo === null}
                onClick={() => void handlePrimaryAction("betOrRaise")}
                type="button"
              >
                {tableView.actionTray.betOrRaiseLabel}
              </button>
              <button
                className="secondary-button"
                disabled={submitting}
                onClick={() => void handlePrimaryAction("allIn")}
                type="button"
              >
                All-in
              </button>
            </div>
            {tableView.actionTray.maxRaiseTo === null ? (
              <p className="field-hint">Raise opens when the next legal raise becomes available.</p>
            ) : null}
            <div className="raise-controls">
              <label className="field" htmlFor="raise-slider">
                <span>Raise size</span>
                <input
                  id="raise-slider"
                  max={tableView.actionTray.maxRaiseTo ?? 0}
                  min={tableView.actionTray.minRaiseTo ?? 0}
                  onChange={(event) => setRaiseAmount(Number(event.target.value))}
                  step={1}
                  type="range"
                  value={raiseAmount ?? defaultRaiseAmount(tableView.actionTray)}
                />
              </label>
              <p className="field-hint">
                Raise to <strong>{raiseAmount ?? defaultRaiseAmount(tableView.actionTray)}</strong> · Bet {tableView.actionTray.currentBet} · Pot {tableView.actionTray.potTotal}
              </p>
              <div className="button-row quick-size-row">
                {quickSizes.map((option) => (
                  <button
                    key={option.label}
                    className="secondary-button compact-button"
                    onClick={() => setRaiseAmount(option.amount)}
                    type="button"
                  >
                    {option.label}
                  </button>
                ))}
              </div>
            </div>
            {confirmation ? (
              <div className="confirmation-card" role="status">
                <strong>{confirmation.label}</strong>
                <p className="field-hint">
                  {confirmation.actionKind === "betOrRaise"
                    ? `Send a raise to ${raiseAmount ?? defaultRaiseAmount(tableView.actionTray)} chips?`
                    : "Commit the remaining stack as an all-in action?"}
                </p>
                <div className="button-row">
                  <button className="primary-button" disabled={submitting} onClick={() => void confirmQueuedAction()} type="button">
                    Confirm
                  </button>
                  <button className="secondary-button" disabled={submitting} onClick={() => setConfirmation(null)} type="button">
                    Cancel
                  </button>
                </div>
              </div>
            ) : null}
            {actionError ? <div className="inline-banner error">{actionError}</div> : null}
          </SectionCard>
        ) : tableView ? (
          <SectionCard kicker="Action" title="No action required">
            <p>
              {activeViewerMode === "observer"
                ? "Observer preview shows the public table only."
                : "Waiting for the next action."}
            </p>
          </SectionCard>
        ) : null}
      </div>
    </ScreenShell>
  );
}

type PlayingCardProps = {
  card: TableCardView | null;
  placeholderLabel?: string;
  size: "board" | "local" | "compact";
};

function PlayingCard({ card, placeholderLabel, size }: PlayingCardProps) {
  if (!card) {
    return <div className={`card-slot ${size}`}>{placeholderLabel ?? "Waiting"}</div>;
  }

  const cornerRank = card.compactLabel.replace(card.suitSymbol, "");

  return (
    <div className={`playing-card ${size} ${card.tone}`} aria-label={card.label}>
      <div className="playing-card-corner top-left">
        <span>{card.compactLabel}</span>
        <small>{card.suitSymbol}</small>
      </div>
      <div className="playing-card-center" aria-hidden="true">
        <span>{card.suitSymbol}</span>
      </div>
      <div className="playing-card-corner bottom-right">
        <span>{cornerRank}</span>
        <small>{card.suitSymbol}</small>
      </div>
    </div>
  );
}

function HiddenCard() {
  return <div className="hidden-card" aria-hidden="true" />;
}

function buildQuickSizes(actionTray: TableViewSnapshot["actionTray"] | undefined) {
  if (!actionTray || actionTray.minRaiseTo === null || actionTray.maxRaiseTo === null) {
    return [];
  }

  return [
    { label: "Min", amount: clampRaiseAmount(actionTray.minRaiseTo, actionTray) },
    {
      label: "1/2 Pot",
      amount: clampRaiseAmount(Math.round(actionTray.potTotal / 2), actionTray),
    },
    { label: "Pot", amount: clampRaiseAmount(actionTray.potTotal, actionTray) },
    { label: "Max", amount: clampRaiseAmount(actionTray.maxRaiseTo, actionTray) },
  ];
}

function clampRaiseAmount(amount: number, actionTray: NonNullable<TableViewSnapshot["actionTray"]>) {
  if (actionTray.minRaiseTo === null || actionTray.maxRaiseTo === null) {
    return amount;
  }

  return Math.min(actionTray.maxRaiseTo, Math.max(actionTray.minRaiseTo, amount));
}

function defaultRaiseAmount(actionTray: NonNullable<TableViewSnapshot["actionTray"]>) {
  return actionTray.minRaiseTo ?? actionTray.maxRaiseTo ?? actionTray.currentBet;
}

function isWithinRaiseBounds(
  amount: number,
  actionTray: NonNullable<TableViewSnapshot["actionTray"]>,
) {
  if (actionTray.minRaiseTo === null || actionTray.maxRaiseTo === null) {
    return false;
  }

  return amount >= actionTray.minRaiseTo && amount <= actionTray.maxRaiseTo;
}

function getErrorMessage(caughtError: unknown) {
  return caughtError instanceof Error ? caughtError.message : "Unknown table error";
}
