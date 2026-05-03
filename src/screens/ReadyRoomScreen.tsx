import { SectionCard } from "../components/shared/SectionCard";
import { ScreenShell } from "./ScreenShell";
import type { ScreenProps } from "./types";

export function ReadyRoomScreen({ bootstrap }: ScreenProps) {
  return (
    <ScreenShell
      title="Ready Room"
      lead="The ready room establishes a dedicated surface for ready-state toggles, start gating, and roster freeze messaging."
      badges={[`Protocol v${bootstrap.protocolVersion}`, "Ready room scaffold"]}
    >
      <div className="content-grid">
        <SectionCard kicker="Readiness" title="Start requirements">
          <ul>
            <li>2 to 10 seated participants</li>
            <li>Every occupied seat marked ready</li>
            <li>Host-authoritative tournament start</li>
            <li>Bound signing/encryption identities</li>
          </ul>
        </SectionCard>

        <SectionCard title="Leave and reconnect">
          <p>
            Later milestones will wire explicit reconnect and safe leave-table
            flows into this surface without changing the screen structure.
          </p>
        </SectionCard>
      </div>
    </ScreenShell>
  );
}
