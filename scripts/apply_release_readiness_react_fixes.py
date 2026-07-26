#!/usr/bin/env python3
"""Apply the bounded React correctness fixes exposed by the ESLint 10 upgrade.

This is a one-shot release-readiness migration helper. It asserts that each
expected source block occurs exactly once so it cannot silently rewrite an
unexpected file shape.
"""

from __future__ import annotations

import re
from pathlib import Path


def replace_once(path: str, old: str, new: str) -> None:
    target = Path(path)
    text = target.read_text(encoding="utf-8")
    count = text.count(old)
    if count != 1:
        raise RuntimeError(f"expected one exact block in {path}, found {count}")
    target.write_text(text.replace(old, new, 1), encoding="utf-8")


def sub_once(path: str, pattern: str, replacement: str, *, flags: int = 0) -> None:
    target = Path(path)
    text = target.read_text(encoding="utf-8")
    updated, count = re.subn(pattern, replacement, text, count=1, flags=flags)
    if count != 1:
        raise RuntimeError(
            f"expected one regex block in {path}, found {count}: {pattern}"
        )
    target.write_text(updated, encoding="utf-8")


def fix_device_settings() -> None:
    path = "src/screens/DeviceSettingsScreen.tsx"
    replace_once(
        path,
        '''  // When the user explicitly changes the provider dropdown, reset endpoint/model
  // placeholders only if it differs from the loaded settings' provider.
  useEffect(() => {
    if (loadedSettings && selectedProvider === loadedSettings.provider) {
      return;
    }
    setEndpointUrl("");
    setModel("");
  }, [selectedProvider, loadedSettings]);
''',
        '''  function handleProviderChange(nextProvider: LlmProviderType) {
    setSelectedProvider(nextProvider);
    setProviderError(null);
    setProviderStatus(null);

    if (loadedSettings?.provider === nextProvider) {
      setEndpointUrl(loadedSettings.endpointUrl ?? "");
      setModel(loadedSettings.model ?? "");
      return;
    }

    setEndpointUrl("");
    setModel("");
  }
''',
    )
    sub_once(
        path,
        r'''onChange=\{\(e\) =>\s*setSelectedProvider\(e\.target\.value as LlmProviderType\)\s*\}''',
        '''onChange={(e) =>
                  handleProviderChange(e.target.value as LlmProviderType)
                }''',
    )


def fix_npc_profiles() -> None:
    replace_once(
        "src/screens/NpcProfilesScreen.tsx",
        '''  useEffect(() => {
    void reload();
  }, []);
''',
        '''  useEffect(() => {
    let cancelled = false;

    void listNpcProfiles()
      .then((result) => {
        if (cancelled) {
          return;
        }
        setProfiles(result.profiles);
        setProfileErrors(result.errors);
        setListError(null);
      })
      .catch((error: unknown) => {
        if (cancelled) {
          return;
        }
        setListError(
          error instanceof Error ? error.message : "Failed to load profiles.",
        );
      })
      .finally(() => {
        if (!cancelled) {
          setLoading(false);
        }
      });

    return () => {
      cancelled = true;
    };
  }, []);
''',
    )


def fix_join_screen() -> None:
    path = "src/screens/JoinTournamentScreen.tsx"
    replace_once(
        path,
        '''  const [validationState, setValidationState] = useState<ValidationState>({
    status: "idle",
  });
  const [inviteBanner, setInviteBanner] = useState<string | null>(null);
''',
        '''  const [validationState, setValidationState] = useState<ValidationState>(() => {
    if (joinPayloadDraft !== bootstrap.launchJoinPayload) {
      return { status: "idle" };
    }
    if (bootstrap.parsedLaunchJoinPayload) {
      return {
        status: "valid",
        payload: bootstrap.parsedLaunchJoinPayload,
      };
    }
    if (bootstrap.launchJoinPayloadError) {
      return {
        status: "invalid",
        message: bootstrap.launchJoinPayloadError,
      };
    }
    return { status: "idle" };
  });
  const [inviteBanner, setInviteBanner] = useState<string | null>(() =>
    joinPayloadDraft === bootstrap.launchJoinPayload &&
    bootstrap.parsedLaunchJoinPayload
      ? "Invite already attached to this launch."
      : null,
  );
''',
    )
    sub_once(
        path,
        r'''  useEffect\(\(\) => \{\n    if \(joinPayloadDraft !== bootstrap\.launchJoinPayload\) \{.*?\n  \}, \[\n    bootstrap\.launchJoinPayload,\n    bootstrap\.launchJoinPayloadError,\n    bootstrap\.parsedLaunchJoinPayload,\n    joinPayloadDraft,\n  \]\);\n\n''',
        "",
        flags=re.DOTALL,
    )
    replace_once(
        path,
        "{launchJoinAttemptedForPayload.current ? (",
        "{bootstrap.launchJoinPayload ? (",
    )


def fix_main_table() -> None:
    path = "src/screens/MainTableScreen.tsx"
    replace_once(
        path,
        '''  useEffect(() => {
    const actionTray = tableView?.actionTray;
    if (!actionTray) {
      setRaiseAmount(null);
      return;
    }

    setRaiseAmount((currentAmount) => {
      if (
        currentAmount !== null &&
        isWithinRaiseBounds(currentAmount, actionTray)
      ) {
        return currentAmount;
      }

      return defaultRaiseAmount(actionTray);
    });
  }, [tableView]);

''',
        "",
    )
    replace_once(
        path,
        '''  const quickSizes = useMemo(
    () => buildQuickSizes(tableView?.actionTray),
    [tableView],
  );
''',
        '''  const quickSizes = useMemo(
    () => buildQuickSizes(tableView?.actionTray),
    [tableView],
  );
  const effectiveRaiseAmount = useMemo(() => {
    const actionTray = tableView?.actionTray;
    if (!actionTray) {
      return null;
    }
    if (
      raiseAmount !== null &&
      isWithinRaiseBounds(raiseAmount, actionTray)
    ) {
      return raiseAmount;
    }
    return defaultRaiseAmount(actionTray);
  }, [raiseAmount, tableView]);
''',
    )
    replace_once(
        path,
        '''confirmation.actionKind === "betOrRaise"
        ? (raiseAmount ?? undefined)''',
        '''confirmation.actionKind === "betOrRaise"
        ? (effectiveRaiseAmount ?? undefined)''',
    )
    target = Path(path)
    text = target.read_text(encoding="utf-8")
    updated, count = re.subn(
        r'''raiseAmount\s*\?\?\s*defaultRaiseAmount\(tableView\.actionTray\)''',
        "effectiveRaiseAmount ?? defaultRaiseAmount(tableView.actionTray)",
        text,
    )
    if count < 1:
        raise RuntimeError("expected at least one rendered raise amount expression")
    target.write_text(updated, encoding="utf-8")


def main() -> None:
    fix_device_settings()
    fix_npc_profiles()
    fix_join_screen()
    fix_main_table()


if __name__ == "__main__":
    main()
