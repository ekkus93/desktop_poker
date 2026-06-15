import { useEffect, useState, type ReactNode } from "react";
import { Navigate, Route, Routes } from "react-router-dom";
import { DebugPanel } from "../components/debug/DebugPanel";
import { AppFrame } from "../components/layout/AppFrame";
import {
  getClientSessionStatus,
  getHostSessionStatus,
  type ClientSessionStatus,
  type HostSessionStatus,
} from "../api/desktop";
import { DesktopShellProvider } from "./DesktopShellProvider";
import { DeviceSettingsScreen } from "../screens/DeviceSettingsScreen";
import { ErrorStateScreen } from "../screens/ErrorStateScreen";
import { HandHistoryScreen } from "../screens/HandHistoryScreen";
import { HomeScreen } from "../screens/HomeScreen";
import { HostTournamentSetupScreen } from "../screens/HostTournamentSetupScreen";
import { JoinTournamentScreen } from "../screens/JoinTournamentScreen";
import { MainTableScreen } from "../screens/MainTableScreen";
import { RulesHelpScreen } from "../screens/RulesHelpScreen";
import { TournamentCompleteScreen } from "../screens/TournamentCompleteScreen";
import { TournamentLobbyScreen } from "../screens/TournamentLobbyScreen";
import { NpcProfilesScreen } from "../screens/NpcProfilesScreen";
import { useDesktopBootstrap } from "./useDesktopBootstrap";

function hasScreenRoute(
  bootstrap: NonNullable<ReturnType<typeof useDesktopBootstrap>["bootstrap"]>,
  route: string,
) {
  return bootstrap.screens.some((screen) => screen.route === route);
}

type SessionRouteMode = "lobby" | "table";

function LiveSessionRoute({
  children,
  mode,
}: {
  children: ReactNode;
  mode: SessionRouteMode;
}) {
  const [state, setState] = useState<{
    status: "loading" | "ready";
    redirectTo: string | null;
  }>({
    status: "loading",
    redirectTo: null,
  });

  useEffect(() => {
    let active = true;

    void Promise.all([getHostSessionStatus(), getClientSessionStatus()])
      .then(([hostSession, clientSession]) => {
        if (!active) {
          return;
        }

        const liveSession: HostSessionStatus | ClientSessionStatus | null =
          hostSession ?? clientSession;

        if (!liveSession) {
          setState({ status: "ready", redirectTo: "/" });
          return;
        }

        if (mode === "table" && liveSession.phase !== "running") {
          setState({ status: "ready", redirectTo: "/lobby" });
          return;
        }

        setState({ status: "ready", redirectTo: null });
      })
      .catch(() => {
        if (active) {
          setState({ status: "ready", redirectTo: "/errors" });
        }
      });

    return () => {
      active = false;
    };
  }, [mode]);

  if (state.status === "loading") {
    return (
      <main className="loading-shell">
        <section className="loading-card">
          <p className="kicker">Checking session</p>
          <h2>Verifying live table state</h2>
          <p>
            Waiting for the desktop runtime to confirm the current session
            route.
          </p>
        </section>
      </main>
    );
  }

  if (state.redirectTo) {
    return <Navigate replace to={state.redirectTo} />;
  }

  return <>{children}</>;
}

export function AppShell() {
  const { bootstrap, error, loading } = useDesktopBootstrap();

  if (loading) {
    return (
      <main className="loading-shell">
        <section className="loading-card">
          <p className="kicker">Bootstrapping</p>
          <h2>Loading desktop runtime contract</h2>
          <p>
            Waiting for the Rust backend to publish the initial app state and
            per-instance profile metadata.
          </p>
        </section>
      </main>
    );
  }

  if (error || !bootstrap) {
    return (
      <main className="error-shell">
        <section className="error-card">
          <p className="kicker">Backend bridge error</p>
          <h2>Desktop bootstrap unavailable</h2>
          <p>
            {error ?? "No bootstrap payload was returned by the Rust backend."}
          </p>
        </section>
      </main>
    );
  }

  const routeTitles = new Map(
    bootstrap.screens.map((screen) => [screen.route, screen.title]),
  );
  const homeRouteAvailable = hasScreenRoute(bootstrap, "/");
  const hostRouteAvailable = hasScreenRoute(bootstrap, "/host");
  const joinRouteAvailable = hasScreenRoute(bootstrap, "/join");
  const lobbyRouteAvailable = hasScreenRoute(bootstrap, "/lobby");
  const tableRouteAvailable = hasScreenRoute(bootstrap, "/table");
  const completeRouteAvailable = hasScreenRoute(bootstrap, "/complete");
  const historyRouteAvailable = hasScreenRoute(bootstrap, "/history");
  const navigation = [
    { to: "/", label: routeTitles.get("/") ?? "Home" },
    { to: "/host", label: routeTitles.get("/host") ?? "Host" },
    { to: "/join", label: routeTitles.get("/join") ?? "Join" },
    { to: "/lobby", label: routeTitles.get("/lobby") ?? "Lobby" },
    { to: "/table", label: routeTitles.get("/table") ?? "Main Table" },
    { to: "/complete", label: routeTitles.get("/complete") ?? "Complete" },
    { to: "/history", label: routeTitles.get("/history") ?? "History" },
    { to: "/rules", label: "Help" },
    { to: "/settings", label: "Settings" },
  ];

  return (
    <DesktopShellProvider bootstrap={bootstrap}>
      <AppFrame bootstrap={bootstrap} navigation={navigation}>
        <Routes>
          {homeRouteAvailable ? (
            <Route path="/" element={<HomeScreen bootstrap={bootstrap} />} />
          ) : null}
          {hostRouteAvailable ? (
            <Route
              path="/host"
              element={<HostTournamentSetupScreen bootstrap={bootstrap} />}
            />
          ) : null}
          {joinRouteAvailable ? (
            <Route
              path="/join"
              element={<JoinTournamentScreen bootstrap={bootstrap} />}
            />
          ) : null}
          {lobbyRouteAvailable ? (
            <Route
              path="/lobby"
              element={
                <LiveSessionRoute mode="lobby">
                  <TournamentLobbyScreen bootstrap={bootstrap} />
                </LiveSessionRoute>
              }
            />
          ) : null}
          {tableRouteAvailable ? (
            <Route
              path="/table"
              element={
                <LiveSessionRoute mode="table">
                  <MainTableScreen bootstrap={bootstrap} />
                </LiveSessionRoute>
              }
            />
          ) : null}
          {historyRouteAvailable ? (
            <Route
              path="/history"
              element={<HandHistoryScreen bootstrap={bootstrap} />}
            />
          ) : null}
          <Route path="/settings" element={<DeviceSettingsScreen />} />
          <Route path="/npc-profiles" element={<NpcProfilesScreen />} />
          {completeRouteAvailable ? (
            <Route path="/complete" element={<TournamentCompleteScreen />} />
          ) : null}
          <Route path="/rules" element={<RulesHelpScreen />} />
          <Route
            path="/errors"
            element={<ErrorStateScreen bootstrap={bootstrap} />}
          />
          {bootstrap.debugToolsEnabled ? (
            <Route
              path="/debug"
              element={<DebugPanel bootstrap={bootstrap} asScreen />}
            />
          ) : null}
          <Route
            path="*"
            element={
              <Navigate replace to={homeRouteAvailable ? "/" : "/settings"} />
            }
          />
        </Routes>
      </AppFrame>
    </DesktopShellProvider>
  );
}
