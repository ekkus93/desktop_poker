import { Navigate, Route, Routes } from "react-router-dom";
import { DebugPanel } from "../components/debug/DebugPanel";
import { AppFrame } from "../components/layout/AppFrame";
import { DesktopShellProvider } from "./DesktopShellProvider";
import { ErrorStateScreen } from "../screens/ErrorStateScreen";
import { HandHistoryScreen } from "../screens/HandHistoryScreen";
import { HomeScreen } from "../screens/HomeScreen";
import { HostTournamentSetupScreen } from "../screens/HostTournamentSetupScreen";
import { JoinTournamentScreen } from "../screens/JoinTournamentScreen";
import { MainTableScreen } from "../screens/MainTableScreen";
import { RulesHelpScreen } from "../screens/RulesHelpScreen";
import { TournamentCompleteScreen } from "../screens/TournamentCompleteScreen";
import { TournamentLobbyScreen } from "../screens/TournamentLobbyScreen";
import { useDesktopBootstrap } from "./useDesktopBootstrap";

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

  const primaryRoutes = new Set(["/", "/host", "/join", "/history", "/rules"]);
  const navigation = bootstrap.screens
    .filter((screen) => primaryRoutes.has(screen.route))
    .map((screen) => ({
      to: screen.route,
      label: screen.title,
    }));

  return (
    <DesktopShellProvider bootstrap={bootstrap}>
      <AppFrame bootstrap={bootstrap} navigation={navigation}>
        <Routes>
          <Route path="/" element={<HomeScreen bootstrap={bootstrap} />} />
          <Route
            path="/host"
            element={<HostTournamentSetupScreen bootstrap={bootstrap} />}
          />
          <Route
            path="/join"
            element={<JoinTournamentScreen bootstrap={bootstrap} />}
          />
          <Route
            path="/lobby"
            element={<TournamentLobbyScreen bootstrap={bootstrap} />}
          />
          <Route
            path="/ready-room"
            element={<Navigate replace to="/lobby" />}
          />
          <Route path="/table" element={<MainTableScreen bootstrap={bootstrap} />} />
          <Route
            path="/history"
            element={<HandHistoryScreen bootstrap={bootstrap} />}
          />
          <Route
            path="/complete"
            element={<TournamentCompleteScreen />}
          />
          <Route path="/rules" element={<RulesHelpScreen />} />
          <Route path="/errors" element={<ErrorStateScreen bootstrap={bootstrap} />} />
          {bootstrap.debugToolsEnabled ? (
            <Route
              path="/debug"
              element={<DebugPanel bootstrap={bootstrap} asScreen />}
            />
          ) : null}
          <Route path="*" element={<Navigate replace to="/" />} />
        </Routes>
      </AppFrame>
    </DesktopShellProvider>
  );
}
