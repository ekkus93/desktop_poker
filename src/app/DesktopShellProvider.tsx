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
  readStoredValue,
  storageKey,
} from "./shell";

type DesktopShellContextValue = {
  displayName: string;
  hostDraft: HostDraft;
  joinPayloadDraft: string;
  readySeats: number[];
  recentJoinPayloads: string[];
  setDisplayName: (value: string) => void;
  updateHostDraft: (patch: Partial<HostDraft>) => void;
  resetHostDraft: () => void;
  setJoinPayloadDraft: (value: string) => void;
  rememberJoinPayload: (value: string) => void;
  clearRecentJoinPayloads: () => void;
  toggleSeatReady: (seatIndex: number) => void;
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

  const [displayName, setDisplayName] = useState(() =>
    readStoredValue<string>(
      localStorage.getItem(storageKey(bootstrap.storageNamespace, "display-name")),
      defaultDisplayName,
    ),
  );
  const [hostDraft, setHostDraft] = useState(() =>
    readStoredValue<HostDraft>(
      localStorage.getItem(storageKey(bootstrap.storageNamespace, "host-draft")),
      defaultHostDraft,
    ),
  );
  const [joinPayloadDraft, setJoinPayloadDraft] = useState(() =>
    readStoredValue<string>(
      localStorage.getItem(storageKey(bootstrap.storageNamespace, "join-draft")),
      bootstrap.launchJoinPayload ?? "",
    ),
  );
  const [readySeats, setReadySeats] = useState(() =>
    readStoredValue<number[]>(
      localStorage.getItem(storageKey(bootstrap.storageNamespace, "ready-seats")),
      [],
    ),
  );
  const [recentJoinPayloads, setRecentJoinPayloads] = useState(() =>
    readStoredValue<string[]>(
      localStorage.getItem(
        storageKey(bootstrap.storageNamespace, "recent-join-payloads"),
      ),
      [],
    ),
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
      storageKey(bootstrap.storageNamespace, "ready-seats"),
      JSON.stringify(readySeats),
    );
  }, [bootstrap.storageNamespace, readySeats]);

  useEffect(() => {
      localStorage.setItem(
      storageKey(bootstrap.storageNamespace, "recent-join-payloads"),
      JSON.stringify(recentJoinPayloads),
    );
  }, [bootstrap.storageNamespace, recentJoinPayloads]);

  const value = useMemo<DesktopShellContextValue>(
    () => ({
      displayName,
      hostDraft,
      joinPayloadDraft,
      readySeats,
      recentJoinPayloads,
      setDisplayName,
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

        setRecentJoinPayloads((currentPayloads) => [
          trimmed,
          ...currentPayloads.filter((payload) => payload !== trimmed),
        ].slice(0, 5));
      },
      clearRecentJoinPayloads: () => setRecentJoinPayloads([]),
      toggleSeatReady: (seatIndex) => {
        setReadySeats((currentSeats) =>
          currentSeats.includes(seatIndex)
            ? currentSeats.filter((candidate) => candidate !== seatIndex)
            : [...currentSeats, seatIndex].sort((left, right) => left - right),
        );
      },
    }),
    [
      defaultHostDraft,
      displayName,
      hostDraft,
      joinPayloadDraft,
      readySeats,
      recentJoinPayloads,
    ],
  );

  return (
    <DesktopShellContext.Provider value={value}>
      {children}
    </DesktopShellContext.Provider>
  );
}

export { DesktopShellContext };
