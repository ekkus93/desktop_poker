import { invoke } from "@tauri-apps/api/core";
import type {
  TableCardView,
  TableHistoryEntryView,
  TableViewSnapshot,
} from "../api/desktop";
import {
  readStoredValue,
  readStoredValueWithStatus,
  storageKey,
  writeStoredValueWithStatus,
} from "./shell";

export type PersistedHandHistory = {
  updatedAtMs: number;
  entries: TableHistoryEntryView[];
};

export type PersistenceWriteFailureHandler = (message: string) => void;

type PersistedWindowState = {
  width: number;
  height: number;
  x: number;
  y: number;
  maximized: boolean;
};

const HAND_HISTORY_STORAGE_SUFFIX = "hand-history-summaries";
const WINDOW_STATE_STORAGE_SUFFIX = "window-state";

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null;
}

function isFiniteNumber(value: unknown): value is number {
  return typeof value === "number" && Number.isFinite(value);
}

function isStringArray(value: unknown): value is string[] {
  return (
    Array.isArray(value) && value.every((entry) => typeof entry === "string")
  );
}

function isTableCardView(value: unknown): value is TableCardView {
  return (
    isRecord(value) &&
    typeof value.label === "string" &&
    typeof value.compactLabel === "string" &&
    typeof value.suitSymbol === "string" &&
    (value.tone === "red" || value.tone === "dark")
  );
}

function isTableHistoryEntryView(
  value: unknown,
): value is TableHistoryEntryView {
  return (
    isRecord(value) &&
    isFiniteNumber(value.handNumber) &&
    typeof value.summary === "string" &&
    isFiniteNumber(value.potTotal) &&
    isStringArray(value.winningPlayers) &&
    isStringArray(value.eliminatedPlayers) &&
    Array.isArray(value.boardCards) &&
    value.boardCards.every(isTableCardView)
  );
}

function normalizeHandHistoryEntries(entries: unknown[]) {
  const entriesByHandNumber = new Map<number, TableHistoryEntryView>();

  for (const entry of entries) {
    if (isTableHistoryEntryView(entry)) {
      entriesByHandNumber.set(entry.handNumber, entry);
    }
  }

  return [...entriesByHandNumber.values()].sort(
    (left, right) => right.handNumber - left.handNumber,
  );
}

function normalizePersistedHandHistory(
  value: unknown,
): PersistedHandHistory | null {
  if (!isRecord(value) || !Array.isArray(value.entries)) {
    return null;
  }

  return {
    updatedAtMs:
      isFiniteNumber(value.updatedAtMs) && value.updatedAtMs >= 0
        ? value.updatedAtMs
        : 0,
    entries: normalizeHandHistoryEntries(value.entries),
  };
}

function normalizePersistedWindowState(
  value: unknown,
): PersistedWindowState | null {
  if (
    !isRecord(value) ||
    !isFiniteNumber(value.width) ||
    value.width <= 0 ||
    !isFiniteNumber(value.height) ||
    value.height <= 0 ||
    !isFiniteNumber(value.x) ||
    !isFiniteNumber(value.y) ||
    typeof value.maximized !== "boolean"
  ) {
    return null;
  }

  return {
    width: value.width,
    height: value.height,
    x: value.x,
    y: value.y,
    maximized: value.maximized,
  };
}

function reportPersistenceWriteFailure(
  context: string,
  error: string,
  onWriteFailure?: PersistenceWriteFailureHandler,
) {
  const message = `Failed to persist ${context}: ${error}`;
  if (onWriteFailure) {
    onWriteFailure(message);
  } else {
    console.error(message);
  }
}

export function readPersistedHandHistory(storageNamespace: string) {
  return readPersistedHandHistoryWithStatus(storageNamespace).history;
}

export function readPersistedHandHistoryWithStatus(storageNamespace: string) {
  const storedValue = readStoredValueWithStatus<unknown>(
    localStorage.getItem(
      storageKey(storageNamespace, HAND_HISTORY_STORAGE_SUFFIX),
    ),
    null,
  );

  return {
    history: normalizePersistedHandHistory(storedValue.value),
    hadParseError: storedValue.hadParseError,
  };
}

function writePersistedHandHistory(
  storageNamespace: string,
  entries: TableHistoryEntryView[],
  updatedAtMs = Date.now(),
  onWriteFailure?: PersistenceWriteFailureHandler,
): PersistedHandHistory {
  const history = {
    updatedAtMs,
    entries: normalizeHandHistoryEntries(entries),
  } satisfies PersistedHandHistory;

  const result = writeStoredValueWithStatus(
    storageKey(storageNamespace, HAND_HISTORY_STORAGE_SUFFIX),
    history,
  );
  if (!result.ok) {
    reportPersistenceWriteFailure(
      "cached hand history",
      result.error,
      onWriteFailure,
    );
  }

  return history;
}

export function persistHandHistory(
  storageNamespace: string,
  entries: TableHistoryEntryView[],
  onWriteFailure?: PersistenceWriteFailureHandler,
) {
  writePersistedHandHistory(
    storageNamespace,
    entries,
    Date.now(),
    onWriteFailure,
  );
}

export function mergePersistedHandHistory(
  storageNamespace: string,
  entries: TableHistoryEntryView[],
  onWriteFailure?: PersistenceWriteFailureHandler,
) {
  const existingEntries =
    readPersistedHandHistory(storageNamespace)?.entries ?? [];

  return writePersistedHandHistory(
    storageNamespace,
    [...existingEntries, ...entries],
    Date.now(),
    onWriteFailure,
  );
}

export async function mergeDurableHandHistory(
  storageNamespace: string,
  entries: TableHistoryEntryView[],
  onWriteFailure?: PersistenceWriteFailureHandler,
): Promise<PersistedHandHistory> {
  const cachedHistory = mergePersistedHandHistory(
    storageNamespace,
    entries,
    onWriteFailure,
  );
  if (!hasUsableTauriWindowRuntime()) {
    return cachedHistory;
  }

  const currentDurable = await invoke<PersistedHandHistory | null>(
    "read_persisted_hand_history",
  );
  const currentNumbers = new Set(
    (currentDurable?.entries ?? []).map((entry) => entry.handNumber),
  );
  const incomingNumbers = new Set(entries.map((entry) => entry.handNumber));
  const migrationEntries = cachedHistory.entries.filter(
    (entry) =>
      !currentNumbers.has(entry.handNumber) &&
      !incomingNumbers.has(entry.handNumber),
  );
  const authoritativeEntries = [...migrationEntries, ...entries];
  if (authoritativeEntries.length === 0 && currentDurable) {
    return writePersistedHandHistory(
      storageNamespace,
      currentDurable.entries,
      currentDurable.updatedAtMs,
      onWriteFailure,
    );
  }
  const durableHistory = await invoke<PersistedHandHistory>(
    "merge_persisted_hand_history",
    { entries: authoritativeEntries },
  );
  return writePersistedHandHistory(
    storageNamespace,
    durableHistory.entries,
    durableHistory.updatedAtMs,
    onWriteFailure,
  );
}

export async function readDurableHandHistory(
  storageNamespace: string,
  onWriteFailure?: PersistenceWriteFailureHandler,
): Promise<PersistedHandHistory | null> {
  const cachedHistory = readPersistedHandHistory(storageNamespace);
  if (!hasUsableTauriWindowRuntime()) {
    return cachedHistory;
  }

  const durableHistory = await invoke<PersistedHandHistory | null>(
    "read_persisted_hand_history",
  );
  // Durable disk state is authoritative. Cached browser entries are only a
  // migration source for hand numbers not yet present in the durable store.
  // In particular, never write stale cache copies back over a newer record.
  const durableNumbers = new Set(
    (durableHistory?.entries ?? []).map((entry) => entry.handNumber),
  );
  const missingCachedEntries = (cachedHistory?.entries ?? []).filter(
    (entry) => !durableNumbers.has(entry.handNumber),
  );
  if (missingCachedEntries.length === 0) {
    if (!durableHistory) {
      return null;
    }
    return writePersistedHandHistory(
      storageNamespace,
      durableHistory.entries,
      durableHistory.updatedAtMs,
      onWriteFailure,
    );
  }

  const mergedHistory = await invoke<PersistedHandHistory>(
    "merge_persisted_hand_history",
    { entries: missingCachedEntries },
  );
  return writePersistedHandHistory(
    storageNamespace,
    mergedHistory.entries,
    mergedHistory.updatedAtMs,
    onWriteFailure,
  );
}

/** Tournament-aware V2 records are separate from the immutable legacy V1 file. */
export type PersistedHistoryEntryV2 = TableHistoryEntryView & {
  provenance: "live" | "importedDurableV1" | "importedCacheV1";
  tableId?: string;
  sessionEpoch?: number;
  legacyOriginId?: string;
  tournamentName?: string;
  sessionStartedAtMs?: number;
};

export type PersistedHandHistoryV2 = {
  schemaVersion: 2;
  updatedAtMs: number;
  entries: PersistedHistoryEntryV2[];
};

function cacheV1Entries(storageNamespace: string): PersistedHistoryEntryV2[] {
  return (readPersistedHandHistory(storageNamespace)?.entries ?? []).map(
    (entry) => ({
      ...entry,
      provenance: "importedCacheV1",
      legacyOriginId: `cache-v1:${storageNamespace}`,
    }),
  );
}

function liveV2Entries(snapshot: TableViewSnapshot): PersistedHistoryEntryV2[] {
  if (
    !snapshot.tableId.trim() ||
    !Number.isSafeInteger(snapshot.sessionEpoch) ||
    snapshot.sessionEpoch <= 0
  ) {
    throw new Error(
      "Cannot save live hand history without a valid session identity",
    );
  }
  return snapshot.handHistory.map((entry) => ({
    ...entry,
    provenance: "live",
    tableId: snapshot.tableId,
    sessionEpoch: snapshot.sessionEpoch,
    tournamentName: snapshot.tournamentName,
  }));
}

function missingCacheImports(
  persisted: PersistedHandHistoryV2 | null,
  cached: PersistedHistoryEntryV2[],
): PersistedHistoryEntryV2[] {
  const existing = new Set(
    (persisted?.entries ?? [])
      .filter((entry) => entry.provenance === "importedCacheV1")
      .map((entry) => `${entry.legacyOriginId}:${entry.handNumber}`),
  );
  return cached.filter(
    (entry) => !existing.has(`${entry.legacyOriginId}:${entry.handNumber}`),
  );
}

export async function readDurableHandHistoryV2(
  storageNamespace: string,
): Promise<PersistedHandHistoryV2 | null> {
  const cached = cacheV1Entries(storageNamespace);
  if (!hasUsableTauriWindowRuntime()) {
    if (cached.length === 0) {
      return null;
    }
    return {
      schemaVersion: 2,
      updatedAtMs: readPersistedHandHistory(storageNamespace)?.updatedAtMs ?? 0,
      entries: cached,
    };
  }
  const persisted = await invoke<PersistedHandHistoryV2 | null>(
    "read_persisted_hand_history_v2",
  );
  const missing = missingCacheImports(persisted, cached);
  if (missing.length === 0) {
    return persisted;
  }
  return invoke<PersistedHandHistoryV2>("merge_persisted_hand_history_v2", {
    entries: missing,
  });
}

export async function mergeDurableHandHistoryV2(
  storageNamespace: string,
  snapshot: TableViewSnapshot,
  onWriteFailure?: PersistenceWriteFailureHandler,
): Promise<PersistedHandHistoryV2> {
  const live = liveV2Entries(snapshot);
  if (!hasUsableTauriWindowRuntime()) {
    const cachedHistory = mergePersistedHandHistory(
      storageNamespace,
      snapshot.handHistory,
      onWriteFailure,
    );
    return {
      schemaVersion: 2,
      updatedAtMs: cachedHistory.updatedAtMs,
      entries: live,
    };
  }
  const existing = await invoke<PersistedHandHistoryV2 | null>(
    "read_persisted_hand_history_v2",
  );
  const cached = missingCacheImports(
    existing,
    cacheV1Entries(storageNamespace),
  );
  if (cached.length === 0 && live.length === 0 && existing) {
    return existing;
  }
  // The V2 Rust store validates every tuple and merges under the profile lock.
  // Cached legacy entries remain clearly unattributed, never current-session
  // entries and never preferred over an already durable V2 identity.
  return invoke<PersistedHandHistoryV2>("merge_persisted_hand_history_v2", {
    entries: [...cached, ...live],
  });
}

function hasUsableTauriWindowRuntime(): boolean {
  if (typeof window === "undefined") {
    return false;
  }
  const w = window as Window & {
    __TAURI_INTERNALS__?: unknown;
    __TAURI__?: unknown;
  };
  return Boolean(w.__TAURI_INTERNALS__ || w.__TAURI__);
}

export function initializeWindowStatePersistence(
  storageNamespace: string,
  onWriteFailure?: PersistenceWriteFailureHandler,
) {
  if (!hasUsableTauriWindowRuntime()) {
    return () => {};
  }

  let cancelled = false;
  const cleanupCallbacks: Array<() => void> = [];
  const storedState = normalizePersistedWindowState(
    readStoredValue<unknown>(
      localStorage.getItem(
        storageKey(storageNamespace, WINDOW_STATE_STORAGE_SUFFIX),
      ),
      null,
    ),
  );

  const persistWindowState = (nextState: PersistedWindowState) => {
    const result = writeStoredValueWithStatus(
      storageKey(storageNamespace, WINDOW_STATE_STORAGE_SUFFIX),
      nextState,
    );
    if (!result.ok) {
      reportPersistenceWriteFailure(
        "window state",
        result.error,
        onWriteFailure,
      );
    }
  };

  void import("@tauri-apps/api/window")
    .then(async ({ LogicalPosition, LogicalSize, getCurrentWindow }) => {
      if (cancelled) {
        return;
      }

      const appWindow = getCurrentWindow();

      if (storedState) {
        if (storedState.maximized) {
          await appWindow.maximize();
        } else {
          await appWindow.setSize(
            new LogicalSize(storedState.width, storedState.height),
          );
          await appWindow.setPosition(
            new LogicalPosition(storedState.x, storedState.y),
          );
        }
      }

      const captureWindowState = async () => {
        const [size, position, maximized] = await Promise.all([
          appWindow.innerSize(),
          appWindow.outerPosition(),
          appWindow.isMaximized(),
        ]);

        persistWindowState({
          width: size.width,
          height: size.height,
          x: position.x,
          y: position.y,
          maximized,
        });
      };

      await captureWindowState();
      cleanupCallbacks.push(
        await appWindow.onMoved(() => void captureWindowState()),
      );
      cleanupCallbacks.push(
        await appWindow.onResized(() => void captureWindowState()),
      );
    })
    .catch((error: unknown) => {
      console.error("Failed to initialize window state persistence.", error);
    });

  return () => {
    cancelled = true;
    cleanupCallbacks.forEach((cleanupCallback) => cleanupCallback());
  };
}
