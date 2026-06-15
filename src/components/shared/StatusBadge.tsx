import type { ReactNode } from "react";
import { AlertTriangle, CheckCircle2, Info } from "lucide-react";

const TONE_ICONS: Record<string, ReactNode> = {
  success: (
    <CheckCircle2
      aria-hidden="true"
      className="badge-icon"
      size={12}
      strokeWidth={2}
    />
  ),
  warning: (
    <AlertTriangle
      aria-hidden="true"
      className="badge-icon"
      size={12}
      strokeWidth={2}
    />
  ),
  error: (
    <AlertTriangle
      aria-hidden="true"
      className="badge-icon"
      size={12}
      strokeWidth={2}
    />
  ),
  info: (
    <Info aria-hidden="true" className="badge-icon" size={12} strokeWidth={2} />
  ),
};

export function StatusBadge({
  children,
  tone = "info",
}: {
  children: ReactNode;
  tone?: "info" | "success" | "warning" | "accent";
}) {
  const icon = TONE_ICONS[tone] ?? null;
  return (
    <span className={`status-badge ${tone}`}>
      {icon}
      {children}
    </span>
  );
}
