import { useEffect, useMemo, useRef, useState } from "react";
import { History, PanelRight, Trophy, WifiOff } from "lucide-react";
import { Link, useNavigate } from "react-router-dom";
import {
  getTableView,
  onTableUpdate,
  submitTableAction,
  type DesktopTableActionKind,
  type TableViewSnapshot,
  type TableViewerMode,
} from "../api/desktop";
import { useDesktopShell } from "../app/useDesktopShell";
import { SectionCard } from "../components/shared/SectionCard";
import { StatusBadge } from "../components/shared/StatusBadge";
import { ScreenShell } from "./ScreenShell";
import { PlayingCard, SeatCard } from "./MainTableCards";
import { TableSidePanel } from "./MainTableSidePanel";
import {
  buildQuickSizes,
  defaultRaiseAmount,
  getErrorMessage,
  isWithinRaiseBounds,
} from "./mainTableRaise";
import type { ScreenProps } from "./types";

const BOARD_SLOT_COUNT = 5;
const TABLE_POLL_NORMAL_MS = 5000;
const TABLE_POLL_SLOW_MS = 10000;
const POLL_BACKOFF_THRESHOLD = 3;
const POLL_ERROR_LIMIT = 10;

export function MainTableScreen({ bootstrap }: ScreenProps) {
  void bootstrap;
  const {
    displayName,
    persistHandHistory,
    tableSidePanelOpen,
    setTableSidePanelOpen,
  } = useDesktopShell();
  const navigate = useNavigate();
  const viewerMode: TableViewerMode = "local";
  const [tableView, setTableView] = useState<TableViewSnapshot | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [connectionSlow, setConnectionSlow] = useState(false);
  const [actionError, setActionError] = useState<string | null>(null);
  const [submitting, setSubmitting] = useState(false);
  const [raiseAmount, setRaiseAmount] = useState<number | null>(null);
  const [confirmation, setConfirmation] = useState<{
    actionKind: "betOrRaise" | "allIn";
    label: string;
  } | null>(null);
  const consecutiveErrorsRef = useRef(0);

  useEffect(() => {
    let cancelled = false;
    let timeoutId: number | undefined;

    const scheduleNext = (delayMs: number) => {
      if (cancelled) {
        return;
      }

      timeoutId = window.setTimeout(() => {
        void poll();
      }, delayMs);
    };

    const poll = async () => {
      try {
        const snapshot = await getTableView(viewerMode);
        if (cancelled) {
          return;
        }

        consecutiveErrorsRef.current = 0;
        setTableView(snapshot);
        setError(null);
        setConnectionSlow(false);
        scheduleNext(TABLE_POLL_NORMAL_MS);
      } catch (caughtError: unknown) {
        if (cancelled) {
          return;
        }

        const msg = getErrorMessage(caughtError).toLowerCase();
        if (msg.includes("disconnected from host")) {
          navigate("/errors", { replace: true });
          return;
        }

        consecutiveErrorsRef.current += 1;
        if (consecutiveErrorsRef.current >= POLL_ERROR_LIMIT) {
          navigate("/errors", { replace: true });
          return;
        }

        setConnectionSlow(
          consecutiveErrorsRef.current >= POLL_BACKOFF_THRESHOLD,
        );
        scheduleNext(
          consecutiveErrorsRef.current >= POLL_BACKOFF_THRESHOLD
            ? TABLE_POLL_SLOW_MS
            : TABLE_POLL_NORMAL_MS,
        );
      }
    };

    const loadInitial = async () => {
      setLoading(true);
      setError(null);
      setActionError(null);
      setConfirmation(null);

      try {
        const snapshot = await getTableView(viewerMode);
        if (!cancelled) {
          consecutiveErrorsRef.current = 0;
          setTableView(snapshot);
          setError(null);
          setConnectionSlow(false);
        }
      } catch (caughtError: unknown) {
        if (!cancelled) {
          setError(getErrorMessage(caughtError));
          setTableView(null);
        }
      } finally {
        if (!cancelled) {
          setLoading(false);
          scheduleNext(TABLE_POLL_NORMAL_MS);
        }
      }
    };

    void loadInitial();

    // Subscribe to table-update events for immediate refresh on state changes.
    // The fallback poll above catches any missed events.
    const unlistenPromise = onTableUpdate(() => {
      if (cancelled) {
        return;
      }

      void getTableView(viewerMode)
        .then((snapshot) => {
          if (!cancelled) {
            consecutiveErrorsRef.current = 0;
            setTableView(snapshot);
            setError(null);
            setConnectionSlow(false);
          }
        })
        .catch(() => {
          // Ignore event-driven refresh errors; the fallback poll handles recovery
        });
    });

    return () => {
      cancelled = true;
      window.clearTimeout(timeoutId);
      void unlistenPromise.then((unlisten) => unlisten());
    };
  }, [navigate, viewerMode]);

  useEffect(() => {
    const actionTray = tableView?.actionTray;
    if (!actionTray) {
      setRaiseAmount(null);
      return;
    }

    setRaiseAmount((currentAmount) => {
      if (
        currentAmount !== null &&
        isWithinRaiseBounds(currentAmount, actionTray)
      ) {
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
    return Array.from(
      { length: BOARD_SLOT_COUNT },
      (_, index) => visibleCards[index] ?? null,
    );
  }, [tableView]);

  const quickSizes = useMemo(
    () => buildQuickSizes(tableView?.actionTray),
    [tableView],
  );
  const handStarted = tableView?.currentHandNumber !== null;
  const denseSeatMap = (tableView?.seats.length ?? 0) > 6;

  const submitAction = async (
    actionKind: DesktopTableActionKind,
    nextRaiseAmount?: number,
  ) => {
    setSubmitting(true);
    setActionError(null);
    try {
      const snapshot = await submitTableAction(
        viewerMode,
        actionKind,
        nextRaiseAmount,
      );
      setTableView(snapshot);
      setConfirmation(null);
    } catch (caughtError) {
      setActionError(getErrorMessage(caughtError));
    } finally {
      setSubmitting(false);
    }
  };

  const queueConfirmation = (
    actionKind: "betOrRaise" | "allIn",
    label: string,
  ) => {
    setConfirmation({ actionKind, label });
    setActionError(null);
  };

  const confirmQueuedAction = async () => {
    if (!confirmation || !tableView?.actionTray) {
      return;
    }

    const nextRaiseAmount =
      confirmation.actionKind === "betOrRaise"
        ? (raiseAmount ?? undefined)
        : undefined;
    await submitAction(confirmation.actionKind, nextRaiseAmount);
  };

  const handlePrimaryAction = async (actionKind: DesktopTableActionKind) => {
    if (actionKind === "betOrRaise") {
      if (!tableView?.actionTray) {
        return;
      }

      await submitAction(
        actionKind,
        raiseAmount ?? defaultRaiseAmount(tableView.actionTray),
      );
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
      badges={[
        handStarted
          ? (tableView?.blindLevelLabel ?? "Loading table")
          : (tableView?.phaseLabel ?? "Loading table"),
      ]}
      className="table-screen-layout"
    >
      <div
        className={`table-screen-shell ${denseSeatMap ? "dense-table-screen-shell" : ""}`}
      >
        <div className="table-top-row">
          <div className="button-row">
            <button
              className="secondary-button compact-button"
              onClick={() => setTableSidePanelOpen(!tableSidePanelOpen)}
              type="button"
            >
              <span className="button-content">
                <PanelRight className="button-icon" strokeWidth={1.9} />
                <span>
                  {tableSidePanelOpen ? "Hide details" : "Table details"}
                </span>
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
            <span className="field-hint">
              Requesting the current table state from the host.
            </span>
          </SectionCard>
        ) : null}

        {tableView?.sessionConnection === "reconnecting" ? (
          <div className="inline-banner info">
            <WifiOff className="button-icon" strokeWidth={1.9} />
            Reconnecting to host…
          </div>
        ) : connectionSlow ? (
          <div className="inline-banner info">
            <WifiOff className="button-icon" strokeWidth={1.9} />
            Connection slow — retrying…
          </div>
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
          <div
            className={`desktop-table-layout ${tableSidePanelOpen ? "with-side-panel" : ""}`}
          >
            <div
              className={`table-main-column ${denseSeatMap ? "dense-table-main-column" : ""}`}
            >
              <section
                className={`table-surface enhanced-table-surface ${denseSeatMap ? "dense-table-surface" : ""}`}
              >
                <div
                  className={`table-headline-card ${tableView.actionTray ? "is-active-turn" : "is-waiting"}`}
                >
                  <p className="kicker">
                    {handStarted
                      ? tableView.actionTray
                        ? "Your move"
                        : "Waiting"
                      : "Not started"}
                  </p>
                  <h3>
                    {handStarted
                      ? tableView.actionTray
                        ? `${tableView.actionTray.ownerLabel} to act`
                        : tableView.actionOwnerLabel
                      : "Tournament has not started"}
                  </h3>
                </div>

                <header className="table-surface-header">
                  <div>
                    <p className="kicker">{tableView.tableName}</p>
                    <h3>{tableView.tournamentName}</h3>
                    <p className="field-hint">
                      Hand {tableView.currentHandNumber ?? "—"} ·{" "}
                      {tableView.phaseLabel}
                    </p>
                  </div>
                  <div className="badge-row">
                    <StatusBadge tone="success">
                      {tableView.streetLabel}
                    </StatusBadge>
                    <StatusBadge tone="info">
                      {tableView.blindLevelLabel}
                    </StatusBadge>
                    <StatusBadge tone="accent">
                      Pot {tableView.potTotal}
                    </StatusBadge>
                  </div>
                </header>

                {tableView.observerBanner ? (
                  <div className="inline-banner info observer-banner">
                    <strong>Observer mode</strong>
                    <span>{tableView.observerBanner}</span>
                    {tableView.currentHandNumber !== null ? (
                      <span>Watching hand #{tableView.currentHandNumber}</span>
                    ) : null}
                  </div>
                ) : null}

                {handStarted ? (
                  <>
                    {denseSeatMap ? (
                      <p className="field-hint dense-table-note">
                        Action: {tableView.actionOwnerLabel}.{" "}
                        {tableView.eliminationSummary}
                      </p>
                    ) : (
                      <div className="table-summary-row">
                        <div className="pot-summary-card">
                          <span className="surface-label">Pot</span>
                          <strong>{tableView.potTotal} chips</strong>
                        </div>
                        <div className="pot-summary-card action-owner-card">
                          <span className="surface-label">Action</span>
                          <strong>{tableView.actionOwnerLabel}</strong>
                        </div>
                        <div className="pot-summary-card elimination-card">
                          <span className="surface-label">Table note</span>
                          <strong>{tableView.eliminationSummary}</strong>
                        </div>
                      </div>
                    )}

                    <div
                      className="community-cards centered-community-cards"
                      aria-label="Community cards"
                    >
                      {boardCards.map((card, index) => (
                        <PlayingCard
                          key={index}
                          card={card}
                          placeholderAriaLabel={`Community card ${index + 1}`}
                          size="board"
                        />
                      ))}
                    </div>
                  </>
                ) : (
                  <SectionCard title="Before the first hand">
                    <p>
                      The tournament is live but the first hand has not started
                      yet. The host will deal once all players are seated and
                      ready.
                    </p>
                  </SectionCard>
                )}

                <div className="seat-grid enhanced-seat-grid">
                  {tableView.seats.map((seat) => (
                    <SeatCard
                      key={seat.seatIndex}
                      seat={seat}
                      displayName={displayName}
                    />
                  ))}
                </div>
              </section>

              {tableView.actionTray ? (
                <SectionCard
                  kicker="Action"
                  title="Play this spot"
                  className="table-action-card"
                >
                  <div className="action-owner-banner">
                    <strong>{tableView.actionTray.ownerLabel}</strong>
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
                      disabled={
                        submitting || tableView.actionTray.maxRaiseTo === null
                      }
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
                  {submitting ? (
                    <div className="inline-banner info">
                      Sending your action to the host…
                    </div>
                  ) : null}
                  {tableView.actionTray.maxRaiseTo === null ? (
                    <p className="field-hint">
                      Raise unavailable until a legal raise size is offered for
                      this spot.
                    </p>
                  ) : (
                    <div className="raise-controls">
                      <label className="field" htmlFor="raise-slider">
                        <span>Raise size</span>
                        <input
                          aria-label="Raise amount"
                          aria-valuemax={tableView.actionTray.maxRaiseTo}
                          aria-valuemin={
                            tableView.actionTray.minRaiseTo ??
                            tableView.actionTray.maxRaiseTo
                          }
                          aria-valuenow={
                            raiseAmount ??
                            defaultRaiseAmount(tableView.actionTray)
                          }
                          id="raise-slider"
                          max={tableView.actionTray.maxRaiseTo}
                          min={
                            tableView.actionTray.minRaiseTo ??
                            tableView.actionTray.maxRaiseTo
                          }
                          onChange={(event) =>
                            setRaiseAmount(Number(event.target.value))
                          }
                          step={1}
                          type="range"
                          value={
                            raiseAmount ??
                            defaultRaiseAmount(tableView.actionTray)
                          }
                        />
                      </label>
                      <p className="field-hint">
                        To{" "}
                        <strong>
                          {raiseAmount ??
                            defaultRaiseAmount(tableView.actionTray)}
                        </strong>{" "}
                        · Call {tableView.actionTray.callAmount} · Pot{" "}
                        {tableView.actionTray.potTotal}
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
                  )}
                  {confirmation ? (
                    <div className="confirmation-card" role="status">
                      <strong>{confirmation.label}</strong>
                      <p className="field-hint">
                        {confirmation.actionKind === "betOrRaise"
                          ? `Send a raise to ${raiseAmount ?? defaultRaiseAmount(tableView.actionTray)} chips?`
                          : "Commit the remaining stack as an all-in action?"}
                      </p>
                      <div className="button-row">
                        <button
                          className="primary-button"
                          disabled={submitting}
                          onClick={() => void confirmQueuedAction()}
                          type="button"
                        >
                          Confirm
                        </button>
                        <button
                          className="secondary-button"
                          disabled={submitting}
                          onClick={() => setConfirmation(null)}
                          type="button"
                        >
                          Cancel
                        </button>
                      </div>
                    </div>
                  ) : null}
                  {actionError ? (
                    <div className="inline-banner error">{actionError}</div>
                  ) : null}
                </SectionCard>
              ) : (
                <SectionCard
                  kicker="Action"
                  title="Waiting"
                  className="table-action-card"
                >
                  <p>
                    {handStarted
                      ? "Waiting for the next action from the host."
                      : "No hand is running yet."}
                  </p>
                </SectionCard>
              )}
            </div>

            {tableSidePanelOpen ? (
              <TableSidePanel tableView={tableView} displayName={displayName} />
            ) : null}
          </div>
        ) : null}
      </div>
    </ScreenShell>
  );
}
