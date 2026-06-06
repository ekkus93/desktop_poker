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
  const [debugState, setDebugState] = useState<DebugInspectorState | null>(null);
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
          setError(caughtError instanceof Error ? caughtError.message : "Unknown debug error");
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
        caughtError instanceof Error ? caughtError.message : "Could not launch client",
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
        caughtError instanceof Error ? caughtError.message : "Could not copy payload",
      );
    }
  };

  const content = (
    <div className={asScreen ? "screen-shell" : "debug-panel"}>
      <SectionCard kicker="Internal only" title="Debug tools">
        <p className="field-hint">
          These tools are for development and QA. They should stay outside the normal player path.
        </p>
      </SectionCard>

      <div className="button-row">
        <button
          className={viewerMode === "local" ? "primary-button compact-button" : "secondary-button compact-button"}
          onClick={() => setViewerMode("local")}
          type="button"
        >
          Inspect local view
        </button>
        <button
          className={viewerMode === "observer" ? "primary-button compact-button" : "secondary-button compact-button"}
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
            <strong>Profile ID:</strong> <span className="mono-value">{bootstrap.instanceId}</span>
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
            <strong>Current sequence:</strong> {debugState?.currentSequence ?? "—"}
          </li>
          <li>
            <strong>Current hand:</strong> {debugState?.currentHandNumber ?? "—"}
          </li>
        </ul>
        <pre className="debug-pre">{debugState?.snapshotJson ?? "Loading snapshot…"}</pre>
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
          {debugState?.actionWindowSummary ?? "No open action window at the moment."}
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
          <button className="secondary-button" onClick={() => void handleCopyPayload()} type="button">
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
        {copyStatus ? <div className="inline-banner success">{copyStatus}</div> : null}
        {launchStatus ? <div className="inline-banner info">{launchStatus}</div> : null}
      </SectionCard>

      {debugState && Object.keys(debugState.npcTiltLevels).some((k) => debugState.npcTiltLevels[k] !== "none") ? (
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
