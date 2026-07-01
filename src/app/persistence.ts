import type { TableCardView, TableHistoryEntryView } from "../api/desktop";
import {
  readStoredValue,
  readStoredValueWithStatus,
  storageKey,
} from "./shell";

export type PersistedHandHistory = {
  updatedAtMs: number;
  entries: TableHistoryEntryView[];
};

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

export function persistHandHistory(
  storageNamespace: string,
  entries: TableHistoryEntryView[],
) {
  const normalizedEntries = normalizeHandHistoryEntries(entries);

  localStorage.setItem(
    storageKey(storageNamespace, HAND_HISTORY_STORAGE_SUFFIX),
    JSON.stringify({
      updatedAtMs: Date.now(),
      entries: normalizedEntries,
    } satisfies PersistedHandHistory),
  );
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

export function initializeWindowStatePersistence(storageNamespace: string) {
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
    localStorage.setItem(
      storageKey(storageNamespace, WINDOW_STATE_STORAGE_SUFFIX),
      JSON.stringify(nextState),
    );
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
