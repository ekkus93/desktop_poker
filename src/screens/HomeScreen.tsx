import { Link } from "react-router-dom";
import { useDesktopShell } from "../app/useDesktopShell";
import { SectionCard } from "../components/shared/SectionCard";
import { ScreenShell } from "./ScreenShell";
import type { ScreenProps } from "./types";

export function HomeScreen({ bootstrap }: ScreenProps) {
  const { displayName, hostDraft, recentJoinPayloads } = useDesktopShell();

  return (
    <ScreenShell
      title="Desktop Poker"
      lead="Start a real LAN host or join flow, inspect product rules, and keep debug-only tooling separated from the production path."
      badges={[bootstrap.frontendStack, `Instance ${bootstrap.instanceId}`]}
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
            <Link className="secondary-button" to="/rules#settings">
              Settings
            </Link>
            {bootstrap.debugToolsEnabled ? (
              <Link className="secondary-button" to="/debug">
                Internal Tools
              </Link>
            ) : null}
          </div>
          <p className="field-hint">
            The default runtime path stays on real LAN TCP. No simulator mode is
            exposed in the production UI.
          </p>
        </SectionCard>

        <SectionCard title="Current shell snapshot">
          <ul>
            <li>
              <strong>Display name:</strong> {displayName}
            </li>
            <li>
              <strong>Next host draft:</strong> {hostDraft.tournamentName}
            </li>
            <li>
              <strong>Remembered join payloads:</strong> {recentJoinPayloads.length}
            </li>
            <li>
              <strong>Profile namespace:</strong>{" "}
              <span className="mono-value">{bootstrap.profileDirectory}</span>
            </li>
          </ul>
        </SectionCard>

        <SectionCard title="Launch payload and runtime stance">
          {bootstrap.launchJoinPayload ? (
            <p>
              A direct join payload was provided at launch and is ready on the
              Join screen.
            </p>
          ) : (
            <p>
              No launch payload is currently attached. Paste a <code>pkr1_</code>{" "}
              payload on the Join screen or pass one from CLI/deep-link entry.
            </p>
          )}
          {bootstrap.launchJoinPayloadError ? (
            <p className="inline-banner error">{bootstrap.launchJoinPayloadError}</p>
          ) : null}
          <ul>
            <li>Real TCP transport only.</li>
            <li>Room-code discovery remains hidden until implemented.</li>
            <li>Debug/internal tools stay behind debug builds only.</li>
          </ul>
        </SectionCard>
      </div>
    </ScreenShell>
  );
}
