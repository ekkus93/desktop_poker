import type { ReactNode } from "react";

export function StatusBadge({
  children,
  tone = "info",
}: {
  children: ReactNode;
  tone?: "info" | "success" | "warning" | "accent";
}) {
  return <span className={`status-badge ${tone}`}>{children}</span>;
}
