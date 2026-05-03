import type { DesktopBootstrapState } from "../../api/desktop";
import { SectionCard } from "../shared/SectionCard";
import { StatusBadge } from "../shared/StatusBadge";

export function JoinPayloadCard({
  bootstrap,
}: {
  bootstrap: DesktopBootstrapState;
}) {
  return (
    <SectionCard kicker="Host / join flows" title="Join payload contract">
      <p>
        Desktop M0 freezes the launch contract around the Android-compatible
        direct payload path. The real payload parser, generator, QR support, and
        validation arrive in later milestones.
      </p>
      <div className="badge-row">
        <StatusBadge tone="info">{bootstrap.joinPayloadEncoding}</StatusBadge>
        <StatusBadge tone="success">{bootstrap.framingStrategy}</StatusBadge>
      </div>
      <p className="surface-label">Launch payload captured by Rust</p>
      <p className="mono-value">
        {bootstrap.launchJoinPayload ?? "No payload was provided at launch."}
      </p>
      <p className="field-hint">
        Supported launch sources: <code>--join-payload</code> and{" "}
        <code>DESKTOP_POKER_JOIN_PAYLOAD</code>.
      </p>
    </SectionCard>
  );
}
