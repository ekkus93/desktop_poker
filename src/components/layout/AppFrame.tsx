import type { ReactNode } from "react";
import { ChevronLeft, Flag, History, Home, LogIn, Settings } from "lucide-react";
import { NavLink, useLocation } from "react-router-dom";
import type { DesktopBootstrapState } from "../../api/desktop";
import { useDesktopShell } from "../../app/useDesktopShell";

export type NavigationItem = {
  to: string;
  label: string;
};

function getNavigationIcon(label: string) {
  switch (label) {
    case "Home":
      return Home;
    case "Host":
      return Flag;
    case "Join":
      return LogIn;
    case "History":
      return History;
    case "Rules":
      return Settings;
    default:
      return null;
  }
}

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
  const supportNavigation = navigation.filter((item) => item.to === "/history" || item.to === "/rules");
  const primaryNavigation = navigation.filter((item) => item.to !== "/history" && item.to !== "/rules");
  const currentPrimaryRoute = primaryNavigation.find((item) => item.to === location.pathname);

  if (!inTournament) {
    return (
      <div className="app-frame landing-frame">
        <header className="topbar">
          <div className="topbar-brand">
            <p className="kicker">Desktop Poker</p>
            <h1>{bootstrap.appName}</h1>
          </div>

          {location.pathname === "/" ? <div className="topbar-spacer" aria-hidden="true" /> : (
            <nav aria-label="Desktop poker navigation" className="topbar-nav">
              <NavLink className="nav-link topbar-link" to="/">
                <span className="button-content">
                  <ChevronLeft className="button-icon" strokeWidth={1.8} />
                  <span>Back home</span>
                </span>
              </NavLink>
              {currentPrimaryRoute ? (
                <span className="topbar-current-surface">
                  {(() => {
                    const Icon = getNavigationIcon(currentPrimaryRoute.label);
                    return Icon ? <Icon className="button-icon" strokeWidth={1.8} /> : null;
                  })()}
                  <span>{currentPrimaryRoute.label}</span>
                </span>
              ) : null}
            </nav>
          )}

          <div className="topbar-meta">
            <p className="player-pill">Playing as {displayName}</p>
          </div>
        </header>

        <main className="content">{children}</main>
      </div>
    );
  }

  return (
    <div className="app-frame">
      <aside className="sidebar">
        <header className="brand">
          <p className="kicker">Desktop Poker</p>
          <h1>{bootstrap.appName}</h1>
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
                <span className="button-content">
                  {(() => {
                    const Icon = getNavigationIcon(item.label);
                    return Icon ? <Icon className="button-icon" strokeWidth={1.8} /> : null;
                  })()}
                  <span>{item.label}</span>
                </span>
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
                <span className="button-content">
                  {(() => {
                    const Icon = getNavigationIcon(item.label);
                    return Icon ? <Icon className="button-icon" strokeWidth={1.8} /> : null;
                  })()}
                  <span>{item.label}</span>
                </span>
              </NavLink>
            ))}
          </nav>
        </section>

        <footer className="sidebar-footer">
          <p className="sidebar-note">
            Player <strong>{displayName}</strong>
          </p>
        </footer>
      </aside>

      <main className="content">{children}</main>
    </div>
  );
}
