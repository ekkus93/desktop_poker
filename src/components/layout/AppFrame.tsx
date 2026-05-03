import type { ReactNode } from "react";
import { NavLink } from "react-router-dom";
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

  return (
    <div className="app-frame">
      <aside className="sidebar">
        <header className="brand">
          <p className="kicker">M6 frontend shell</p>
          <h1>{bootstrap.appName}</h1>
          <p>
            Real LAN hosting and join flows stay Rust-owned; the React shell now
            exposes the full route map without falling back to a simulator path.
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

        <nav aria-label="Desktop poker screens">
          {navigation.map((item) => (
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

        <footer className="sidebar-footer">
          <p className="sidebar-note">
            Display name <strong>{displayName}</strong>
          </p>
          <p className="sidebar-note">
            Instance <strong>{bootstrap.instanceLabel}</strong>
          </p>
          {bootstrap.instanceLabel !== bootstrap.instanceId ? (
            <p className="sidebar-note">
              Profile ID <strong>{bootstrap.instanceId}</strong>
            </p>
          ) : null}
          <p className="sidebar-note">
            Profile namespace: <span className="mono-value">{bootstrap.profileDirectory}</span>
          </p>
          <p className="footer-copy">
            Recent direct payloads remembered locally: {recentJoinPayloads.length}
          </p>
        </footer>
      </aside>

      <main className="content">{children}</main>
    </div>
  );
}
