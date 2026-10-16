# Desktop Poker — Claude Code development handoff

**Prepared:** October 15, 2026 (Pacific Time; GitHub evidence checked October 16 UTC)  
**Repository:** https://github.com/ekkus93/desktop_poker  
**Branch:** `master` — work directly here; do not open a branch/PR per task.  
**Observed master when this handoff was prepared:** `8dfeeac7bf7c894bb818868e627207a7c3858757`  
**Overall disposition:** **REMEDIATION IN PROGRESS; NOT SOFTWARE-QUALIFIED; DO NOT PUBLISH A RELEASE.**

> This is a point-in-time operational handoff, **not** a replacement for the authoritative remediation spec/TODO. The current master may advance through other Ralph sessions or GitHub Actions evidence publishers. Claude Code must re-read the actual master head and current evidence before changing anything. **Unchecked TODO boxes mean not verified, not necessarily absent.**

## 1. What to read first (sources of truth)

1. `docs/DESKTOP_POKER_CODE_REVIEW_REMEDIATION_SPEC_2026-10-15.md` — binding design choices, invariants N-1..N-4 / H-1..H-6 / S-1..S-2 / U-1 / Q-1, complete qualification criteria and dependency order.
2. `docs/DESKTOP_POKER_CODE_REVIEW_REMEDIATION_TODO_2026-10-15.md` — authoritative DPCR-000 / NET / HIST / IO / SEC / UI / DOC / QA item IDs; checkboxes and rolling execution evidence.
3. `docs/DESKTOP_POKER_CURRENT_BACKLOG.md` — larger product/release backlog, nine DP-REVIEW findings, independent non-software gates.
4. `docs/DESKTOP_POKER_RELEASE_READINESS_REPORT.md`, `docs/PHYSICAL_LAN_RELEASE_VALIDATION.md`, `docs/MANUAL_QA_CHECKLIST.md` and `docs/runtime-validation/*-latest.json` — older qualification and release-gate evidence.
5. The October 15 source review, `DESKTOP_POKER_CODE_REVIEW_2026-10-15.md`, if available in the checkout or accompanying materials. The remediation spec captures its nine findings.

**Reconcile source against documents.** The TODO still starts with the historical word “Proposed,” and most boxes remain unchecked even though a substantial implementation already exists on master. Do not treat the heading as proof that code has not been written; likewise do not mark a whole subsystem done because its implementation compiles.

## 2. Working application and scope

Desktop Poker is an advanced Linux desktop multiplayer Texas Hold'em MVP, built with Rust/Tauri plus a React/TypeScript/Vite frontend. The engine, tournament rules, signed/encrypted networking, reconnect protocol, local persistence, NPC providers, desktop packaging and real WebKit runtime tests already exist. Node **24** and Rust stable are the expected validation toolchains.

Previous acceptance evidence proves, within its tested scope:

- Linux direct release binary, installed `.deb` and AppImage build and launch; the installed package's desktop entry and uninstall path have dedicated tests.
- Two isolated *same-machine* Tauri release instances host/join, seat/ready/start, play real TCP hands, reject illegal actions, settle a tournament, agree on standings, respect host/client/observer state and restore histories after process restart. A fresh third profile is isolated.
- Lobby/active-play reconnect, duplicate-action rejection and terminal host-loss behavior have Linux protocol/release matrix tests.
- Rule-based NPC and embedded local GGUF NPC tournaments, plus Secret Service credential persistence/negative tests, have passing Linux workflows.

**Do not equate same-machine TCP with a two-physical-machine LAN qualification.** The deferred physical gate `DP-RR-P0-004` is independently open. Mixed Android/Desktop interoperation, advertised remote LLM provider live-failure matrices, and manual screen-reader/accessibility checks are also independently open. Neither this handoff nor passing loopback CI authorizes publication.

## 3. Exactly where development stopped

### Current CI result — first issue to address

The most recent **completed main CI** was [run 38848772935](https://github.com/ekkus93/desktop_poker/actions/runs/38848772935), executed at code SHA `002f76041c6d88f0190adf6e0d15fd02ab695a95`, concluded **FAILURE** on October 16 UTC. Its **only failing Verify step** was **“Enforce npm dependency audit results.”**

Everything before that step in Verify succeeded: `cargo fmt`, Clippy with warnings denied, Rust workspace tests, direct `poker-core` tests, frontend Prettier, frontend ESLint, frontend Vitest, frontend production build, and the browser geometry job. In its main Rust test suite, **555 passed, 0 failed, 3 ignored**; a separate `poker-core` suite showed **131 passed, 0 failed**. The evidence is tied to **that exact SHA**, not automatically to every later evidence-only or code commit.

The CI job printed:

- `npm audit --omit=dev --json`: **0 vulnerabilities** in production dependencies.
- `npm audit --json`: **8 development-tree vulnerabilities: 2 critical, 4 high, 2 moderate**. Packages reported: `@vitest/mocker`, `baseline-browser-mapping`, `brace-expansion`, `browserslist`, `nanoid`, `source-map-js`, `tinypool`, and `vitest`.
- Particularly relevant: `tinypool` reported critical worker-option prototype-pollution/RCE advisories **GHSA-5gmw-xhrv-c9v3** and **GHSA-85c8-ppgw-ccpr**. `vitest` was a direct development dependency in the vulnerable tree; the captured audit offered `vitest 5.0.5` as a **SemVer-major** fix. `package.json` presently specifies `"vitest": "^3.1.3"`.

**Next:** Reproduce against the current lockfile; inspect `package.json`, `package-lock.json`, `npm audit --json` and transitive upgrade constraints. Make the smallest safe dependency update, without blindly using `npm audit fix --force` or suppressing the policy gate. Upgrade Vitest and impacted transitive dependencies compatibly, update the lockfile, run full Vitest, browser geometry, lint, format and build, and run a fresh production **and** full audit. Requalify with a completed exact-code-head CI run. New registry advisories may change the count.

CI evidence was persisted on master in `docs/runtime-validation/ci-latest.json`:
`result: FAIL`, `validatedCommit: 002f76041c6d88f0190adf6e0d15fd02ab695a95`, `workflowRunId: 38848772935`, `verifyResult: failure`, `geometryResult: success`.

### Release workflows that did pass

At that same code SHA `002f76041c6d88f0190adf6e0d15fd02ab695a95`:

| Workflow | Run | Outcome |
|---|---|---|
| Linux release runtime validation | [38848773003](https://github.com/ekkus93/desktop_poker/actions/runs/38848773003) | **PASS** (release build + single/multi-instance smoke + evidence publication) |
| Linux release full-game validation | [38848772933](https://github.com/ekkus93/desktop_poker/actions/runs/38848772933) | **PASS** (three settled hands, winner/standings, fresh-profile isolation, host/client history after restart) |
| Main CI | [38848772935](https://github.com/ekkus93/desktop_poker/actions/runs/38848772935) | **FAIL** (full npm dev dependency audit only) |

At source SHA `9416a68e18fe3f2681885a21e62629e8f2030767`, separate passing workflows include reconnect matrix [38848578257](https://github.com/ekkus93/desktop_poker/actions/runs/38848578257), keychain [38848578234](https://github.com/ekkus93/desktop_poker/actions/runs/38848578234), rule-based tournament [38848578248](https://github.com/ekkus93/desktop_poker/actions/runs/38848578248), Debian package [38848578230](https://github.com/ekkus93/desktop_poker/actions/runs/38848578230), AppImage [38848578202](https://github.com/ekkus93/desktop_poker/actions/runs/38848578202), embedded tiny-model NPC [38848578337](https://github.com/ekkus93/desktop_poker/actions/runs/38848578337), and release runtime [38848578185](https://github.com/ekkus93/desktop_poker/actions/runs/38848578185). Those successes are not interchangeable with a passing *main* CI at the final executable SHA.

Earlier many runs were **cancelled** when later master updates superseded them. Do not count cancelled checks as passes; use run IDs/head SHAs and actual job conclusions.

## 4. What was implemented/reconciled in the most recent ChatGPT Ralph session

All listed changes below were committed **directly to master** with Ralph Bridge; no per-task branches/PRs were created:

| Commit | Change | Meaning / limitations |
|---|---|---|
| `8062292cf99f0f2c31cf2cea63a5bfe053997c53` | Added `stale_join_cleanup_does_not_remove_newer_connection` to `src-tauri/src/networking/runtime/tests/admission_rollback.rs` | N-3 conditional-rollback regression; does **not** prove full same-ID concurrent join/reconnect ownership or all post-accept failure points. |
| `0da2fda5d5a1720e8f26abe21788cbf41be0024e` | Fixed rustfmt for that test | Earlier CI [38847304763](https://github.com/ekkus93/desktop_poker/actions/runs/38847304763) had failed `cargo fmt` before this fix. |
| `57f74c3f289a7bbdcdf1b2ab45bf994b969e3e2a` | Recorded baseline/CI in canonical remediation TODO | Purely evidence/documentation; no review finding signed off. |
| `a5e6d12512891aa5fe9e3eef701fa0b0849a3385` | Applied exact Prettier changes to six frontend files | Cleared earlier format discrepancies in `persistence{,.test}.ts`, `HandHistoryScreen{,.test}.tsx`, `NpcProfilesScreen.test.tsx`, `useLobbySession.test.ts`. Subsequent CI at `002f760` confirms frontend format PASS. |
| `9416a68e18fe3f2681885a21e62629e8f2030767` | Added `history_profile_lock_preserves_existing_lock_file_contents` and `failed_history_lock_acquisition_preserves_existing_v2_bytes` in `history_storage_v2.rs` | Adds lock-file preservation and failed-lock non-destructive persistence regressions. Not the entire fsync/rename/fault-injection matrix. |
| `002f76041c6d88f0190adf6e0d15fd02ab695a95` | Runtime workflow now publishes evidence using `scripts/push_runtime_evidence.sh` | Replaced fragile one-shot `git push` with existing fetch/rebase/retry publisher. Subsequent runtime [38848773003](https://github.com/ekkus93/desktop_poker/actions/runs/38848773003) passed. |
| `8c25514db1ecf2b8fc2a231af74b1ff6b29732da` and subsequent evidence-only commits | Updated remediation TODO and published CI/runtime evidence | Not proof of additional code remediation; always reconcile current master. |

A **temporary read-only formatter-diff GitHub workflow** was created solely to obtain the exact Prettier repair, ran [38847603576](https://github.com/ekkus93/desktop_poker/actions/runs/38847603576), and was removed. **Do not recreate it unnecessarily.**

Runtime evidence workflows may commit `docs/runtime-validation/*-latest.{json,md}` directly to master. Such autogenerated records can advance HEAD while another session is working; compare-and-swap/rebase rather than overwrite them. Avoid making parallel writes to the same canonical TODO/backlog without reconciling intervening changes.

## 5. Nine review findings: current implementation versus acceptance gaps

**All DP-REVIEW-001..009 remain OPEN in the canonical TODO outcome table**, not because no work exists, but because complete proof and checklist reconciliation are not yet recorded. Recommended dependency order: network + security; coordinated history V2/storage; history/UI async; backlog; final QA.

| Review ID | Implementation already present (verify on fresh master) | What remains unverified / to finish |
|---|---|---|
| **001** Transactional admission | `src-tauri/src/networking/runtime/{mod,handlers,host,host_session}.rs`; rollback guard and `tests/admission_rollback.rs` tests snapshot construction failure, failed reconnect snapshot, unregistered cleanup, stale loser cleanup. | Inject snapshot **send** and post-accept faults; timeout clear/clone/registry/spawn failures; conditional attempt generations; same-ID concurrent winners; repeated retry/capacity recovery; actual abrupt TCP disconnect; error redaction and lock order. |
| **002** Tournament-scoped history | `src-tauri/src/history_storage_v2.rs`; `TableViewSnapshot.sessionEpoch`, Rust and TS DTOs; `hand-history.v2.json` compound identity; two-tournament unit tests. | End-to-end two separate tournaments both hand 1 **across process restart**; V2 contract negative tests; downgrade/re-upgrade handling and identity propagation everywhere. |
| **003** Durable over cache | `src/app/persistence.ts` V2 readers/mergers and cache-v1 provenance; V2 Rust lock/merge. | One-time imported-cache marker/fingerprint and non-destructive failure/retry; stale cached conflicting identity; malformed/unsupported V1/V2; passive load must not destructively write; ensure no legacy V1 API unexpectedly remains authoritative in production paths. |
| **004** Newest-first history paging | `src/screens/HandHistoryScreen{,.test}.tsx` has grouped tournament selection, 100/101/205 entry tests. | Prove grouped legacy/current/saved session ordering, complete access to older history, stable compound keys and accessibility under real V2/restart/error conditions; update DP-UX-P2-001 evidence. |
| **005** Invitation decode bounds | `src-tauri/src/protocol/join_payload.rs` caps encoded compact 16 KiB, compressed payload 12 KiB, decompressed and legacy JSON 64 KiB and has oversize/bomb regressions. | Exhaustive boundary, malformed base64/gzip/UTF-8, error sanitization, round-trip and exact-head security negative qualification. |
| **006** Plaintext provider API-key migration | `src-tauri/src/npc/provider_storage.rs` migration and error-preservation tests; Linux Secret Service release validation passes. | Full old `claude-api-key.txt` / embedded key migration, preexisting/differing secure key, keychain failure, restart and no-plaintext/synthetic-secret logging matrix with evidence. |
| **007** History transactional IO | `history_storage_v2.rs` uses OS file locking, random create-new temporary, file/directory sync and tested concurrent merges; new lock error tests at `9416a68`. | Prove same-profile multi-process writer policy; atomic create/write/fsync/rename/backup fault injection, Windows recovery semantics, permissions, no loss/false success, process-level race evidence. |
| **008** Monotonic UI refresh | Request generation guards in `src/screens/useLobbySession.ts` and `MainTableScreen.tsx`; tests for stale event/poll inversions. | Deferred-promise success/error races, session replacement and terminal monotonicity, cleanup/unmount, timers and no reconnect/backoff regression. |
| **009** Backlog/document consistency | Current backlog cross-references review items; ready-state/NPC slug regressions have been added to the source/test base. | Verify `DP-UX-P1-001`, `DP-UX-P1-003`, `DP-UX-P2-001` in current tests; update precise statuses, README/architecture/migration notes and final DP-REVIEW evidence table. |

**Important**: the Phase 2 schema implementation resides primarily in `history_storage_v2.rs`, not solely `history_storage.rs`. The older V1 store/API may still exist for compatibility. Do not delete V1 data, invent legacy tournament identity, globally dedupe different tournaments by `handNumber`, or treat browser localStorage as durable acknowledgment.

## 6. Concrete next actions for Claude Code

1. **Start with live reconnaissance.** Run `git status --short`, `git rev-parse HEAD`, `git log -n 12 --oneline`; fetch/reconcile master safely; inspect existing Ralph sessions and PRs, latest `docs/runtime-validation` evidence, current TODO/backlog, and whether a newer CI run already repaired the audit. The recorded head above is only a checkpoint. Do not create a task branch or duplicate an ongoing fix.
2. **Resolve the npm full-audit regression first.** See §3. Work against the latest lockfile and Node 24; evaluate Vitest 5 migration impacts and exact advisories, make minimal secure changes, rerun all frontend commands and `npm audit --omit=dev` plus full `npm audit`. Do not weaken CI just to turn the badge green. Commit directly to master and complete exact-head CI; do not mark QA done while any required workflow is failed/cancelled.
3. **Reconcile Phase 0 and network safety.** Compare active master with DPCR-000 and NET tasks, add deterministic fault injection and winner-ownership tests where missing, run real TCP interrupted join/reconnect and host shutdown matrix. Document each proven subtask immediately under its item, but avoid checkbox inflation.
4. **Finish one coherent V2 history contract (Phases 2–3).** Audit all producer/consumer paths, preserve V1 files byte-for-byte, make cache import idempotent and durable-authoritative, test stale cache, two tournaments with overlapping hand numbers, corrupt/unsupported files, process restart/downgrade/re-upgrade, concurrent same-profile writers and fault recovery. Specifically examine any missing durable V1 source fingerprint/import marker or cache-migration marker.
5. **Close security and UI matrices (Phases 4–5).** Exercise invite negative bounds, provider secret-migration failures, history paging/legacy groups and frontend poll/event inversions, including terminal/unmount behavior. Use synthetic secrets and never log real tokens, hole cards or keys.
6. **Reconcile backlog and sign-off (Phases 6–7).** Attach actual commit SHAs, test names/counts and run/artifact URLs to every checked TODO group and DP-REVIEW-001..009 row. Preserve the physical LAN and other independently deferred gates; publish neither a release nor an unqualified PASS.

### Recommended validation commands (from repo root)

```bash
npm ci
npm audit --omit=dev
npm audit
npm run format:check
npm run lint
npm run test
npm run test:geometry
npm run build

cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-targets --all-features -- --test-threads=2
cargo test -p poker-core --all-targets --all-features
```

Also run focused Rust/TS test filters while developing; final acceptance requires corresponding GitHub Actions `.github/workflows/ci.yml`, `runtime-validation.yml`, `runtime-full-game-validation.yml`, reconnect, keychain and relevant release-package/model workflows against the correct executable code SHA. Validate generated JSON/artifact content, not simply workflow names. CI setup requires Tauri/WebKit Linux native dependencies; use the workflow as the environment source of truth.

## 7. Workflow, repository safety and reporting rules

- The owner **explicitly requests direct, coherent commits to `master`**, with **no branch or PR for each task/subtask**. Do not force push. Check the exact branch head before writing and reconcile source/evidence generated by other sessions.
- In the ChatGPT Ralph workflow, **Ralph Bridge is the exclusive GitHub and CI integration; do not use ChatGPT's default GitHub tool**. Claude Code may run normal local development commands in its checkout; follow the owner's configured GitHub/Ralph integration and avoid duplicating concurrent GitHub writes or circumventing repository authorization. If its environment cannot use Ralph Bridge, ask the owner before switching remote write mechanisms.
- `scripts/push_runtime_evidence.sh` already retries/rebases evidence-only commits. The runtime workflow now uses it; monitor for new publication races rather than assuming the bug can never recur.
- Separate **IMPLEMENTED**, **TESTED LOCALLY**, **CI QUALIFIED**, **RUN-TIME QUALIFIED**, **BLOCKED BY HARDWARE**. Never mark a task complete from code inspection alone. Failed/cancelled CI must remain labelled as such.
- No raw invitation/join/reconnect secrets, API credentials, private hole-card data, or full private user settings in tests, logs, artifacts or documentation. Use synthetic fixtures, redact diagnostics, preserve legacy user data.
- Each commit should be focused and non-destructive. After each genuine merge or source advance, reload the TODO and backlog from current master. Where a mandatory test needs unavailable physical hardware, keep that **specific** external gate open rather than stopping independent software remediation.

## 8. Practical final handoff state

At this handoff's observation point:

- **Master:** `8dfeeac7bf7c894bb818868e627207a7c3858757` (most recent checked GitHub master before adding this handoff; may advance).
- **Last exact-code-head main CI:** **FAILED solely on full npm development dependency audit**, run `38848772935`, tested SHA `002f76041c6d88f0190adf6e0d15fd02ab695a95`.
- **Last exact-code-head runtime/full-game:** **PASS**, runs `38848773003` / `38848772933` at that same SHA.
- **Checklist:** some reconnaissance boxes checked; all nine review outcome rows still **OPEN** pending complete qualifying evidence.
- **Immediate engineering work:** repair development dependency audit, rerun complete CI, then close remaining verification/implementation gaps by priority.
- **Needs owner/hardware later:** two-physical-machine LAN gate `DP-RR-P0-004` and the separately tracked Android/remote-provider/manual acceptance. These should not prevent finishing ordinary software tasks.

**Claude Code objective:** Continue the canonical `docs/DESKTOP_POKER_CODE_REVIEW_REMEDIATION_TODO_2026-10-15.md` dependency-first until every software task and subtask is implemented, tested, reconciled, evidenced and qualified on master, or a **specific** user-only prerequisite blocks further progress. Do not stop merely to report progress after one fix.
