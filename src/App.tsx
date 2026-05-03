import { HashRouter } from "react-router-dom";
import "./App.css";
import { AppShell } from "./app/AppShell";
import { DesktopBootstrapProvider } from "./app/DesktopBootstrapProvider";

function App() {
  return (
    <HashRouter>
      <DesktopBootstrapProvider>
        <AppShell />
      </DesktopBootstrapProvider>
    </HashRouter>
  );
}

export default App;
