import { useDesktopShell } from "../app/useDesktopShell";
import { SectionCard } from "../components/shared/SectionCard";
import { ScreenShell } from "./ScreenShell";
import type { ScreenProps } from "./types";

export function HandHistoryScreen({ bootstrap }: ScreenProps) {
  const { hostDraft, recentJoinPayloads } = useDesktopShell();
  const historyEntries = [
    {
      title: "Host draft prepared",
      detail: `Tournament "${hostDraft.tournamentName}" is ready to bind on port ${hostDraft.hostPort}.`,
    },
    {
      title: "Launch payload capture",
      detail: bootstrap.launchJoinPayload
        ? "A join payload was captured at launch and routed into the Join screen."
        : "No launch payload was provided to this instance.",
    },
    {
      title: "Recent join payload cache",
      detail: `${recentJoinPayloads.length} direct payload(s) are remembered for this instance.`,
    },
  ];

  return (
    <ScreenShell
      title="Hand History"
      lead="The history route is ready for settled hands and tournament outcomes; until live events arrive, it surfaces real shell checkpoints and storage-backed context."
      badges={[bootstrap.serializationStrategy, "History shell"]}
    >
      <div className="content-grid">
        <SectionCard kicker="History" title="Current shell timeline">
          <div className="stacked-list">
            {historyEntries.map((entry) => (
              <article key={entry.title} className="list-panel history-row">
                <div>
                  <strong>{entry.title}</strong>
                  <p className="field-hint">{entry.detail}</p>
                </div>
              </article>
            ))}
          </div>
        </SectionCard>

        <SectionCard title="Future hand feed">
          <ul>
            <li>Blind level and dealer button metadata per hand</li>
            <li>Street reveals, showdown summaries, and side-pot outcomes</li>
            <li>Elimination ordering and final placement records</li>
          </ul>
        </SectionCard>
      </div>
    </ScreenShell>
  );
}
