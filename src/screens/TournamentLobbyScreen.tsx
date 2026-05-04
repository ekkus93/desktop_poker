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
  const localSeat = participants.find((seat) => seat.kind !== "open" && seat.label === displayName);
  const localSeatReady = localSeat?.ready ?? false;

  return (
    <ScreenShell
      title="Tournament Lobby"
      lead="Wait here until everyone is seated and ready, then start from this screen."
      badges={[hostDraft.tournamentName, `${activeSeats.length}/${hostDraft.maxPlayers} seated` ]}
    >
      <div className="content-grid">
        <SectionCard kicker="Ready state" title="What happens next?">
          <div className="stacked-list">
            <article className="list-panel">
              <div>
                <strong>{localSeatReady ? "You are ready" : "You are not ready yet"}</strong>
                <p className="field-hint">
                  {localSeatReady
                    ? "Wait for the rest of the table, or start when everyone is ready."
                    : "Mark your seat ready before the game can start."}
                </p>
              </div>
            </article>
            <article className="list-panel">
              <div>
                <strong>{canStart ? "The game can start now" : "The game cannot start yet"}</strong>
                <p className="field-hint">
                  {canStart
                    ? "At least two players are seated and every occupied seat is ready."
                    : "Every occupied seat must be marked ready before the host starts the game."}
                </p>
              </div>
            </article>
          </div>
        </SectionCard>

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

        <SectionCard title="Start from here">
          <p>The game can start once at least two seated players are ready.</p>
          <ul>
            <li>The seated roster locks when the game starts.</li>
            <li>New players cannot join after the table is running.</li>
            <li>Disconnected players must reconnect instead of joining again.</li>
          </ul>
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
              Mark each seated player ready, then start from here.
            </p>
          ) : null}
        </SectionCard>

        <SectionCard title="Shared state">
          <ul>
            <li>
              <strong>Remembered payloads:</strong> {recentJoinPayloads.length}
            </li>
            <li>
              <strong>Host:</strong> {displayName}
            </li>
            <li>
              <strong>Table:</strong> {hostDraft.tournamentName}
            </li>
          </ul>
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
