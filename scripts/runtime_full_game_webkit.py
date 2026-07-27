#!/usr/bin/env python3
"""Run the release full-game harness with WebKit-safe UI compatibility."""

from __future__ import annotations

from typing import Any

import runtime_full_game_smoke
from runtime_webdriver_smoke import WebDriverClient


def click_first_enabled_text(client: WebDriverClient, text: str) -> None:
    clicked = client.execute(
        """
const text = arguments[0];
const normalize = (value) => String(value || '').replace(/\s+/g, ' ').trim();
const target = Array.from(document.querySelectorAll('button:not([disabled]), a')).find(
  (candidate) => normalize(candidate.textContent) === text
);
if (!target) return false;
target.scrollIntoView({ block: 'center', inline: 'center' });
target.click();
return true;
""",
        [text],
    )
    if clicked is not True:
        raise AssertionError(f"No enabled button or link found with text {text!r}")


def canonical_table_view(client: WebDriverClient) -> dict[str, Any]:
    view = ORIGINAL_TABLE_VIEW(client)
    tray = view.get("actionTray")
    if not isinstance(tray, dict):
        return view

    legal = tray.get("legalActions")
    if not isinstance(legal, list):
        return view

    normalized = {
        "".join(character for character in str(action).lower() if character.isalnum())
        for action in legal
    }
    canonical = list(legal)
    if "allin" in normalized and "allIn" not in canonical:
        canonical.append("allIn")
    if normalized.intersection({"call", "check"}) and "checkOrCall" not in canonical:
        canonical.append("checkOrCall")
    tray["legalActions"] = canonical
    return view


ORIGINAL_TABLE_VIEW = runtime_full_game_smoke.table_view


def main() -> int:
    runtime_full_game_smoke.click_first_enabled_text = click_first_enabled_text
    runtime_full_game_smoke.table_view = canonical_table_view
    return runtime_full_game_smoke.main()


if __name__ == "__main__":
    raise SystemExit(main())
