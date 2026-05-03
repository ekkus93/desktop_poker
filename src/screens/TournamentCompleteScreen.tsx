import { SectionCard } from "../components/shared/SectionCard";
import { ScreenShell } from "./ScreenShell";
import type { ScreenProps } from "./types";

export function TournamentCompleteScreen({ bootstrap }: ScreenProps) {
  return (
    <ScreenShell
      title="Tournament Complete"
      lead="The completion route is ready for final standings, elimination order, and post-game actions once the tournament coordinator exists."
      badges={[`Profile ${bootstrap.instanceId}`, "Completion surface"]}
    >
      <div className="content-grid">
        <SectionCard kicker="Standings" title="Result shell">
          <ul>
            <li>Champion summary</li>
            <li>Placement list</li>
            <li>Return to lobby/home action</li>
          </ul>
        </SectionCard>
      </div>
    </ScreenShell>
  );
}
