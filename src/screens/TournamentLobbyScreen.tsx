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
    recentJoinPayloads,
  );
  const activeSeats = participants.filter((seat) => seat.kind !== "open");
  const canStart = activeSeats.length >= 2 && activeSeats.every((seat) => seat.ready);
  const localSeat = participants.find((seat) => seat.kind !== "open" && seat.label === displayName);
  const localSeatReady = localSeat?.ready ?? false;
  const readySeatCount = activeSeats.filter((seat) => seat.ready).length;
  const seatsStillWaiting = activeSeats.filter((seat) => !seat.ready).length;

  return (
    <ScreenShell
      title="Tournament Lobby"
      lead="Everyone checks in here before the first hand."
      badges={[hostDraft.tournamentName, canStart ? "Ready to deal" : `${activeSeats.length}/${hostDraft.maxPlayers} seated`]}
      className="pregame-screen-shell"
    >
      <div className="pregame-workstation lobby-station-layout">
        <SectionCard kicker="Waiting room" title="Gather around the table" className="workstation-main-card">
          <div className="lobby-workstation-grid">
            <article className="lobby-progress-card compact-lobby-progress">
              <strong>{canStart ? "Everyone is ready for the first hand" : `${readySeatCount} of ${activeSeats.length} seated players are ready`}</strong>
              <p className="field-hint">
                {canStart
                  ? "Deal as soon as the table is set."
                  : seatsStillWaiting === 1
                    ? "One seated player still needs to check in."
                    : `${seatsStillWaiting} seated players still need to check in.`}
              </p>
              <div className="lobby-status-row">
                <span className={`status-badge ${localSeatReady ? "success" : "warning"}`}>
                  {localSeatReady ? <Check className="button-icon" strokeWidth={1.9} /> : <Clock3 className="button-icon" strokeWidth={1.9} />}
                  {localSeatReady ? "You are ready" : "You still need to mark ready"}
                </span>
                <span className={`status-badge ${canStart ? "success" : "info"}`}>
                  {canStart ? <Check className="button-icon" strokeWidth={1.9} /> : <Clock3 className="button-icon" strokeWidth={1.9} />}
                  {canStart ? "Table can start" : "Waiting on the table"}
                </span>
              </div>
            </article>

            <div className="seat-grid lobby-seat-grid compact-lobby-seat-grid">
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

            <section className="lobby-action-rail compact-status-panel">
              <p>{canStart ? "Everyone is checked in. Start when you want the cards in the air." : "At least two seated players must be ready before the game can begin."}</p>
              <div className="button-row workstation-actions">
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
                  className="secondary-button compact-button"
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
                <p className="field-hint">Mark each seated player ready, then start the table.</p>
              ) : null}
            </section>
          </div>
        </SectionCard>
      </div>
      {showLeaveFlow ? (
        <section className="dialog-card">
          <p className="kicker">Exit lobby</p>
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
