import { useState } from "react";
import { useDesktopShell } from "../app/useDesktopShell";
import { SectionCard } from "../components/shared/SectionCard";
import { ScreenShell } from "./ScreenShell";

export function DeviceSettingsScreen() {
  const {
    displayName,
    setDisplayName,
    resetHostDraft,
    recentJoinPayloads,
    clearRecentJoinPayloads,
  } = useDesktopShell();
  const [confirmClear, setConfirmClear] = useState(false);
  const [confirmReset, setConfirmReset] = useState(false);
  const [clearSuccess, setClearSuccess] = useState(false);
  const [resetSuccess, setResetSuccess] = useState(false);

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