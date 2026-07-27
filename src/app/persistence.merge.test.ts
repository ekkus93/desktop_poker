import { beforeEach, describe, expect, it } from "vitest";
import { createHistoryEntry } from "../test/persistenceFixtures";
import {
  mergePersistedHandHistory,
  persistHandHistory,
  readPersistedHandHistory,
} from "./persistence";

describe("completion hand-history merging", () => {
  beforeEach(() => {
    localStorage.clear();
  });

  it("retains newer saved hands when a delayed snapshot contains only older hands", () => {
    persistHandHistory("desktop-poker:test", [
      createHistoryEntry({ handNumber: 1, summary: "Hand one" }),
      createHistoryEntry({ handNumber: 2, summary: "Hand two" }),
      createHistoryEntry({ handNumber: 3, summary: "Hand three" }),
      createHistoryEntry({ handNumber: 4, summary: "Final hand" }),
    ]);

    const merged = mergePersistedHandHistory("desktop-poker:test", [
      createHistoryEntry({ handNumber: 1, summary: "Hand one" }),
      createHistoryEntry({ handNumber: 2, summary: "Hand two" }),
      createHistoryEntry({ handNumber: 3, summary: "Updated hand three" }),
    ]);

    expect(merged.entries).toEqual([
      createHistoryEntry({ handNumber: 4, summary: "Final hand" }),
      createHistoryEntry({ handNumber: 3, summary: "Updated hand three" }),
      createHistoryEntry({ handNumber: 2, summary: "Hand two" }),
      createHistoryEntry({ handNumber: 1, summary: "Hand one" }),
    ]);
    expect(readPersistedHandHistory("desktop-poker:test")?.entries).toEqual(
      merged.entries,
    );
  });
});
