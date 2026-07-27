#!/usr/bin/env python3
"""Persist authoritative final history from the tournament completion screen."""

from __future__ import annotations

from pathlib import Path


def replace_once(path: str, old: str, new: str) -> None:
    file_path = Path(path)
    content = file_path.read_text(encoding="utf-8")
    occurrences = content.count(old)
    if occurrences != 1:
        raise RuntimeError(f"expected one match in {path}, found {occurrences}")
    file_path.write_text(content.replace(old, new, 1), encoding="utf-8")


def insert_before_last(path: str, marker: str, addition: str) -> None:
    file_path = Path(path)
    content = file_path.read_text(encoding="utf-8")
    if addition.strip() in content:
        raise RuntimeError(f"repair already present in {path}")
    index = content.rfind(marker)
    if index < 0:
        raise RuntimeError(f"final marker not found in {path}")
    file_path.write_text(
        content[:index] + addition.rstrip() + "\n" + content[index:],
        encoding="utf-8",
    )


def main() -> None:
    replace_once(
        "src/screens/TournamentCompleteScreen.tsx",
        "  const { hostDraft, persistedHandHistoryCount, wasHost } = useDesktopShell();\n",
        "  const {\n"
        "    hostDraft,\n"
        "    persistedHandHistoryCount,\n"
        "    persistHandHistory,\n"
        "    wasHost,\n"
        "  } = useDesktopShell();\n",
    )
    replace_once(
        "src/screens/TournamentCompleteScreen.tsx",
        "        if (!cancelled) {\n"
        "          setTableView(snapshot);\n"
        "          setError(null);\n"
        "        }\n",
        "        if (!cancelled) {\n"
        "          if (snapshot.handHistory.length > 0) {\n"
        "            persistHandHistory(snapshot.handHistory);\n"
        "          }\n"
        "          setTableView(snapshot);\n"
        "          setError(null);\n"
        "        }\n",
    )
    replace_once(
        "src/screens/TournamentCompleteScreen.tsx",
        "  }, []);\n",
        "  }, [persistHandHistory]);\n",
    )

    replace_once(
        "src/screens/TournamentCompleteScreen.test.tsx",
        'import { screen } from "@testing-library/react";\n',
        'import { screen, waitFor } from "@testing-library/react";\n',
    )
    replace_once(
        "src/screens/TournamentCompleteScreen.test.tsx",
        'import { getTableView } from "../api/desktop";\n',
        'import { getTableView } from "../api/desktop";\n'
        'import { readPersistedHandHistory } from "../app/persistence";\n',
    )
    insert_before_last(
        "src/screens/TournamentCompleteScreen.test.tsx",
        "});\n",
        "\n  it(\"persists the authoritative final history before rendering completion\", async () => {\n"
        "    const bootstrap = createBootstrap({\n"
        "      storageNamespace: \"desktop-poker:completion-persistence-test\",\n"
        "    });\n"
        "    const handHistory = [\n"
        "      {\n"
        "        handNumber: 3,\n"
        "        summary: \"Maya won 3000 chip(s).\",\n"
        "        potTotal: 3000,\n"
        "        winningPlayers: [\"Maya\"],\n"
        "        eliminatedPlayers: [\"You\"],\n"
        "        boardCards: [],\n"
        "      },\n"
        "    ];\n"
        "    mockedGetTableView.mockResolvedValue(\n"
        "      createTableViewSnapshot({\n"
        "        tournamentPhase: \"complete\",\n"
        "        phaseLabel: \"Complete\",\n"
        "        handHistory,\n"
        "        standings: [\n"
        "          {\n"
        "            rank: 1,\n"
        "            displayName: \"Maya\",\n"
        "            chipCount: 3000,\n"
        "            statusLabel: \"Winner\",\n"
        "            note: \"Won the tournament\",\n"
        "            isLocal: false,\n"
        "            isObserver: false,\n"
        "          },\n"
        "        ],\n"
        "      }),\n"
        "    );\n\n"
        "    renderWithProviders(<TournamentCompleteScreen />, { bootstrap });\n\n"
        "    expect(await screen.findByText(handHistory[0].summary)).toBeTruthy();\n"
        "    await waitFor(() => {\n"
        "      expect(\n"
        "        readPersistedHandHistory(bootstrap.storageNamespace)?.entries,\n"
        "      ).toEqual(handHistory);\n"
        "    });\n"
        "  });\n\n",
    )


if __name__ == "__main__":
    main()
