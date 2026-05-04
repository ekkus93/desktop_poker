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
      title="Pull up a chair"
      lead="Host the table here or join with an invite."
      badges={[]}
    >
      <div className={`content-grid home-screen-grid${hasSavedProgress ? " home-screen-grid-with-resume" : ""}`}>
        <SectionCard kicker="Tonight's game" title="Choose your seat" className="home-hero-card">
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
              <h3>Deal from this computer</h3>
              <p>Set the table, share the invite, then start when everyone is ready.</p>
            </article>
            <article className="choice-card">
              <p className="kicker">Join</p>
              <h3>Join with your invite</h3>
              <p>Paste the invite you were given, check the table, then head into the lobby.</p>
            </article>
          </div>
          <div className="button-row home-support-row">
            <Link className="secondary-button" to="/rules">
              <span className="button-content">
                <Settings className="button-icon" strokeWidth={1.9} />
                <span>Game Help</span>
              </span>
            </Link>
          </div>
        </SectionCard>

        {hasSavedProgress ? (
        <SectionCard title="Pick up the night">
            <div className="stacked-list compact-home-list">
              <article className="list-panel">
                <div>
                  <strong>{displayName}</strong>
                  <p className="field-hint">Saved host setup: {hostDraft.tournamentName}</p>
                </div>
              </article>
              {hasSavedHistory ? (
                <article className="list-panel">
                  <div>
                    <strong>{persistedHandHistoryCount} saved hand summaries</strong>
                    <p className="field-hint">Look back at the last settled hands from this device.</p>
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
                    <p className="field-hint">Jump back into a remembered table from the join screen.</p>
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
                    <strong>{bootstrap.launchJoinPayloadError ? "Invite needs attention" : "Invite ready"}</strong>
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
                        <span>Open Invite</span>
                      </span>
                    </Link>
                  </div>
                </article>
              ) : null}
            </div>
        </SectionCard>
        ) : null}
      </div>
    </ScreenShell>
  );
}
