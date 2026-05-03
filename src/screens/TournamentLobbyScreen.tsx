import { SectionCard } from "../components/shared/SectionCard";
import { ScreenShell } from "./ScreenShell";
import type { ScreenProps } from "./types";

export function TournamentLobbyScreen({ bootstrap }: ScreenProps) {
  return (
    <ScreenShell
      title="Tournament Lobby"
      lead="The lobby route exists now so the later host/join runtime can transition into seating, readiness, and roster freeze without reshaping the frontend."
      badges={[`Instance ${bootstrap.instanceId}`, "Lobby ready"]}
    >
      <div className="content-grid">
        <SectionCard kicker="Seat map" title="Seating scaffold">
          <div className="seat-grid">
            {Array.from({ length: 6 }, (_, index) => (
              <article key={index} className="seat-card">
                <strong>Seat {index + 1}</strong>
                <span>{index < 3 ? "Reserved for participant state" : "Open"}</span>
              </article>
            ))}
          </div>
        </SectionCard>

        <SectionCard title="Participant registry">
          <ul>
            <li>Registry state stays separate from seat occupancy.</li>
            <li>Ready state changes will flow through Rust commands/events.</li>
            <li>Roster freeze happens only when tournament start succeeds.</li>
          </ul>
        </SectionCard>
      </div>
    </ScreenShell>
  );
}
