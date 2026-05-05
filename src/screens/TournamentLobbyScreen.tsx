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
  const openSeatCount = hostDraft.maxPlayers - activeSeats.length;
  const showsHostControlledSeats = bootstrap.debugToolsEnabled && activeSeats.some((seat) => !seat.isLocal);

  return (
    <ScreenShell
      title="Lobby"
      badges={[hostDraft.tournamentName, canStart ? "Ready to deal" : `${activeSeats.length}/${hostDraft.maxPlayers} seated`]}
    >
      <div className="content-grid lobby-content-grid">
        <SectionCard kicker="Ready room" title="What to do now" className="lobby-brief-card">
          <p className="lobby-brief-copy">Mark players ready. Start when everyone you need is ready.</p>
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
          <div className="lobby-summary-grid">
            <div className="compact-status-panel">
              <strong>{readySeatCount}/{activeSeats.length} ready</strong>
              <p className="field-hint">Need at least 2 seated players, then everyone at the table ready.</p>
              {showsHostControlledSeats ? (
                <p className="field-hint">Pending seats use host-controlled ready toggles in debug mode.</p>
              ) : null}
            </div>
            <div className="compact-status-panel lobby-action-rail">
              <strong>{canStart ? "Ready to start" : "Start is locked until everyone is ready"}</strong>
              <div className="button-row workstation-actions compact-workstation-actions">
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
                    <span>Leave table</span>
                  </span>
                </button>
              </div>
            </div>
          </div>
        </SectionCard>

        <SectionCard title="Players at the table" className="lobby-roster-card">
          <div className="stacked-list compact-lobby-list">
            {activeSeats.map((seat) => {
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
                className={`list-panel compact-list-panel lobby-list-panel ${seat.isLocal ? "lobby-seat-local" : ""} ${seat.ready ? "lobby-seat-ready" : ""}`}
              >
                <div className="seat-card-header">
                  <div>
                    <strong>{seat.label}</strong>
                    <p className="field-hint">Seat {seat.seatIndex} · {seat.detail}</p>
                  </div>
                  <span className={`status-badge ${seat.ready ? "success" : "info"}`}>
                    {seat.ready ? <Check className="button-icon" strokeWidth={1.9} /> : <Clock3 className="button-icon" strokeWidth={1.9} />}
                    {seatState}
                  </span>
                </div>
                {(seat.isLocal || bootstrap.debugToolsEnabled) ? (
                  <button
                    className={seat.ready ? "primary-button compact-button" : "secondary-button compact-button"}
                    onClick={() => {
                      toggleSeatReady(seat.seatIndex);
                    }}
                    type="button"
                  >
                    {readyButtonLabel}
                  </button>
                ) : (
                  <p className="field-hint">Waiting on player readiness.</p>
                )}
              </article>
            );})}
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
