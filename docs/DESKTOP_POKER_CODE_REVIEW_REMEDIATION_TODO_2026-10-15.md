# Desktop Poker — Code Review Remediation TODO

**Status:** In progress; unchecked boxes mean **not verified**, not necessarily missing in a later checkout. Remaining work needs the original review attachment and cleanup of identified pre-fix workflow artifacts.

**Date:** 2026-10-15  
**Repository:** `https://github.com/ekkus93/desktop_poker`  
**Snapshot reviewed:** `master` `fb7452e9ab74250bbc5b47283692ea66f35ac760`  
**Authoritative specification:** `docs/DESKTOP_POKER_CODE_REVIEW_REMEDIATION_SPEC_2026-10-15.md`  
**Source of findings:** `DESKTOP_POKER_CODE_REVIEW_2026-10-15.md`

> **Ralph Loop operating contract (once development is authorized):** Use Ralph Bridge **exclusively** for all GitHub/CI operations; never use the default GitHub tool. Inspect current master, branches/open PRs, exact-head CI and active work before writing. Reconcile existing qualifying work first. Work directly on `master` (no per-task branches/PRs). Do not skip ahead of unsatisfied dependencies or mark a task done merely because code exists. Implement, test, commit in coherent increments, qualify, update this checklist and continue through dependency-satisfied work. Do not claim a run happened unless the tool evidence exists. Creating these files does not itself authorize repository writes.

## Checklist conventions

- `[ ]` Not started or not yet verified on current head; `[x]` completed **only with evidence**.
- Tasks have stable identifiers `DPCR-...`; cross-reference original review items `DP-REVIEW-001..009`.
- Attach evidence immediately below a completed group: master SHA, test command/result, workflow run URL, artifact path/hash if applicable, notes on remaining risks.
- Mark external/manual dependencies `BLOCKED` only if the real prerequisite is missing. Transient network/CI/tool errors are `PENDING` with captured diagnostics and retry plan.
- Use the companion spec for invariant-level acceptance criteria. Update checklist statuses in the same commit as changes where feasible.

## Phase 0 — Reconnaissance, baseline and reproductions (must precede coding)

- [ ] **DPCR-000.1** Read full remediation spec and review. Confirm HEAD SHA and compare the nine findings to **current** `master` (do not assume October 15 snapshot is latest).
- [x] **DPCR-000.2** Inspect open branches/PRs, active Ralph sessions/CI, completed and running workflow runs, recent work and consolidated current backlog; reconcile prior/duplicate fixes before proceeding.
- [x] **DPCR-000.3** Confirm `cargo`, Rust stable/rustfmt/clippy, Node 24, npm, Tauri Linux dependencies and test runner availability; record baseline versions.
- [x] **DPCR-000.4** Run baseline Rust tests, Clippy, formatting, frontend tests/lint/format/build and existing CI-parity commands; note ignored/flaky tests and link exact SHA.
- [x] **DPCR-000.5** Add focused *failing* repro for failed post-admission snapshot send or injectable post-accept failure; verify phantom/cleanup invariant.
- [x] **DPCR-000.6** Add focused failing fixture with two tournaments both containing hand 1 and a restart; verify global-hand collision on old schema.
- [x] **DPCR-000.7** Add failing fixture with durable newer hand and stale cache older same hand; show incorrect overwrite precedence.
- [x] **DPCR-000.8** Confirm existing 205-hand UI test asserts reverse history paging; document test-oracle correction needed.
- [x] **DPCR-000.9** Add bounded decode/legacy-secret/concurrent-writer/out-of-order-request reproducers where deterministic; label static risks not runtime-proven until reproduced.
- [x] **DPCR-000.10** Record baseline in TODO/evidence without raw secrets, join tokens, hole cards or private settings.

**Phase 0 exit:** Baseline SHA and executable regression plan recorded; no duplicate fixes started.

## Phase 1 — Network admission transactional cleanup (DP-REVIEW-001; High)

- [x] **DPCR-NET-1.1** Trace both `handle_join_request` and `handle_reconnect_request`, `handle_initial_client_request`, host accept, `spawn_host_client_session`, registration and all post-accept returns.
- [x] **DPCR-NET-1.2** Document lock order and choose per-attempt provisional state/rollback guard plus unique attempt generation; do not hold global state locks during blocking socket I/O.
- [x] **DPCR-NET-1.3** Capture and restore pre-attempt reconnect state and tokens/expiry, and remove only an uncommitted initial join; rollback is conditional on attempt ownership.
- [x] **DPCR-NET-1.4** Close failed transport, free pending and active-client slot guards and emit categorized/redacted diagnostics for **every** failure after admission.
- [x] **DPCR-NET-1.5** Audit and, if necessary, expose errors from `spawn_host_client_session`; transfer cleanup responsibility exactly once after registration.
- [x] **DPCR-NET-1.6** Provide deterministic injection points for snapshot build/send, timeout clear, stream clone, registry lock/insert and session spawn; avoid fragile sleeps for core unit tests.
- [x] **DPCR-NET-1.7** Test abrupt disconnect before initial snapshot, repeated retries, seat/capacity recovery and absence of connected phantom in authoritative state.
- [x] **DPCR-NET-1.8** Test failed reconnect snapshot delivery followed by later valid reconnect; preserve correct key/token/expiry/phase rules.
- [x] **DPCR-NET-1.9** Test concurrent same-ID join/reconnect, already-connected rejection, and that loser cleanup never disrupts winner.
- [x] **DPCR-NET-1.10** Rerun `networking/runtime/tests/{join,reconnect,reconnect_expiry,host_shutdown}.rs` and relevant integration reconnect matrix; no leaked secret/private state, no lock deadlock.
- [x] **DPCR-NET-1.11** Commit coherent network fix directly to master when authorized; record exact-head CI/focused TCP proof.

**Phase 1 exit:** N-1 through N-4 hold under success and injected failure conditions.

**Phase 1 evidence (2026-10-16, Claude Code; commit `2e98e04` on master; TESTED LOCALLY and CI QUALIFIED at that exact head):**

- **1.1 trace:** accept thread (`host.rs`) -> `handle_initial_client_request` -> `handle_join_request` / `handle_reconnect_request` (state mutated under one authoritative-state critical section) -> `admission::complete_admission` (snapshot write -> read-timeout reset -> stream clone -> `register_client` -> `spawn_host_client_session`). All post-accept returns now flow through `complete_admission`.
- **1.2 lock order / generation:** documented in `networking/runtime/client_registry.rs` and `admission.rs`: `transition_lock -> authoritative_state`; the client registry is a leaf lock, never held across socket I/O or while taking state (previously every broadcast/snapshot-sync write ran while holding the registry lock). Each registration has a process-unique generation.
- **1.3 precise conditional rollback:** `AdmissionKind::{NewJoin{replaced}, Reconnect(ReconnectPrior)}`; reconnect rollback restores only connection state, expiry and (if unchanged) participant state, so concurrent gameplay commits survive (previously the whole pre-attempt entry was re-inserted, which could un-eliminate a player). Ownership = `Connected` + attempt token; rationale in `rollback_admission` docs.
- **1.4 cleanup/diagnostics:** every post-accept failure settles state, releases the slot, then closes the transport (close is last, so a peer that sees EOF retries against settled state). Health records `admission failed during <stage>: <io error>` with existing counters; tests assert no token in diagnostics.
- **1.5 spawn:** `spawn_host_client_session` already returned launch errors; it now takes `HostSessionContext` + generation, and spawn failure retires exactly this attempt's registration. Compensation is disarmed only after a successful spawn.
- **1.6 injection:** `AdmissionFaults::fail_at(stage)` for snapshot write, timeout reset, clone, registration, spawn (test-only constructor). Snapshot *build* failure remains covered by the existing poisoned-key tests in `admission_rollback.rs`. Registry poison is exercised for real, not injected.
- **1.8:** `failed_reconnect_at_every_stage_restores_eligibility_and_later_retry_succeeds` — prior state/token/expiry restored byte-equal, wrong token and wrong key still rejected, later valid reconnect succeeds. Injected, not an OS-level interrupted reconnect.
- **1.9:** `concurrent_same_identity_attempts_have_exactly_one_winner` (20 rounds x 8 racers, join and reconnect; reconnect losers get `RECONNECT_ALREADY_CONNECTED`); `stale_session_exit_does_not_retire_or_demote_newer_registration`; `failed_write_on_stale_handle_does_not_retire_newer_registration`; `registration_never_replaces_an_existing_connection`. Mutation check: reverting the generation guard or the precise rollback makes the corresponding tests fail.
- **1.7:** Added `peer_disconnected_before_initial_snapshot_can_retry_without_phantom_or_slot_leak`: four real TCP attempts close the peer before the initial snapshot write; every failure removes the participant, releases the active-client slot, and allows a valid same-identity retry. The existing `abrupt_disconnects_after_join_request_leave_no_connected_phantom_or_slot_leak` also repeats disconnect/retry beyond configured capacity and verifies a fresh client is admitted. Both tests pass locally and in exact-head CI at `88489f2`. Capacity policy follows the existing tested contract: reconnect-eligible participants continue to count toward capacity until explicitly removed; an expired reconnect token prevents reconnection but does not implicitly remove the participant or release capacity.
- **Snapshot-registration gap:** Commit `88489f2` serializes initial snapshot creation with transitions, then checks the public-event sequence before registration. If it advanced while the initial snapshot was being written, the server builds a current catch-up snapshot, locks the client writer before registration, registers under the transition lock, and sends the catch-up before later broadcasts. `admission_snapshot_sync_covers_public_event_before_registration` proves the client receives the stale initial snapshot followed by the newer authoritative snapshot.
- **1.10:** `cargo test --workspace --all-targets --all-features -- --test-threads=2`: 566 passed, 3 ignored; `poker-core` 131 passed; networking suite 15/15 repeated runs green; fmt and clippy `-D warnings` clean. Exact-head CI at `2e98e04`: all nine workflows **success** — main CI [38861050988](https://github.com/ekkus93/desktop_poker/actions/runs/38861050988) (Rust 566 passed / 3 ignored, poker-core 131), reconnect matrix [38861050975](https://github.com/ekkus93/desktop_poker/actions/runs/38861050975), full game [38861050965](https://github.com/ekkus93/desktop_poker/actions/runs/38861050965), runtime [38861051139](https://github.com/ekkus93/desktop_poker/actions/runs/38861051139), keychain [38861051047](https://github.com/ekkus93/desktop_poker/actions/runs/38861051047), Debian [38861050881](https://github.com/ekkus93/desktop_poker/actions/runs/38861050881), AppImage [38861050877](https://github.com/ekkus93/desktop_poker/actions/runs/38861050877), rule-based NPC [38861050952](https://github.com/ekkus93/desktop_poker/actions/runs/38861050952), embedded model [38861050941](https://github.com/ekkus93/desktop_poker/actions/runs/38861050941).
- **1.11:** transactional network fixes are committed on master (`2e98e04`, `9ac6077`, `88489f2`). The exact `88489f2` CI and eight release workflows passed; evidence is recorded below. DP-REVIEW-001 is implemented and qualified, with the repository's existing reconnect-capacity behavior preserved.
- **Residual risk:** physical two-machine LAN remains a separate release gate. Session replies and broadcasts share per-client writer serialization; `concurrent_session_writes_remain_individual_frames` covers frame integrity.

## Phase 2 — One reviewed V2 history contract (DP-REVIEW-002/003/007; High)

- [x] **DPCR-HIST-2.1** Inventory all history writers/readers/mergers: Rust commands, `DesktopShellProvider`, `MainTableScreen`, `TournamentCompleteScreen`, `HandHistoryScreen`, `src/app/persistence.ts`, storage fixtures and integration scripts.
- [x] **DPCR-HIST-2.2** Define, write tests for, and freeze V2 JSON schema (`schemaVersion:2`), typed live tuple `(tableId,sessionEpoch,handNumber)`, optional display metadata and explicit legacy-origin union; reject malformed identities.
- [x] **DPCR-HIST-2.3** Add authoritative `sessionEpoch` to Rust `TableViewSnapshot` and frontend `TableViewSnapshot`, ensuring existing session status, observer, debug fixtures, and DTO tests agree; do not infer from `tableId` string.
- [x] **DPCR-HIST-2.4** Implement V2 Rust serialization and validation in `history_storage_v2.rs`; place V2 in `hand-history.v2.json` and preserve V1 `hand-history.json` byte-for-byte.
- [x] **DPCR-HIST-2.5** Replace hand-number-only normalization/merge with deterministic compound identity and descending per-tournament ordering. Verify the same hand number in two sessions survives.
- [x] **DPCR-HIST-2.6** Add V2-specific Tauri read/merge commands; enforce schema at backend trust boundary; add command contract tests.
- [x] **DPCR-HIST-2.7** Implement one-time, crash-recoverable V1 disk import into labeled legacy group(s); the atomic V2 file itself is the durable import marker, with deterministic provenance/identity and no separate source fingerprint. Ensure repeated reads/restarts do not duplicate or rewrite.
- [x] **DPCR-HIST-2.8** On missing/invalid/unsupported V1/V2, retain original bytes, distinguish missing from invalid, fail non-destructively and surface useful errors.
- [x] **DPCR-HIST-2.9** Implement explicit durable-source precedence over cache for matching identity; passive load must not issue a destructive merge/write.
- [x] **DPCR-HIST-2.10** Implement one-time V1 cached-data import; missing entries may be added, conflicting/ambiguous data stays labeled by provenance, never falsely assigned to current table. Preserve cache until V2 durability verified.
- [x] **DPCR-HIST-2.11** Change TS validation, normalization, caching, and history callbacks to carry session identity end-to-end; prohibit global `handNumber` dedupe.
- [x] **DPCR-HIST-2.12** Preserve final-hand durable acknowledgment and error handling. A cache-only write does not produce `Final history saved.`
- [x] **DPCR-HIST-2.13** Test two tournaments each with hand 1+, two profile namespaces, process restart, legacy V1 imports, duplicate updates within same tournament, stale cache and corrupt-file recovery.
- [x] **DPCR-HIST-2.14** Test upgrading from V1, rolling back to old binary without deleting V2, and re-upgrading without losing V2/new data; document downgrade visibility caveat.

**Phase 2 exit:** H-1, H-2, H-3 and H-5 hold with green Rust/TS tests and restart evidence. Complete Phase 3 before calling history integrity fully resolved.

## Phase 3 — History transactions and unique temporaries (DP-REVIEW-007; Medium)

- [x] **DPCR-IO-3.1** Decide shared-profile process policy; where same-profile multi-process writes are possible, implement OS-backed interprocess locking or enforce single-writer ownership with a visible error.
- [x] **DPCR-IO-3.2** Put Rust read–modify–merge–write under one scoped per-profile transaction lock; avoid deadlock with UI/network state locks.
- [x] **DPCR-IO-3.3** Replace PID+millisecond temporary name with unpredictable unique create-new path; retain file sync, replace, directory sync and temporary cleanup.
- [x] **DPCR-IO-3.4** Audit Windows backup/replace behavior for recovery (without claiming Windows full runtime support); Windows full-runtime qualification is not claimed.
- [x] **DPCR-IO-3.5** Prevent no-op read from rewriting/updating timestamp. Ensure idempotent merging of identical authoritative data.
- [x] **DPCR-IO-3.6** Concurrent-write test with at least two disjoint batches targeting the same profile; final V2 contains union, no overwrite/lost updates. Add process-level variant if same profile may have multiple processes.
- [x] **DPCR-IO-3.7** Fault inject create/write/fsync/replace/lock failures; verify earlier file remains readable and no success acknowledgment is sent.
- [x] **DPCR-IO-3.8** Verify file permissions and cleanup; run history storage unit/integration tests and commit with exact-head proof.

**Phase 3 exit:** H-4 holds for the supported profile/process model.

## Phase 4 — Invitation and secret security hardening (DP-REVIEW-005,006; Medium)

- [x] **DPCR-SEC-4.1** Bound entire compact invite to 16 KiB and decoded gzip bytes to 12 KiB, before dangerous allocation.
- [x] **DPCR-SEC-4.2** Bound decompressed compact JSON to 64 KiB using bounded reader (+1 overflow sentinel); bound legacy raw JSON to 64 KiB before parse.
- [x] **DPCR-SEC-4.3** Preserve valid compact/legacy invitation round-trips and clear non-secret rejection messages; reject malformed base64, gzip, UTF-8 and oversized input.
- [x] **DPCR-SEC-4.4** Add compact expansion/bomb test, under/at/over-limit fixtures and log-redaction assertions; no crash or unbounded allocation.
- [x] **DPCR-SEC-4.5** In provider settings migration, detect embedded `apiKey` even if `read_key()` already yields `Some`; prefer current secure-store key, then scrub embedded key atomically.
- [x] **DPCR-SEC-4.6** For secure-store-empty, write/verify the embedded key before scrubbing; on secure read/write error or sanitization failure, show error and retain recoverable bytes.
- [x] **DPCR-SEC-4.7** Review and complete one-time migration/retirement of `claude-api-key.txt`; never silently reintroduce plaintext in release; document debug secret-store exception.
- [x] **DPCR-SEC-4.8** Write tests for pre-existing key, differing legacy key, empty keychain, read/write/rename failure, keyless provider, corrupt settings, legacy filename and restart persistence.
- [x] **DPCR-SEC-4.9** Verify synthetic tests never log secrets, use restrictive permissions, run keychain release workflow and focused protocol/provider suites on exact code head.

**Phase 4 exit:** S-1 and S-2 hold; no new incompatible invitation encoding needed.

## Phase 5 — History UI paging and async state ordering (DP-REVIEW-004,008)

- [x] **DPCR-UI-5.1** Update `HandHistoryScreen` to group live/archived/legacy sessions and show all retained histories; no current-session history may mask older saved sessions.
- [x] **DPCR-UI-5.2** Ensure per-session newest-first ordering and `slice(0, visibleCount)` or equivalent; initial 100 **newest** entries, later `Show older` loads genuinely older entries.
- [x] **DPCR-UI-5.3** Fix React keys to compound identity and refine badges, saved counts, labels, selection and accessible status text for V2 provenance.
- [x] **DPCR-UI-5.4** Replace existing reverse-paging expectations in `HandHistoryScreen.test.tsx`; add empty/100/101/205/two-session/legacy tests and verify bounded DOM.
- [x] **DPCR-UI-5.5** Add shared or per-hook monotonic refresh guards for `useLobbySession` and `MainTableScreen`, used by both event-triggered fetches and polling.
- [x] **DPCR-UI-5.6** Invalidate stale results at unmount/session change; keep a terminal session terminal, without preventing a genuinely new session identity from becoming active.
- [x] **DPCR-UI-5.7** Do not reset reconnect/error/backoff states on discarded results; ensure no listener/timer leaks or extra polling loops.
- [x] **DPCR-UI-5.8** Deferred-promise tests for A/B request inversions, success/failure interleave, terminal then stale connected, cleanup, replacement session, and continuing normal poll/updates.
- [x] **DPCR-UI-5.9** Run automated frontend accessibility/interaction regressions for history, lobby and table; manual screen-reader audit remains a separate release gate.

**Phase 5 exit:** H-6 and U-1 hold, no existing reconnect UX regression.

## Phase 6 — Documentation, backlog and release-gate reconciliation (DP-REVIEW-009)

- [x] **DPCR-DOC-6.1** Inspect `DP-UX-P1-001` ready state in source/tests and mark implemented/partial/open precisely, with test IDs.
- [x] **DPCR-DOC-6.2** Inspect `DP-UX-P1-003` NPC profile dirty-state and slug validation and mark precise implementation/verification scope.
- [x] **DPCR-DOC-6.3** Reconcile `DP-UX-P2-001` long history paging with corrected V2 grouping and test evidence.
- [x] **DPCR-DOC-6.4** Add DP-REVIEW-001..009 references and focused tests/status to `docs/DESKTOP_POKER_CURRENT_BACKLOG.md`; preserve earlier history and source spec.
- [x] **DPCR-DOC-6.5** Update README and architecture documentation for V2 history schema, V1 read-only migration, tournament grouping, profile lock and limitations.
- [x] **DPCR-DOC-6.6** Keep `docs/PHYSICAL_LAN_RELEASE_VALIDATION.md` physical two-machine gate **NOT RUN/BLOCKED** until actual physical-machine evidence; do not substitute loopback CI.
- [x] **DPCR-DOC-6.7** Keep Android mixed-runtime interop and live remote LLM-provider qualifications independently open/partial because no matching direct evidence exists.
- [ ] **DPCR-DOC-6.8** Audit logs/artifacts for confidential material and ensure no prohibited test fixtures/secrets are committed.

**Phase 6 exit:** The code and docs describe the same state; no unsupported PASS or release-ready claim.

## Phase 7 — Full integration, exact-head qualification and closure

- [x] **DPCR-QA-7.1** Re-read current master/spec/TODO/backlog at `ebfbfca` after the implementation commits; confirmed the current source and evidence docs match, with no duplicate/conflicting implementation. The original October 15 source review remains unavailable, as recorded under DPCR-000.1.
- [x] **DPCR-QA-7.2** Run `cargo fmt --all -- --check`; `cargo clippy --workspace --all-targets --all-features -- -D warnings`; `cargo test --workspace --all-targets --all-features -- --test-threads=2` and record counts/ignored tests.
- [x] **DPCR-QA-7.3** Run `npm ci`; `npm run format:check`; `npm run lint`; `npm run test`; `npm run build` under Node 24; record results.
- [x] **DPCR-QA-7.4** Complete relevant dependency audits, contract tests and canonical JSON checks without changing network protocol unnecessarily.
- [x] **DPCR-QA-7.5** Execute Linux real TCP join interruption/reconnect, history V1 migration, two-tournament full-game completion, final save + process restart, keychain and invitation security negative tests.
- [x] **DPCR-QA-7.6** Execute `.github/workflows/ci.yml`, `runtime-full-game-validation.yml`, `runtime-validation.yml` and applicable reconnect/release-package/keychain workflows on the **current executable code SHA**; poll to terminal state and inspect linked artifacts/results.
- [x] **DPCR-QA-7.7** Check persisted data does not lose hands, two profiles remain isolated, no phantom connected participants, no credential/log leak, terminal status is monotonic, and no regression in observer/private cards.
- [x] **DPCR-QA-7.8** Record passing run URLs, actual SHA, proof artifacts, limitations, and excluded hardware/manual gates for every fixed finding.
- [x] **DPCR-QA-7.9** Reconciled checklist items and all nine review statuses against the remediation spec, current source/tests, and exact-head CI. DP-REVIEW-009 remains explicitly partial with its evidence and dependencies recorded; the missing original-review comparison remains open under DPCR-000.1.
- [ ] **DPCR-QA-7.10** Update final backlog and sign off the remediation as software-qualified only if all required code, migration, tests and exact-head CI pass; do not publish a GitHub Release without separately satisfying the intended release acceptance.

## October 15 Ralph Bridge execution evidence (rolling; not sign-off)

- **Baseline examined:** `master` `41022ed8b495ead8dcb909730d1fb5a374efa557` (documentation-only successor to tested executable `bda5515a07971c2a8832e96fba7a2c2a54c1efb1`). Ralph Bridge 0.6.2, build `0acea4b23362e2ccecab8f6c9db4418b162426cf`, authenticated through GitHub App.
- **Reconciliation / DPCR-000.2:** Ralph Bridge branch inventory at baseline had `master`, `agent/release-readiness-baseline`, `agent/runtime-hardening-code-review`, and `chatgpt/fix-durable-hand-history`; no open PRs attached to any listed branch. The authoritative TODO was unchanged and entirely unchecked at inspection. No per-task branch or PR was created.
- **Prior executable qualification:** all eight non-main runtime workflows at `bda5515a07971c2a8832e96fba7a2c2a54c1efb1` passed (full game `38843947115`, runtime `38843947132`, reconnect `38843947108`, keychain `38843947051`, AppImage `38843947152`, Debian package `38843947189`, rule-based NPC `38843947097`, embedded-model `38843947133`). Main CI `38843947106` **FAILED** frontend formatting: `src/app/persistence{,.test}.ts`, `src/screens/HandHistoryScreen{,.test}.tsx`, `src/screens/NpcProfilesScreen.test.tsx`, and `src/screens/useLobbySession.test.ts`. Rust tests in that run: 552 passed, 3 ignored, and 131 poker-core tests passed. These prior runs are **not** an exact-head pass for later commits.
- **Verified source/test inventory (not equivalent to test execution at this head):** `networking/runtime/tests/admission_rollback.rs` already exercises initial-join snapshot construction failure, reconnect snapshot failure, and unregistered-session cleanup. `history_storage_v2.rs` contains tournament collision, legacy migration, concurrent merge, corrupt data preservation, idempotence and profile isolation tests. `join_payload.rs` contains compact input, compressed data and gzip expansion bounds. `HandHistoryScreen.test.tsx` contains 100/101/205 pagination and two-session cases.
- **New gap-focused commit:** `8062292cf99f0f2c31cf2cea63a5bfe053997c53` adds `stale_join_cleanup_does_not_remove_newer_connection`, exercising N-3's conditional rollback. CI `38847304763` failed `cargo fmt --check` on the new test; its formatting was corrected in `0da2fda5d5a1720e8f26abe21788cbf41be0024e`. Do not treat that earlier run as a test pass. The reconnect same-token attempt generation / winner-preservation case still requires distinct verification; this new test does not qualify N-3 by itself.
- **Baseline posture:** CI red and multiple checklist items unverified; software remediation **IN PROGRESS**. Do not mark DP-REVIEW-001..009 PASS without completed exact-head evidence. Physical two-machine LAN, Android mixed-runtime, remote LLM and manual accessibility remain independent unqualified gates; no release publication authorized.

## October 16 evidence-publisher conflict

- Exact executable SHA `a5e6d12512891aa5fe9e3eef701fa0b0849a3385`: Linux runtime build and both smoke scenarios passed, but workflow `38847846526` failed when rebasing a generated latest-result documentation commit over concurrent master updates. This is a publisher failure, not runtime failure; retain the failed CI conclusion until rerun and repair.

## October 16 focused regression and formatting remediation

- **Frontend formatting repair:** commit `a5e6d12512891aa5fe9e3eef701fa0b0849a3385` applied the exact Prettier diff from the transient read-only diagnostic run [38847603576](https://github.com/ekkus93/desktop_poker/actions/runs/38847603576), to the six files identified in main CI 38843947106. The diagnostic workflow was removed afterward; the formatter did not independently push code. Its main CI [38847846478](https://github.com/ekkus93/desktop_poker/actions/runs/38847846478) was **CANCELLED** when newer changes advanced master, so formatting requires a completed later CI run.
- **Runtime proof at `a5e6d125`:** reconnect [38847846482](https://github.com/ekkus93/desktop_poker/actions/runs/38847846482), keychain [38847846457](https://github.com/ekkus93/desktop_poker/actions/runs/38847846457), rule-based tournament [38847846443](https://github.com/ekkus93/desktop_poker/actions/runs/38847846443), Debian [38847846464](https://github.com/ekkus93/desktop_poker/actions/runs/38847846464), embedded [38847846442](https://github.com/ekkus93/desktop_poker/actions/runs/38847846442), AppImage [38847846511](https://github.com/ekkus93/desktop_poker/actions/runs/38847846511), and full game [38847846452](https://github.com/ekkus93/desktop_poker/actions/runs/38847846452) succeeded. The regular release runtime [38847846526](https://github.com/ekkus93/desktop_poker/actions/runs/38847846526) built and passed single/multi-instance smokes but concluded failure solely because its final generated-evidence push conflicted with ongoing master changes.
- **History fault tests:** commit `9416a68e18fe3f2681885a21e62629e8f2030767` adds `history_profile_lock_preserves_existing_lock_file_contents` and `failed_history_lock_acquisition_preserves_existing_v2_bytes`, requiring actual exact-head Rust test execution before marking DPCR-IO-3.7 completed.
- **Workflow repair:** commit `002f76041c6d88f0190adf6e0d15fd02ab695a95` switches runtime result publication to existing `scripts/push_runtime_evidence.sh`, which fetches/rebases/retries concurrent master updates and uses a nonrecursive evidence-only commit. Main CI [38848772935](https://github.com/ekkus93/desktop_poker/actions/runs/38848772935) and runtime [38848773003](https://github.com/ekkus93/desktop_poker/actions/runs/38848773003) were **PENDING/RUNNING** when this note was made.
- **Remaining:** complete exact-head CI, review full-game persistence/restart and V1+V2 migration fixtures, exercise transactional admission failure paths and generation ownership, and reconcile all unchecked TODO and DP-REVIEW rows. Do not infer physical LAN, Android, remote-provider or manual accessibility qualification from Linux loopback workflows.

## October 16 Ralph loop — current qualification

- **Source review prerequisite:** `DESKTOP_POKER_CODE_REVIEW_2026-10-15.md` is absent from the checkout and tracked docs inventory. Ralph Bridge `get_file` on the expected repository path returned `INVALID_ARGUMENT`. The companion remediation specification and current backlog were reviewed; direct item-by-item comparison to the original source review still needs that file.
- **Current executable SHA:** `88489f26252c9a32e41e9611695d8b7cdb4a46d1` (`fix: synchronize snapshots during client admission`), descendant of privacy fix `568da27`. It closes the snapshot-to-registration public-event gap using transition-lock sequencing and a writer-serialized catch-up snapshot; adds real-TCP before-snapshot disconnect/retry coverage; and includes the runtime evidence privacy protections described below.
- **Network regressions:** `peer_disconnected_before_initial_snapshot_can_retry_without_phantom_or_slot_leak` (four pre-snapshot disconnect attempts, rollback, slot recovery, same-identity retry) and `admission_snapshot_sync_covers_public_event_before_registration` (stale initial snapshot followed by current catch-up snapshot) pass. `abrupt_disconnects_after_join_request_leave_no_connected_phantom_or_slot_leak` adds repeated capacity recovery and fresh-client admission.
- **Local Rust verification at 88489f2:** `cargo test --workspace --all-targets --all-features -- --test-threads=2` passed; exact main-CI counts were 583 Tauri tests passed, 2 ignored, and 131 poker-core tests passed. `cargo fmt --all -- --check`, `cargo clippy --workspace --all-targets --all-features -- -D warnings`, and `git diff --check` passed.
- **Live local Ollama check:** with the available `qwen3.5:9b` model, `OLLAMA_TEST_MODEL=qwen3.5:9b cargo test -p desktop-poker --lib ollama_live_tests -- --ignored --test-threads=1` passed both preflop and postflop legal-action tests. This is supplemental local Ollama evidence; it does not qualify remote provider release scenarios.
- **Focused privacy check:** `python -m unittest discover -s scripts -p 'test_runtime_evidence.py' -v` passed 3 tests. `py_compile` passed for the helper, tests, and changed runtime harnesses. A scan of tracked `docs/runtime-validation/*.json` finds no `holeCards`, `privateHoleCards`, local/private projection/state, or `actionTray` fields. The previously stored reconnect result was redacted in `568da27`; older CI artifacts remain a separate removal task.
- **Exact-head CI at 88489f2:** all nine workflows passed: CI [38890639601](https://github.com/ekkus93/desktop_poker/actions/runs/38890639601), runtime [38890639587](https://github.com/ekkus93/desktop_poker/actions/runs/38890639587), full game [38890639467](https://github.com/ekkus93/desktop_poker/actions/runs/38890639467), reconnect [38890639602](https://github.com/ekkus93/desktop_poker/actions/runs/38890639602), keychain [38890639554](https://github.com/ekkus93/desktop_poker/actions/runs/38890639554), Debian package [38890639497](https://github.com/ekkus93/desktop_poker/actions/runs/38890639497), AppImage [38890639588](https://github.com/ekkus93/desktop_poker/actions/runs/38890639588), rule-based NPC [38890639505](https://github.com/ekkus93/desktop_poker/actions/runs/38890639505), and embedded model/tournament [38890639563](https://github.com/ekkus93/desktop_poker/actions/runs/38890639563). Every workflow concluded success. Evidence artifacts include CI validation `12041298525`, full game `12042480265`, and reconnect `12042390858`.
- **Documentation-head CI:** the evidence checklist/backlog commit `9739c0e56f129834f16a8d9e165a8c2a5d3ddc11` passed main CI [38891364725](https://github.com/ekkus93/desktop_poker/actions/runs/38891364725); its publisher recorded current results in `6394f58d107be77567c8c382b07ac1ece59e9afd`.
- **Exact-head CI at executable parent 3dcf25a:** all nine workflows passed, including keychain [38885326304](https://github.com/ekkus93/desktop_poker/actions/runs/38885326304), Debian package [38885326193](https://github.com/ekkus93/desktop_poker/actions/runs/38885326193), and AppImage [38885326255](https://github.com/ekkus93/desktop_poker/actions/runs/38885326255). The privacy commit changes only runtime evidence handling and its tracked artifact; application, keychain and packaging code did not change.
- **External artifact cleanup still required:** pre-fix workflow artifacts remain available until 2026-11-15, and Ralph Bridge has no artifact-delete operation. Known gameplay artifacts containing active-hand captures or raw table data: reconnect runs `38883558839` (`12038731013`), `38884571927` (`12038897167`), `38885326343` (`12039761711`); rule-based runs `38883558711` (`12039340271`), `38884571954` (`12039087454`), `38885326320` (`12038788421`); embedded tournament runs `38883558624` (`12038024864`), `38884571674` (`12039217712`), `38885326213` (`12039272638`). Remove these through GitHub Actions artifact controls before closing DPCR-DOC-6.8. The small embedded-model decision artifacts and history-only full-game screenshots are not in this affected list.
- **Network capacity policy:** preserve the existing rule implemented by `counted_capacity` and tested by `reconnect_eligible_disconnected_participants_count_toward_join_capacity`: reconnect-eligible participants retain their capacity until explicit removal. Token expiry only rejects reconnect and does not automatically remove a participant. This remediation does not change that policy.
- **Remaining independent release gates:** physical two-machine LAN, Android mixed-runtime, remote LLM-provider scenarios, and manual screen-reader audit remain unqualified.

## Required outcome and evidence table

Fill these cells **after** verified implementation; blanks are intentional. Never invent hashes or run IDs.

| Review item | Implementation SHA(s) | Focused regression | Exact-head CI / runtime run | Result |
|---|---|---|---|---|
| DP-REVIEW-001 | `2e98e04`; `9ac6077`; `88489f2` | Pre-snapshot disconnect/retry, gap catch-up snapshot, rollback, frame and reconnect-capacity tests | All nine workflows passed at `88489f2` | IMPLEMENTED; existing reconnect-capacity policy preserved |
| DP-REVIEW-002 | `88489f2` and history implementation ancestors | compound identity/restart/migration fixtures in `history_storage_v2.rs` | CI `38890639601`, full game `38890639467` passed | IMPLEMENTED; see history acceptance notes |
| DP-REVIEW-003 | `88489f2` and persistence implementation ancestors | durable precedence and durable-save gate tests | CI `38890639601`, full game `38890639467` passed | IMPLEMENTED |
| DP-REVIEW-004 | `88489f2` and history UI implementation ancestors | 100/101/205 and multi-session tests in `HandHistoryScreen.test.tsx` | CI `38890639601` passed | IMPLEMENTED |
| DP-REVIEW-005 | `88489f2` and join-payload implementation ancestors | bounded compact/legacy/gzip negative tests in `join_payload.rs` | CI `38890639601` passed | IMPLEMENTED |
| DP-REVIEW-006 | `88489f2` and provider-migration implementation ancestors | key migration/redaction tests; keychain `38890639554` passed | CI `38890639601`; keychain `38890639554` passed | IMPLEMENTED; historical artifact removal separate |
| DP-REVIEW-007 | `88489f2` and history transaction implementation ancestors | process-level writer, fault recovery and profile isolation tests | CI `38890639601` passed | IMPLEMENTED |
| DP-REVIEW-008 | `88489f2` and frontend refresh implementation ancestors | deferred inversion/terminal-state tests in lobby/table suites | CI `38890639601` passed | IMPLEMENTED |
| DP-REVIEW-009 | `88489f2` | backlog/source reconciliation; privacy checklist still awaits old artifact removal | CI `38890639601` passed | PARTIAL |

## External release dependencies — tracked, not silently folded into this TODO

- **Physical LAN:** Real two-machine matching release artifacts, gameplay, reconnect/failure evidence per `docs/PHYSICAL_LAN_RELEASE_VALIDATION.md`. Remains a separate release gate, even if DPCR tasks all pass.
- **Android/Desktop:** Canonical event payload comparison and mixed-runtime acceptance remain independent milestones.
- **Remote LLM providers:** Per-provider accepted-action/timeout/invalid-response/negative-case matrix remains independent of rule-based and embedded-GGUF proof.
- **Manual accessibility:** Focus and screen-reader audit remains independent if no human/manual proof available.
- **Release publication:** A GitHub Release is neither requested nor authorized by a planning/checklist document alone.

## Final acceptance declaration (leave blank until proven)

- Final master SHA: `____________________________`
- Exact-head CI run URLs: `____________________________`
- Final Linux runtime artifacts: `____________________________`
- Storage migration/version evidence: `____________________________`
- Network fault-injection/real TCP evidence: `____________________________`
- Remaining independent release gates: `____________________________`
- Result: `NOT STARTED` / `IN PROGRESS` / `SOFTWARE QUALIFIED` / `BLOCKED (specific user prerequisite)`
