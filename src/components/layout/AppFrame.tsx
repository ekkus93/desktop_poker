import type { ReactNode } from "react";
import { NavLink } from "react-router-dom";
import type { DesktopBootstrapState } from "../../api/desktop";
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
  return (
    <div className="app-frame">
      <aside className="sidebar">
        <header className="brand">
          <p className="kicker">M0 scaffold</p>
          <h1>{bootstrap.appName}</h1>
          <p>
            Tauri + Rust LAN runtime shell with React-rendered screens and a
            Rust-owned authority boundary.
          </p>
        </header>

        <div className="badge-row">
          <StatusBadge tone="info">Protocol v{bootstrap.protocolVersion}</StatusBadge>
          <StatusBadge tone="success">Port {bootstrap.defaultHostPort}</StatusBadge>
          {bootstrap.debugToolsEnabled ? (
            <StatusBadge tone="warning">Debug tools enabled</StatusBadge>
          ) : null}
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
            Instance <strong>{bootstrap.instanceId}</strong>
          </p>
          <p className="sidebar-note">
            Profile namespace:{" "}
            <span className="mono-value">{bootstrap.profileDirectory}</span>
          </p>
          <p className="footer-copy">
            Direct join payloads come from the Rust bootstrap layer via CLI or
            environment input.
          </p>
        </footer>
      </aside>

      <main className="content">{children}</main>
    </div>
  );
}
