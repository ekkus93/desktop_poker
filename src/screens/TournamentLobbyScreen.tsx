import { useState } from "react";
import { Check, Clock3, LogOut, Play } from "lucide-react";
import { Link } from "react-router-dom";
import { useDesktopShell } from "../app/useDesktopShell";
import { buildParticipantShell } from "../app/shell";
import { SectionCard } from "../components/shared/SectionCard";
import { ScreenShell } from "./ScreenShell";
import type { ScreenProps } from "./types";

export function TournamentLobbyScreen({ bootstrap }: ScreenProps) {
  const { hostDraft, readySeats, recentJoinPayloads, toggleSeatReady } =
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
  const localSeat = participants.find((seat) => seat.isLocal);
  const localSeatReady = localSeat?.ready ?? false;
  const readySeatCount = activeSeats.filter((seat) => seat.ready).length;
  const seatsStillWaiting = activeSeats.filter((seat) => !seat.ready).length;
  const showsHostControlledSeats = bootstrap.debugToolsEnabled && activeSeats.some((seat) => !seat.isLocal);

  return (
    <ScreenShell
      title="Lobby"
      badges={[hostDraft.tournamentName, canStart ? "Ready to deal" : `${activeSeats.length}/${hostDraft.maxPlayers} seated`]}
      className="pregame-screen-shell"
    >
      <div className="pregame-workstation lobby-station-layout">
        <SectionCard kicker="Table" title="Ready room" className="workstation-main-card">
          <div className="lobby-workstation-grid">
            <article className="lobby-progress-card compact-lobby-progress">
              <strong>{canStart ? "Ready to start" : `${readySeatCount}/${activeSeats.length} ready`}</strong>
              <div className="lobby-status-row">
                <span className={`status-badge ${localSeatReady ? "success" : "info"}`}>
                  {localSeatReady ? <Check className="button-icon" strokeWidth={1.9} /> : <Clock3 className="button-icon" strokeWidth={1.9} />}
                  {localSeatReady ? "You: Ready" : "You: Waiting"}
                </span>
                <span className={`status-badge ${canStart ? "success" : "info"}`}>
                  {canStart ? <Check className="button-icon" strokeWidth={1.9} /> : <Clock3 className="button-icon" strokeWidth={1.9} />}
                  {canStart ? "Table: Ready" : `${seatsStillWaiting} waiting`}
                </span>
              </div>
              {showsHostControlledSeats ? (
                <p className="field-hint">Pending seats use host-controlled ready toggles in debug mode.</p>
              ) : null}
            </article>

            <div className="seat-grid lobby-seat-grid compact-lobby-seat-grid">
            {participants.map((seat) => {
              const seatState = seat.kind === "open" ? "Open" : seat.ready ? "Ready" : "Waiting";
              const readyButtonLabel = seat.isLocal
                ? seat.ready
                  ? "Ready"
                  : "Mark ready"
                : seat.ready
                  ? "Host marked ready"
                  : "Host marks ready";

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
                  <span className={`status-badge ${seat.kind === "open" ? "info" : seat.ready ? "success" : "info"}`}>
                    {seat.kind === "open" ? <Clock3 className="button-icon" strokeWidth={1.9} /> : seat.ready ? <Check className="button-icon" strokeWidth={1.9} /> : <Clock3 className="button-icon" strokeWidth={1.9} />}
                    {seatState}
                  </span>
                </div>
                {seat.kind !== "open" && seat.detail ? (
                  <p className="seat-detail">{seat.detail}</p>
                ) : null}
                {seat.kind !== "open" && (seat.isLocal || bootstrap.debugToolsEnabled) ? (
                  <button
                    className={seat.ready ? "primary-button compact-button" : "secondary-button compact-button"}
                    onClick={() => {
                      toggleSeatReady(seat.seatIndex);
                    }}
                    type="button"
                  >
                    {readyButtonLabel}
                  </button>
                ) : seat.kind !== "open" ? (
                  <p className="field-hint">Waiting on player readiness.</p>
                ) : null}
              </article>
            );})}
            </div>

            <section className="lobby-action-rail compact-status-panel">
              <strong>{canStart ? "Start when ready" : "Need 2 ready players"}</strong>
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
            </section>
          </div>
        </SectionCard>
      </div>
      {showLeaveFlow ? (
        <section className="dialog-card">
          <p className="kicker">Leave</p>
          <h3>Leave this lobby?</h3>
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
