import { createHashRouter, RouterProvider } from "react-router";
import "./App.css";
import { AppShell } from "./app/AppShell";
import { DesktopBootstrapProvider } from "./app/DesktopBootstrapProvider";

const router = createHashRouter([
  {
    path: "*",
    element: (
      <DesktopBootstrapProvider>
        <AppShell />
      </DesktopBootstrapProvider>
    ),
  },
]);

function App() {
  return <RouterProvider router={router} />;
}

export default App;
