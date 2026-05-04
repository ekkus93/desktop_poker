import { useState } from "react";
import { Check, Clock3, LogOut, Play } from "lucide-react";
import { Link } from "react-router-dom";
import { useDesktopShell } from "../app/useDesktopShell";
import { buildParticipantShell } from "../app/shell";
import { SectionCard } from "../components/shared/SectionCard";
import { ScreenShell } from "./ScreenShell";
import type { ScreenProps } from "./types";

export function TournamentLobbyScreen({ bootstrap }: ScreenProps) {
  const { displayName, hostDraft, readySeats, recentJoinPayloads, toggleSeatReady } =
    useDesktopShell();
  const [showLeaveFlow, setShowLeaveFlow] = useState(false);
  const participants = buildParticipantShell(
    bootstrap,
    hostDraft,
    readySeats,
    displayName,
    recentJoinPayloads,
  );
  const activeSeats = participants.filter((seat) => seat.kind !== "open");
  const canStart = activeSeats.length >= 2 && activeSeats.every((seat) => seat.ready);
  const localSeat = participants.find((seat) => seat.kind !== "open" && seat.label === displayName);
  const localSeatReady = localSeat?.ready ?? false;

  return (
    <ScreenShell
      title="Tournament Lobby"
      lead="Seat everyone, mark ready, then start the table."
      badges={[hostDraft.tournamentName, canStart ? "Ready to start" : `${activeSeats.length}/${hostDraft.maxPlayers} seated`]}
    >
      <div className="content-grid">
        <SectionCard kicker="Ready state" title="Ready to start?">
          <div className="stacked-list">
            <article className="list-panel">
              <div>
                <strong>{localSeatReady ? "Your seat is ready" : "Your seat is waiting"}</strong>
                <p className="field-hint">{localSeatReady ? "You are set." : "Mark ready when you are set."}</p>
              </div>
            </article>
            <article className="list-panel">
              <div>
                <strong>{canStart ? "The table can start now" : "The table is still waiting"}</strong>
                <p className="field-hint">{canStart ? "Everyone seated is ready." : "Every occupied seat must be ready."}</p>
              </div>
            </article>
          </div>
        </SectionCard>

        <SectionCard kicker="Seat map" title="Seats">
          <div className="seat-grid">
            {participants.map((seat) => {
              const seatState = seat.kind === "open" ? "Open" : seat.ready ? "Ready" : "Waiting";

              return (
              <article
                key={seat.seatIndex}
                className={`seat-card lobby-seat-card ${seat.kind === "open" ? "lobby-seat-open" : "lobby-seat-filled"} ${seat.kind === "host" ? "lobby-seat-local" : ""} ${seat.ready ? "lobby-seat-ready" : ""}`}
              >
                <div className="seat-card-header">
                  <div>
                    <strong>Seat {seat.seatIndex}</strong>
                    <span>{seat.label}</span>
                  </div>
                  <span className={`status-badge ${seat.kind === "open" ? "info" : seat.ready ? "success" : "warning"}`}>
                    {seat.kind === "open" ? <Clock3 className="button-icon" strokeWidth={1.9} /> : seat.ready ? <Check className="button-icon" strokeWidth={1.9} /> : <Clock3 className="button-icon" strokeWidth={1.9} />}
                    {seatState}
                  </span>
                </div>
                {seat.kind === "open" ? (
                  <p className="seat-detail">Open for the next player.</p>
                ) : (
                  <p className="seat-detail">{seat.detail}</p>
                )}
                {seat.kind !== "open" ? (
                  <button
                    className={seat.ready ? "primary-button compact-button" : "secondary-button compact-button"}
                    onClick={() => {
                      toggleSeatReady(seat.seatIndex);
                    }}
                    type="button"
                  >
                    {seat.ready ? "Ready" : "Mark ready"}
                  </button>
                ) : null}
              </article>
            );})}
          </div>
        </SectionCard>

        <SectionCard title="Start table">
          <p>{canStart ? "Everyone is ready. Start when the table is set." : "At least two seated players must be ready before the game can start."}</p>
          <div className="button-row">
            {canStart ? (
              <Link className="primary-button" to="/table">
                <span className="button-content">
                  <Play className="button-icon" strokeWidth={1.9} />
                  <span>Start tournament</span>
                </span>
              </Link>
            ) : (
              <button className="primary-button" disabled type="button">
                <span className="button-content">
                  <Play className="button-icon" strokeWidth={1.9} />
                  <span>Start tournament</span>
                </span>
              </button>
            )}
            <button
              className="secondary-button"
              onClick={() => {
                setShowLeaveFlow(true);
              }}
              type="button"
            >
              <span className="button-content">
                <LogOut className="button-icon" strokeWidth={1.9} />
                <span>Leave table</span>
              </span>
            </button>
          </div>
          {!canStart ? (
            <p className="field-hint">
              Mark each seated player ready, then start the table.
            </p>
          ) : null}
        </SectionCard>
      </div>
      {showLeaveFlow ? (
        <section className="dialog-card">
          <p className="kicker">Leave table flow</p>
          <h3>Leave this lobby?</h3>
          <p>
            Leave this table and go back home?
          </p>
          <div className="button-row">
            <Link className="primary-button" to="/">
              Leave table
            </Link>
            <button
              className="secondary-button"
              onClick={() => {
                setShowLeaveFlow(false);
              }}
              type="button"
            >
              Stay in lobby
            </button>
          </div>
        </section>
      ) : null}
    </ScreenShell>
  );
}
