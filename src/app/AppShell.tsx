import { Navigate, Route, Routes } from "react-router-dom";
import { AppFrame, type NavigationItem } from "../components/layout/AppFrame";
import { DebugPanel } from "../components/debug/DebugPanel";
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

  const navigation: NavigationItem[] = [
    { to: "/", label: "Home" },
    { to: "/host", label: "Host Tournament" },
    { to: "/join", label: "Join Tournament" },
    { to: "/lobby", label: "Tournament Lobby" },
    { to: "/ready-room", label: "Ready Room" },
    { to: "/table", label: "Main Table" },
    { to: "/history", label: "Hand History" },
    { to: "/complete", label: "Tournament Complete" },
    { to: "/rules", label: "Rules / Help" },
    { to: "/errors", label: "Reconnect / Errors" },
  ];

  if (bootstrap.debugToolsEnabled) {
    navigation.push({ to: "/debug", label: "Debug Tools" });
  }

  return (
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
          element={<ReadyRoomScreen bootstrap={bootstrap} />}
        />
        <Route path="/table" element={<MainTableScreen bootstrap={bootstrap} />} />
        <Route
          path="/history"
          element={<HandHistoryScreen bootstrap={bootstrap} />}
        />
        <Route
          path="/complete"
          element={<TournamentCompleteScreen bootstrap={bootstrap} />}
        />
        <Route path="/rules" element={<RulesHelpScreen bootstrap={bootstrap} />} />
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
        <Route path="*" element={<Navigate replace to="/" />} />
      </Routes>
    </AppFrame>
  );
}
