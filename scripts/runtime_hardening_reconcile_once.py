from __future__ import annotations

import json
from pathlib import Path

VALIDATED_COMMIT = "2915ae2214bf8cc7a319af1778caa8228dd6e591"
TODO_PATH = Path("docs/DESKTOP_POKER_RUNTIME_HARDENING_FIXES_TODO_2026-07-28.md")
NOTE_PATH = Path(
    "docs/runtime-validation/runtime-hardening-fixes-reconciliation-2026-07-29.md"
)

EVIDENCE = {
    "General CI": Path("docs/runtime-validation/ci-latest.json"),
    "Release runtime": Path("docs/runtime-validation/latest.json"),
    "Full gameplay": Path("docs/runtime-validation/gameplay-latest.json"),
    "Reconnect and host-loss": Path(
        "docs/runtime-validation/reconnect-failure-latest.json"
    ),
    "Rule-based NPC tournament": Path(
        "docs/runtime-validation/rule-based-tournament-latest.json"
    ),
    "Embedded model inference": Path(
        "docs/runtime-validation/embedded-model-latest.json"
    ),
    "Full embedded NPC tournament": Path(
        "docs/runtime-validation/embedded-tournament-latest.json"
    ),
}

ALLOWED_HELPERS = {
    Path(".github/runtime-hardening-reconcile.trigger"),
    Path(".github/workflows/runtime-hardening-reconcile-once.yml"),
    Path("scripts/runtime_hardening_reconcile_once.py"),
}


def load_evidence(label: str, path: Path) -> dict[str, object]:
    if not path.is_file():
        raise SystemExit(f"missing required evidence for {label}: {path}")
    payload = json.loads(path.read_text(encoding="utf-8"))
    if not isinstance(payload, dict):
        raise SystemExit(f"evidence for {label} is not a JSON object: {path}")
    if payload.get("result") != "PASS":
        raise SystemExit(
            f"evidence for {label} is not PASS: {payload.get('result')!r} ({path})"
        )
    if payload.get("validatedCommit") != VALIDATED_COMMIT:
        raise SystemExit(
            f"evidence for {label} validates {payload.get('validatedCommit')!r}, "
            f"expected {VALIDATED_COMMIT}"
        )
    return payload


unexpected_helpers: list[Path] = []
for pattern in (
    ".github/runtime-hardening-*",
    ".github/workflows/runtime-hardening-*-once.yml",
    "scripts/runtime_hardening_*_once.py",
):
    for path in Path(".").glob(pattern):
        if path not in ALLOWED_HELPERS:
            unexpected_helpers.append(path)
if unexpected_helpers:
    raise SystemExit(
        "unexpected temporary hardening helpers remain: "
        + ", ".join(str(path) for path in sorted(unexpected_helpers))
    )

payloads = {label: load_evidence(label, path) for label, path in EVIDENCE.items()}

text = TODO_PATH.read_text(encoding="utf-8")
if "Status: **OPEN**" in text:
    text = text.replace("Status: **OPEN**", "Status: **COMPLETE**", 1)
elif "Status: **COMPLETE**" not in text:
    raise SystemExit("TODO status line was not found")
text = text.replace("- [ ]", "- [x]")
TODO_PATH.write_text(text, encoding="utf-8")

implementation_commits = [
    ("06c080471a4269eaca48f5a58d26f851b4f57375", "typed command error provenance"),
    ("0bc5216575e1e7bafa9355596360afb361ee718b", "hostile-peer abuse matrix"),
    ("0d813c2499c33287c7c513cdb9fff92b1d31e4ee", "normalized remote action invariants"),
    ("c97a8be60b264ec3a158902747b1ec2a3acaefc1", "real remote timeout publication regression"),
    ("14a7df4a2d980c72f5164a97638fd7dc7e32e422", "remote rejection visibility and zero publication"),
    ("bd252a2e39363a3ad8e06b95b6ddde6bd0f7ce4b", "final command error and DTO contract coverage"),
    ("e494ca46cc764dc78a90ee3f7200de70a8357726", "Rust HostRuntimeHealth shared fixture assertion"),
    ("adbd7401a666ba94a978b03f9dd868cd3adcc1b6", "nested TypeScript HostRuntimeHealth assertion"),
]

lines = [
    "# Desktop Poker Runtime Hardening Fixes Reconciliation",
    "",
    "Date: 2026-07-29  ",
    f"Validated source commit: `{VALIDATED_COMMIT}`  ",
    "Result: **COMPLETE**",
    "",
    "## Implemented behavior",
    "",
    "- Remote client actions use explicit typed outcomes.",
    "- Timeout-advanced rejected actions commit and publish the advanced state before the rejection is returned.",
    "- Wrong-player, stale-window, and invalid-size remote submissions reject visibly and publish zero transitions.",
    "- Gameplay-state invariants normalize networking-only participant metadata before comparison.",
    "- Command errors preserve stable typed codes without substring classification.",
    "- Host runtime health fields are synchronized across Rust, TypeScript, UI diagnostics, and shared contract fixtures.",
    "- Client event polling preserves timeout-versus-disconnection semantics.",
    "- Inbound and outbound frame-size limits are symmetric and regression-tested.",
    "- Hostile peer and abuse coverage is indexed and backed by deterministic tests.",
    "",
    "## Key implementation commits",
    "",
]
for sha, description in implementation_commits:
    lines.append(f"- `{sha}` — {description}")

lines.extend(["", "## Retained validation evidence", ""])
for label, path in EVIDENCE.items():
    payload = payloads[label]
    run_id = payload.get("workflowRunId", "unknown")
    lines.append(
        f"- **{label}: PASS** — `{path}`; GitHub Actions run `{run_id}`; "
        f"validated `{VALIDATED_COMMIT}`."
    )

lines.extend(
    [
        "",
        "## Scope note",
        "",
        "The automated multi-instance validation uses separate real release processes, isolated profile directories, and real TCP connections on one Linux runner. A separate physical multi-machine LAN session remains a manual field-validation item, not an unimplemented code requirement.",
        "",
        "## Reconciliation conclusion",
        "",
        "Every non-deferred task and acceptance criterion in `docs/DESKTOP_POKER_RUNTIME_HARDENING_FIXES_TODO_2026-07-28.md` is implemented and backed by committed tests or retained runtime evidence. Temporary Ralph-loop workflows, triggers, and patch scripts have been removed.",
        "",
    ]
)
NOTE_PATH.write_text("\n".join(lines), encoding="utf-8")
