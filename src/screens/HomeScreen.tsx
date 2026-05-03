import { Link } from "react-router-dom";
import { SectionCard } from "../components/shared/SectionCard";
import { ScreenShell } from "./ScreenShell";
import type { ScreenProps } from "./types";

export function HomeScreen({ bootstrap }: ScreenProps) {
  return (
    <ScreenShell
      title="Desktop Poker"
      lead="Rust owns the game and LAN authority; the frontend presents host, join, lobby, and table flows."
      badges={[bootstrap.frontendStack, `Port ${bootstrap.defaultHostPort}`]}
    >
      <div className="content-grid">
        <SectionCard kicker="Primary actions" title="Entry points">
          <div className="button-row">
            <Link className="primary-button" to="/host">
              Host Tournament
            </Link>
            <Link className="secondary-button" to="/join">
              Join Tournament
            </Link>
            <Link className="secondary-button" to="/rules">
              Rules
            </Link>
          </div>
        </SectionCard>

        <SectionCard title="Frozen implementation choices">
          <ul>
            <li>React + TypeScript shell inside Tauri 2.</li>
            <li>Rust-owned LAN TCP runtime and authoritative state projection.</li>
            <li>Length-prefixed JSON envelopes over raw TCP.</li>
            <li>Per-instance profile namespaces for multi-instance testing.</li>
          </ul>
        </SectionCard>
      </div>
    </ScreenShell>
  );
}
