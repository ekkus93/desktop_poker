import { useEffect, useState } from "react";
import { useNavigate } from "react-router";
import {
  clearLlmProviderConfig,
  getLlmProviderConfig,
  saveNonSecretProviderSettings,
  setLlmProviderConfig,
  type LlmProviderConfig,
  type LlmProviderSettings,
  type LlmProviderType,
} from "../api/desktop";
import { useDesktopShell } from "../app/useDesktopShell";
import { SectionCard } from "../components/shared/SectionCard";
import { ScreenShell } from "./ScreenShell";

const PROVIDER_OPTIONS: { value: LlmProviderType; label: string }[] = [
  { value: "anthropic", label: "Anthropic (Claude)" },
  { value: "openAi", label: "OpenAI" },
  { value: "ollama", label: "Ollama (local)" },
  { value: "llamaServer", label: "llama-server (local)" },
];

const DEFAULT_ENDPOINTS: Record<LlmProviderType, string> = {
  anthropic: "https://api.anthropic.com",
  openAi: "https://api.openai.com",
  ollama: "http://localhost:11434",
  llamaServer: "http://localhost:8080",
};

const DEFAULT_MODELS: Record<LlmProviderType, string> = {
  anthropic: "claude-haiku-4-5-20251001",
  openAi: "gpt-4o-mini",
  ollama: "llama3.2",
  llamaServer: "",
};

function requiresApiKey(provider: LlmProviderType) {
  return provider === "anthropic" || provider === "openAi";
}

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

  // Provider config form state.
  const [selectedProvider, setSelectedProvider] = useState<LlmProviderType>(
    (bootstrap.llmProviderType as LlmProviderType | null) ?? "anthropic",
  );
  const [apiKey, setApiKey] = useState("");
  const [endpointUrl, setEndpointUrl] = useState("");
  const [model, setModel] = useState("");
  const [providerStatus, setProviderStatus] = useState<string | null>(null);
  const [providerError, setProviderError] = useState<string | null>(null);
  const [saving, setSaving] = useState(false);
  const [clearing, setClearing] = useState(false);
  // Track whether the current form fields were loaded from an existing config.
  const [loadedSettings, setLoadedSettings] =
    useState<LlmProviderSettings | null>(null);

  const providerConfigured = bootstrap.llmApiKeyConfigured;

  // Load existing non-secret provider config on mount.
  useEffect(() => {
    getLlmProviderConfig()
      .then((settings) => {
        if (settings) {
          setLoadedSettings(settings);
          setSelectedProvider(settings.provider);
          setEndpointUrl(settings.endpointUrl ?? "");
          setModel(settings.model ?? "");
        }
      })
      .catch((e: unknown) => {
        setProviderError(
          e instanceof Error
            ? `Failed to load provider config: ${e.message}`
            : "Failed to load provider config.",
        );
      });
  }, []);

  // When the user explicitly changes the provider dropdown, reset endpoint/model
  // placeholders only if it differs from the loaded settings' provider.
  useEffect(() => {
    if (loadedSettings && selectedProvider === loadedSettings.provider) {
      return;
    }
    setEndpointUrl("");
    setModel("");
  }, [selectedProvider, loadedSettings]);

  // P0.3: A stored key is only reusable when saving to the same provider type.
  // An Anthropic key must not be preserved when switching to OpenAI, and vice versa.
  const storedProviderMatchesSelected =
    loadedSettings?.provider === selectedProvider;
  const hasExistingKeyForProvider =
    providerConfigured && storedProviderMatchesSelected;

  async function handleSave() {
    setProviderError(null);
    setProviderStatus(null);
    const trimmedKey = apiKey.trim();
    // Require a key if the provider needs one and there is no key for this provider.
    if (
      requiresApiKey(selectedProvider) &&
      !trimmedKey &&
      !hasExistingKeyForProvider
    ) {
      setProviderError("An API key is required for this provider.");
      return;
    }
    setSaving(true);
    try {
      if (trimmedKey || !requiresApiKey(selectedProvider)) {
        // Full save: new key entered, or provider doesn't use API keys.
        const config: LlmProviderConfig = {
          provider: selectedProvider,
          apiKey: trimmedKey || null,
          endpointUrl: endpointUrl.trim() || null,
          model: model.trim() || null,
        };
        await setLlmProviderConfig(config);
        setApiKey("");
      } else {
        // Key-requiring provider, blank key, same provider — preserve existing key.
        const settings: LlmProviderSettings = {
          provider: selectedProvider,
          endpointUrl: endpointUrl.trim() || null,
          model: model.trim() || null,
        };
        await saveNonSecretProviderSettings(settings);
      }
      setProviderStatus("Provider saved.");
    } catch (e) {
      setProviderError(
        e instanceof Error ? e.message : "Failed to save provider.",
      );
    } finally {
      setSaving(false);
    }
  }

  async function handleClear() {
    setProviderError(null);
    setProviderStatus(null);
    setClearing(true);
    try {
      await clearLlmProviderConfig();
      setProviderStatus("Provider cleared.");
    } catch (e) {
      setProviderError(
        e instanceof Error ? e.message : "Failed to clear provider.",
      );
    } finally {
      setClearing(false);
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
          title="LLM provider"
          className="support-card device-settings-card"
        >
          <p className="field-hint">
            Status:{" "}
            <strong>
              {providerConfigured
                ? `Configured (${bootstrap.llmProviderType ?? "unknown"})`
                : "Not configured"}
            </strong>
          </p>
          {bootstrap.providerConfigError ? (
            <p
              className="inline-banner error"
              data-testid="provider-config-error"
            >
              Config file error: {bootstrap.providerConfigError}
            </p>
          ) : null}

          <div className="form-grid">
            <label className="field">
              Provider
              <select
                value={selectedProvider}
                onChange={(e) =>
                  setSelectedProvider(e.target.value as LlmProviderType)
                }
                aria-label="LLM provider"
              >
                {PROVIDER_OPTIONS.map((opt) => (
                  <option key={opt.value} value={opt.value}>
                    {opt.label}
                  </option>
                ))}
              </select>
            </label>

            {requiresApiKey(selectedProvider) ? (
              <>
                <label className="field">
                  API key
                  <input
                    type="password"
                    placeholder={
                      hasExistingKeyForProvider
                        ? "Leave blank to keep existing key"
                        : selectedProvider === "anthropic"
                          ? "sk-ant-..."
                          : "sk-..."
                    }
                    value={apiKey}
                    onChange={(e) => setApiKey(e.target.value)}
                    aria-label="API key"
                  />
                </label>
                <p className="field-hint field-hint--warning">
                  Release builds store API keys in the OS keychain. Development
                  builds use a local file. Use a dedicated low-privilege key; do
                  not use production keys with access to sensitive resources.
                </p>
              </>
            ) : null}

            <label className="field">
              Endpoint URL{" "}
              <span style={{ fontWeight: 400, opacity: 0.7 }}>(optional)</span>
              <input
                placeholder={DEFAULT_ENDPOINTS[selectedProvider]}
                value={endpointUrl}
                onChange={(e) => setEndpointUrl(e.target.value)}
                aria-label="Endpoint URL"
              />
            </label>

            <label className="field">
              Model{" "}
              <span style={{ fontWeight: 400, opacity: 0.7 }}>(optional)</span>
              <input
                placeholder={DEFAULT_MODELS[selectedProvider] || "loaded model"}
                value={model}
                onChange={(e) => setModel(e.target.value)}
                aria-label="Model"
              />
            </label>
          </div>

          <div className="button-row">
            <button
              className="primary-button"
              onClick={handleSave}
              disabled={
                saving ||
                (requiresApiKey(selectedProvider) &&
                  !apiKey.trim() &&
                  !hasExistingKeyForProvider)
              }
              type="button"
            >
              Save
            </button>
            {providerConfigured ? (
              <button
                className="secondary-button"
                onClick={handleClear}
                disabled={clearing}
                type="button"
              >
                Clear
              </button>
            ) : null}
          </div>

          {providerStatus ? (
            <p className="inline-banner success">{providerStatus}</p>
          ) : null}
          {providerError ? (
            <p className="inline-banner error">{providerError}</p>
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
                onClick={() => {
                  setConfirmReset(true);
                  setResetSuccess(false);
                }}
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
                onClick={() => {
                  setConfirmClear(true);
                  setClearSuccess(false);
                }}
                type="button"
              >
                Clear saved invites ({recentJoinPayloads.length})
              </button>
            )}
          </div>
          {resetSuccess ? (
            <p className="inline-banner success">Host setup reset.</p>
          ) : null}
          {clearSuccess ? (
            <p className="inline-banner success">Saved invites cleared.</p>
          ) : null}
          <p className="field-hint">
            {recentJoinPayloads.length} saved invite
            {recentJoinPayloads.length === 1 ? "" : "s"}.
          </p>
        </SectionCard>
      </div>
    </ScreenShell>
  );
}
