import type { TableHistoryEntryView } from "../api/desktop";
import { storageKey } from "../app/shell";

export function createHistoryEntry(
  overrides: Partial<TableHistoryEntryView> = {},
): TableHistoryEntryView {
  return {
    handNumber: 1,
    summary: "Alice won 120 chip(s).",
    potTotal: 120,
    winningPlayers: ["Alice"],
    eliminatedPlayers: [],
    boardCards: [],
    ...overrides,
  };
}

export function seedPersistedHandHistory(
  storageNamespace: string,
  entries: TableHistoryEntryView[],
  updatedAtMs = 1234,
) {
  localStorage.setItem(
    storageKey(storageNamespace, "hand-history-summaries"),
    JSON.stringify({ updatedAtMs, entries }),
  );
}
