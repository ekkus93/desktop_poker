export function TablePlaceholder() {
  const seats = Array.from({ length: 9 }, (_, index) => index + 1);

  return (
    <section className="table-surface">
      <p className="kicker">Table rendering scaffold</p>
      <h3>Main table layout shell</h3>
      <p>
        The frontend already has a dedicated table rendering surface, but the
        gameplay state projection stays in Rust as later milestones fill in
        engine and protocol data.
      </p>

      <div className="community-cards">
        {Array.from({ length: 5 }, (_, index) => (
          <div key={index} className="card-slot">
            Board {index + 1}
          </div>
        ))}
      </div>

      <div className="seat-grid">
        {seats.map((seat) => (
          <article
            key={seat}
            className={`seat-card ${seat === 5 ? "window-indicator" : ""}`}
          >
            <strong>Seat {seat}</strong>
            <span>{seat === 5 ? "Acting player window" : "Waiting for roster"}</span>
          </article>
        ))}
      </div>
    </section>
  );
}
