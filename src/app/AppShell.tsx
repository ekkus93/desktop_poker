import { Navigate, Route, Routes } from "react-router-dom";
import { DebugPanel } from "../components/debug/DebugPanel";
import { AppFrame } from "../components/layout/AppFrame";
import { DesktopShellProvider } from "./DesktopShellProvider";
import { DeviceSettingsScreen } from "../screens/DeviceSettingsScreen";
import { ErrorStateScreen } from "../screens/ErrorStateScreen";
import { HandHistoryScreen } from "../screens/HandHistoryScreen";
import { HomeScreen } from "../screens/HomeScreen";
import { HostTournamentSetupScreen } from "../screens/HostTournamentSetupScreen";
import { JoinTournamentScreen } from "../screens/JoinTournamentScreen";
import { MainTableScreen } from "../screens/MainTableScreen";
import { ReadyRoomScreen } from "../screens/ReadyRoomScreen";
import { RulesHelpScreen } from "../screens/RulesHelpScreen";
import { TournamentCompleteScreen } from "../screens/TournamentCompleteScreen";
import { TournamentLobbyScreen } from "../screens/TournamentLobbyScreen";
import { useDesktopBootstrap } from "./useDesktopBootstrap";

function hasScreenRoute(
  bootstrap: NonNullable<ReturnType<typeof useDesktopBootstrap>["bootstrap"]>,
  route: string,
) {
  return bootstrap.screens.some((screen) => screen.route === route);
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
          <p>{error ?? "No bootstrap payload was returned by the Rust backend."}</p>
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
  const readyRoomRouteAvailable = hasScreenRoute(bootstrap, "/ready-room");
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
              element={<TournamentLobbyScreen bootstrap={bootstrap} />}
            />
          ) : null}
          {readyRoomRouteAvailable ? (
            <Route
              path="/ready-room"
              element={<ReadyRoomScreen bootstrap={bootstrap} />}
            />
          ) : null}
          {tableRouteAvailable ? (
            <Route path="/table" element={<MainTableScreen bootstrap={bootstrap} />} />
          ) : null}
          {historyRouteAvailable ? (
            <Route
              path="/history"
              element={<HandHistoryScreen bootstrap={bootstrap} />}
            />
          ) : null}
          <Route path="/settings" element={<DeviceSettingsScreen />} />
          {completeRouteAvailable ? (
            <Route
              path="/complete"
              element={<TournamentCompleteScreen />}
            />
          ) : null}
          <Route path="/rules" element={<RulesHelpScreen />} />
          <Route path="/errors" element={<ErrorStateScreen bootstrap={bootstrap} />} />
          {bootstrap.debugToolsEnabled ? (
            <Route
              path="/debug"
              element={<DebugPanel bootstrap={bootstrap} asScreen />}
            />
          ) : null}
          <Route
            path="*"
            element={<Navigate replace to={homeRouteAvailable ? "/" : "/settings"} />}
          />
        </Routes>
      </AppFrame>
    </DesktopShellProvider>
  );
}
