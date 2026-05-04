import { useState } from "react";
import { Link } from "react-router-dom";
import { useDesktopShell } from "../app/useDesktopShell";
import { buildParticipantShell } from "../app/shell";
import { SectionCard } from "../components/shared/SectionCard";
import { ScreenShell } from "./ScreenShell";
import type { ScreenProps } from "./types";

export function ReadyRoomScreen({ bootstrap }: ScreenProps) {
  const { hostDraft, readySeats, recentJoinPayloads, toggleSeatReady } =
    useDesktopShell();
  const [showLeaveFlow, setShowLeaveFlow] = useState(false);
  const participants = buildParticipantShell(
    bootstrap,
    hostDraft,
    readySeats,
    recentJoinPayloads,
  ).filter((seat) => seat.kind !== "open");
  const allReady = participants.length >= 2 && participants.every((seat) => seat.ready);

  return (
    <ScreenShell
      title="Ready Room"
      badges={[`Turn timer ${hostDraft.turnTimerSeconds}s`, `${participants.length} participants` ]}
    >
      <div className="content-grid">
        <SectionCard kicker="Readiness" title="Participant readiness">
          <div className="stacked-list">
            {participants.map((seat) => (
              <article key={seat.seatIndex} className="list-panel">
                <div>
                  <strong>{seat.label}</strong>
                  <p className="field-hint">Seat {seat.seatIndex} · {seat.detail}</p>
                </div>
                <button
                  className={seat.ready ? "primary-button compact-button" : "secondary-button compact-button"}
                  onClick={() => {
                    toggleSeatReady(seat.seatIndex);
                  }}
                  type="button"
                >
                  {seat.ready ? "Ready" : "Not ready"}
                </button>
              </article>
            ))}
          </div>
        </SectionCard>

        <SectionCard title="Before start">
          <ul>
            <li>The seated roster locks when the game starts.</li>
            <li>New players cannot join after the table is running.</li>
            <li>Disconnected players must reconnect instead of joining again.</li>
          </ul>
          <div className="button-row">
            {allReady ? (
              <Link className="primary-button" to="/table">
                Start tournament
              </Link>
            ) : (
              <button className="primary-button" disabled type="button">
                Start tournament
              </button>
            )}
            <Link className="secondary-button" to="/lobby">
              Back to lobby
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
          {!allReady ? (
            <p className="field-hint">All seats must be ready.</p>
          ) : null}
        </SectionCard>
      </div>
      {showLeaveFlow ? (
        <section className="dialog-card">
          <p className="kicker">Leave table flow</p>
          <h3>Leave before start?</h3>
          <p>
            Leave this table and return home?
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
              Stay ready
            </button>
          </div>
        </section>
      ) : null}
    </ScreenShell>
  );
}
