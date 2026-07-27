#!/usr/bin/env python3
"""Publish the latest full-game release runtime result into tracked docs."""

from __future__ import annotations

import json
import os
import subprocess
from datetime import datetime, timezone
from pathlib import Path
from typing import Any

SOURCE = Path("runtime-gameplay-evidence/release-full-game-result.json")
OUTPUT_DIR = Path("docs/runtime-validation")
NON_RESULTS = {"cancelled", "skipped"}


def load_result() -> dict[str, Any]:
    if not SOURCE.is_file():
        return {
            "result": "FAIL",
            "error": "Full-game result file was not produced.",
            "steps": [],
        }
    try:
        value = json.loads(SOURCE.read_text(encoding="utf-8"))
    except (json.JSONDecodeError, OSError) as error:
        return {
            "result": "FAIL",
            "error": f"Could not parse full-game result: {error}",
            "steps": [],
        }
    if not isinstance(value, dict):
        return {
            "result": "FAIL",
            "error": "Full-game result was not a JSON object.",
            "steps": [],
        }
    return value


def publish_evidence() -> None:
    subprocess.run(
        [
            "bash",
            "scripts/push_runtime_evidence.sh",
            "docs: record Linux full-game runtime result",
            "docs/runtime-validation/gameplay-latest.json",
            "docs/runtime-validation/gameplay-latest.md",
        ],
        check=True,
    )


def should_skip_publication(build_outcome: str, gameplay_outcome: str) -> bool:
    if build_outcome in NON_RESULTS:
        return True
    return build_outcome == "success" and gameplay_outcome in NON_RESULTS


def main() -> None:
    build_outcome = os.environ.get("BUILD_OUTCOME") or "not-run"
    gameplay_outcome = os.environ.get("GAMEPLAY_OUTCOME") or "not-run"
    if should_skip_publication(build_outcome, gameplay_outcome):
        print(
            "Skipping durable full-game evidence publication because the workflow "
            "was cancelled or the gameplay step never started."
        )
        return

    result = load_result()
    payload: dict[str, Any] = {
        "result": result.get("result", "FAIL"),
        "validatedCommit": os.environ["GITHUB_SHA"],
        "workflowRunId": os.environ["GITHUB_RUN_ID"],
        "workflowRunAttempt": os.environ["GITHUB_RUN_ATTEMPT"],
        "buildOutcome": build_outcome,
        "gameplayOutcome": gameplay_outcome,
        "recordedAtUtc": datetime.now(timezone.utc).isoformat(),
        "evidenceArtifact": "linux-release-full-game-evidence",
        "fullGame": result,
    }
    OUTPUT_DIR.mkdir(parents=True, exist_ok=True)
    (OUTPUT_DIR / "gameplay-latest.json").write_text(
        json.dumps(payload, indent=2, sort_keys=True) + "\n",
        encoding="utf-8",
    )

    lines = [
        "# Latest Linux Release Full-Game Validation",
        "",
        f"- Result: **{payload['result']}**",
        f"- Validated commit: `{payload['validatedCommit']}`",
        f"- GitHub Actions run: `{payload['workflowRunId']}`",
        f"- Build outcome: `{payload['buildOutcome']}`",
        f"- Gameplay outcome: `{payload['gameplayOutcome']}`",
        f"- Recorded at: `{payload['recordedAtUtc']}`",
        f"- Evidence artifact: `{payload['evidenceArtifact']}`",
        "",
    ]
    if result.get("error"):
        lines.extend(["## Failure", "", str(result["error"]), ""])
    if result.get("completedHands") is not None:
        lines.extend(
            [
                "## Result summary",
                "",
                f"- Completed hands: `{result['completedHands']}`",
                f"- Host instance: `{result.get('hostInstanceId')}`",
                f"- Client instance: `{result.get('clientInstanceId')}`",
                "",
            ]
        )
    steps = result.get("steps")
    if isinstance(steps, list):
        lines.extend(["## Executed checks", ""])
        for step in steps:
            if isinstance(step, dict):
                lines.append(
                    f"- **{step.get('result', 'UNKNOWN')}** — "
                    f"{step.get('name', 'unnamed check')}"
                )
        lines.append("")
    (OUTPUT_DIR / "gameplay-latest.md").write_text(
        "\n".join(lines), encoding="utf-8"
    )
    publish_evidence()


if __name__ == "__main__":
    main()
