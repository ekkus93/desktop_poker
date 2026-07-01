import { useEffect, useState } from "react";
import {
  getDebugState,
  launchAdditionalClientInstance,
  type DebugInspectorState,
  type TableViewerMode,
} from "../../api/desktop";
import { useDesktopShell } from "../../app/useDesktopShell";
import type { DesktopBootstrapState } from "../../api/desktop";
import { SectionCard } from "../shared/SectionCard";

export function DebugPanel({
  bootstrap,
  asScreen = false,
}: {
  bootstrap: DesktopBootstrapState;
  asScreen?: boolean;
}) {
  const {
    displayName,
    hostDraft,
    joinPayloadDraft,
    recentJoinPayloads,
    setJoinPayloadDraft,
  } = useDesktopShell();
  const [viewerMode, setViewerMode] = useState<TableViewerMode>("local");
  const [debugState, setDebugState] = useState<DebugInspectorState | null>(
    null,
  );
  const [error, setError] = useState<string | null>(null);
  const [launchStatus, setLaunchStatus] = useState<string | null>(null);
  const [copyStatus, setCopyStatus] = useState<string | null>(null);

  useEffect(() => {
    let cancelled = false;

    void getDebugState(viewerMode)
      .then((snapshot) => {
        if (!cancelled) {
          setDebugState(snapshot);
          setError(null);
        }
      })
      .catch((caughtError: unknown) => {
        if (!cancelled) {
          setError(
            caughtError instanceof Error
              ? caughtError.message
              : "Unknown debug error",
          );
        }
      });

    return () => {
      cancelled = true;
    };
  }, [viewerMode]);

  const handleLaunch = async (withPayload: boolean) => {
    const trimmedPayload = joinPayloadDraft.trim();
    const payload = withPayload && trimmedPayload ? trimmedPayload : null;

    try {
      if (payload) {
        await navigator.clipboard.writeText(payload);
        setCopyStatus("Copied join payload for the next instance.");
      }

      const instanceId = await launchAdditionalClientInstance(payload);
      setLaunchStatus(
        payload
          ? `Launched ${instanceId} with the copied join payload attached.`
          : `Launched ${instanceId}`,
      );
    } catch (caughtError) {
      setLaunchStatus(
        caughtError instanceof Error
          ? caughtError.message
          : "Could not launch client",
      );
    }
  };

  const handleCopyPayload = async () => {
    const trimmedPayload = joinPayloadDraft.trim();
    if (!trimmedPayload) {
      setCopyStatus("Paste or load a join payload first.");
      return;
    }

    try {
      await navigator.clipboard.writeText(trimmedPayload);
      setCopyStatus("Copied join payload for another local instance.");
    } catch (caughtError) {
      setCopyStatus(
        caughtError instanceof Error
          ? caughtError.message
          : "Could not copy payload",
      );
    }
  };

  const content = (
    <div className={asScreen ? "screen-shell" : "debug-panel"}>
      <SectionCard kicker="Internal only" title="Debug tools">
        <p className="field-hint">
          These tools are for development and QA. They should stay outside the
          normal player path.
        </p>
      </SectionCard>

      <div className="button-row">
        <button
          className={
            viewerMode === "local"
              ? "primary-button compact-button"
              : "secondary-button compact-button"
          }
          onClick={() => setViewerMode("local")}
          type="button"
        >
          Inspect local view
        </button>
        <button
          className={
            viewerMode === "observer"
              ? "primary-button compact-button"
              : "secondary-button compact-button"
          }
          onClick={() => setViewerMode("observer")}
          type="button"
        >
          Inspect observer view
        </button>
      </div>

      <SectionCard kicker="Debug / internal tools" title="Bootstrap inspection">
        <ul>
          <li>
            <strong>Frontend:</strong> {bootstrap.frontendStack}
          </li>
          <li>
            <strong>Serialization:</strong> {bootstrap.serializationStrategy}
          </li>
          <li>
            <strong>Transport:</strong> {bootstrap.runtimeTransport}
          </li>
          <li>
            <strong>Display name:</strong> {displayName}
          </li>
          <li>
            <strong>Instance label:</strong> {bootstrap.instanceLabel}
          </li>
          <li>
            <strong>Profile ID:</strong>{" "}
            <span className="mono-value">{bootstrap.instanceId}</span>
          </li>
          <li>
            <strong>Storage namespace:</strong>{" "}
            <span className="mono-value">{bootstrap.storageNamespace}</span>
          </li>
          <li>
            <strong>Session identity:</strong>{" "}
            <span className="mono-value">{bootstrap.sessionIdentity}</span>
          </li>
          <li>
            <strong>Reconnect namespace:</strong>{" "}
            <span className="mono-value">{bootstrap.reconnectNamespace}</span>
          </li>
          <li>
            <strong>Profile folder:</strong>{" "}
            <span className="mono-value">{bootstrap.profileDirectory}</span>
          </li>
        </ul>
      </SectionCard>

      <SectionCard title="Current snapshot inspector">
        {error ? <p>{error}</p> : null}
        <ul>
          <li>
            <strong>Host draft:</strong> {hostDraft.tournamentName}
          </li>
          <li>
            <strong>Host port:</strong> {hostDraft.hostPort}
          </li>
          <li>
            <strong>Recent payloads:</strong> {recentJoinPayloads.length}
          </li>
          <li>
            <strong>Screen catalog size:</strong> {bootstrap.screens.length}
          </li>
          <li>
            <strong>Current sequence:</strong>{" "}
            {debugState?.currentSequence ?? "—"}
          </li>
          <li>
            <strong>Current hand:</strong>{" "}
            {debugState?.currentHandNumber ?? "—"}
          </li>
        </ul>
        <pre className="debug-pre">
          {debugState?.snapshotJson ?? "Loading snapshot…"}
        </pre>
      </SectionCard>

      <SectionCard title="Protocol log viewer">
        <div className="stacked-list event-feed-list">
          {debugState?.protocolLog.map((event) => (
            <article key={event.sequence} className="list-panel history-row">
              <div>
                <strong>
                  Seq {event.sequence} · {event.kind}
                </strong>
                <p className="field-hint">{event.message}</p>
              </div>
            </article>
          ))}
        </div>
      </SectionCard>

      <SectionCard title="Action-window inspector">
        <p className="field-hint">
          {debugState?.actionWindowSummary ??
            "No open action window at the moment."}
        </p>
      </SectionCard>

      <SectionCard title="Launch additional client instance helper">
        <p className="field-hint">{debugState?.launchHint}</p>
        <label className="field">
          Join payload for handoff
          <textarea
            onChange={(event) => {
              setJoinPayloadDraft(event.target.value);
              setCopyStatus(null);
              setLaunchStatus(null);
            }}
            rows={5}
            value={joinPayloadDraft}
          />
        </label>
        <div className="button-row">
          <button
            className="secondary-button"
            onClick={() => void handleCopyPayload()}
            type="button"
          >
            Copy payload
          </button>
          <button
            className="primary-button"
            disabled={!joinPayloadDraft.trim()}
            onClick={() => void handleLaunch(true)}
            type="button"
          >
            Launch extra client with payload
          </button>
          <button
            className="secondary-button"
            onClick={() => void handleLaunch(false)}
            type="button"
          >
            Launch extra client
          </button>
        </div>
        {copyStatus ? (
          <div className="inline-banner success">{copyStatus}</div>
        ) : null}
        {launchStatus ? (
          <div className="inline-banner info">{launchStatus}</div>
        ) : null}
      </SectionCard>

      {debugState &&
      Object.keys(debugState.npcTiltLevels).some(
        (k) => debugState.npcTiltLevels[k] !== "none",
      ) ? (
        <SectionCard title="NPC tilt state" data-testid="npc-tilt-section">
          <ul data-testid="npc-tilt-list">
            {Object.entries(debugState.npcTiltLevels)
              .filter(([, level]) => level !== "none")
              .map(([playerId, level]) => (
                <li key={playerId}>
                  <strong>{playerId}:</strong> {level}
                </li>
              ))}
          </ul>
        </SectionCard>
      ) : null}

      {debugState?.lastLlmFallback ? (
        <SectionCard
          title="Last LLM fallback"
          data-testid="llm-fallback-section"
        >
          <p
            data-testid="llm-fallback-reason"
            className="inline-banner warning"
          >
            {debugState.lastLlmFallback}
          </p>
        </SectionCard>
      ) : null}

      {debugState?.lastNpcActionError ? (
        <SectionCard
          title="Last NPC action error"
          data-testid="npc-action-error-section"
        >
          <p
            data-testid="npc-action-error-message"
            className="inline-banner error"
          >
            {debugState.lastNpcActionError.message}
          </p>
          <ul data-testid="npc-action-error-fields">
            <li>
              <strong>Reason:</strong>{" "}
              <span data-testid="npc-action-error-reason">
                {debugState.lastNpcActionError.reason}
              </span>
            </li>
            <li>
              <strong>Submitted:</strong>{" "}
              <span data-testid="npc-action-error-submitted">
                {debugState.lastNpcActionError.submitted ? "yes" : "no"}
              </span>
            </li>
            {debugState.lastNpcActionError.playerId != null && (
              <li>
                <strong>Player:</strong>{" "}
                {debugState.lastNpcActionError.playerId}
              </li>
            )}
            {debugState.lastNpcActionError.action != null && (
              <li>
                <strong>Action:</strong> {debugState.lastNpcActionError.action}
              </li>
            )}
            {debugState.lastNpcActionError.handNumber != null && (
              <li>
                <strong>Hand:</strong>{" "}
                {debugState.lastNpcActionError.handNumber}
              </li>
            )}
          </ul>
        </SectionCard>
      ) : null}

      {debugState?.hostRuntimeHealth &&
      (debugState.hostRuntimeHealth.acceptErrorCount > 0 ||
        debugState.hostRuntimeHealth.tickAdvanceErrorCount > 0 ||
        debugState.hostRuntimeHealth.publishErrorCount > 0 ||
        debugState.hostRuntimeHealth.stateLockErrorCount > 0 ||
        debugState.hostRuntimeHealth.streamTimeoutErrorCount > 0 ||
        debugState.hostRuntimeHealth.streamCloneErrorCount > 0 ||
        debugState.hostRuntimeHealth.clientRegistryErrorCount > 0 ||
        debugState.hostRuntimeHealth.reconnectMarkErrorCount > 0 ||
        debugState.hostRuntimeHealth.snapshotSyncErrorCount > 0 ||
        debugState.hostRuntimeHealth.lastError != null) ? (
        <SectionCard
          title="Host runtime health"
          data-testid="host-runtime-health-section"
        >
          <ul data-testid="host-runtime-health-fields">
            {debugState.hostRuntimeHealth.acceptErrorCount > 0 && (
              <li>
                <strong>Accept errors:</strong>{" "}
                {debugState.hostRuntimeHealth.acceptErrorCount}
              </li>
            )}
            {debugState.hostRuntimeHealth.tickAdvanceErrorCount > 0 && (
              <li>
                <strong>Tick errors:</strong>{" "}
                {debugState.hostRuntimeHealth.tickAdvanceErrorCount}
              </li>
            )}
            {debugState.hostRuntimeHealth.publishErrorCount > 0 && (
              <li>
                <strong>Publish errors:</strong>{" "}
                {debugState.hostRuntimeHealth.publishErrorCount}
              </li>
            )}
            {debugState.hostRuntimeHealth.stateLockErrorCount > 0 && (
              <li>
                <strong>Lock errors:</strong>{" "}
                {debugState.hostRuntimeHealth.stateLockErrorCount}
              </li>
            )}
            {debugState.hostRuntimeHealth.streamTimeoutErrorCount > 0 && (
              <li>
                <strong>Stream timeout errors:</strong>{" "}
                {debugState.hostRuntimeHealth.streamTimeoutErrorCount}
              </li>
            )}
            {debugState.hostRuntimeHealth.streamCloneErrorCount > 0 && (
              <li>
                <strong>Stream clone errors:</strong>{" "}
                {debugState.hostRuntimeHealth.streamCloneErrorCount}
              </li>
            )}
            {debugState.hostRuntimeHealth.clientRegistryErrorCount > 0 && (
              <li>
                <strong>Client registry errors:</strong>{" "}
                {debugState.hostRuntimeHealth.clientRegistryErrorCount}
              </li>
            )}
            {debugState.hostRuntimeHealth.reconnectMarkErrorCount > 0 && (
              <li>
                <strong>Reconnect mark errors:</strong>{" "}
                {debugState.hostRuntimeHealth.reconnectMarkErrorCount}
              </li>
            )}
            {debugState.hostRuntimeHealth.snapshotSyncErrorCount > 0 && (
              <li>
                <strong>Snapshot sync errors:</strong>{" "}
                {debugState.hostRuntimeHealth.snapshotSyncErrorCount}
              </li>
            )}
            {debugState.hostRuntimeHealth.lastError != null && (
              <li>
                <strong>Last error:</strong>{" "}
                <span data-testid="host-runtime-health-last-error">
                  {debugState.hostRuntimeHealth.lastError}
                </span>
              </li>
            )}
          </ul>
        </SectionCard>
      ) : null}

      <SectionCard title="Rust backend module map">
        <ul>
          {bootstrap.backendModules.map((moduleInfo) => (
            <li key={moduleInfo.name}>
              <strong>{moduleInfo.name}</strong>
              <span>{moduleInfo.responsibility}</span>
            </li>
          ))}
        </ul>
      </SectionCard>
    </div>
  );

  if (asScreen) {
    return content;
  }

  return <section className="debug-panel">{content}</section>;
}
