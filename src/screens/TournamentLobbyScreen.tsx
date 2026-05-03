import { useState } from "react";
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

  return (
    <ScreenShell
      title="Tournament Lobby"
      lead="Seat assignment, participant visibility, ready toggles, and host start controls live here before the tournament moves into the main table flow."
      badges={[hostDraft.tournamentName, `${activeSeats.length}/${hostDraft.maxPlayers} seated` ]}
    >
      <div className="content-grid">
        <SectionCard kicker="Seat map" title="Lobby seating">
          <div className="seat-grid">
            {participants.map((seat) => (
              <article key={seat.seatIndex} className="seat-card">
                <strong>Seat {seat.seatIndex}</strong>
                <span>{seat.label}</span>
                <p className="seat-detail">{seat.detail}</p>
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
            ))}
          </div>
        </SectionCard>

        <SectionCard title="Participant list">
          <div className="stacked-list">
            {activeSeats.map((seat) => (
              <article key={seat.seatIndex} className="list-panel">
                <div>
                  <strong>{seat.label}</strong>
                  <p className="field-hint">{seat.detail}</p>
                </div>
                <span className={`status-badge ${seat.ready ? "success" : "warning"}`}>
                  {seat.ready ? "Ready" : "Waiting"}
                </span>
              </article>
            ))}
          </div>
        </SectionCard>

        <SectionCard title="Host controls">
          <p>
            Roster freeze happens only after the host starts the tournament and
            the Rust backend confirms the seated, ready roster.
          </p>
          <div className="button-row">
            {canStart ? (
              <Link className="primary-button" to="/table">
                Start tournament
              </Link>
            ) : (
              <button className="primary-button" disabled type="button">
                Start tournament
              </button>
            )}
            <Link className="secondary-button" to="/ready-room">
              Open ready room
            </Link>
            <button
              className="secondary-button"
              onClick={() => {
                setShowLeaveFlow(true);
              }}
              type="button"
            >
              Leave table
            </button>
          </div>
          {!canStart ? (
            <p className="field-hint">
              At least two occupied seats must be marked ready before the host can start.
            </p>
          ) : null}
        </SectionCard>
      </div>
      {showLeaveFlow ? (
        <section className="dialog-card">
          <p className="kicker">Leave table flow</p>
          <h3>Leave this lobby?</h3>
          <p>
            Leaving drops the current local shell state and returns you to the
            home flow. A real runtime disconnect command will slot into this action.
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
