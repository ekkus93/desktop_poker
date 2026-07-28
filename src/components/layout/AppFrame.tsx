import { useEffect, useRef, type ReactNode } from "react";
import {
  ChevronLeft,
  CircleHelp,
  Flag,
  History,
  Home,
  LogIn,
  Settings,
} from "lucide-react";
import { NavLink, useLocation } from "react-router";
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
    case "Help":
      return CircleHelp;
    case "Settings":
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
  const mainContentRef = useRef<HTMLElement>(null);
  const previousPathRef = useRef(location.pathname);
  const inTournament = ["/lobby", "/table", "/complete"].includes(
    location.pathname,
  );
  const supportRoutes = new Set(["/history", "/rules", "/settings"]);
  const supportNavigation = navigation.filter((item) =>
    supportRoutes.has(item.to),
  );
  const tournamentPlayRoutes = new Set(["/", "/lobby"]);
  const tournamentNavigation = navigation.filter(
    (item) =>
      tournamentPlayRoutes.has(item.to) && item.to !== location.pathname,
  );
  const currentSurfaceRoute = navigation.find(
    (item) => item.to === location.pathname,
  );
  const landingNavigation = navigation.filter(
    (item) => item.to === "/host" && item.to !== location.pathname,
  );

  useEffect(() => {
    if (previousPathRef.current === location.pathname) {
      return;
    }
    previousPathRef.current = location.pathname;
    const animationFrame = window.requestAnimationFrame(() => {
      mainContentRef.current?.focus();
    });
    return () => window.cancelAnimationFrame(animationFrame);
  }, [location.pathname]);

  if (!inTournament) {
    return (
      <div
        className="app-frame landing-frame"
        style={{ height: "100dvh", overflow: "hidden" }}
      >
        <a className="skip-link" href="#main-content">
          Skip to main content
        </a>
        <header className="topbar">
          <div className="topbar-brand">
            <p className="kicker">Desktop Poker</p>
            <h1>{bootstrap.appName}</h1>
          </div>

          {location.pathname === "/" ? (
            <div className="topbar-spacer" aria-hidden="true" />
          ) : (
            <nav aria-label="Desktop poker navigation" className="topbar-nav">
              <NavLink className="nav-link topbar-link" to="/">
                <span className="button-content">
                  <ChevronLeft
                    aria-hidden="true"
                    className="button-icon"
                    strokeWidth={1.8}
                  />
                  <span>Back home</span>
                </span>
              </NavLink>
              {landingNavigation.map((item) => (
                <NavLink
                  key={item.to}
                  className="nav-link topbar-link"
                  to={item.to}
                >
                  <span className="button-content">
                    {(() => {
                      const Icon = getNavigationIcon(item.label);
                      return Icon ? (
                        <Icon
                          aria-hidden="true"
                          className="button-icon"
                          strokeWidth={1.8}
                        />
                      ) : null;
                    })()}
                    <span>{item.label}</span>
                  </span>
                </NavLink>
              ))}
              {currentSurfaceRoute ? (
                <span className="topbar-current-surface">
                  {(() => {
                    const Icon = getNavigationIcon(currentSurfaceRoute.label);
                    return Icon ? (
                      <Icon
                        aria-hidden="true"
                        className="button-icon"
                        strokeWidth={1.8}
                      />
                    ) : null;
                  })()}
                  <span>{currentSurfaceRoute.label}</span>
                </span>
              ) : null}
            </nav>
          )}

          <div className="topbar-meta">
            <p className="player-pill">Playing as {displayName}</p>
          </div>
        </header>

        <main
          className="content"
          id="main-content"
          ref={mainContentRef}
          style={{ height: "100%", overflow: "hidden" }}
          tabIndex={-1}
        >
          {children}
        </main>
      </div>
    );
  }

  return (
    <div
      className="app-frame tournament-frame"
      style={{ height: "100dvh", overflow: "hidden" }}
    >
      <a className="skip-link" href="#main-content">
        Skip to main content
      </a>
      <header className="topbar tournament-topbar">
        <div className="topbar-brand">
          <p className="kicker">Desktop Poker</p>
          <h1>{bootstrap.appName}</h1>
        </div>

        <nav
          aria-label="Desktop poker navigation"
          className="topbar-nav tournament-play-nav"
        >
          {tournamentNavigation.map((item) => (
            <NavLink
              key={item.to}
              className={({ isActive }) =>
                isActive
                  ? "nav-link topbar-link active"
                  : "nav-link topbar-link"
              }
              to={item.to}
            >
              <span className="button-content">
                {(() => {
                  const Icon = getNavigationIcon(item.label);
                  return Icon ? (
                    <Icon
                      aria-hidden="true"
                      className="button-icon"
                      strokeWidth={1.8}
                    />
                  ) : null;
                })()}
                <span>{item.label}</span>
              </span>
            </NavLink>
          ))}
        </nav>

        <div className="topbar-meta tournament-topbar-meta">
          <nav
            aria-label="Desktop poker support navigation"
            className="topbar-support-links"
          >
            {supportNavigation.map((item) => (
              <NavLink
                key={item.to}
                className={({ isActive }) =>
                  isActive
                    ? "nav-link support-link active"
                    : "nav-link support-link"
                }
                to={item.to}
              >
                <span className="button-content">
                  {(() => {
                    const Icon = getNavigationIcon(item.label);
                    return Icon ? (
                      <Icon
                        aria-hidden="true"
                        className="button-icon"
                        strokeWidth={1.8}
                      />
                    ) : null;
                  })()}
                  <span>{item.label}</span>
                </span>
              </NavLink>
            ))}
          </nav>
          <p className="player-pill">Playing as {displayName}</p>
        </div>
      </header>

      <main
        className="content"
        id="main-content"
        ref={mainContentRef}
        style={{ height: "100%", overflow: "hidden" }}
        tabIndex={-1}
      >
        {children}
      </main>
    </div>
  );
}
