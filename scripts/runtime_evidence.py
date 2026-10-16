"""Helpers for producing runtime artifacts without private table state."""

from __future__ import annotations

from typing import Any


_PRIVATE_EVIDENCE_KEYS = {
    "actiontray",
    "holecards",
    "localholecards",
    "privatedata",
    "privateholecards",
    "privatepayload",
    "privateprojection",
    "privatestate",
}


def public_evidence(value: Any) -> Any:
    """Return a JSON-safe copy with private card and action state omitted."""
    if isinstance(value, dict):
        return {
            key: public_evidence(child)
            for key, child in value.items()
            if str(key).replace("_", "").lower() not in _PRIVATE_EVIDENCE_KEYS
        }
    if isinstance(value, list):
        return [public_evidence(child) for child in value]
    if isinstance(value, tuple):
        return [public_evidence(child) for child in value]
    return value
