#!/usr/bin/env python3
"""Publish the latest GitHub Actions runtime evidence into tracked docs."""

from __future__ import annotations

import json
import os
from datetime import datetime, timezone
from pathlib import Path
from typing import Any

EVIDENCE_DIR = Path("runtime-validation-evidence")
OUTPUT_DIR = Path("docs/runtime-validation")


def load_result(filename: str, label: str) -> dict[str, Any]:
    source = EVIDENCE_DIR / filename
    if not source.is_file():
        return {
            "result": "FAIL",
            "error": f"{label} result file was not produced.",
            "steps": [],
        }
    try:
        value = json.loads(source.read_text(encoding="utf-8"))
    except (json.JSONDecodeError, OSError) as error:
        return {
            "result": "FAIL",
            "error": f"Could not parse {label} result: {error}",
            "steps": [],
        }
    if not isinstance(value, dict):
        return {
            "result": "FAIL",
            "error": f"{label} result was not a JSON object.",
            "steps": [],
        }
    return value


def render_section(heading: str, result: dict[str, Any]) -> list[str]:
    lines = [f"## {heading}", "", f"Result: **{result.get('result', 'UNKNOWN')}**", ""]
    if result.get("error"):
        lines.extend([f"Failure: {result['error']}", ""])
    steps = result.get("steps")
    if isinstance(steps, list):
        for step in steps:
            if isinstance(step, dict):
                lines.append(
                    f"- **{step.get('result', 'UNKNOWN')}** — "
                    f"{step.get('name', 'unnamed check')}"
                )
        lines.append("")
    return lines


def main() -> None:
    single = load_result("release-runtime-result.json", "single-instance runtime")
    multi = load_result(
        "release-multi-instance-result.json", "multi-instance runtime"
    )
    overall = (
        "PASS"
        if single.get("result") == "PASS" and multi.get("result") == "PASS"
        else "FAIL"
    )
    payload: dict[str, Any] = {
        "result": overall,
        "validatedCommit": os.environ["GITHUB_SHA"],
        "workflowRunId": os.environ["GITHUB_RUN_ID"],
        "workflowRunAttempt": os.environ["GITHUB_RUN_ATTEMPT"],
        "buildOutcome": os.environ.get("BUILD_OUTCOME") or "not-run",
        "singleInstanceOutcome": os.environ.get("SMOKE_OUTCOME") or "not-run",
        "multiInstanceOutcome": os.environ.get("MULTI_OUTCOME") or "not-run",
        "recordedAtUtc": datetime.now(timezone.utc).isoformat(),
        "evidenceArtifact": "linux-release-runtime-evidence",
        "singleInstance": single,
        "multiInstance": multi,
    }

    OUTPUT_DIR.mkdir(parents=True, exist_ok=True)
    (OUTPUT_DIR / "latest.json").write_text(
        json.dumps(payload, indent=2, sort_keys=True) + "\n", encoding="utf-8"
    )

    lines = [
        "# Latest Linux Release Runtime Validation",
        "",
        f"- Overall result: **{overall}**",
        f"- Validated commit: `{payload['validatedCommit']}`",
        f"- GitHub Actions run: `{payload['workflowRunId']}`",
        f"- Build outcome: `{payload['buildOutcome']}`",
        f"- Single-instance outcome: `{payload['singleInstanceOutcome']}`",
        f"- Multi-instance outcome: `{payload['multiInstanceOutcome']}`",
        f"- Recorded at: `{payload['recordedAtUtc']}`",
        f"- Evidence artifact: `{payload['evidenceArtifact']}`",
        "",
    ]
    lines.extend(render_section("Single-instance release smoke", single))
    lines.extend(render_section("Live multi-instance release smoke", multi))
    (OUTPUT_DIR / "latest.md").write_text("\n".join(lines), encoding="utf-8")


if __name__ == "__main__":
    main()
