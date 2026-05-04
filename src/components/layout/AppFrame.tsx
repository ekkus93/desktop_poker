import type { ReactNode } from "react";
import { NavLink, useLocation } from "react-router-dom";
import type { DesktopBootstrapState } from "../../api/desktop";
import { useDesktopShell } from "../../app/useDesktopShell";

export type NavigationItem = {
  to: string;
  label: string;
};

export function AppFrame({
  bootstrap,
  navigation,
  children,
}: {
  bootstrap: DesktopBootstrapState;
  navigation: NavigationItem[];
  children: ReactNode;
}) {
  const { displayName } = useDesktopShell();
  const location = useLocation();
  const inTournament = ["/lobby", "/table", "/complete"].includes(location.pathname);
  const supportNavigation = navigation.filter((item) => item.to === "/history" || item.to === "/rules" || item.to === "/debug");
  const primaryNavigation = navigation.filter((item) => item.to !== "/history" && item.to !== "/rules" && item.to !== "/debug");

  return (
    <div className="app-frame">
      <aside className="sidebar">
        <header className="brand">
          <p className="kicker">Desktop Poker</p>
          <h1>{bootstrap.appName}</h1>
          <p>
            {inTournament
              ? "Play the hand, keep the table readable, and leave everything else secondary."
              : "Choose a seat at the table: host the game here or join with an invite."}
          </p>
        </header>

        <section className="sidebar-group">
          <p className="sidebar-section-label">Play</p>
          <nav aria-label="Desktop poker navigation">
            {primaryNavigation.map((item) => (
              <NavLink
                key={item.to}
                className={({ isActive }) =>
                  isActive ? "nav-link active" : "nav-link"
                }
                to={item.to}
              >
                {item.label}
              </NavLink>
            ))}
          </nav>
        </section>

        <section className="sidebar-group">
          <p className="sidebar-section-label">Support</p>
          <nav aria-label="Desktop poker support navigation">
            {supportNavigation.map((item) => (
              <NavLink
                key={item.to}
                className={({ isActive }) =>
                  isActive ? "nav-link active" : "nav-link"
                }
                to={item.to}
              >
                {item.label}
              </NavLink>
            ))}
          </nav>
        </section>

        <footer className="sidebar-footer">
          <p className="sidebar-note">
            Player <strong>{displayName}</strong>
          </p>
          {bootstrap.debugToolsEnabled ? <p className="footer-copy">Internal tools available in this build.</p> : null}
        </footer>
      </aside>

      <main className="content">{children}</main>
    </div>
  );
}
