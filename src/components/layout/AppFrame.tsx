import type { ReactNode } from "react";
import { NavLink, useLocation } from "react-router-dom";
import type { DesktopBootstrapState } from "../../api/desktop";
import { useDesktopShell } from "../../app/useDesktopShell";
import { StatusBadge } from "../shared/StatusBadge";

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
  const { displayName, recentJoinPayloads } = useDesktopShell();
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
              ? "Stay focused on the table, check the standings, and recover cleanly if something goes wrong."
              : "Host a local game, join with a payload, and keep each instance separate."}
          </p>
        </header>

        <div className="badge-row">
          <StatusBadge tone="info">Protocol v{bootstrap.protocolVersion}</StatusBadge>
          <StatusBadge tone="success">Port {bootstrap.defaultHostPort}</StatusBadge>
          {bootstrap.debugToolsEnabled ? (
            <StatusBadge tone="warning">Debug tools enabled</StatusBadge>
          ) : (
            <StatusBadge tone="success">Production path only</StatusBadge>
          )}
        </div>

        <section className="sidebar-group">
          <p className="sidebar-section-label">Main actions</p>
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

        {inTournament ? (
          <section className="sidebar-focus-card">
            <p className="kicker">Current focus</p>
            <h2>Stay in the same flow</h2>
            <p>
              The main player path is lobby, table, then history. Everything else
              is secondary while a game is running.
            </p>
          </section>
        ) : null}

        <footer className="sidebar-footer">
          <p className="sidebar-note">
            Player <strong>{displayName}</strong>
          </p>
          <p className="sidebar-note">
            This window <strong>{bootstrap.instanceLabel}</strong>
          </p>
          {bootstrap.instanceLabel !== bootstrap.instanceId ? (
            <p className="sidebar-note">
              Internal ID <strong>{bootstrap.instanceId}</strong>
            </p>
          ) : null}
          <p className="sidebar-note">
            Profile folder: <span className="mono-value">{bootstrap.profileDirectory}</span>
          </p>
          <p className="footer-copy">
            Saved join payloads: {recentJoinPayloads.length}
          </p>
        </footer>
      </aside>

      <main className="content">{children}</main>
    </div>
  );
}
