import type { ReactNode } from "react";
import { StatusBadge } from "../components/shared/StatusBadge";

export function ScreenShell({
  title,
  lead,
  children,
  badges = [],
}: {
  title: string;
  lead: string;
  children: ReactNode;
  badges?: string[];
}) {
  return (
    <section className="screen-shell">
      <header className="screen-header">
        <div className="screen-copy">
          <p className="kicker">Desktop shell</p>
          <h2>{title}</h2>
          <p className="screen-lead">{lead}</p>
        </div>
        <div className="badge-row">
          {badges.map((badge) => (
            <StatusBadge key={badge}>{badge}</StatusBadge>
          ))}
        </div>
      </header>
      {children}
    </section>
  );
}
