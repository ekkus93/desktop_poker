import { useState } from "react";
import { useNavigate } from "react-router-dom";
import { clearLlmApiKey, setLlmApiKey } from "../api/desktop";
import { useDesktopShell } from "../app/useDesktopShell";
import { SectionCard } from "../components/shared/SectionCard";
import { ScreenShell } from "./ScreenShell";

export function DeviceSettingsScreen() {
  const {
    bootstrap,
    displayName,
    setDisplayName,
    resetHostDraft,
    recentJoinPayloads,
    clearRecentJoinPayloads,
  } = useDesktopShell();
  const navigate = useNavigate();
  const [confirmClear, setConfirmClear] = useState(false);
  const [confirmReset, setConfirmReset] = useState(false);
  const [clearSuccess, setClearSuccess] = useState(false);
  const [resetSuccess, setResetSuccess] = useState(false);

  const [apiKeyInput, setApiKeyInput] = useState("");
  const [apiKeyStatus, setApiKeyStatus] = useState<string | null>(null);
  const [apiKeyError, setApiKeyError] = useState<string | null>(null);
  const [keySaving, setKeySaving] = useState(false);
  const [keyClearing, setKeyClearing] = useState(false);

  const keyConfigured = bootstrap.llmApiKeyConfigured;

  async function handleSaveKey() {
    setApiKeyError(null);
    setApiKeyStatus(null);
    setKeySaving(true);
    try {
      await setLlmApiKey(apiKeyInput.trim());
      setApiKeyInput("");
      setApiKeyStatus("Key saved.");
    } catch (e) {
      setApiKeyError(e instanceof Error ? e.message : "Failed to save key.");
    } finally {
      setKeySaving(false);
    }
  }

  async function handleClearKey() {
    setApiKeyError(null);
    setApiKeyStatus(null);
    setKeyClearing(true);
    try {
      await clearLlmApiKey();
      setApiKeyStatus("Key cleared.");
    } catch (e) {
      setApiKeyError(e instanceof Error ? e.message : "Failed to clear key.");
    } finally {
      setKeyClearing(false);
    }
  }

  return (
    <ScreenShell
      title="Settings"
      badges={["This device"]}
      className="support-screen-shell"
    >
      <div className="help-grid">
        <SectionCard
          kicker="Profile"
          title="Player name"
          className="support-card device-settings-card"
        >
          <div className="form-grid">
            <label className="field">
              Display name
              <input
                onChange={(event) => {
                  setDisplayName(event.target.value);
                }}
                value={displayName}
              />
            </label>
            <p className="field-hint">Shown at the table.</p>
          </div>
        </SectionCard>

        <SectionCard
          kicker="AI Players"
          title="Claude API key"
          className="support-card device-settings-card"
        >
          <p className="field-hint">
            Status:{" "}
            <strong>{keyConfigured ? "Key configured" : "No key set"}</strong>
          </p>
          <div className="form-grid">
            <label className="field">
              API key
              <input
                type="password"
                placeholder="sk-ant-..."
                value={apiKeyInput}
                onChange={(e) => setApiKeyInput(e.target.value)}
              />
            </label>
          </div>
          <div className="button-row">
            <button
              className="primary-button"
              onClick={handleSaveKey}
              disabled={keySaving || apiKeyInput.trim().length === 0}
              type="button"
            >
              Save key
            </button>
            {keyConfigured ? (
              <button
                className="secondary-button"
                onClick={handleClearKey}
                disabled={keyClearing}
                type="button"
              >
                Clear key
              </button>
            ) : null}
          </div>
          {apiKeyStatus ? (
            <p className="inline-banner success">{apiKeyStatus}</p>
          ) : null}
          {apiKeyError ? (
            <p className="inline-banner error">{apiKeyError}</p>
          ) : null}
          <div className="button-row">
            <button
              className="secondary-button"
              onClick={() => navigate("/npc-profiles")}
              type="button"
            >
              Manage AI profiles
            </button>
          </div>
        </SectionCard>

        <SectionCard
          kicker="Saved on this device"
          title="Local setup"
          className="support-card device-settings-card"
        >
          <div className="button-row">
            {confirmReset ? (
              <>
                <button
                  className="primary-button"
                  onClick={() => {
                    resetHostDraft();
                    setConfirmReset(false);
                    setResetSuccess(true);
                  }}
                  type="button"
                >
                  Confirm reset
                </button>
                <button
                  className="secondary-button"
                  onClick={() => setConfirmReset(false)}
                  type="button"
                >
                  Cancel
                </button>
              </>
            ) : (
              <button
                className="secondary-button"
                onClick={() => { setConfirmReset(true); setResetSuccess(false); }}
                type="button"
              >
                Reset host setup
              </button>
            )}
            {confirmClear ? (
              <>
                <button
                  className="primary-button"
                  onClick={() => {
                    clearRecentJoinPayloads();
                    setConfirmClear(false);
                    setClearSuccess(true);
                  }}
                  type="button"
                >
                  Confirm clear
                </button>
                <button
                  className="secondary-button"
                  onClick={() => setConfirmClear(false)}
                  type="button"
                >
                  Cancel
                </button>
              </>
            ) : (
              <button
                className="secondary-button"
                onClick={() => { setConfirmClear(true); setClearSuccess(false); }}
                type="button"
              >
                Clear saved invites ({recentJoinPayloads.length})
              </button>
            )}
          </div>
          {resetSuccess ? <p className="inline-banner success">Host setup reset.</p> : null}
          {clearSuccess ? <p className="inline-banner success">Saved invites cleared.</p> : null}
          <p className="field-hint">
            {recentJoinPayloads.length} saved invite{recentJoinPayloads.length === 1 ? "" : "s"}.
          </p>
        </SectionCard>
      </div>
    </ScreenShell>
  );
}
