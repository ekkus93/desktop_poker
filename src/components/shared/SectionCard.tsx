import type { ReactNode } from "react";

export function SectionCard({
  title,
  kicker,
  children,
  className,
}: {
  title: string;
  kicker?: string;
  children: ReactNode;
  className?: string;
}) {
  return (
    <section className={className ? `section-card ${className}` : "section-card"}>
      {kicker ? <p className="kicker">{kicker}</p> : null}
      <h3>{title}</h3>
      {children}
    </section>
  );
}
