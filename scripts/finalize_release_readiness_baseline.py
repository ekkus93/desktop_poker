#!/usr/bin/env python3
"""Finalize the release-readiness evidence documents from verified CI results."""

from __future__ import annotations

import os
import re
from pathlib import Path

SOURCE_SHA = os.environ.get(
    "SOURCE_SHA", "fd8369ba7267fe76a827cdf48384c9f826159719"
)
MERGE_SHA = os.environ.get(
    "MERGE_SHA", "c79c9f2473f92310c3d65afe8b834f97b2875c5d"
)
RUN_ID = os.environ.get("RUN_ID", "30224757296")


def read(path: str) -> str:
    return Path(path).read_text(encoding="utf-8")


def write(path: str, text: str) -> None:
    Path(path).write_text(text, encoding="utf-8")


def replace_once(text: str, old: str, new: str, *, label: str) -> str:
    count = text.count(old)
    if count != 1:
        raise RuntimeError(f"{label}: expected one occurrence, found {count}")
    return text.replace(old, new, 1)


def check_item(text: str, item: str) -> str:
    unchecked = f"- [ ] {item}"
    checked = f"- [x] {item}"
    if checked in text:
        return text
    return replace_once(text, unchecked, checked, label=item)


def build_report() -> str:
    return f"""# Desktop Poker Release Readiness Report

## Executive result

**Outcome: Additional desktop runtime validation is required before a Linux release tag or Android/UniFFI milestone.**

The current automated baseline and Linux release-candidate build are green. A direct release binary and Debian package were built and inspected. The remaining release blockers require a graphical desktop, multiple live application instances, two physical LAN machines, a Linux Secret Service/keychain session, and configured live LLM providers. Those scenarios are recorded as `BLOCKED`; they are not inferred from unit, integration, browser, or packaging tests.

This report is the authoritative evidence record for `docs/DESKTOP_POKER_RELEASE_READINESS_BASELINE_TODO.md`.

## Result vocabulary

- `PASS` — the named command or scenario was executed successfully against the recorded revision.
- `FAIL` — the named command or scenario was executed and produced a product or validation failure.
- `BLOCKED` — execution requires a concrete unavailable dependency or environment.
- `NOT RUN` — execution was not attempted for a stated reason.

## Tested revision and environment

### Repository revision

- Repository: `ekkus93/desktop_poker`
- Base branch: `master`
- Baseline base SHA: `f4fde4d70fb8fe205bf74be7469912efd682045c`
- Execution branch: `agent/release-readiness-baseline`
- Tested branch source SHA: `{SOURCE_SHA}`
- GitHub pull-request merge SHA used by CI: `{MERGE_SHA}`
- GitHub Actions run: `{RUN_ID}`

The evidence-generating workflow checked out the pull-request merge revision. The branch source SHA and merge SHA are both recorded so no result is attributed to an unrecorded revision.

### GitHub Actions environment

- Runner: Ubuntu 24.04.4 LTS, x86-64, GitHub-hosted Azure VM
- Kernel: Linux 6.17.0-1020-azure
- Node.js: `v24.18.0`
- npm: `11.16.0`
- Rust: `rustc 1.97.1 (8bab26f4f 2026-07-14)`
- Cargo: `cargo 1.97.1 (c980f4866 2026-06-30)`
- Active toolchain: `stable-x86_64-unknown-linux-gnu`
- Graphical desktop/WebView session: unavailable
- Two physical LAN machines: unavailable
- Local Ollama/llama-server endpoint: unavailable
- Linux Secret Service/keychain session: unavailable

## Automated validation

| Validation | Result | Current evidence |
|---|---|---|
| `npm ci` | PASS | Committed lockfile installed in Verify, geometry, and release-candidate jobs. |
| `npm audit --omit=dev` | PASS | 0 info, low, moderate, high, critical; total 0. |
| `npm audit` | PASS | 0 info, low, moderate, high, critical; total 0. |
| `npm run format:check` | PASS | Prettier check completed successfully. |
| `npm run lint` | PASS | ESLint completed with zero warnings and zero errors. |
| `npm run test` | PASS | 30 files, 273 tests passed, 0 failed, 0 skipped/todo. |
| `npm run build` | PASS | TypeScript and Vite production build passed; 1,835 modules transformed. |
| `npm run test:geometry` | PASS | Playwright geometry job passed in the pinned Playwright 1.59.1 container. |
| `cargo fmt --check` | PASS | Workspace formatting passed. |
| `cargo clippy --workspace --all-targets --all-features -- -D warnings` | PASS | Clippy passed with warnings denied. |
| `cargo test --workspace --all-targets --all-features -- --test-threads=2` | PASS | Desktop adapter: 463 passed, 3 ignored; `poker-core`: 125 passed; total 588 passed, 0 failed, 3 ignored. |
| `cargo test -p poker-core --all-targets --all-features` | PASS | 125 passed, 0 failed, 0 ignored. |
| `cargo tree -p poker-core` | PASS | Only `rand_core`, `serde`, `serde_json`, `thiserror`, and their platform-neutral transitive dependencies. |

### Ignored Rust tests

The current workspace run contains exactly three ignored tests:

1. `npc::llm_strategy::ollama_live_tests::ollama_llama32_postflop_returns_legal_poker_action` — requires a live local Ollama model.
2. `npc::llm_strategy::ollama_live_tests::ollama_llama32_preflop_returns_legal_poker_action` — requires a live local Ollama model.
3. `protocol::models::tests::canonical_bytes_for_join_request_match_known_android_fixture` — requires the captured Android CanonicalJson fixture referenced by `INT_TEST3_TODO.md` §11.1.

None is claimed as passed.

### Frontend output quality

- Test output contained no unhandled promise rejection, React warning, window-persistence initialization failure, or recurring stderr error.
- Production output: `index.html` 0.46 kB, CSS 28.22 kB, main JS 337.47 kB, successful build in 2.29 seconds on the recorded runner.

## `poker-core` platform-neutrality audit

**Result: PASS for the current source revision.**

- The dependency tree contains no Tauri, Android, keyring, HTTP client, LAN transport, filesystem path-discovery, async runtime, or LLM dependencies.
- Repository source search found no `tauri`, `keyring`, `reqwest`, `local_ip`, `std::net`, TCP/UDP socket, process-spawning `Command`, or thread-spawn references under the shared core.
- `crates/poker-core/Cargo.toml` remains limited to deterministic serialization/error/randomness primitives.
- `EngineCommand` domain terminology is not process-spawning `std::process::Command` usage.

## Linux release artifact inventory

### Direct release binary

- Result: **PASS — build and static inspection**
- Path: `target/release/desktop-poker`
- Type: ELF 64-bit LSB PIE executable, x86-64, dynamically linked
- Exact size: 21,908,136 bytes (reported as 21 MB)
- SHA-256: `9f49324fcf431fcef5202d35dd7c000184992305568f2652b6a6c23896c23211`
- Build ID: `e1fd586bd6391d84b9c089a1506bdbc13aa7d11e`
- Graphical launch: **BLOCKED** — no desktop/WebView session is available in the execution environment.

### Debian package

- Result: **PASS — package build and metadata inspection**
- Filename: `Desktop Poker_0.1.0_amd64.deb`
- Path: `target/release/bundle/deb/Desktop Poker_0.1.0_amd64.deb`
- Exact size: 6,738,672 bytes (reported as 6.5 MB)
- SHA-256: `fc534010dfd8c0468511d9e2a24ad6a34e0375de7a55c030d113d43d06286ac7`
- Package: `desktop-poker`
- Version: `0.1.0`
- Architecture: `amd64`
- Installed size: 21,458 KiB
- Maintainer: `ekkus93`
- Dependencies: `libwebkit2gtk-4.1-0`, `libgtk-3-0`
- Installed binary path: `/usr/bin/desktop-poker`
- Desktop entry and 32/128/256@2 icon assets are present.
- Installation and graphical launch: **BLOCKED** — no disposable graphical Linux session is available.

### AppImage

- Result: **BLOCKED**
- This pass intentionally built the deterministic direct binary and `.deb` bundle. AppImage tooling and graphical launch were not available/proven, so AppImage is not claimed working.

The GitHub Actions release-candidate artifact retains the binary, `.deb`, complete release build log, and artifact inventory for 30 days.

## Production reachability and security audit

### Static findings

- `src/main.tsx` imports the layout probe only inside `import.meta.env.DEV`.
- Browser mocks in `src/api/desktop.ts` are guarded by development/test environment checks.
- `src-tauri/tauri.conf.json` uses a restrictive CSP and limits frontend connections to Tauri IPC.
- Release provider secrets select OS keychain storage; debug builds use the explicitly development-only local file.
- `llm-provider.json` contains non-secret settings only; storage/redaction tests pass.

### Runtime findings

| Scenario | Result | Reason |
|---|---|---|
| Release `/debug` route reachability | BLOCKED | Requires a running release WebView. |
| Browser-mock substitution in release | BLOCKED | Static guards pass; live release exercise is unavailable. |
| Release keychain write/read/clear | BLOCKED | Requires Linux Secret Service and a low-privilege test credential. |
| Plaintext-key search in app data/logs | BLOCKED | Requires application data produced by a real release run. |

## Defects discovered and fixes applied

### DP-RR-FIX-001 — Vulnerable production React Router dependency

- `npm audit --omit=dev` exposed a high-severity advisory through `react-router` 7.18.0 that the previous CI did not run.
- Fixed by migrating from `react-router-dom` 7 to `react-router` 8.3.0 and React/React DOM 19.2.8.
- All imports, tests, production build, and browser geometry pass after migration.

### DP-RR-FIX-002 — Vulnerable development dependency graph

- The full audit exposed the ESLint 9 `minimatch`/`brace-expansion` chain and a vulnerable PostCSS version.
- Fixed by upgrading the ESLint toolchain, pinning `@eslint/js` 10.0.1, and overriding PostCSS to 8.5.23.
- Both production and full audits now report zero vulnerabilities.

### DP-RR-FIX-003 — React correctness findings exposed by ESLint 10

- The stricter lint rules found four state-synchronization patterns and one render-time ref read.
- Fixed provider changes in the user event, asynchronous NPC profile loading, bootstrap-derived join state, render-safe launch-attempt presentation, and derived raise sizing.
- No lint rule was disabled or weakened. Formatting, lint, 273 frontend tests, build, audits, and geometry all pass.

### DP-RR-DOC-001 — Stale workspace release paths

- README release paths incorrectly named `src-tauri/target/release` after the repository became a root Cargo workspace.
- Corrected to `target/release` and documented the current audit/release-candidate workflow.

## Manual runtime evidence

### Two-local-instance full tournament

**Result: BLOCKED.** No graphical desktop session is available to launch two Tauri release instances. Host/join, seat claims, readiness, gameplay, private-card isolation, elimination, completion, persistence, and per-instance isolation are not marked passed.

### Two-machine LAN tournament

**Result: BLOCKED.** Two physical machines on the same LAN are unavailable. Loopback integration tests do not substitute for real LAN evidence.

### Reconnect and failure matrix

Lobby reconnect, active-hand reconnect, reconnect expiry, eliminated-observer reconnect, host loss, port conflict, invalid invite UI, and unreachable-host UI are **BLOCKED** pending controllable live processes and network interruption.

### Rule-based NPC tournament

**Result: BLOCKED.** Automated decision/runner coverage passes, but no live release tournament was run.

### Live LLM NPC scenarios

**Result: BLOCKED.** No Ollama/llama-server endpoint or low-privilege remote provider credential was available. The two ignored Ollama tests remain ignored and are not claimed passed.

### Release keychain scenario

**Result: BLOCKED.** Static implementation and automated storage/redaction tests pass, but OS-keychain behavior requires a real Secret Service session.

The executable manual procedure is `docs/MANUAL_QA_CHECKLIST.md`.

## Remaining release blockers

The authoritative list is `docs/DESKTOP_POKER_CURRENT_BACKLOG.md`. The fresh automated baseline is complete, and direct binary/`.deb` creation and inspection are proven. Desktop release readiness remains blocked by:

- graphical launch of the direct binary and installed package;
- two-local-instance full-tournament and storage-isolation QA;
- two-machine LAN full-tournament QA;
- live reconnect, expiry, observer, host-loss, and error-state QA;
- release OS-keychain write/read/clear and failure behavior;
- live rule-based NPC tournament;
- live LLM provider scenarios and the captured Android canonical fixture.

## Android/Desktop interoperability status

Status remains **audited but not fully proven**. Known payload-shape differences for action-window-opened and action-rejected events remain backlog items. The ignored Android canonical fixture test is explicit evidence that mixed-runtime canonical bytes are not yet fully proven.

## Final milestone recommendation

**Continue desktop runtime validation. Do not begin the Android/UniFFI milestone yet.**

The repository now has a trustworthy automated and packaging baseline. The next work should execute the manual release-binary, package-install, local multiplayer, LAN multiplayer, reconnect, keychain, and NPC gates against the recorded source revision. Any reproduced failure should receive a stable backlog ID, a focused regression test where practical, and the smallest correct fix.
"""


def update_backlog() -> None:
    path = "docs/DESKTOP_POKER_CURRENT_BACKLOG.md"
    text = read(path)
    text = replace_once(
        text,
        "- `DP-RR-P0-001` through `DP-RR-P0-006`\n- `DP-RR-P1-001`",
        "- `DP-RR-P0-002` through `DP-RR-P0-006`\n- `DP-RR-P1-001`",
        label="backlog blocker summary",
    )
    text = replace_once(
        text,
        "- **Current behavior:** The repository contains strong historical test evidence, but current results must be produced against the release-readiness branch/final SHA.",
        "- **Current behavior:** Current CI evidence is complete against branch source SHA `fd8369ba7267fe76a827cdf48384c9f826159719` and pull-request merge SHA `c79c9f2473f92310c3d65afe8b834f97b2875c5d`.",
        label="P0-001 behavior",
    )
    text = replace_once(
        text,
        "- **Status:** In progress.",
        "- **Status:** Completed — GitHub Actions run `30224757296` passed all automated gates and retained evidence artifacts.",
        label="P0-001 status",
    )
    text = replace_once(
        text,
        "- **Current behavior:** Tauri build configuration and release paths are documented, but current binary/package hashes and launch evidence are absent.",
        "- **Current behavior:** The direct release binary and Debian package build successfully and have recorded hashes/metadata. Graphical launch and installed-package execution remain unproven.",
        label="P0-002 behavior",
    )
    text = replace_once(
        text,
        "- **Status:** Blocked by current execution environment.",
        "- **Status:** Partially complete — build and inspection PASS; graphical launch/install verification BLOCKED by the current execution environment.",
        label="P0-002 status",
    )
    marker = "\n---\n\n## Desktop P1/P2 items"
    resolved = f"""
---

## Resolved during the release-readiness baseline

### DP-RR-P0-001 — Fresh automated baseline

- **Resolved:** GitHub Actions run `30224757296` passed formatting, lint, Rust workspace tests, focused `poker-core` tests, frontend tests/build, geometry, and both npm audits.
- **Totals:** 588 Rust tests passed, 3 explicitly ignored; 273 frontend tests passed; zero npm vulnerabilities.
- **Evidence:** `docs/DESKTOP_POKER_RELEASE_READINESS_REPORT.md` and retained `validation-evidence` artifact.

### DP-RR-FIX-001 — Production dependency vulnerability

- **Resolved:** Migrated vulnerable React Router 7 production dependency to React Router 8.3.0 with React/React DOM 19.2.8.

### DP-RR-FIX-002 — Development dependency vulnerabilities

- **Resolved:** Upgraded the ESLint toolchain and PostCSS dependency graph; production and full audits now report zero vulnerabilities.

### DP-RR-FIX-003 — React correctness lint findings

- **Resolved:** Reworked state synchronization and render-time ref usage without suppressing lint rules; all frontend checks pass.

---

## Desktop P1/P2 items"""
    text = replace_once(text, marker, "\n" + resolved, label="resolved backlog section")
    write(path, text)


def update_todo() -> None:
    path = "docs/DESKTOP_POKER_RELEASE_READINESS_BASELINE_TODO.md"
    text = read(path)
    status_block = f"""
## Execution status — 2026-07-26

- Automated validation: **PASS** against branch source SHA `{SOURCE_SHA}` and pull-request merge SHA `{MERGE_SHA}`.
- Release binary and Debian package build/inspection: **PASS**.
- Graphical binary/package launch, local multi-instance play, physical LAN play, reconnect interruption, release keychain, and live provider tests: **BLOCKED** by unavailable runtime dependencies.
- Evidence: `docs/DESKTOP_POKER_RELEASE_READINESS_REPORT.md`, GitHub Actions run `{RUN_ID}`.

Unchecked manual-runtime boxes below are intentionally retained; they are not inferred from automated coverage.

"""
    text = replace_once(text, "## Mandatory operating rules\n", status_block + "## Mandatory operating rules\n", label="TODO status block")

    items = [
        "`docs/DESKTOP_POKER_CURRENT_BACKLOG.md`",
        "`docs/DESKTOP_POKER_RELEASE_READINESS_REPORT.md`",
        "Current automated validation evidence in the release-readiness report",
        "Current manual local multi-instance evidence in the release-readiness report",
        "Current two-machine LAN evidence in the release-readiness report",
        "Current NPC and LLM evidence in the release-readiness report",
        "Updated `README.md` status and release instructions when results require changes",
        "A concise final ledger entry in `memory.md`",
        "Confirm the current branch is `master` or record the actual branch being tested.",
        "Confirm the working tree is clean before baseline validation.",
        "Record the exact commit SHA:",
        "Record operating system information:",
        "Record tool versions:",
        "Verify Node.js reports a 24.x version.",
        "Verify Rust stable is active.",
        "Record whether the environment is a local workstation, VM, container, or CI runner.",
        "Record whether a graphical desktop session is available.",
        "Record whether two physical LAN machines are available.",
        "Record whether Ollama or llama-server is available for live LLM testing.",
        "The report contains the exact tested SHA and environment information.",
        "No later test result is attributed to a different unrecorded commit.",
        "Any commit change during defect repair is recorded before the final validation rerun.",
        "Install or confirm the Tauri 2 Linux dependencies:",
        "Record any distribution-specific package substitutions.",
        "Do not change source code to compensate for a missing system package.",
        "Required native dependencies are available.",
        "Environment-only failures are documented separately from product defects.",
        "Run:",
        "Record install success or the exact failure.",
        "Record production and full dependency audit results.",
        "Do not run a forced audit fix automatically.",
        "If vulnerabilities are reported, classify whether they are reachable in the packaged desktop application before changing dependencies.",
        "`npm ci` succeeds using the committed lockfile.",
        "Dependency audit results are recorded honestly.",
        "No lockfile churn is introduced without a documented reason.",
        "Formatting:",
        "Lint:",
        "Vitest:",
        "number of test files;",
        "number of tests passed;",
        "number of tests failed;",
        "skipped or todo tests;",
        "warnings or stderr noise.",
        "Production frontend build:",
        "Browser geometry tests:",
        "If local Playwright browser dependencies are unavailable, install the matching browser version or use the repository's pinned Playwright container.",
        "Do not mark geometry tests passed based only on CI workflow configuration.",
        "Inspect test output for hidden persistence errors, unhandled promise rejections, React warnings, and test teardown leakage.",
        "Specifically check for window-persistence noise:",
        "Formatting passes.",
        "Lint passes with zero warnings.",
        "All non-ignored frontend tests pass.",
        "Production frontend build passes.",
        "Geometry tests pass or are explicitly `BLOCKED` with an environment reason.",
        "Expected stderr is documented; unexplained recurring errors are treated as defects.",
        "Clippy:",
        "Full workspace tests with CI-equivalent thread limit:",
        "Focused shared-core tests:",
        "Dependency tree:",
        "test totals by crate where visible;",
        "ignored tests;",
        "doctest results;",
        "warnings;",
        "failures;",
        "any test that accepts multiple contradictory outcomes.",
        "Inspect ignored tests:",
        "Do not claim ignored LLM tests passed until they are run against a configured provider.",
        "Rust formatting passes.",
        "Clippy passes with warnings denied.",
        "All non-ignored workspace tests pass.",
        "Focused `poker-core` tests pass.",
        "The actual totals are copied from current output, not `memory.md`.",
        "Run the focused source audit:",
        "Inspect every hit.",
        "Distinguish `EngineCommand` false positives from process-spawning `Command` usage.",
        "Verify `crates/poker-core/Cargo.toml` contains only platform-neutral dependencies.",
        "Verify no feature flag conditionally pulls in Tauri, Android, networking, keychain, filesystem path discovery, or LLM dependencies.",
        "Verify deterministic state and command APIs remain available without the desktop crate.",
        "`poker-core` remains reusable by future Android bindings.",
        "No platform dependency is introduced to make desktop validation easier.",
        "Every grep hit is explained in the report.",
        "Build:",
        "Confirm this file exists and is executable:",
        "Record binary size and SHA-256:",
        "The report contains its size and hash.",
        "Locate the generated `.deb`.",
        "Record package filename, size, and SHA-256.",
        "Inspect metadata:",
        "Verify application identifier, version, binary path, desktop entry, and icons.",
        "`.deb` packaging succeeds.",
        "Package metadata is coherent.",
        "Verify release builds select `KeychainSecretStore`.",
        "Verify debug builds may use the explicitly documented local secret file.",
        "Verify provider settings JSON excludes the API key.",
        "Verify `Debug` formatting redacts the API key.",
        "Verify debug inspector state contains no key.",
        "No release plaintext fallback exists.",
        "No API key appears in JSON, logs, snapshots, debug output, or committed files.",
        "Secret-storage failure is explicit.",
    ]

    # Generic labels such as "Run:" and "Build:" occur multiple times. Handle
    # only unambiguous items through the generic helper, then patch section-local
    # command labels separately below.
    ambiguous = {"Run:", "Formatting:", "Build:", "Record binary size and SHA-256:", "Inspect metadata:", "Dependency tree:"}
    for item in items:
        if item in ambiguous:
            continue
        text = check_item(text, item)

    # Mark command-label boxes inside their named sections without touching later
    # manual sections that use the same label.
    section_replacements = [
        ("## P0.3 — Perform a clean dependency installation", "## P0.4 — Run the complete frontend validation suite", "- [ ] Run:", 2),
        ("## P0.4 — Run the complete frontend validation suite", "## P0.5 — Run the complete Rust validation suite", "- [ ] Formatting:", 1),
        ("## P0.5 — Run the complete Rust validation suite", "## P0.6 — Audit `poker-core` platform neutrality", "- [ ] Formatting:", 1),
        ("## P0.5 — Run the complete Rust validation suite", "## P0.6 — Audit `poker-core` platform neutrality", "- [ ] Dependency tree:", 1),
        ("## P0.6 — Audit `poker-core` platform neutrality", "# P0 — Build and inspect production artifacts", "- [ ] Run:", 1),
        ("## P0.7 — Build the release binary", "## P0.8 — Build and inspect the Debian package", "- [ ] Build:", 1),
        ("## P0.7 — Build the release binary", "## P0.8 — Build and inspect the Debian package", "- [ ] Record binary size and SHA-256:", 1),
        ("## P0.8 — Build and inspect the Debian package", "## P0.9 — Build AppImage when tooling is available", "- [ ] Build:", 1),
        ("## P0.8 — Build and inspect the Debian package", "## P0.9 — Build AppImage when tooling is available", "- [ ] Inspect metadata:", 1),
    ]
    for start, end, token, expected in section_replacements:
        start_index = text.index(start)
        end_index = text.index(end, start_index)
        section = text[start_index:end_index]
        count = section.count(token)
        if count != expected:
            raise RuntimeError(f"{start}: expected {expected} occurrences of {token}, found {count}")
        section = section.replace(token, token.replace("[ ]", "[x]"), expected)
        text = text[:start_index] + section + text[end_index:]

    text = text.replace("src-tauri/target/release/desktop-poker", "target/release/desktop-poker")
    write(path, text)


def update_readme() -> None:
    path = "README.md"
    text = read(path)
    text = text.replace("src-tauri/target/release", "target/release")
    text = replace_once(
        text,
        "```bash\nnpm install\nnpm run tauri build\n```",
        "```bash\nnpm ci\nnpm run tauri build\n```",
        label="README production build command",
    )
    text = replace_once(
        text,
        "npm run build\ncargo fmt --check",
        "npm run build\nnpm audit --omit=dev\nnpm audit\ncargo fmt --check",
        label="README validation audit commands",
    )
    text = replace_once(
        text,
        "This compiles the frontend and Rust backend, then produces release bundles under `target/release/bundle/`.",
        "This compiles the frontend and Rust backend, then produces release bundles under `target/release/bundle/`. Pull-request CI also builds a direct Linux binary and Debian package, records hashes/package metadata, and retains them as a temporary `linux-release-candidate` artifact; graphical launch still requires a real desktop session.",
        label="README release output paragraph",
    )
    write(path, text)


def append_memory() -> None:
    path = "memory.md"
    text = read(path).rstrip() + "\n\n"
    entry = f"""## 2026-07-26T16:25:00-07:00 - GPT-5.6 Thinking - Execute release-readiness baseline

- Established `docs/DESKTOP_POKER_CURRENT_BACKLOG.md` and `docs/DESKTOP_POKER_RELEASE_READINESS_REPORT.md` as the authoritative backlog/evidence pair.
- Automated baseline passed on branch source `{SOURCE_SHA}` / PR merge `{MERGE_SHA}` in Actions run `{RUN_ID}`: 588 Rust tests passed with 3 explicit ignores, 273 frontend tests passed, formatting/lint/build/geometry passed, and both npm audits reported zero vulnerabilities.
- Fixed a production React Router advisory, development ESLint/PostCSS advisories, and five stricter React correctness findings without weakening lint rules.
- Built and inspected `target/release/desktop-poker` (21,908,136 bytes, SHA-256 `9f49324fcf431fcef5202d35dd7c000184992305568f2652b6a6c23896c23211`) and `Desktop Poker_0.1.0_amd64.deb` (6,738,672 bytes, SHA-256 `fc534010dfd8c0468511d9e2a24ad6a34e0375de7a55c030d113d43d06286ac7`).
- Remaining blockers are manual: graphical launch/install, two-instance and two-machine LAN tournaments, reconnect/failure injection, release keychain behavior, and live rule-based/LLM NPC scenarios. Android/UniFFI should not begin until those gates are resolved or explicitly reclassified.
"""
    if "GPT-5.6 Thinking - Execute release-readiness baseline" not in text:
        text += entry
    write(path, text)


def main() -> None:
    write("docs/DESKTOP_POKER_RELEASE_READINESS_REPORT.md", build_report())
    update_backlog()
    update_todo()
    update_readme()
    append_memory()


if __name__ == "__main__":
    main()
