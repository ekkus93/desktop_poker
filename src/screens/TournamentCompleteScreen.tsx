import { Link } from "react-router-dom";
import { useDesktopShell } from "../app/useDesktopShell";
import { SectionCard } from "../components/shared/SectionCard";
import { ScreenShell } from "./ScreenShell";
import type { ScreenProps } from "./types";

export function TournamentCompleteScreen({ bootstrap }: ScreenProps) {
  const { displayName, hostDraft, recentJoinPayloads } = useDesktopShell();

  return (
    <ScreenShell
      title="Tournament Complete"
      lead="The completion route is ready for final standings, rematch actions, and return-to-home flow while staying aligned with backend-owned tournament truth."
      badges={[hostDraft.tournamentName, `Profile ${bootstrap.instanceId}`]}
    >
      <div className="content-grid">
        <SectionCard kicker="Standings" title="Completion surface">
          <p>
            Final placements will be rendered here once the Rust tournament
            coordinator publishes an authoritative complete state.
          </p>
          <ul>
            <li>Champion banner for {displayName}</li>
            <li>Placement list with elimination order</li>
            <li>Rematch / host again / return home actions</li>
          </ul>
        </SectionCard>

        <SectionCard title="What survives locally">
          <ul>
            <li>Last host settings for {hostDraft.tournamentName}</li>
            <li>{recentJoinPayloads.length} remembered join payloads</li>
            <li>Per-instance profile isolation for reconnect-safe state</li>
          </ul>
          <div className="button-row">
            <Link className="primary-button" to="/host">
              Host again
            </Link>
            <Link className="secondary-button" to="/history">
              Review history
            </Link>
            <Link className="secondary-button" to="/">
              Return home
            </Link>
          </div>
        </SectionCard>
      </div>
    </ScreenShell>
  );
}
