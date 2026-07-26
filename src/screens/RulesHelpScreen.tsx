import { useEffect } from "react";
import { useLocation } from "react-router";
import { SectionCard } from "../components/shared/SectionCard";
import { ScreenShell } from "./ScreenShell";

export function RulesHelpScreen() {
  const location = useLocation();
  const context =
    (location.state as { context?: string } | null)?.context ?? null;

  useEffect(() => {
    const scrollTo = (id: string) => {
      const el = document.getElementById(id);
      if (el?.scrollIntoView) {
        el.scrollIntoView({ behavior: "smooth", block: "start" });
      }
    };

    if (context === "join-failure") {
      scrollTo("help-join-flow");
    } else if (context === "lan-error") {
      scrollTo("help-host-flow");
    }
  }, [context]);

  return (
    <ScreenShell
      title="Help"
      badges={["Help"]}
      className="support-screen-shell"
    >
      <div className="help-grid">
        <SectionCard
          kicker="Rules"
          title="Table basics"
          className="support-card"
        >
          <ul>
            <li>Single-table Sit 'n Go No-Limit Texas Hold'em</li>
            <li>2 to 10 players with equal starting stacks</li>
            <li>Everyone joins the same host on the local network</li>
            <li>Busted players stay to watch as observers</li>
          </ul>
        </SectionCard>

        <div id="help-host-flow">
          <SectionCard
            title="Host flow"
            className={`support-card${context === "lan-error" ? " help-section-highlighted" : ""}`}
          >
            <ul>
              <li>
                Start on Host Tournament and confirm the LAN address is ready.
              </li>
              <li>Share the invite card with everyone joining the table.</li>
              <li>
                Open the lobby only after the host machine shows a reachable
                address.
              </li>
              <li>
                If LAN detection fails, check that the device is connected to
                the same network as the other players.
              </li>
            </ul>
          </SectionCard>
        </div>

        <div id="help-join-flow">
          <SectionCard
            title="Join flow"
            className={`support-card${context === "join-failure" ? " help-section-highlighted" : ""}`}
          >
            <ul>
              <li>
                The host shares an invite from the host screen — ask for the
                invite code starting with <code>pkr1_</code>.
              </li>
              <li>
                Paste that invite on Join Tournament and click Check invite.
              </li>
              <li>Check the table preview, then continue into the lobby.</li>
              <li>
                If the join fails, confirm the host is still running and sharing
                the same invite.
              </li>
            </ul>
          </SectionCard>
        </div>

        <SectionCard title="Observers and history" className="support-card">
          <ul>
            <li>Eliminated players remain connected as public observers.</li>
            <li>
              Hand History keeps saved summaries on this device for offline
              review.
            </li>
            <li>
              Settings stores your local name, host draft, and recent invites.
            </li>
          </ul>
        </SectionCard>
      </div>
    </ScreenShell>
  );
}
