import type { ParticipantShell } from "../../app/shell";

export function TablePlaceholder({
  participants,
  blindLabel,
  turnTimerSeconds,
}: {
  participants: ParticipantShell[];
  blindLabel: string;
  turnTimerSeconds: number;
}) {
  return (
    <section className="table-surface">
      <p className="kicker">Main table surface</p>
      <h3>Main table layout shell</h3>
      <p>
        Public board state, seat markers, and action controls render here once
        the Rust runtime publishes authoritative table projections.
      </p>

      <div className="community-cards">
        {Array.from({ length: 5 }, (_, index) => (
          <div key={index} className="card-slot">
            Board {index + 1}
          </div>
        ))}
      </div>

      <div className="table-toolbar">
        <span className="status-badge info">Blind level {blindLabel}</span>
        <span className="status-badge warning">Turn timer {turnTimerSeconds}s</span>
      </div>

      <div className="seat-grid">
        {participants.map((seat) => (
          <article
            key={seat.seatIndex}
            className={`seat-card ${seat.seatIndex === 1 ? "window-indicator" : ""}`}
          >
            <strong>Seat {seat.seatIndex}</strong>
            <span>{seat.label}</span>
            <p className="seat-detail">{seat.detail}</p>
          </article>
        ))}
      </div>
    </section>
  );
}
