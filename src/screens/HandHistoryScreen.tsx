import { SectionCard } from "../components/shared/SectionCard";
import { ScreenShell } from "./ScreenShell";
import type { ScreenProps } from "./types";

export function HandHistoryScreen({ bootstrap }: ScreenProps) {
  return (
    <ScreenShell
      title="Hand History"
      lead="A dedicated history route exists from day one so settled hands, elimination ordering, and replay-friendly state can slot in without changing navigation."
      badges={[bootstrap.serializationStrategy, "History route ready"]}
    >
      <div className="content-grid">
        <SectionCard kicker="History" title="Projected event feed">
          <ul>
            <li>Hand start and blind level metadata</li>
            <li>Street reveals and showdown summaries</li>
            <li>Pot settlement and elimination records</li>
          </ul>
        </SectionCard>
      </div>
    </ScreenShell>
  );
}
