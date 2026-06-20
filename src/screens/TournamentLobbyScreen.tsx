import { useEffect, useRef, useState } from "react";
import { Check, Clock3, LogOut, Play, WifiOff } from "lucide-react";
import { useNavigate } from "react-router-dom";
import {
  clientClaimLobbySeat,
  clientSetLobbyReadyState,
  getClientSessionStatus,
  getHostSessionStatus,
  hostClaimLobbySeat,
  hostSetLobbyReadyState,
  hostStartTournament,
  leaveClientSession,
  onSessionUpdate,
  stopHostSession,
  type ClientSessionStatus,
  type HostSessionStatus,
} from "../api/desktop";
import { useDesktopShell } from "../app/useDesktopShell";
import { ScreenShell } from "./ScreenShell";
import type { ScreenProps } from "./types";

type LobbySeatView = {
  seatIndex: number;
  label: string;
  detail: string;
  kind: "host" | "pending" | "open";
  isLocal: boolean;
  ready: boolean;
  isNpc: boolean;
};

function buildLiveSeats(
  liveSession: HostSessionStatus | ClientSessionStatus,
): LobbySeatView[] {
  const totalSeats = liveSession.activeSeatCount + liveSession.openSeatCount;
  const seats: LobbySeatView[] = Array.from(
    { length: totalSeats },
    (_, index) => ({
      seatIndex: index + 1,
      label: "Open seat",
      detail: "",
      kind: "open",
      isLocal: false,
      ready: false,
      isNpc: false,
    }),
  );
  const localPlayerId =
    "localPlayerId" in liveSession ? liveSession.localPlayerId : "local-player";

  for (const participant of liveSession.participants) {
    const preferredIndex =
      participant.seatIndex ?? seats.findIndex((seat) => seat.kind === "open");
    if (preferredIndex < 0 || preferredIndex >= seats.length) {
      continue;
    }

    const isNpc = participant.playerId.startsWith("npc-seat-");
    seats[preferredIndex] = {
      seatIndex: preferredIndex + 1,
      label:
        participant.playerId === localPlayerId
          ? "You"
          : participant.displayName,
      detail: isNpc
        ? "(AI) · Always ready"
        : participant.seatIndex === null
          ? `${participant.participantState} · ${participant.connectionState} · awaiting seat`
          : participant.isHost
            ? `Host · ${participant.connectionState}`
            : participant.connectionState,
      kind: participant.seatIndex === null ? "pending" : "host",
      isLocal: participant.playerId === localPlayerId,
      ready: isNpc ? true : participant.isReady,
      isNpc,
    };
  }

  return seats;
}

const LOBBY_POLL_NORMAL_MS = 5000;
const LOBBY_POLL_SLOW_MS = 10000;
const LOBBY_BACKOFF_THRESHOLD = 3;
const LOBBY_ERROR_LIMIT = 10;

export function TournamentLobbyScreen({ bootstrap }: ScreenProps) {
  void bootstrap;
  const navigate = useNavigate();
  const { setLastEndedSession } = useDesktopShell();
  const [showLeaveFlow, setShowLeaveFlow] = useState(false);
  const [hostSession, setHostSession] = useState<HostSessionStatus | null>(
    null,
  );
  const [clientSession, setClientSession] =
    useState<ClientSessionStatus | null>(null);
  const [sessionStatus, setSessionStatus] = useState<"loading" | "ready">(
    "loading",
  );
  const [lobbyError, setLobbyError] = useState<string | null>(null);
  const [hostRecoveryError, setHostRecoveryError] = useState<string | null>(
    null,
  );
  const [connectionSlow, setConnectionSlow] = useState(false);
  const [clientReconnecting, setClientReconnecting] = useState(false);
  const [clientTerminated, setClientTerminated] = useState(false);
  const [submitting, setSubmitting] = useState(false);
  const [optimisticReadyOverride, setOptimisticReadyOverride] = useState<
    boolean | null
  >(null);
  const hostSessionRef = useRef<HostSessionStatus | null>(null);
  const consecutiveErrorsRef = useRef(0);

  useEffect(() => {
    hostSessionRef.current = hostSession;
  }, [hostSession]);

  useEffect(() => {
    let active = true;
    let timeoutId: number | undefined;

    const scheduleNext = (delayMs: number) => {
      if (!active) {
        return;
      }

      timeoutId = window.setTimeout(refresh, delayMs);
    };

    const refresh = () => {
      void Promise.all([getHostSessionStatus(), getClientSessionStatus()])
        .then(([nextHostSession, nextClientSession]) => {
          if (!active) {
            return;
          }

          consecutiveErrorsRef.current = 0;
          setConnectionSlow(false);

          const previousHostSession = hostSessionRef.current;

          if (
            previousHostSession &&
            previousHostSession.phase !== "running" &&
            !nextHostSession &&
            !nextClientSession
          ) {
            setHostRecoveryError(
              "Hosting stopped before the table went live. Start hosting again or return home.",
            );
          } else if (nextHostSession) {
            setHostRecoveryError(null);
          }

          setHostSession(nextHostSession);
          setClientSession(nextClientSession);
          setSessionStatus("ready");

          if (nextClientSession) {
            if (nextClientSession.terminated) {
              setClientTerminated(true);
              setClientReconnecting(false);
              return; // stop polling — session is terminal
            }
            setClientReconnecting(nextClientSession.reconnecting);
          } else {
            setClientReconnecting(false);
            setClientTerminated(false);
          }

          scheduleNext(LOBBY_POLL_NORMAL_MS);
        })
        .catch(() => {
          if (!active) {
            return;
          }

          consecutiveErrorsRef.current += 1;

          if (consecutiveErrorsRef.current >= LOBBY_ERROR_LIMIT) {
            navigate("/errors", { replace: true });
            return;
          }

          // Keep the last-known session on screen and surface the "connection
          // slow" banner instead of dropping to the no-session/recovery screen on
          // a transient poll failure. A genuine host-stop is detected on the
          // success path (the poll resolves to null), so a thrown error here is
          // treated as recoverable degradation, not a stopped session.
          setConnectionSlow(
            consecutiveErrorsRef.current >= LOBBY_BACKOFF_THRESHOLD,
          );
          setSessionStatus("ready");
          scheduleNext(
            consecutiveErrorsRef.current >= LOBBY_BACKOFF_THRESHOLD
              ? LOBBY_POLL_SLOW_MS
              : LOBBY_POLL_NORMAL_MS,
          );
        });
    };

    refresh();

    // Subscribe to session-update events for immediate refresh on state changes.
    // The fallback poll above catches any missed events.
    const unlistenPromise = onSessionUpdate(() => {
      if (active) {
        void Promise.all([getHostSessionStatus(), getClientSessionStatus()])
          .then(([nextHostSession, nextClientSession]) => {
            if (!active) {
              return;
            }

            consecutiveErrorsRef.current = 0;
            setConnectionSlow(false);

            const previousHostSession = hostSessionRef.current;
            if (
              previousHostSession &&
              previousHostSession.phase !== "running" &&
              !nextHostSession &&
              !nextClientSession
            ) {
              setHostRecoveryError(
                "Hosting stopped before the table went live. Start hosting again or return home.",
              );
            } else if (nextHostSession) {
              setHostRecoveryError(null);
            }

            setHostSession(nextHostSession);
            setClientSession(nextClientSession);
            setSessionStatus("ready");

            if (nextClientSession) {
              setClientTerminated(nextClientSession.terminated);
              setClientReconnecting(
                !nextClientSession.terminated && nextClientSession.reconnecting,
              );
            } else {
              setClientReconnecting(false);
              setClientTerminated(false);
            }
          })
          .catch(() => {
            // Ignore event-driven refresh errors; the fallback poll handles recovery
          });
      }
    });

    return () => {
      active = false;
      window.clearTimeout(timeoutId);
      void unlistenPromise.then((unlisten) => unlisten());
    };
  }, [navigate]);

  const liveSession = hostSession ?? clientSession;
  const liveLocalPlayerId = hostSession
    ? "local-player"
    : (clientSession?.localPlayerId ?? null);
  const liveLocalParticipant =
    liveSession?.participants.find(
      (participant) => participant.playerId === liveLocalPlayerId,
    ) ?? null;
  const participants = liveSession ? buildLiveSeats(liveSession) : [];
  const activeSeats = participants.filter((seat) => seat.kind !== "open");
  const localSeat = participants.find((seat) => seat.isLocal);
  const localSeatReady = optimisticReadyOverride ?? localSeat?.ready ?? false;
  const seatsStillWaiting = activeSeats.filter(
    (seat) => !seat.ready && !seat.isNpc,
  ).length;
  const openSeatCount = liveSession?.openSeatCount ?? 0;
  const leaveTitle =
    localSeat?.kind === "host" ? "Close this table?" : "Leave this table?";
  const leaveActionLabel =
    localSeat?.kind === "host" ? "Close table" : "Leave table";
  const tournamentName = liveSession?.tournamentName ?? "Lobby";
  const totalSeats = liveSession
    ? liveSession.activeSeatCount + liveSession.openSeatCount
    : 0;
  const denseLobbyLayout = totalSeats > 8;
  const liveCanStart = Boolean(
    hostSession &&
    liveSession?.phase === "readyCheck" &&
    liveLocalParticipant?.isHost,
  );
  const liveLobbyActionsEnabled = Boolean(
    liveSession && liveSession.phase !== "running",
  );
  const tableReady = liveSession?.phase === "running" || liveCanStart;
  const lobbyBadge =
    liveSession?.phase === "running"
      ? "Table live"
      : tableReady
        ? "Ready to deal"
        : liveSession
          ? `${activeSeats.length}/${totalSeats} connected`
          : "Checking session";

  useEffect(() => {
    if (liveSession?.phase === "running") {
      navigate("/table", { replace: true });
    }
  }, [liveSession?.phase, navigate]);

  if (hostRecoveryError && !liveSession) {
    return (
      <ScreenShell title="Lobby" badges={["Host stopped"]}>
        <div className="content-grid lobby-shell-grid">
          <section className="section-card lobby-stage-card">
            <div className="lobby-stage-layout">
              <div className="lobby-stage-summary">
                <div className="lobby-table-meta">
                  <strong className="lobby-table-name">Host stopped</strong>
                  <span className="field-hint">
                    Recovery required before players can join again.
                  </span>
                </div>
                <p className="inline-banner error">{hostRecoveryError}</p>
                <div className="button-row workstation-actions compact-workstation-actions">
                  <button
                    className="primary-button compact-button"
                    onClick={() => {
                      navigate("/host", { replace: true });
                    }}
                    type="button"
                  >
                    Host again
                  </button>
                  <button
                    className="secondary-button compact-button"
                    onClick={() => {
                      navigate("/", { replace: true });
                    }}
                    type="button"
                  >
                    Return home
                  </button>
                </div>
              </div>
            </div>
          </section>
        </div>
      </ScreenShell>
    );
  }

  if (sessionStatus === "loading" && !liveSession) {
    return (
      <ScreenShell title="Lobby" badges={["Checking session"]}>
        <div className="content-grid lobby-shell-grid">
          <section className="section-card lobby-stage-card">
            <div className="lobby-stage-layout">
              <div className="lobby-stage-summary">
                <div className="lobby-table-meta">
                  <strong className="lobby-table-name">
                    Loading live lobby
                  </strong>
                  <span className="field-hint">
                    Waiting for the desktop runtime to publish the active
                    session.
                  </span>
                </div>
              </div>
            </div>
          </section>
        </div>
      </ScreenShell>
    );
  }

  if (!liveSession) {
    return (
      <ScreenShell title="Lobby" badges={["Lobby unavailable"]}>
        <div className="content-grid lobby-shell-grid">
          <section className="section-card lobby-stage-card">
            <div className="lobby-stage-layout">
              <div className="lobby-stage-summary">
                <div className="lobby-table-meta">
                  <strong className="lobby-table-name">
                    No live lobby session
                  </strong>
                  <span className="field-hint">
                    This screen only works when a real host or client session is
                    active.
                  </span>
                </div>
                <div className="button-row workstation-actions compact-workstation-actions">
                  <button
                    className="primary-button compact-button"
                    onClick={() => {
                      navigate("/", { replace: true });
                    }}
                    type="button"
                  >
                    Return home
                  </button>
                </div>
              </div>
            </div>
          </section>
        </div>
      </ScreenShell>
    );
  }

  const claimLiveSeat = async (seatIndex: number) => {
    setSubmitting(true);
    setLobbyError(null);

    try {
      if (hostSession) {
        setHostSession(await hostClaimLobbySeat({ seatIndex }));
      } else if (clientSession) {
        setClientSession(await clientClaimLobbySeat({ seatIndex }));
      }
    } catch (error) {
      setLobbyError(
        error instanceof Error ? error.message : "Unable to claim that seat.",
      );
    } finally {
      setSubmitting(false);
    }
  };

  const toggleLiveReadyState = async () => {
    setSubmitting(true);
    setLobbyError(null);
    const nextReady = !localSeatReady;
    setOptimisticReadyOverride(nextReady);

    try {
      if (hostSession) {
        setHostSession(await hostSetLobbyReadyState({ isReady: nextReady }));
      } else if (clientSession) {
        setClientSession(
          await clientSetLobbyReadyState({ isReady: nextReady }),
        );
      }
    } catch (error) {
      setLobbyError(
        error instanceof Error
          ? error.message
          : "Unable to change ready state.",
      );
      setOptimisticReadyOverride(null);
    } finally {
      setSubmitting(false);
      setOptimisticReadyOverride(null);
    }
  };

  const startLiveTournament = async () => {
    setSubmitting(true);
    setLobbyError(null);

    try {
      setHostSession(await hostStartTournament());
    } catch (error) {
      setLobbyError(
        error instanceof Error
          ? error.message
          : "Unable to start the tournament.",
      );
    } finally {
      setSubmitting(false);
    }
  };

  const leaveLiveSession = async () => {
    setSubmitting(true);
    setLobbyError(null);

    try {
      if (hostSession) {
        setLastEndedSession({
          role: "host",
          tournamentName: hostSession.tournamentName,
        });
        await stopHostSession();
        setHostSession(null);
      } else if (clientSession) {
        setLastEndedSession({
          role: "client",
          tournamentName: clientSession.tournamentName,
        });
        await leaveClientSession();
        setClientSession(null);
      }

      setShowLeaveFlow(false);
      navigate("/", { replace: true });
    } catch (error) {
      setLobbyError(
        error instanceof Error
          ? error.message
          : "Unable to leave the current table.",
      );
    } finally {
      setSubmitting(false);
    }
  };

  return (
    <ScreenShell title="Lobby" badges={[lobbyBadge]}>
      <div className="content-grid lobby-shell-grid">
        <section className="section-card lobby-stage-card">
          <div
            className={`lobby-stage-layout ${denseLobbyLayout ? "dense-lobby-stage-layout" : ""}`}
          >
            <div className="lobby-stage-summary">
              <div className="lobby-table-meta">
                <strong className="lobby-table-name">{tournamentName}</strong>
                <span className="field-hint">{totalSeats} seats</span>
              </div>
              <div className="lobby-status-row">
                <span
                  className={`status-badge ${localSeatReady ? "success" : "info"}`}
                >
                  {localSeatReady ? (
                    <Check className="button-icon" strokeWidth={1.9} />
                  ) : (
                    <Clock3 className="button-icon" strokeWidth={1.9} />
                  )}
                  {localSeatReady ? "You: Ready" : "You: Waiting"}
                </span>
                <span
                  className={`status-badge ${tableReady ? "success" : "info"}`}
                >
                  {tableReady ? (
                    <Check className="button-icon" strokeWidth={1.9} />
                  ) : (
                    <Clock3 className="button-icon" strokeWidth={1.9} />
                  )}
                  {tableReady ? "Table: Ready" : `${seatsStillWaiting} waiting`}
                </span>
                <span className="status-badge accent">
                  {openSeatCount > 0
                    ? `${openSeatCount} open seats`
                    : "Table full"}
                </span>
              </div>
              {clientTerminated ? (
                <div className="inline-banner error">
                  <WifiOff className="button-icon" strokeWidth={1.9} />
                  Disconnected from host — session could not be recovered.
                </div>
              ) : clientReconnecting ? (
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
              <div className="button-row lobby-primary-actions workstation-actions compact-workstation-actions">
                {liveLobbyActionsEnabled &&
                liveLocalParticipant?.seatIndex !== null ? (
                  <button
                    className={
                      localSeatReady
                        ? "primary-button compact-button"
                        : "secondary-button compact-button"
                    }
                    disabled={submitting}
                    onClick={() => {
                      void toggleLiveReadyState();
                    }}
                    type="button"
                  >
                    {localSeatReady ? "Undo ready" : "I'm ready"}
                  </button>
                ) : null}
                {liveCanStart ? (
                  <button
                    className="primary-button compact-button"
                    disabled={submitting}
                    onClick={() => {
                      void startLiveTournament();
                    }}
                    type="button"
                  >
                    <span className="button-content">
                      <Play className="button-icon" strokeWidth={1.9} />
                      <span>Start tournament</span>
                    </span>
                  </button>
                ) : (
                  <button
                    className="primary-button compact-button"
                    disabled
                    type="button"
                  >
                    <span className="button-content">
                      <Play className="button-icon" strokeWidth={1.9} />
                      <span>
                        {liveSession.phase === "running"
                          ? "Tournament live"
                          : "Start tournament"}
                      </span>
                    </span>
                  </button>
                )}
                <button
                  className="secondary-button compact-button"
                  onClick={() => {
                    setShowLeaveFlow(true);
                  }}
                  type="button"
                >
                  <span className="button-content">
                    <LogOut className="button-icon" strokeWidth={1.9} />
                    <span>{leaveActionLabel}</span>
                  </span>
                </button>
              </div>
              {lobbyError ? (
                <p className="inline-banner error">{lobbyError}</p>
              ) : null}
            </div>
            <div
              className={`seat-grid lobby-seat-grid ${denseLobbyLayout ? "dense-lobby-seat-grid" : ""}`}
            >
              {participants.map((seat) => {
                const seatState =
                  seat.kind === "open"
                    ? "Open"
                    : seat.ready
                      ? "Ready"
                      : "Waiting";

                return (
                  <article
                    key={seat.seatIndex}
                    className={`seat-card lobby-seat-card ${seat.kind === "open" ? "lobby-seat-open" : "lobby-seat-filled"} ${seat.isLocal ? "lobby-seat-local" : ""} ${seat.ready ? "lobby-seat-ready" : ""}`}
                  >
                    <div className="seat-card-header">
                      <div>
                        <strong>Seat {seat.seatIndex}</strong>
                        <span>{seat.label}</span>
                      </div>
                      <span
                        className={`status-badge ${seat.kind === "open" ? "accent" : seat.ready ? "success" : "info"}`}
                      >
                        {seat.kind === "open" ? null : seat.ready ? (
                          <Check className="button-icon" strokeWidth={1.9} />
                        ) : (
                          <Clock3 className="button-icon" strokeWidth={1.9} />
                        )}
                        {seatState}
                      </span>
                    </div>
                    {seat.kind === "open" ? null : seat.detail ? (
                      <p className="seat-detail">{seat.detail}</p>
                    ) : null}
                    {liveSession &&
                    liveLobbyActionsEnabled &&
                    seat.kind === "open" &&
                    !seat.isNpc ? (
                      <button
                        aria-label={
                          denseLobbyLayout
                            ? `Take seat ${seat.seatIndex}`
                            : undefined
                        }
                        className={`secondary-button compact-button ${denseLobbyLayout ? "dense-lobby-seat-action" : ""}`}
                        disabled={submitting}
                        onClick={() => {
                          void claimLiveSeat(seat.seatIndex - 1);
                        }}
                        type="button"
                      >
                        {denseLobbyLayout ? "Take" : "Take seat"}
                      </button>
                    ) : null}
                  </article>
                );
              })}
            </div>
          </div>
        </section>
      </div>
      {showLeaveFlow ? (
        <section className="dialog-card">
          <p className="kicker">
            {localSeat?.kind === "host" ? "Close" : "Leave"}
          </p>
          <h3>{leaveTitle}</h3>
          {lobbyError ? (
            <p className="inline-banner error">{lobbyError}</p>
          ) : null}
          <div className="button-row">
            <button
              className="primary-button"
              disabled={submitting}
              onClick={() => {
                void leaveLiveSession();
              }}
              type="button"
            >
              {leaveActionLabel}
            </button>
            <button
              className="secondary-button"
              onClick={() => {
                setShowLeaveFlow(false);
              }}
              type="button"
            >
              Stay here
            </button>
          </div>
        </section>
      ) : null}
    </ScreenShell>
  );
}
