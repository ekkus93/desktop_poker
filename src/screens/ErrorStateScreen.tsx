import { SectionCard } from "../components/shared/SectionCard";
import { ScreenShell } from "./ScreenShell";
import type { ScreenProps } from "./types";

export function ErrorStateScreen({ bootstrap }: ScreenProps) {
  return (
    <ScreenShell
      title="Reconnect / Error States"
      lead="The shell already reserves a dedicated surface for reconnect, invalid payload, invalid LAN IP, and host-lost flows."
      badges={[bootstrap.runtimeTransport, "Explicit failures only"]}
    >
      <div className="content-grid">
        <SectionCard kicker="Reconnect UX" title="Planned dialogs">
          <ul>
            <li>Reconnecting banner with original identity requirements</li>
            <li>Reconnect success/failure status</li>
            <li>Host-lost or table-closed dialog</li>
          </ul>
        </SectionCard>

        <SectionCard title="Validation failures">
          <ul>
            <li>Invalid payload</li>
            <li>Invalid LAN host IP</li>
            <li>Join rejection or protocol mismatch</li>
          </ul>
        </SectionCard>
      </div>
    </ScreenShell>
  );
}
