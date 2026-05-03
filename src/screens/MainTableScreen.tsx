import { Link } from "react-router-dom";
import { useDesktopShell } from "../app/useDesktopShell";
import { buildParticipantShell, getBlindPreset } from "../app/shell";
import { SectionCard } from "../components/shared/SectionCard";
import { TablePlaceholder } from "../components/table/TablePlaceholder";
import { ScreenShell } from "./ScreenShell";
import type { ScreenProps } from "./types";

export function MainTableScreen({ bootstrap }: ScreenProps) {
  const { displayName, hostDraft, readySeats, recentJoinPayloads } =
    useDesktopShell();
  const participants = buildParticipantShell(
    bootstrap,
    hostDraft,
    readySeats,
    displayName,
    recentJoinPayloads,
  );
  const blindPreset = getBlindPreset(hostDraft.blindPresetId);

  return (
    <ScreenShell
      title="Main Table"
      lead="The table route now exposes the full rendering shell for community cards, seat status, action windows, and reconnect banners while keeping gameplay truth in Rust."
      badges={[blindPreset.label, bootstrap.runtimeTransport]}
    >
      <div className="content-grid wide-grid">
        <TablePlaceholder
          blindLabel={blindPreset.firstLevel}
          participants={participants}
          turnTimerSeconds={hostDraft.turnTimerSeconds}
        />

        <SectionCard title="Action tray contract">
          <ul>
            <li>Only the acting participant receives enabled action controls.</li>
            <li>Observers and eliminated players remain public-only.</li>
            <li>Reconnect banners must remain explicit during session recovery.</li>
          </ul>
          <div className="button-row">
            <button className="secondary-button" type="button">
              Fold
            </button>
            <button className="secondary-button" type="button">
              Check / Call
            </button>
            <button className="primary-button" type="button">
              Bet / Raise
            </button>
          </div>
          <p className="field-hint">
            Controls stay visible in the shell, but the Rust backend remains the
            sole authority on legal action windows.
          </p>
        </SectionCard>

        <SectionCard title="Table navigation">
          <div className="button-row">
            <Link className="secondary-button" to="/history">
              Hand history
            </Link>
            <Link className="secondary-button" to="/errors">
              Reconnect and errors
            </Link>
            <Link className="secondary-button" to="/complete">
              Tournament complete
            </Link>
          </div>
        </SectionCard>
      </div>
    </ScreenShell>
  );
}
