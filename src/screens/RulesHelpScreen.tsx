import { SectionCard } from "../components/shared/SectionCard";
import { ScreenShell } from "./ScreenShell";
import type { ScreenProps } from "./types";

export function RulesHelpScreen({ bootstrap }: ScreenProps) {
  return (
    <ScreenShell
      title="Rules / Help"
      lead="This help surface captures the fixed product rules so the desktop shell reflects the same tournament semantics as the Android implementation."
      badges={[`Port ${bootstrap.defaultHostPort}`, "Sit 'n Go MVP"]}
    >
      <div className="content-grid">
        <SectionCard kicker="Rules" title="MVP game model">
          <ul>
            <li>Single-table Sit 'n Go No-Limit Texas Hold'em</li>
            <li>2 to 10 players</li>
            <li>Host-authoritative LAN TCP gameplay</li>
            <li>Eliminated players remain read-only observers</li>
          </ul>
        </SectionCard>

        <SectionCard title="Networking help">
          <ul>
            <li>Direct payload join is the canonical v1 path.</li>
            <li>Room-code discovery is not exposed in production UI.</li>
            <li>Multiple instances can run on one machine.</li>
          </ul>
        </SectionCard>
      </div>
    </ScreenShell>
  );
}
