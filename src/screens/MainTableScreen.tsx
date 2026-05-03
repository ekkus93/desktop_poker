import { SectionCard } from "../components/shared/SectionCard";
import { TablePlaceholder } from "../components/table/TablePlaceholder";
import { ScreenShell } from "./ScreenShell";
import type { ScreenProps } from "./types";

export function MainTableScreen({ bootstrap }: ScreenProps) {
  return (
    <ScreenShell
      title="Main Table"
      lead="The main table surface is already separated from the rest of the shell so later milestones can drop in public and private projections without a structural rewrite."
      badges={[bootstrap.runtimeTransport, "Observer-safe rendering"]}
    >
      <div className="content-grid">
        <TablePlaceholder />

        <SectionCard title="Action tray contract">
          <ul>
            <li>Exactly one action window may be open at a time.</li>
            <li>Only the acting participant sees enabled action controls.</li>
            <li>Observer surfaces remain public-only.</li>
          </ul>
        </SectionCard>
      </div>
    </ScreenShell>
  );
}
