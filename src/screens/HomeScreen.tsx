import { Link } from "react-router-dom";
import { useDesktopShell } from "../app/useDesktopShell";
import { SectionCard } from "../components/shared/SectionCard";
import { ScreenShell } from "./ScreenShell";
import type { ScreenProps } from "./types";

export function HomeScreen({ bootstrap }: ScreenProps) {
  const { displayName, hostDraft, recentJoinPayloads, persistedHandHistoryCount } =
    useDesktopShell();
  const hasSavedProgress = recentJoinPayloads.length > 0 || persistedHandHistoryCount > 0;

  return (
    <ScreenShell
      title="Desktop Poker"
      lead="Choose one path: host a game here or join a game with a payload."
      badges={[bootstrap.frontendStack, `Instance ${bootstrap.instanceLabel}`]}
    >
      <div className="content-grid">
        <SectionCard kicker="Start here" title="Pick your path">
          <div className="button-row">
            <Link className="primary-button" to="/host">
              Host Tournament
            </Link>
            <Link className="primary-button ghost-primary-button" to="/join">
              Join Tournament
            </Link>
            {bootstrap.debugToolsEnabled ? (
              <Link className="secondary-button" to="/debug">
                Internal Tools
              </Link>
            ) : null}
          </div>
          <div className="home-choice-grid">
            <article className="choice-card choice-card-primary">
              <p className="kicker">Host</p>
              <h3>Run the table from this computer</h3>
              <p>Set the tournament basics, share the invite, then start when players are ready.</p>
            </article>
            <article className="choice-card">
              <p className="kicker">Join</p>
              <h3>Paste a payload and sit down</h3>
              <p>Use a <code>pkr1_</code> payload, confirm the destination, then continue into the lobby.</p>
            </article>
          </div>
        </SectionCard>

        <SectionCard title="This device">
          <ul>
            <li>
              <strong>Name:</strong> {displayName}
            </li>
            <li>
              <strong>Saved host draft:</strong> {hostDraft.tournamentName}
            </li>
            <li>
              <strong>Saved join payloads:</strong> {recentJoinPayloads.length}
            </li>
            <li>
              <strong>Saved hand summaries:</strong> {persistedHandHistoryCount}
            </li>
            <li>
              <strong>Profile folder:</strong>{" "}
              <span className="mono-value">{bootstrap.profileDirectory}</span>
            </li>
          </ul>
        </SectionCard>

        <SectionCard title="Saved progress">
          {hasSavedProgress ? (
            <div className="button-row">
              <Link className="secondary-button" to="/history">
                Open Hand History
              </Link>
              <Link className="secondary-button" to="/join">
                Open Join Screen
              </Link>
              <Link className="secondary-button" to="/rules#settings">
                Open Settings
              </Link>
            </div>
          ) : (
            <p className="field-hint">This device does not have any saved hands or remembered join payloads yet.</p>
          )}
        </SectionCard>

        <SectionCard title="Join payload status">
          {bootstrap.launchJoinPayload ? (
            <p>
              A launch payload is already loaded. Open Join Tournament to review it and continue.
            </p>
          ) : (
            <p>
              No launch payload is attached. Use Join Tournament when someone shares a payload with you.
            </p>
          )}
          {bootstrap.launchJoinPayloadError ? (
            <p className="inline-banner error">{bootstrap.launchJoinPayloadError}</p>
          ) : null}
          <div className="button-row">
            <Link className="secondary-button" to="/history">
              Hand History
            </Link>
            <Link className="secondary-button" to="/rules">
              Rules and Settings
            </Link>
          </div>
        </SectionCard>
      </div>
    </ScreenShell>
  );
}
