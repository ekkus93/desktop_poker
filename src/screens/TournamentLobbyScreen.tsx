import { useEffect, useState } from "react";
import { Check, Clock3, LogOut, Play } from "lucide-react";
import { Link } from "react-router-dom";
import { useDesktopShell } from "../app/useDesktopShell";
import { buildParticipantShell } from "../app/shell";
import {
  getClientSessionStatus,
  getHostSessionStatus,
  type ClientSessionStatus,
  type HostSessionStatus,
} from "../api/desktop";
import { ScreenShell } from "./ScreenShell";
import type { ScreenProps } from "./types";

type LobbySeatView = {
  seatIndex: number;
  label: string;
  detail: string;
  kind: "host" | "pending" | "open";
  isLocal: boolean;
  ready: boolean;
};

function buildLiveSeats(
  liveSession: HostSessionStatus | ClientSessionStatus,
): LobbySeatView[] {
  const totalSeats = liveSession.activeSeatCount + liveSession.openSeatCount;
  const seats: LobbySeatView[] = Array.from({ length: totalSeats }, (_, index) => ({
    seatIndex: index + 1,
    label: "Open seat",
    detail: "",
    kind: "open",
    isLocal: false,
    ready: false,
  }));
  const localPlayerId = "localPlayerId" in liveSession ? liveSession.localPlayerId : "local-player";

  for (const participant of liveSession.participants) {
    const preferredIndex = participant.seatIndex ?? seats.findIndex((seat) => seat.kind === "open");
    if (preferredIndex < 0 || preferredIndex >= seats.length) {
      continue;
    }

    seats[preferredIndex] = {
      seatIndex: preferredIndex + 1,
      label: participant.playerId === localPlayerId ? "You" : participant.displayName,
      detail: participant.seatIndex === null
        ? `${participant.participantState} · ${participant.connectionState} · awaiting seat`
        : participant.isHost
          ? `Host · ${participant.connectionState}`
          : participant.connectionState,
      kind: participant.seatIndex === null ? "pending" : "host",
      isLocal: participant.playerId === localPlayerId,
      ready: participant.isReady,
    };
  }

  return seats;
}

export function TournamentLobbyScreen({ bootstrap }: ScreenProps) {
  const { hostDraft, readySeats, recentJoinPayloads, toggleSeatReady } =
    useDesktopShell();
  const [showLeaveFlow, setShowLeaveFlow] = useState(false);
  const [hostSession, setHostSession] = useState<HostSessionStatus | null>(null);
  const [clientSession, setClientSession] = useState<ClientSessionStatus | null>(null);

  useEffect(() => {
    let active = true;

    const refresh = () => {
      void Promise.all([getHostSessionStatus(), getClientSessionStatus()])
        .then(([nextHostSession, nextClientSession]) => {
          if (!active) {
            return;
          }

          setHostSession(nextHostSession);
          setClientSession(nextClientSession);
        })
        .catch(() => {
          if (!active) {
            return;
          }

          setHostSession(null);
          setClientSession(null);
        });
    };

    refresh();
    const intervalId = window.setInterval(refresh, 800);

    return () => {
      active = false;
      window.clearInterval(intervalId);
    };
  }, []);

  const liveSession = hostSession ?? clientSession;
  const participants = liveSession
    ? buildLiveSeats(liveSession)
    : buildParticipantShell(
    bootstrap,
    hostDraft,
    readySeats,
    recentJoinPayloads,
    );
  const activeSeats = participants.filter((seat) => seat.kind !== "open");
  const canStart = false;
  const localSeat = participants.find((seat) => seat.isLocal);
  const localSeatReady = localSeat?.ready ?? false;
  const seatsStillWaiting = activeSeats.filter((seat) => !seat.ready).length;
  const openSeatCount = liveSession ? liveSession.openSeatCount : hostDraft.maxPlayers - activeSeats.length;
  const leaveTitle = localSeat?.kind === "host" ? "Close this table?" : "Leave this table?";
  const leaveActionLabel = localSeat?.kind === "host" ? "Close table" : "Leave table";
  const tournamentName = liveSession ? liveSession.tournamentName : hostDraft.tournamentName;
  const totalSeats = liveSession
    ? liveSession.activeSeatCount + liveSession.openSeatCount
    : hostDraft.maxPlayers;

  return (
    <ScreenShell
      title="Lobby"
      badges={[canStart ? "Ready to deal" : `${activeSeats.length}/${totalSeats} connected`]}
    >
      <div className="content-grid lobby-shell-grid">
        <section className="section-card lobby-stage-card">
          <div className="lobby-stage-layout">
            <div className="lobby-stage-summary">
              <div className="lobby-table-meta">
                <strong className="lobby-table-name">{tournamentName}</strong>
                <span className="field-hint">{totalSeats} seats</span>
              </div>
              <div className="lobby-status-row">
                <span className={`status-badge ${localSeatReady ? "success" : "info"}`}>
                  {localSeatReady ? <Check className="button-icon" strokeWidth={1.9} /> : <Clock3 className="button-icon" strokeWidth={1.9} />}
                  {localSeatReady ? "You: Ready" : "You: Waiting"}
                </span>
                <span className={`status-badge ${canStart ? "success" : "info"}`}>
                  {canStart ? <Check className="button-icon" strokeWidth={1.9} /> : <Clock3 className="button-icon" strokeWidth={1.9} />}
                  {canStart ? "Table: Ready" : `${seatsStillWaiting} waiting`}
                </span>
                <span className="status-badge accent">
                  {openSeatCount > 0 ? `${openSeatCount} open seats` : "Table full"}
                </span>
              </div>
              <div className="button-row lobby-primary-actions workstation-actions compact-workstation-actions">
                {localSeat && !liveSession ? (
                  <button
                    className={localSeatReady ? "primary-button compact-button" : "secondary-button compact-button"}
                    onClick={() => {
                      toggleSeatReady(localSeat.seatIndex);
                    }}
                    type="button"
                  >
                    {localSeatReady ? "Undo ready" : "I'm ready"}
                  </button>
                ) : null}
                {canStart ? (
                  <Link className="primary-button compact-button" to="/table">
                    <span className="button-content">
                      <Play className="button-icon" strokeWidth={1.9} />
                      <span>Start tournament</span>
                    </span>
                  </Link>
                ) : (
                  <button className="primary-button compact-button" disabled type="button">
                    <span className="button-content">
                      <Play className="button-icon" strokeWidth={1.9} />
                      <span>Start tournament</span>
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
            </div>
            <div className="seat-grid lobby-seat-grid">
              {participants.map((seat) => {
                const seatState = seat.kind === "open" ? "Open" : seat.ready ? "Ready" : "Waiting";

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
                      <span className={`status-badge ${seat.kind === "open" ? "accent" : seat.ready ? "success" : "info"}`}>
                        {seat.kind === "open" ? null : seat.ready ? <Check className="button-icon" strokeWidth={1.9} /> : <Clock3 className="button-icon" strokeWidth={1.9} />}
                        {seatState}
                      </span>
                    </div>
                    {seat.kind === "open" ? null : seat.detail ? <p className="seat-detail">{seat.detail}</p> : null}
                  </article>
                );
              })}
            </div>
          </div>
        </section>
      </div>
      {showLeaveFlow ? (
        <section className="dialog-card">
          <p className="kicker">{localSeat?.kind === "host" ? "Close" : "Leave"}</p>
          <h3>{leaveTitle}</h3>
          <div className="button-row">
            <Link className="primary-button" to="/">
              {leaveActionLabel}
            </Link>
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
