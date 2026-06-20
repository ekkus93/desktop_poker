import {
  createContext,
  type ReactNode,
  useEffect,
  useMemo,
  useState,
} from "react";
import type { DesktopBootstrapState } from "../api/desktop";
import {
  createDefaultDisplayName,
  createDefaultHostDraft,
  type HostDraft,
  normalizeHostDraft,
  readStoredValueWithStatus,
  storageKey,
} from "./shell";
import {
  initializeWindowStatePersistence,
  persistHandHistory,
  readPersistedHandHistoryWithStatus,
} from "./persistence";

export type LastEndedSession = {
  role: "host" | "client";
  tournamentName: string;
};

type DesktopShellContextValue = {
  bootstrap: DesktopBootstrapState;
  displayName: string;
  hostDraft: HostDraft;
  joinPayloadDraft: string;
  recentJoinPayloads: string[];
  persistedHandHistoryCount: number;
  startupWarnings: string[];
  lastEndedSession: LastEndedSession | null;
  wasHost: boolean;
  tableSidePanelOpen: boolean;
  setDisplayName: (value: string) => void;
  setLastEndedSession: (value: LastEndedSession | null) => void;
  setWasHost: (value: boolean) => void;
  setTableSidePanelOpen: (value: boolean) => void;
  updateHostDraft: (patch: Partial<HostDraft>) => void;
  resetHostDraft: () => void;
  setJoinPayloadDraft: (value: string) => void;
  rememberJoinPayload: (value: string) => void;
  clearRecentJoinPayloads: () => void;
  persistHandHistory: (
    entries: import("../api/desktop").TableHistoryEntryView[],
  ) => void;
};

const DesktopShellContext = createContext<DesktopShellContextValue | undefined>(
  undefined,
);

export function DesktopShellProvider({
  bootstrap,
  children,
}: {
  bootstrap: DesktopBootstrapState;
  children: ReactNode;
}) {
  const defaultHostDraft = useMemo(
    () => createDefaultHostDraft(bootstrap),
    [bootstrap],
  );
  const defaultDisplayName = useMemo(
    () => createDefaultDisplayName(bootstrap),
    [bootstrap],
  );

  const storedDisplayName = useMemo(
    () =>
      readStoredValueWithStatus<string>(
        localStorage.getItem(
          storageKey(bootstrap.storageNamespace, "display-name"),
        ),
        defaultDisplayName,
      ),
    [bootstrap.storageNamespace, defaultDisplayName],
  );
  const storedHostDraft = useMemo(
    () =>
      readStoredValueWithStatus<unknown>(
        localStorage.getItem(
          storageKey(bootstrap.storageNamespace, "host-draft"),
        ),
        defaultHostDraft,
      ),
    [bootstrap.storageNamespace, defaultHostDraft],
  );
  const storedJoinDraft = useMemo(
    () =>
      readStoredValueWithStatus<string>(
        localStorage.getItem(
          storageKey(bootstrap.storageNamespace, "join-draft"),
        ),
        bootstrap.launchJoinPayload ?? "",
      ),
    [bootstrap.launchJoinPayload, bootstrap.storageNamespace],
  );
  const storedRecentJoinPayloads = useMemo(
    () =>
      readStoredValueWithStatus<string[]>(
        localStorage.getItem(
          storageKey(bootstrap.storageNamespace, "recent-join-payloads"),
        ),
        [],
      ),
    [bootstrap.storageNamespace],
  );
  const storedHandHistory = useMemo(
    () => readPersistedHandHistoryWithStatus(bootstrap.storageNamespace),
    [bootstrap.storageNamespace],
  );
  const storedLastEndedSession = useMemo(
    () =>
      readStoredValueWithStatus<LastEndedSession | null>(
        localStorage.getItem(
          storageKey(bootstrap.storageNamespace, "last-ended-session"),
        ),
        null,
      ),
    [bootstrap.storageNamespace],
  );
  const startupWarnings = useMemo(() => {
    const warnings = new Set<string>();

    if (
      storedDisplayName.hadParseError ||
      storedHostDraft.hadParseError ||
      storedJoinDraft.hadParseError ||
      storedRecentJoinPayloads.hadParseError
    ) {
      warnings.add(
        "Some saved local preferences were unreadable and were reset to safe defaults.",
      );
    }

    if (storedHandHistory.hadParseError) {
      warnings.add(
        "Saved hand history was unreadable and has been ignored for this session.",
      );
    }

    return [...warnings];
  }, [
    storedDisplayName.hadParseError,
    storedHandHistory.hadParseError,
    storedHostDraft.hadParseError,
    storedJoinDraft.hadParseError,
    storedRecentJoinPayloads.hadParseError,
  ]);

  const [displayName, setDisplayName] = useState(() => storedDisplayName.value);
  const [lastEndedSession, setLastEndedSessionState] = useState<LastEndedSession | null>(
    () => storedLastEndedSession.value,
  );
  const [wasHost, setWasHost] = useState(false);
  const [tableSidePanelOpen, setTableSidePanelOpen] = useState(false);
  const [hostDraft, setHostDraft] = useState(() =>
    normalizeHostDraft(storedHostDraft.value, defaultHostDraft),
  );
  const [joinPayloadDraft, setJoinPayloadDraft] = useState(
    () => storedJoinDraft.value,
  );
  const [recentJoinPayloads, setRecentJoinPayloads] = useState(
    () => storedRecentJoinPayloads.value,
  );
  const [persistedHandHistoryCount, setPersistedHandHistoryCount] = useState(
    () => storedHandHistory.history?.entries.length ?? 0,
  );

  useEffect(
    () => initializeWindowStatePersistence(bootstrap.storageNamespace),
    [bootstrap.storageNamespace],
  );

  useEffect(() => {
    localStorage.setItem(
      storageKey(bootstrap.storageNamespace, "display-name"),
      JSON.stringify(displayName),
    );
  }, [bootstrap.storageNamespace, displayName]);

  useEffect(() => {
    localStorage.setItem(
      storageKey(bootstrap.storageNamespace, "host-draft"),
      JSON.stringify(hostDraft),
    );
  }, [bootstrap.storageNamespace, hostDraft]);

  useEffect(() => {
    localStorage.setItem(
      storageKey(bootstrap.storageNamespace, "join-draft"),
      JSON.stringify(joinPayloadDraft),
    );
  }, [bootstrap.storageNamespace, joinPayloadDraft]);

  useEffect(() => {
    localStorage.setItem(
      storageKey(bootstrap.storageNamespace, "recent-join-payloads"),
      JSON.stringify(recentJoinPayloads),
    );
  }, [bootstrap.storageNamespace, recentJoinPayloads]);


  const value = useMemo<DesktopShellContextValue>(
    () => ({
      bootstrap,
      displayName,
      hostDraft,
      joinPayloadDraft,
      recentJoinPayloads,
      persistedHandHistoryCount,
      startupWarnings,
      lastEndedSession,
      wasHost,
      tableSidePanelOpen,
      setDisplayName,
      setLastEndedSession: (v) => {
        localStorage.setItem(
          storageKey(bootstrap.storageNamespace, "last-ended-session"),
          JSON.stringify(v),
        );
        setLastEndedSessionState(v);
      },
      setWasHost,
      setTableSidePanelOpen,
      updateHostDraft: (patch) => {
        setHostDraft((currentDraft) => ({
          ...currentDraft,
          ...patch,
        }));
      },
      resetHostDraft: () => setHostDraft(defaultHostDraft),
      setJoinPayloadDraft,
      rememberJoinPayload: (value) => {
        const trimmed = value.trim();
        if (!trimmed) {
          return;
        }

        setRecentJoinPayloads((currentPayloads) =>
          [
            trimmed,
            ...currentPayloads.filter((payload) => payload !== trimmed),
          ].slice(0, 5),
        );
      },
      clearRecentJoinPayloads: () => setRecentJoinPayloads([]),
      persistHandHistory: (entries) => {
        persistHandHistory(bootstrap.storageNamespace, entries);
        setPersistedHandHistoryCount(entries.length);
      },
    }),
    [
      bootstrap,
      defaultHostDraft,
      displayName,
      hostDraft,
      joinPayloadDraft,
      lastEndedSession,
      persistedHandHistoryCount,
      recentJoinPayloads,
      startupWarnings,
      tableSidePanelOpen,
      wasHost,
    ],
  );

  return (
    <DesktopShellContext.Provider value={value}>
      {children}
    </DesktopShellContext.Provider>
  );
}

export { DesktopShellContext };
