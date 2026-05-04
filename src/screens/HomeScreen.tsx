import { Link } from "react-router-dom";
import { Flag, History, LogIn, Settings } from "lucide-react";
import { useDesktopShell } from "../app/useDesktopShell";
import { SectionCard } from "../components/shared/SectionCard";
import { ScreenShell } from "./ScreenShell";
import type { ScreenProps } from "./types";

export function HomeScreen({ bootstrap }: ScreenProps) {
  const { displayName, hostDraft, recentJoinPayloads, persistedHandHistoryCount } =
    useDesktopShell();
  const hasSavedProgress = recentJoinPayloads.length > 0 || persistedHandHistoryCount > 0;
  const hasLaunchPayload = Boolean(bootstrap.launchJoinPayload || bootstrap.launchJoinPayloadError);
  const hasSavedHistory = persistedHandHistoryCount > 0;
  const hasSavedInvites = recentJoinPayloads.length > 0 || hasLaunchPayload;

  return (
    <ScreenShell
      title="Your next table"
      lead="Host a game here or join with an invite."
      badges={[]}
    >
      <div className="content-grid">
        <SectionCard kicker="Start here" title="Host or join">
          <div className="button-row">
            <Link className="primary-button" to="/host">
              <span className="button-content">
                <Flag className="button-icon" strokeWidth={1.9} />
                <span>Host Tournament</span>
              </span>
            </Link>
            <Link className="primary-button ghost-primary-button" to="/join">
              <span className="button-content">
                <LogIn className="button-icon" strokeWidth={1.9} />
                <span>Join Tournament</span>
              </span>
            </Link>
          </div>
          <div className="home-choice-grid">
            <article className="choice-card choice-card-primary">
              <p className="kicker">Host</p>
              <h3>Run the table from this computer</h3>
              <p>Set the tournament basics, share the invite, then start when players are ready.</p>
            </article>
            <article className="choice-card">
              <p className="kicker">Join</p>
              <h3>Paste an invite and sit down</h3>
              <p>Paste the invite you were given, confirm the table, then continue into the lobby.</p>
            </article>
          </div>
        </SectionCard>

        <SectionCard title="Continue where you left off">
          {hasSavedProgress ? (
            <div className="stacked-list compact-home-list">
              <article className="list-panel">
                <div>
                  <strong>{displayName}</strong>
                  <p className="field-hint">Saved host draft: {hostDraft.tournamentName}</p>
                </div>
              </article>
              {hasSavedHistory ? (
                <article className="list-panel">
                  <div>
                    <strong>{persistedHandHistoryCount} saved hand summaries</strong>
                    <p className="field-hint">Review the last settled hands from this device.</p>
                  </div>
                  <div className="button-row">
                    <Link className="secondary-button" to="/history">
                      <span className="button-content">
                        <History className="button-icon" strokeWidth={1.9} />
                        <span>Open Hand History</span>
                      </span>
                    </Link>
                  </div>
                </article>
              ) : null}
              {hasSavedInvites ? (
                <article className="list-panel">
                  <div>
                    <strong>Saved invites</strong>
                    <p className="field-hint">Jump back into a remembered invite or review the latest one.</p>
                  </div>
                  <div className="button-row">
                    <Link className="secondary-button" to="/join">
                      <span className="button-content">
                        <LogIn className="button-icon" strokeWidth={1.9} />
                        <span>Open Join Screen</span>
                      </span>
                    </Link>
                  </div>
                </article>
              ) : null}
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
                      <span className="button-content">
                        <LogIn className="button-icon" strokeWidth={1.9} />
                        <span>Review Invite</span>
                      </span>
                    </Link>
                  </div>
                </article>
              ) : null}
            </div>
          ) : (
            <p className="field-hint">No saved tables or invites yet.</p>
          )}
          <div className="button-row">
            <Link className="secondary-button" to="/rules">
              <span className="button-content">
                <Settings className="button-icon" strokeWidth={1.9} />
                <span>Game Help</span>
              </span>
            </Link>
          </div>
        </SectionCard>
      </div>
    </ScreenShell>
  );
}
