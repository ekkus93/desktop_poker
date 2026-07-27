#!/usr/bin/env python3
"""Run the release full-game harness with WebKit-safe UI compatibility."""

from __future__ import annotations

from typing import Any

import runtime_full_game_smoke
from runtime_multi_instance_smoke import wait_for_source
from runtime_webdriver_smoke import WebDriverClient


def click_exact_enabled_text(client: WebDriverClient, text: str) -> bool:
    return (
        client.execute(
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
        is True
    )


def click_first_enabled_text(client: WebDriverClient, text: str) -> None:
    if click_exact_enabled_text(client, text):
        return

    # When calling consumes the responder's entire remaining stack, the product
    # correctly exposes the action as All-in rather than a separate Call button.
    # Preserve the harness's semantic "call the all-in" flow by selecting and
    # confirming that equivalent UI action.
    if text.startswith("Call ") and click_exact_enabled_text(client, "All-in"):
        wait_for_source(client, "Confirm all-in")
        if click_exact_enabled_text(client, "Confirm"):
            return
        raise AssertionError("All-in confirmation button was not enabled")

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

    # An all-in that exactly covers a positive call amount is the responder's
    # only legal way to call. Expose the semantic alias expected by the generic
    # tournament driver; click_first_enabled_text maps it to the actual UI.
    call_amount = tray.get("callAmount")
    if (
        "allin" in normalized
        and isinstance(call_amount, (int, float))
        and call_amount > 0
        and "checkOrCall" not in canonical
    ):
        canonical.append("checkOrCall")

    tray["legalActions"] = canonical
    return view


def wait_for_route_with_completion_sync(
    client: WebDriverClient, route: str, text: str
) -> None:
    ORIGINAL_WAIT_FOR_ROUTE(client, route, text)
    if route == "/complete":
        wait_for_source(client, "Final history saved.", timeout=15.0)


ORIGINAL_TABLE_VIEW = runtime_full_game_smoke.table_view
ORIGINAL_WAIT_FOR_ROUTE = runtime_full_game_smoke.wait_for_route


def main() -> int:
    runtime_full_game_smoke.click_first_enabled_text = click_first_enabled_text
    runtime_full_game_smoke.table_view = canonical_table_view
    runtime_full_game_smoke.wait_for_route = wait_for_route_with_completion_sync
    return runtime_full_game_smoke.main()


if __name__ == "__main__":
    raise SystemExit(main())
