import type { ComponentPropsWithoutRef, ReactNode } from "react";

export function SectionCard({
  title,
  kicker,
  children,
  className,
  ...rest
}: {
  title: string;
  kicker?: string;
  children: ReactNode;
  className?: string;
} & Omit<ComponentPropsWithoutRef<"section">, "title">) {
  return (
    <section
      {...rest}
      className={className ? `section-card ${className}` : "section-card"}
    >
      {kicker ? <p className="kicker">{kicker}</p> : null}
      <h3>{title}</h3>
      {children}
    </section>
  );
}
