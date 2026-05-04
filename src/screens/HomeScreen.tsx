import { Link } from "react-router-dom";
import { Flag, History, LogIn, Settings } from "lucide-react";
import { useDesktopShell } from "../app/useDesktopShell";
import { SectionCard } from "../components/shared/SectionCard";
import { ScreenShell } from "./ScreenShell";
import type { ScreenProps } from "./types";

export function HomeScreen({ bootstrap }: ScreenProps) {
  const { hostDraft, recentJoinPayloads, persistedHandHistoryCount } =
    useDesktopShell();
  const hasSavedProgress = recentJoinPayloads.length > 0 || persistedHandHistoryCount > 0;
  const hasLaunchPayload = Boolean(bootstrap.launchJoinPayload || bootstrap.launchJoinPayloadError);
  const hasSavedHistory = persistedHandHistoryCount > 0;
  const hasSavedInvites = recentJoinPayloads.length > 0 || hasLaunchPayload;

  return (
    <ScreenShell
      title="Choose a table"
      badges={[]}
      className="pregame-screen-shell home-screen-shell"
    >
      <div className={`home-stage${hasSavedProgress ? " has-recovery" : ""}`}>
        <SectionCard kicker="Tonight's game" title="Host or join" className="home-hero-card pregame-hero-card">
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
          <div className="button-row home-support-row compact-support-row">
            <Link className="secondary-button" to="/rules">
              <span className="button-content">
                <Settings className="button-icon" strokeWidth={1.9} />
                <span>Rules</span>
              </span>
            </Link>
          </div>
        </SectionCard>

        {hasSavedProgress ? (
          <SectionCard title="Resume" className="recovery-rail-card">
            <div className="recovery-rail">
              <article className="list-panel compact-list-panel">
                <div>
                  <strong>Host setup</strong>
                  <p className="field-hint">{hostDraft.tournamentName}</p>
                </div>
              </article>
              {hasSavedHistory ? (
                <article className="list-panel compact-list-panel">
                  <div>
                    <strong>{persistedHandHistoryCount} hands</strong>
                  </div>
                  <div className="button-row">
                    <Link className="secondary-button" to="/history">
                      <span className="button-content">
                        <History className="button-icon" strokeWidth={1.9} />
                        <span>History</span>
                      </span>
                    </Link>
                  </div>
                </article>
              ) : null}
              {hasSavedInvites ? (
                <article className="list-panel compact-list-panel">
                  <div>
                    <strong>Invites</strong>
                  </div>
                  <div className="button-row">
                    <Link className="secondary-button" to="/join">
                      <span className="button-content">
                        <LogIn className="button-icon" strokeWidth={1.9} />
                        <span>Join</span>
                      </span>
                    </Link>
                  </div>
                </article>
              ) : null}
              {hasLaunchPayload ? (
                <article className="list-panel compact-list-panel">
                  <div>
                    <strong>{bootstrap.launchJoinPayloadError ? "Invite needs attention" : "Invite ready"}</strong>
                    {bootstrap.launchJoinPayloadError ? (
                      <p className="field-hint">{bootstrap.launchJoinPayloadError}</p>
                    ) : null}
                  </div>
                  <div className="button-row">
                    <Link className="secondary-button" to="/join">
                      <span className="button-content">
                        <LogIn className="button-icon" strokeWidth={1.9} />
                        <span>Open</span>
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
