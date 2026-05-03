import { JoinPayloadCard } from "../components/host/JoinPayloadCard";
import { SectionCard } from "../components/shared/SectionCard";
import { ScreenShell } from "./ScreenShell";
import type { ScreenProps } from "./types";

export function HostTournamentSetupScreen({ bootstrap }: ScreenProps) {
  return (
    <ScreenShell
      title="Host Tournament Setup"
      lead="The host flow is scaffolded around the real runtime contract that later milestones will connect to TCP hosting, payload generation, and tournament orchestration."
      badges={["Host setup", "Tauri bridge wired"]}
    >
      <div className="content-grid">
        <SectionCard kicker="Host configuration" title="Setup form skeleton">
          <div className="form-grid">
            <label className="field">
              Tournament name
              <input readOnly value="Friday Night Sit 'n Go" />
            </label>
            <label className="field">
              Max players
              <select value="6" disabled>
                <option>6 players</option>
              </select>
            </label>
            <label className="field">
              Starting stack
              <select value="1500" disabled>
                <option>1500 chips</option>
              </select>
            </label>
            <label className="field">
              Host port
              <input readOnly value={bootstrap.defaultHostPort} />
            </label>
          </div>
        </SectionCard>

        <JoinPayloadCard bootstrap={bootstrap} />
      </div>
    </ScreenShell>
  );
}
