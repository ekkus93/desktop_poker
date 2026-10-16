import { useEffect, useMemo, useRef, useState } from "react";
import { History, PanelRight, Trophy, WifiOff } from "lucide-react";
import { Link, useNavigate } from "react-router";
import {
  getTableView,
  onTableUpdate,
  submitTableAction,
  type DesktopTableActionKind,
  type TableViewSnapshot,
  type TableViewerMode,
} from "../api/desktop";
import { useDesktopShell } from "../app/useDesktopShell";
import { AccessibleDialog } from "../components/shared/AccessibleDialog";
import { SectionCard } from "../components/shared/SectionCard";
import { StatusBadge } from "../components/shared/StatusBadge";
import { ScreenShell } from "./ScreenShell";
import { PlayingCard, SeatCard } from "./MainTableCards";
import { TableSidePanel } from "./MainTableSidePanel";
import {
  buildQuickSizes,
  defaultRaiseAmount,
  getErrorMessage,
  isWithinRaiseBounds,
} from "./mainTableRaise";
import type { ScreenProps } from "./types";

const BOARD_SLOT_COUNT = 5;
const TABLE_POLL_NORMAL_MS = 5000;
const TABLE_POLL_SLOW_MS = 10000;
const POLL_BACKOFF_THRESHOLD = 3;
const POLL_ERROR_LIMIT = 10;

export function MainTableScreen({ bootstrap }: ScreenProps) {
  void bootstrap;
  const {
    displayName,
    persistHandHistory,
    tableSidePanelOpen,
    setTableSidePanelOpen,
  } = useDesktopShell();
  const navigate = useNavigate();
  const viewerMode: TableViewerMode = "local";
  const [tableView, setTableView] = useState<TableViewSnapshot | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [connectionSlow, setConnectionSlow] = useState(false);
  const [actionError, setActionError] = useState<string | null>(null);
  const [submitting, setSubmitting] = useState(false);
  const [raiseAmount, setRaiseAmount] = useState<number | null>(null);
  const [confirmation, setConfirmation] = useState<{
    actionKind: "betOrRaise" | "allIn";
    label: string;
  } | null>(null);
  const consecutiveErrorsRef = useRef(0);
  const latestRequestRef = useRef(0);
  const terminatedSessionRef = useRef<string | null>(null);

  useEffect(() => {
    let cancelled = false;
    let timeoutId: number | undefined;

    const scheduleNext = (delayMs: number) => {
      if (cancelled) {
        return;
      }

      window.clearTimeout(timeoutId);
      timeoutId = window.setTimeout(() => {
        void poll();
      }, delayMs);
    };

    const isCurrentSnapshot = (
      snapshot: TableViewSnapshot,
      generation: number,
    ) => {
      if (cancelled || generation !== latestRequestRef.current) {
        return false;
      }
      const sessionId = `${snapshot.tableId}:${snapshot.sessionEpoch}`;
      if (
        terminatedSessionRef.current === sessionId &&
        snapshot.sessionConnection !== "terminated"
      ) {
        return false;
      }
      if (snapshot.sessionConnection === "terminated") {
        terminatedSessionRef.current = sessionId;
      } else if (terminatedSessionRef.current !== sessionId) {
        terminatedSessionRef.current = null;
      }
      return true;
    };

    const poll = async () => {
      const generation = ++latestRequestRef.current;
      try {
        const snapshot = await getTableView(viewerMode);
        if (!isCurrentSnapshot(snapshot, generation)) {
          return;
        }

        consecutiveErrorsRef.current = 0;
        setTableView(snapshot);
        setLoading(false);
        setError(null);
        setConnectionSlow(false);
        scheduleNext(TABLE_POLL_NORMAL_MS);
      } catch (caughtError: unknown) {
        if (cancelled || generation !== latestRequestRef.current) {
          return;
        }

        const msg = getErrorMessage(caughtError).toLowerCase();
        if (msg.includes("disconnected from host")) {
          navigate("/errors", { replace: true });
          return;
        }

        consecutiveErrorsRef.current += 1;
        if (consecutiveErrorsRef.current >= POLL_ERROR_LIMIT) {
          navigate("/errors", { replace: true });
          return;
        }

        setConnectionSlow(
          consecutiveErrorsRef.current >= POLL_BACKOFF_THRESHOLD,
        );
        scheduleNext(
          consecutiveErrorsRef.current >= POLL_BACKOFF_THRESHOLD
            ? TABLE_POLL_SLOW_MS
            : TABLE_POLL_NORMAL_MS,
        );
      }
    };

    const loadInitial = async () => {
      const generation = ++latestRequestRef.current;
      setLoading(true);
      setError(null);
      setActionError(null);
      setConfirmation(null);

      try {
        const snapshot = await getTableView(viewerMode);
        if (isCurrentSnapshot(snapshot, generation)) {
          consecutiveErrorsRef.current = 0;
          setTableView(snapshot);
          setError(null);
          setConnectionSlow(false);
        }
      } catch (caughtError: unknown) {
        if (!cancelled && generation === latestRequestRef.current) {
          setError(getErrorMessage(caughtError));
          setTableView(null);
        }
      } finally {
        if (!cancelled && generation === latestRequestRef.current) {
          setLoading(false);
          scheduleNext(TABLE_POLL_NORMAL_MS);
        }
      }
    };

    void loadInitial();

    // Subscribe to table-update events for immediate refresh on state changes.
    // The fallback poll above catches any missed events.
    const unlistenPromise = onTableUpdate(() => {
      if (cancelled) {
        return;
      }
      const generation = ++latestRequestRef.current;
      void getTableView(viewerMode)
        .then((snapshot) => {
          if (isCurrentSnapshot(snapshot, generation)) {
            consecutiveErrorsRef.current = 0;
            setTableView(snapshot);
            setError(null);
            setConnectionSlow(false);
            setLoading(false);
            if (snapshot.sessionConnection !== "terminated") {
              scheduleNext(TABLE_POLL_NORMAL_MS);
            }
          }
        })
        .catch(() => {
          if (cancelled || generation !== latestRequestRef.current) {
            return;
          }
          // If an event supersedes the initial request and then fails, the
          // original request cannot schedule its fallback poll. Ensure the
          // newest generation always schedules recovery without clearing errors.
          scheduleNext(
            consecutiveErrorsRef.current >= POLL_BACKOFF_THRESHOLD
              ? TABLE_POLL_SLOW_MS
              : TABLE_POLL_NORMAL_MS,
          );
        });
    });

    return () => {
      cancelled = true;
      latestRequestRef.current += 1;
      window.clearTimeout(timeoutId);
      void unlistenPromise.then((unlisten) => unlisten());
    };
  }, [navigate, viewerMode]);

  useEffect(() => {
    if (!tableView?.handHistory.length) {
      return;
    }

    persistHandHistory(tableView);
  }, [persistHandHistory, tableView]);

  const boardCards = useMemo(() => {
    const visibleCards = tableView?.boardCards ?? [];
    return Array.from(
      { length: BOARD_SLOT_COUNT },
      (_, index) => visibleCards[index] ?? null,
    );
  }, [tableView]);

  const quickSizes = useMemo(
    () => buildQuickSizes(tableView?.actionTray),
    [tableView],
  );
  const effectiveRaiseAmount = useMemo(() => {
    const actionTray = tableView?.actionTray;
    if (!actionTray) {
      return null;
    }
    if (raiseAmount !== null && isWithinRaiseBounds(raiseAmount, actionTray)) {
      return raiseAmount;
    }
    return defaultRaiseAmount(actionTray);
  }, [raiseAmount, tableView]);
  const handStarted = tableView?.currentHandNumber !== null;
  const denseSeatMap = (tableView?.seats.length ?? 0) > 6;

  const submitAction = async (
    actionKind: DesktopTableActionKind,
    nextRaiseAmount?: number,
  ) => {
    const generation = ++latestRequestRef.current;
    setSubmitting(true);
    setActionError(null);
    try {
      const snapshot = await submitTableAction(
        viewerMode,
        actionKind,
        nextRaiseAmount,
      );
      const sessionId = `${snapshot.tableId}:${snapshot.sessionEpoch}`;
      if (
        generation === latestRequestRef.current &&
        (terminatedSessionRef.current !== sessionId ||
          snapshot.sessionConnection === "terminated")
      ) {
        setTableView(snapshot);
        setConfirmation(null);
      }
    } catch (caughtError) {
      setActionError(getErrorMessage(caughtError));
    } finally {
      setSubmitting(false);
    }
  };

  const queueConfirmation = (
    actionKind: "betOrRaise" | "allIn",
    label: string,
  ) => {
    setConfirmation({ actionKind, label });
    setActionError(null);
  };

  const confirmQueuedAction = async () => {
    if (!confirmation || !tableView?.actionTray) {
      return;
    }

    const nextRaiseAmount =
      confirmation.actionKind === "betOrRaise"
        ? (effectiveRaiseAmount ?? undefined)
        : undefined;
    await submitAction(confirmation.actionKind, nextRaiseAmount);
  };

  const handlePrimaryAction = async (actionKind: DesktopTableActionKind) => {
    if (actionKind === "betOrRaise") {
      if (!tableView?.actionTray) {
        return;
      }

      await submitAction(
        actionKind,
        effectiveRaiseAmount ?? defaultRaiseAmount(tableView.actionTray),
      );
      return;
    }

    if (actionKind === "allIn") {
      queueConfirmation("allIn", "Confirm all-in");
      return;
    }

    await submitAction(actionKind);
  };
