import type { TableHistoryEntryView } from "../api/desktop";
import { readStoredValue, storageKey } from "./shell";

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

export function readPersistedHandHistory(storageNamespace: string) {
  return readStoredValue<PersistedHandHistory | null>(
    localStorage.getItem(storageKey(storageNamespace, HAND_HISTORY_STORAGE_SUFFIX)),
    null,
  );
}

export function persistHandHistory(
  storageNamespace: string,
  entries: TableHistoryEntryView[],
) {
  localStorage.setItem(
    storageKey(storageNamespace, HAND_HISTORY_STORAGE_SUFFIX),
    JSON.stringify({
      updatedAtMs: Date.now(),
      entries,
    } satisfies PersistedHandHistory),
  );
}

export function initializeWindowStatePersistence(storageNamespace: string) {
  if (typeof window === "undefined" || !("__TAURI_INTERNALS__" in window)) {
    return () => {};
  }

  let cancelled = false;
  const cleanupCallbacks: Array<() => void> = [];
  const storedState = readStoredValue<PersistedWindowState | null>(
    localStorage.getItem(storageKey(storageNamespace, WINDOW_STATE_STORAGE_SUFFIX)),
    null,
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
      cleanupCallbacks.push(await appWindow.onMoved(() => void captureWindowState()));
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
