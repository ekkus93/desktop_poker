import { Link } from "react-router-dom";
import { useDesktopShell } from "../app/useDesktopShell";
import { SectionCard } from "../components/shared/SectionCard";
import { ScreenShell } from "./ScreenShell";
import type { ScreenProps } from "./types";

export function HomeScreen({ bootstrap }: ScreenProps) {
  const { displayName, hostDraft, recentJoinPayloads, persistedHandHistoryCount } =
    useDesktopShell();
  const hasSavedProgress = recentJoinPayloads.length > 0 || persistedHandHistoryCount > 0;
  const hasLaunchPayload = Boolean(bootstrap.launchJoinPayload || bootstrap.launchJoinPayloadError);

  return (
    <ScreenShell
      title="Desktop Poker"
      lead="Host a game here or join a table with an invite."
      badges={[]}
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

        <SectionCard title="Resume here">
          {hasSavedProgress ? (
            <div className="stacked-list compact-home-list">
              <article className="list-panel">
                <div>
                  <strong>{displayName}</strong>
                  <p className="field-hint">Saved host draft: {hostDraft.tournamentName}</p>
                </div>
              </article>
              <article className="list-panel">
                <div>
                  <strong>{persistedHandHistoryCount} saved hand summaries</strong>
                  <p className="field-hint">Open saved results or jump back into a remembered invite.</p>
                </div>
                <div className="button-row">
                  <Link className="secondary-button" to="/history">
                    Open Hand History
                  </Link>
                  <Link className="secondary-button" to="/join">
                    Open Join Screen
                  </Link>
                </div>
              </article>
              {hasLaunchPayload ? (
                <article className="list-panel">
                  <div>
                    <strong>{bootstrap.launchJoinPayloadError ? "Invite needs attention" : "Invite ready to review"}</strong>
                    <p className="field-hint">
                      {bootstrap.launchJoinPayloadError
                        ? bootstrap.launchJoinPayloadError
                        : "A shared invite is already attached to this launch."}
                    </p>
                  </div>
                  <div className="button-row">
                    <Link className="secondary-button" to="/join">
                      Review Invite
                    </Link>
                  </div>
                </article>
              ) : null}
            </div>
          ) : (
            <p className="field-hint">No saved hands or remembered invites yet.</p>
          )}
          <div className="button-row">
            <Link className="secondary-button" to="/rules">
              Rules and Settings
            </Link>
          </div>
        </SectionCard>
      </div>
    </ScreenShell>
  );
}
