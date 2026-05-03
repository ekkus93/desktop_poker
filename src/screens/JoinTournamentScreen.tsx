import { SectionCard } from "../components/shared/SectionCard";
import { ScreenShell } from "./ScreenShell";
import type { ScreenProps } from "./types";

export function JoinTournamentScreen({ bootstrap }: ScreenProps) {
  return (
    <ScreenShell
      title="Join Tournament"
      lead="M0 exposes the real input surfaces for a direct join payload while keeping actual validation and connection flow for protocol and networking milestones."
      badges={[bootstrap.joinPayloadEncoding, "CLI/env ready"]}
    >
      <div className="content-grid">
        <SectionCard kicker="Join flow" title="Join payload input">
          <div className="form-grid">
            <label className="field">
              Direct payload
              <textarea
                readOnly
                rows={6}
                value={
                  bootstrap.launchJoinPayload ??
                  "Paste a pkr1_ payload here or launch with --join-payload."
                }
              />
            </label>
          </div>
        </SectionCard>

        <SectionCard title="Validation stance">
          <ul>
            <li>Protocol version is frozen to Android-compatible v1.</li>
            <li>Raw TCP transport remains the only production runtime path.</li>
            <li>Room-code discovery stays out of the production UI until real.</li>
          </ul>
        </SectionCard>
      </div>
    </ScreenShell>
  );
}
