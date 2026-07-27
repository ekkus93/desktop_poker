#!/usr/bin/env python3
"""Run the full-game harness with reliable DOM-based text clicks for WebKit."""

from __future__ import annotations

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


def main() -> int:
    runtime_full_game_smoke.click_first_enabled_text = click_first_enabled_text
    return runtime_full_game_smoke.main()


if __name__ == "__main__":
    raise SystemExit(main())
