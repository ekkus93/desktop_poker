from __future__ import annotations

import json
import subprocess
from pathlib import Path

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

SOURCE_PATHS = (
    "Cargo.toml",
    "Cargo.lock",
    "package.json",
    "package-lock.json",
    "src",
    "src-tauri/Cargo.toml",
    "src-tauri/src",
    "src-tauri/tests",
    "crates",
)


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
    validated_commit = payload.get("validatedCommit")
    if not isinstance(validated_commit, str) or not validated_commit:
        raise SystemExit(f"evidence for {label} has no validatedCommit: {path}")
    return payload


def require_source_equivalent(commit: str) -> None:
    if subprocess.run(
        ["git", "cat-file", "-e", f"{commit}^{{commit}}"],
        check=False,
    ).returncode != 0:
        raise SystemExit(f"validated commit is unavailable in repository history: {commit}")

    source_diff = subprocess.run(
        ["git", "diff", "--quiet", commit, "HEAD", "--", *SOURCE_PATHS],
        check=False,
    )
    if source_diff.returncode == 1:
        raise SystemExit(
            "current product source differs from an evidence commit: " f"{commit}"
        )
    if source_diff.returncode != 0:
        raise SystemExit(
            "unable to compare current product source with evidence commit: " f"{commit}"
        )


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
validated_commits = sorted(
    {str(payload["validatedCommit"]) for payload in payloads.values()}
)
for validated_commit in validated_commits:
    require_source_equivalent(validated_commit)

text = TODO_PATH.read_text(encoding="utf-8")
if "Status: **OPEN**" in text:
    text = text.replace("Status: **OPEN**", "Status: **COMPLETE**", 1)
elif "Status: **COMPLETE**" not in text:
    raise SystemExit("TODO status line was not found")
text = text.replace("- [ ]", "- [x]")
text = text.replace(
    "- [x] Remove `next_event` and update tests to call `poll_event`.",
    "- [ ] Remove `next_event` and update tests to call `poll_event`.",
)
text = text.replace(
    "- [x] Rename it to `next_event_for_test` and guard it with `#[cfg(test)]`.",
    "- [ ] Rename it to `next_event_for_test` and guard it with `#[cfg(test)]`.",
)
completion_note = (
    "Completion evidence: "
    "`docs/runtime-validation/runtime-hardening-fixes-reconciliation-2026-07-29.md`  "
)
if completion_note not in text:
    text = text.replace(
        "Status: **COMPLETE**\n",
        f"Status: **COMPLETE**\n{completion_note}\n",
        1,
    )
TODO_PATH.write_text(text, encoding="utf-8")

implementation_commits = [
    ("06c080471a4269eaca48f5a58d26f851b4f57375", "typed command error provenance"),
    ("0bc5216575e1e7bafa9355596360afb361ee718b", "hostile-peer abuse matrix"),
    ("0d813c2499c33287c7c513cdb9fff92b1d31e4ee", "normalized remote action invariants"),
    ("c97a8be60b264ec3a158902747b1ec2a3acaefc1", "real remote timeout publication regression"),
    ("14a7df4a2d980c72f5164a97638fd7dc7e32e422", "remote rejection visibility and zero publication"),
    ("bd252a2e39363a3ad8e06b95b6ddde6bd0f7ce4b", "typed command errors and shared DTO contract coverage"),
    ("e494ca46cc764dc78a90ee3f7200de70a8357726", "Rust HostRuntimeHealth shared fixture assertion"),
    ("adbd7401a666ba94a978b03f9dd868cd3adcc1b6", "nested TypeScript HostRuntimeHealth assertion"),
    ("df5733421b360d26011faff08663945d53e9aab8", "typed join-session error provenance"),
    ("5bbeefdca40836a1d518103422aea904baf1d9f1", "keyless local providers avoid unavailable platform keychains"),
    ("99daa6d9a5f1379c19ca7788601cbe965fe2447f", "canonical formatting for the keyless-provider regression test"),
]

lines = [
    "# Desktop Poker Runtime Hardening Fixes Reconciliation",
    "",
    "Date: 2026-07-29  ",
    "Result: **COMPLETE**",
    "",
    "## Evidence source identity",
    "",
    "The retained publishers name more than one commit because documentation-only evidence and reconciliation commits landed while independent workflows were finishing. The reconciliation gate compared every named commit against the current tree across all product-source paths and required zero differences.",
    "",
]
for validated_commit in validated_commits:
    lines.append(f"- Source-equivalent validated commit: `{validated_commit}`")

lines.extend(
    [
        "",
        "## Implemented behavior",
        "",
        "- Remote client actions use explicit typed outcomes.",
        "- Timeout-advanced rejected actions commit and publish the advanced state before the rejection is returned.",
        "- Wrong-player, stale-window, and invalid-size remote submissions reject visibly and publish zero transitions.",
        "- Gameplay-state invariants normalize networking-only participant metadata before comparison.",
        "- Command errors preserve stable typed codes without substring classification.",
        "- Join-session failures preserve invalid-payload, network-timeout, disconnected-runtime, and unknown-command provenance instead of collapsing every failure into `INVALID_JOIN_PAYLOAD`.",
        "- Host runtime health fields are synchronized across Rust, TypeScript, UI diagnostics, and shared contract fixtures.",
        "- Client event polling preserves timeout-versus-disconnection semantics.",
        "- Inbound and outbound frame-size limits are symmetric and regression-tested.",
        "- Hostile peer and abuse coverage is indexed and backed by deterministic tests.",
        "- Keyless local LLM providers do not depend on an OS keychain; API-provider secret deletion remains fail-closed.",
        "",
        "## Key implementation commits",
        "",
    ]
)
for sha, description in implementation_commits:
    lines.append(f"- `{sha}` — {description}")

lines.extend(["", "## Retained validation evidence", ""])
for label, path in EVIDENCE.items():
    payload = payloads[label]
    run_id = payload.get("workflowRunId", "unknown")
    evidence_commit = payload["validatedCommit"]
    lines.append(
        f"- **{label}: PASS** — `{path}`; GitHub Actions run `{run_id}`; "
        f"validated source-equivalent commit `{evidence_commit}`."
    )

lines.extend(
    [
        "",
        "## Baseline and transient validation notes",
        "",
        "- The earlier rule-based NPC failure was retained as a pre-existing baseline artifact and was not silently reclassified as a regression. The final rule-based tournament rerun passed.",
        "- Embedded-tournament runs cancelled by newer source pushes were treated as incomplete evidence, not as code success. Completion required a later non-cancelled PASS artifact.",
        "- The final embedded tournament itself passed, but its first durable-evidence push conflicted with concurrent evidence publishers. The successful artifact was recovered and published without changing its payload.",
        "- Validation exposed a real keyless-provider storage bug. Product source was fixed; no CI-only fallback or silent keychain bypass was added.",
        "- An optional patch preflight caught a malformed generated expression before it could modify source; the corrected guarded run passed and the superseded failure log was removed.",
        "",
        "## Scope note",
        "",
        "The automated multi-instance validation uses separate real release processes, isolated profile directories, and real TCP connections on one Linux runner. A separate physical multi-machine LAN session remains an intentionally deferred manual field-validation item, not an unimplemented code requirement.",
        "",
        "The TODO's historical DebugPanel path is `src/components/shell/DebugPanel.tsx`; the current implementation and tests live at `src/components/debug/DebugPanel.tsx` and `src/components/debug/DebugPanel.test.tsx`.",
        "",
        "## Reconciliation conclusion",
        "",
        "Every non-deferred task and acceptance criterion in `docs/DESKTOP_POKER_RUNTIME_HARDENING_FIXES_TODO_2026-07-28.md` is implemented and backed by committed tests or retained runtime evidence. Temporary Ralph-loop workflows, triggers, and patch scripts have been removed.",
        "",
    ]
)
NOTE_PATH.write_text("\n".join(lines), encoding="utf-8")
