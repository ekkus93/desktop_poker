import { useEffect, useRef, useState } from "react";
import type { NavigateFunction } from "react-router";
import {
  getClientSessionStatus,
  getHostSessionStatus,
  onSessionUpdate,
  type ClientSessionStatus,
  type HostSessionStatus,
} from "../api/desktop";

export type LobbySeatView = {
  seatIndex: number;
  label: string;
  detail: string;
  kind: "host" | "pending" | "open";
  isLocal: boolean;
  ready: boolean;
  isNpc: boolean;
};

export function buildLiveSeats(
  liveSession: HostSessionStatus | ClientSessionStatus,
): LobbySeatView[] {
  const totalSeats = liveSession.activeSeatCount + liveSession.openSeatCount;
  const seats: LobbySeatView[] = Array.from(
    { length: totalSeats },
    (_, index) => ({
      seatIndex: index + 1,
      label: "Open seat",
      detail: "",
      kind: "open",
      isLocal: false,
      ready: false,
      isNpc: false,
    }),
  );
  const localPlayerId =
    "localPlayerId" in liveSession ? liveSession.localPlayerId : "local-player";

  for (const participant of liveSession.participants) {
    const preferredIndex =
      participant.seatIndex ?? seats.findIndex((seat) => seat.kind === "open");
    if (preferredIndex < 0 || preferredIndex >= seats.length) {
      continue;
    }

    const isNpc = participant.playerId.startsWith("npc-seat-");
    seats[preferredIndex] = {
      seatIndex: preferredIndex + 1,
      label:
        participant.playerId === localPlayerId
          ? "You"
          : participant.displayName,
      detail: isNpc
        ? "(AI) · Always ready"
        : participant.seatIndex === null
          ? `${participant.participantState} · ${participant.connectionState} · awaiting seat`
          : participant.isHost
            ? `Host · ${participant.connectionState}`
            : participant.connectionState,
      kind: participant.seatIndex === null ? "pending" : "host",
      isLocal: participant.playerId === localPlayerId,
      ready: isNpc ? true : participant.isReady,
      isNpc,
    };
  }

  return seats;
}

const LOBBY_POLL_NORMAL_MS = 5000;
const LOBBY_POLL_SLOW_MS = 10000;
const LOBBY_BACKOFF_THRESHOLD = 3;
const LOBBY_ERROR_LIMIT = 10;

export type LobbySessionState = {
  hostSession: HostSessionStatus | null;
  clientSession: ClientSessionStatus | null;
  sessionStatus: "loading" | "ready";
  hostRecoveryError: string | null;
  connectionSlow: boolean;
  clientReconnecting: boolean;
  clientTerminated: boolean;
  setHostSession: (status: HostSessionStatus | null) => void;
  setClientSession: (status: ClientSessionStatus | null) => void;
};

export function useLobbySession(navigate: NavigateFunction): LobbySessionState {
  const [hostSession, setHostSession] = useState<HostSessionStatus | null>(
    null,
  );
  const [clientSession, setClientSession] =
    useState<ClientSessionStatus | null>(null);
  const [sessionStatus, setSessionStatus] = useState<"loading" | "ready">(
    "loading",
  );
  const [hostRecoveryError, setHostRecoveryError] = useState<string | null>(
    null,
  );
  const [connectionSlow, setConnectionSlow] = useState(false);
  const [clientReconnecting, setClientReconnecting] = useState(false);
  const [clientTerminated, setClientTerminated] = useState(false);

  const hostSessionRef = useRef<HostSessionStatus | null>(null);
  const consecutiveErrorsRef = useRef(0);

  useEffect(() => {
    hostSessionRef.current = hostSession;
  }, [hostSession]);

  useEffect(() => {
    let active = true;
    let timeoutId: number | undefined;

    const scheduleNext = (delayMs: number) => {
      if (!active) {
        return;
      }
      timeoutId = window.setTimeout(refresh, delayMs);
    };

    const applySnapshot = (
      nextHostSession: HostSessionStatus | null,
      nextClientSession: ClientSessionStatus | null,
    ) => {
      consecutiveErrorsRef.current = 0;
      setConnectionSlow(false);

      const previousHostSession = hostSessionRef.current;
      if (
        previousHostSession &&
        previousHostSession.phase !== "running" &&
        !nextHostSession &&
        !nextClientSession
      ) {
        setHostRecoveryError(
          "Hosting stopped before the table went live. Start hosting again or return home.",
        );
      } else if (nextHostSession) {
        setHostRecoveryError(null);
      }

      setHostSession(nextHostSession);
      setClientSession(nextClientSession);
      setSessionStatus("ready");
    };

    const applyClientConnectionState = (
      nextClientSession: ClientSessionStatus | null,
    ) => {
      if (nextClientSession) {
        setClientTerminated(nextClientSession.terminated);
        setClientReconnecting(
          !nextClientSession.terminated && nextClientSession.reconnecting,
        );
      } else {
        setClientReconnecting(false);
        setClientTerminated(false);
      }
    };

    const refresh = () => {
      void Promise.all([getHostSessionStatus(), getClientSessionStatus()])
        .then(([nextHostSession, nextClientSession]) => {
          if (!active) {
            return;
          }

          applySnapshot(nextHostSession, nextClientSession);

          if (nextClientSession?.terminated) {
            setClientTerminated(true);
            setClientReconnecting(false);
            return; // stop polling — session is terminal
          }
          applyClientConnectionState(nextClientSession);
          scheduleNext(LOBBY_POLL_NORMAL_MS);
        })
        .catch(() => {
          if (!active) {
            return;
          }

          consecutiveErrorsRef.current += 1;

          if (consecutiveErrorsRef.current >= LOBBY_ERROR_LIMIT) {
            navigate("/errors", { replace: true });
            return;
          }

          setConnectionSlow(
            consecutiveErrorsRef.current >= LOBBY_BACKOFF_THRESHOLD,
          );
          setSessionStatus("ready");
          scheduleNext(
            consecutiveErrorsRef.current >= LOBBY_BACKOFF_THRESHOLD
              ? LOBBY_POLL_SLOW_MS
              : LOBBY_POLL_NORMAL_MS,
          );
        });
    };

    refresh();

    const unlistenPromise = onSessionUpdate(() => {
      if (!active) {
        return;
      }
      void Promise.all([getHostSessionStatus(), getClientSessionStatus()])
        .then(([nextHostSession, nextClientSession]) => {
          if (!active) {
            return;
          }
          applySnapshot(nextHostSession, nextClientSession);
          applyClientConnectionState(nextClientSession);
        })
        .catch(() => {
          // Ignore event-driven refresh errors; the fallback poll handles recovery
        });
    });

    return () => {
      active = false;
      window.clearTimeout(timeoutId);
      void unlistenPromise.then((unlisten) => unlisten());
    };
  }, [navigate]);

  return {
    hostSession,
    clientSession,
    sessionStatus,
    hostRecoveryError,
    connectionSlow,
    clientReconnecting,
    clientTerminated,
    setHostSession,
    setClientSession,
  };
}
