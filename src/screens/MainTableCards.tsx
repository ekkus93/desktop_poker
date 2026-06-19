import { StatusBadge } from "../components/shared/StatusBadge";
import type { TableCardView, TableSeatView } from "../api/desktop";

type PlayingCardProps = {
  card: TableCardView | null;
  placeholderAriaLabel?: string;
  size: "board" | "local" | "compact";
};

export function PlayingCard({
  card,
  placeholderAriaLabel,
  size,
}: PlayingCardProps) {
  if (!card) {
    return (
      <div
        aria-label={placeholderAriaLabel}
        className={`card-slot ${size} empty-card-slot`}
        role="img"
      />
    );
  }

  const cornerRank = card.compactLabel.replace(card.suitSymbol, "");

  return (
    <div
      className={`playing-card ${size} ${card.tone}`}
      aria-label={card.label}
    >
      <div className="playing-card-corner top-left">
        <span>{card.compactLabel}</span>
        <small>{card.suitSymbol}</small>
      </div>
      <div className="playing-card-center" aria-hidden="true">
        <span>{card.suitSymbol}</span>
      </div>
      <div className="playing-card-corner bottom-right">
        <span>{cornerRank}</span>
        <small>{card.suitSymbol}</small>
      </div>
    </div>
  );
}

export function HiddenCard() {
  return <div className="hidden-card" aria-hidden="true" />;
}

export function SeatCard({
  seat,
  displayName,
}: {
  seat: TableSeatView;
  displayName: string;
}) {
  const seatLabel = seat.isLocal ? `${displayName} (you)` : seat.displayName;
  return (
    <article
      className={`seat-card enhanced-seat-card ${seat.isLocal ? "local-seat" : ""} ${seat.isActing ? "action-seat" : ""} ${seat.isObserver ? "observer-seat" : ""} ${seat.isEliminated ? "eliminated-seat" : ""} ${seat.isCompact ? "compact-seat" : ""}`}
    >
      <div className="seat-card-header">
        <div>
          <strong>Seat {seat.seatIndex}</strong>
          <span>{seatLabel}</span>
        </div>
        {seat.markerLabel ? (
          <StatusBadge>{seat.markerLabel}</StatusBadge>
        ) : null}
      </div>
      <p className="seat-status-line">
        <span>{seat.statusLabel}</span>
        {seat.chipCount !== null ? <span>{seat.chipCount} chips</span> : null}
      </p>
      <p className="seat-detail">
        {seat.isObserver
          ? "Watching public action"
          : seat.isEliminated
            ? "Eliminated from this hand"
            : `In for ${seat.contribution}`}
      </p>
      <div
        className={`seat-card-grid ${seat.isLocal ? "show-local-cards" : ""}`}
      >
        {seat.cardsHidden ? (
          <div
            className="hidden-card-row"
            aria-label={`Seat ${seat.seatIndex} hidden cards`}
          >
            <HiddenCard />
            <HiddenCard />
          </div>
        ) : (
          <div
            className="seat-hole-cards"
            aria-label={`Seat ${seat.seatIndex} hole cards`}
          >
            {seat.holeCards.map((card) => (
              <PlayingCard
                key={card.compactLabel}
                card={card}
                size={seat.isLocal ? "local" : "compact"}
              />
            ))}
          </div>
        )}
      </div>
      <div className="seat-notes-inline" role="note">
        {seat.detailLines.slice(0, 1).map((line) => (
          <p key={line} className="field-hint">
            {line}
          </p>
        ))}
      </div>
    </article>
  );
}
