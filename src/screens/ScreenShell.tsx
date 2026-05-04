import type { ReactNode } from "react";
import { StatusBadge } from "../components/shared/StatusBadge";

export function ScreenShell({
  title,
  lead,
  children,
  badges = [],
  className,
}: {
  title: string;
  lead?: string;
  children: ReactNode;
  badges?: string[];
  className?: string;
}) {
  return (
    <section className={className ? `screen-shell ${className}` : "screen-shell"}>
      <header className="screen-header">
        <div className="screen-copy">
          <p className="kicker">Desktop Poker</p>
          <h2>{title}</h2>
          {lead ? <p className="screen-lead">{lead}</p> : null}
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
