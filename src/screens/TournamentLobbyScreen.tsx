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
  const localSeatIndex = localSeat?.seatIndex ?? null;
  const nextStepCopy = canStart
    ? "Everyone seated is ready. Start the first hand whenever you want."
    : activeSeats.length < 2
      ? "Seat at least 2 players, then have each seated player mark ready."
      : localSeatReady
        ? `Waiting on ${seatsStillWaiting} player${seatsStillWaiting === 1 ? "" : "s"} to mark ready.`
        : "Mark yourself ready, then wait for every seated player to be ready.";

  return (
    <ScreenShell
      title="Lobby"
      badges={[hostDraft.tournamentName, canStart ? "Ready to deal" : `${activeSeats.length}/${hostDraft.maxPlayers} seated`]}
    >
      <div className="content-grid lobby-shell-grid">
        <SectionCard kicker="Ready room" title="Get this table ready" className="lobby-stage-card">
          <p className="lobby-brief-copy">
            This page is only for confirming who is playing, marking yourself ready,
            and starting the first hand. You do not need to fill every seat.
          </p>
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
          <p className="field-hint">{nextStepCopy}</p>
          <div className="lobby-stage-grid">
            <section className="compact-status-panel lobby-action-panel">
              <strong>What you do here</strong>
              <div className="stacked-list compact-lobby-list">
                <article className="list-panel compact-list-panel">
                  <strong>{localSeatReady ? "You are ready" : "Mark yourself ready"}</strong>
                  <p className="field-hint">Use this when you are ready to begin the tournament.</p>
                  <button
                    className={localSeatReady ? "primary-button compact-button" : "secondary-button compact-button"}
                    disabled={localSeatIndex === null}
                    onClick={() => {
                      if (localSeatIndex !== null) {
                        toggleSeatReady(localSeatIndex);
                      }
                    }}
                    type="button"
                  >
                    {localSeatReady ? "Ready" : "Mark ready"}
                  </button>
                </article>
                <article className="list-panel compact-list-panel">
                  <strong>{canStart ? "Start the tournament" : "Start is locked"}</strong>
                  <p className="field-hint">
                    Start unlocks after at least 2 players are seated and every seated player is ready.
                  </p>
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
                </article>
              </div>
              {showsHostControlledSeats ? (
                <p className="field-hint">Debug mode can mark the other seated players ready from the roster.</p>
              ) : null}
            </section>

            <section className="compact-status-panel lobby-roster-panel">
              <div className="lobby-roster-head">
                <div>
                  <strong>Players at the table</strong>
                  <p className="field-hint">
                    {activeSeats.length} seated. {openSeatCount > 0 ? `${openSeatCount} seats are still open.` : "All seats are filled."}
                  </p>
                </div>
                <span className="status-badge info">{readySeatCount}/{activeSeats.length} ready</span>
              </div>
              <div className="lobby-roster-grid">
                {activeSeats.map((seat) => {
                  const seatState = seat.ready ? "Ready" : "Waiting";
                  const readyButtonLabel = seat.ready ? "Host marked ready" : "Host marks ready";

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
                      {seat.isLocal ? (
                        <p className="field-hint">Your ready button is in the action panel.</p>
                      ) : bootstrap.debugToolsEnabled ? (
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
                  );
                })}
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
