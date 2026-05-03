import type { ReactNode } from "react";

export function SectionCard({
  title,
  kicker,
  children,
}: {
  title: string;
  kicker?: string;
  children: ReactNode;
}) {
  return (
    <section className="section-card">
      {kicker ? <p className="kicker">{kicker}</p> : null}
      <h3>{title}</h3>
      {children}
    </section>
  );
}
