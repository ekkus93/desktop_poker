"""Unit tests for public-only runtime evidence projection."""

from __future__ import annotations

import unittest

from runtime_evidence import public_evidence


class PublicEvidenceTests(unittest.TestCase):
    def test_removes_private_card_and_action_state_recursively(self) -> None:
        projected = public_evidence(
            {
                "seats": [
                    {
                        "displayName": "Local player",
                        "holeCards": [{"rank": "ACE", "suit": "SPADES"}],
                    }
                ],
                "private_projection": {"privateHoleCards": ["SECRET_CARD"]},
                "actionTray": {"legalActions": ["fold"]},
                "boardCards": ["2H"],
            }
        )

        self.assertEqual(
            projected,
            {
                "seats": [{"displayName": "Local player"}],
                "boardCards": ["2H"],
            },
        )

    def test_preserves_public_values_and_returns_new_containers(self) -> None:
        source = {"nested": [{"holeCards": ["hidden"], "phase": "running"}]}
        projected = public_evidence(source)

        self.assertEqual(projected, {"nested": [{"phase": "running"}]})
        self.assertIsNot(projected, source)
        self.assertEqual(source["nested"][0]["holeCards"], ["hidden"])


if __name__ == "__main__":
    unittest.main()
