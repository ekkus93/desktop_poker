# Desktop Poker — Code Review Remediation TODO

**Status:** Proposed; unchecked boxes mean **not verified**, not necessarily missing in a later checkout.  
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
- [ ] **DPCR-000.2** Inspect open branches/PRs, active Ralph sessions/CI, completed and running workflow runs, recent work and consolidated current backlog; reconcile prior/duplicate fixes before proceeding.
- [ ] **DPCR-000.3** Confirm `cargo`, Rust stable/rustfmt/clippy, Node 24, npm, Tauri Linux dependencies and test runner availability; record baseline versions.
- [ ] **DPCR-000.4** Run baseline Rust tests, Clippy, formatting, frontend tests/lint/format/build and existing CI-parity commands; note ignored/flaky tests and link exact SHA.
- [ ] **DPCR-000.5** Add focused *failing* repro for failed post-admission snapshot send or injectable post-accept failure; verify phantom/cleanup invariant.
- [ ] **DPCR-000.6** Add focused failing fixture with two tournaments both containing hand 1 and a restart; verify global-hand collision on old schema.
- [ ] **DPCR-000.7** Add failing fixture with durable newer hand and stale cache older same hand; show incorrect overwrite precedence.
- [ ] **DPCR-000.8** Confirm existing 205-hand UI test asserts reverse history paging; document test-oracle correction needed.
- [ ] **DPCR-000.9** Add bounded decode/legacy-secret/concurrent-writer/out-of-order-request reproducers where deterministic; label static risks not runtime-proven until reproduced.
- [ ] **DPCR-000.10** Record baseline in TODO/evidence without raw secrets, join tokens, hole cards or private settings.

**Phase 0 exit:** Baseline SHA and executable regression plan recorded; no duplicate fixes started.

## Phase 1 — Network admission transactional cleanup (DP-REVIEW-001; High)

- [ ] **DPCR-NET-1.1** Trace both `handle_join_request` and `handle_reconnect_request`, `handle_initial_client_request`, host accept, `spawn_host_client_session`, registration and all post-accept returns.
- [ ] **DPCR-NET-1.2** Document lock order and choose per-attempt provisional state/rollback guard plus unique attempt generation; do not hold global state locks during blocking socket I/O.
- [ ] **DPCR-NET-1.3** Capture and restore pre-attempt reconnect state and tokens/expiry, and remove only an uncommitted initial join; rollback is conditional on attempt ownership.
- [ ] **DPCR-NET-1.4** Close failed transport, free pending and active-client slot guards and emit categorized/redacted diagnostics for **every** failure after admission.
- [ ] **DPCR-NET-1.5** Audit and, if necessary, expose errors from `spawn_host_client_session`; transfer cleanup responsibility exactly once after registration.
- [ ] **DPCR-NET-1.6** Provide deterministic injection points for snapshot build/send, timeout clear, stream clone, registry lock/insert and session spawn; avoid fragile sleeps for core unit tests.
- [ ] **DPCR-NET-1.7** Test abrupt disconnect before initial snapshot, repeated retries, seat/capacity recovery and absence of connected phantom in authoritative state.
- [ ] **DPCR-NET-1.8** Test failed reconnect snapshot delivery followed by later valid reconnect; preserve correct key/token/expiry/phase rules.
- [ ] **DPCR-NET-1.9** Test concurrent same-ID join/reconnect, already-connected rejection, and that loser cleanup never disrupts winner.
- [ ] **DPCR-NET-1.10** Rerun `networking/runtime/tests/{join,reconnect,reconnect_expiry,host_shutdown}.rs` and relevant integration reconnect matrix; no leaked secret/private state, no lock deadlock.
- [ ] **DPCR-NET-1.11** Commit coherent network fix directly to master when authorized; record exact-head CI/focused TCP proof.

**Phase 1 exit:** N-1 through N-4 hold under success and injected failure conditions.

## Phase 2 — One reviewed V2 history contract (DP-REVIEW-002/003/007; High)

- [ ] **DPCR-HIST-2.1** Inventory all history writers/readers/mergers: Rust commands, `DesktopShellProvider`, `MainTableScreen`, `TournamentCompleteScreen`, `HandHistoryScreen`, `src/app/persistence.ts`, storage fixtures and integration scripts.
- [ ] **DPCR-HIST-2.2** Define, write tests for, and freeze V2 JSON schema (`schemaVersion:2`), typed live tuple `(tableId,sessionEpoch,handNumber)`, optional display metadata and explicit legacy-origin union; reject malformed identities.
- [ ] **DPCR-HIST-2.3** Add authoritative `sessionEpoch` to Rust `TableViewSnapshot` and frontend `TableViewSnapshot`, ensuring existing session status, observer, debug fixtures, and DTO tests agree; do not infer from `tableId` string.
- [ ] **DPCR-HIST-2.4** Implement V2 Rust serialization and validation in `history_storage.rs`; place V2 in `hand-history.v2.json` and preserve V1 `hand-history.json` byte-for-byte.
- [ ] **DPCR-HIST-2.5** Replace hand-number-only normalization/merge with deterministic compound identity and descending per-tournament ordering. Verify the same hand number in two sessions survives.
- [ ] **DPCR-HIST-2.6** Add V2-specific Tauri read/merge commands (or reviewed version-safe equivalent); enforce schema at backend trust boundary; add command contract tests.
- [ ] **DPCR-HIST-2.7** Implement one-time, crash-recoverable V1 disk import into labeled legacy group(s); persist import marker/fingerprint. Ensure repeated reads/restarts do not duplicate or rewrite.
- [ ] **DPCR-HIST-2.8** On missing/invalid/unsupported V1/V2, retain original bytes, distinguish missing from invalid, fail non-destructively and surface useful errors.
- [ ] **DPCR-HIST-2.9** Implement explicit durable-source precedence over cache for matching identity; passive load must not issue a destructive merge/write.
- [ ] **DPCR-HIST-2.10** Implement one-time V1 cached-data import; missing entries may be added, conflicting/ambiguous data stays labeled by provenance, never falsely assigned to current table. Preserve cache until V2 durability verified.
- [ ] **DPCR-HIST-2.11** Change TS validation, normalization, caching, and history callbacks to carry session identity end-to-end; prohibit global `handNumber` dedupe.
- [ ] **DPCR-HIST-2.12** Preserve final-hand durable acknowledgment and error handling. A cache-only write does not produce `Final history saved.`
- [ ] **DPCR-HIST-2.13** Test two tournaments each with hand 1+, two profile namespaces, process restart, legacy V1 imports, duplicate updates within same tournament, stale cache and corrupt-file recovery.
- [ ] **DPCR-HIST-2.14** Test upgrading from V1, rolling back to old binary without deleting V2, and re-upgrading without losing V2/new data; document downgrade visibility caveat.

**Phase 2 exit:** H-1, H-2, H-3 and H-5 hold with green Rust/TS tests and restart evidence. Complete Phase 3 before calling history integrity fully resolved.

## Phase 3 — History transactions and unique temporaries (DP-REVIEW-007; Medium)

- [ ] **DPCR-IO-3.1** Decide shared-profile process policy; where same-profile multi-process writes are possible, implement OS-backed interprocess locking or enforce single-writer ownership with a visible error.
- [ ] **DPCR-IO-3.2** Put Rust read–modify–merge–write under one scoped per-profile transaction lock; avoid deadlock with UI/network state locks.
- [ ] **DPCR-IO-3.3** Replace PID+millisecond temporary name with unpredictable unique create-new path; retain file sync, replace, directory sync and temporary cleanup.
- [ ] **DPCR-IO-3.4** Audit Windows backup/replace behavior for recovery (without claiming Windows full runtime support).
- [ ] **DPCR-IO-3.5** Prevent no-op read from rewriting/updating timestamp. Ensure idempotent merging of identical authoritative data.
- [ ] **DPCR-IO-3.6** Concurrent-write test with at least two disjoint batches targeting the same profile; final V2 contains union, no overwrite/lost updates. Add process-level variant if same profile may have multiple processes.
- [ ] **DPCR-IO-3.7** Fault inject create/write/fsync/rename/lock/backup failures; verify earlier file remains readable and no success acknowledgment is sent.
- [ ] **DPCR-IO-3.8** Verify file permissions and cleanup; run history storage unit/integration tests and commit with exact-head proof.

**Phase 3 exit:** H-4 holds for the supported profile/process model.

## Phase 4 — Invitation and secret security hardening (DP-REVIEW-005,006; Medium)

- [ ] **DPCR-SEC-4.1** Bound entire compact invite to 16 KiB and decoded gzip bytes to 12 KiB, before dangerous allocation.
- [ ] **DPCR-SEC-4.2** Bound decompressed compact JSON to 64 KiB using bounded reader (+1 overflow sentinel); bound legacy raw JSON to 64 KiB before parse.
- [ ] **DPCR-SEC-4.3** Preserve valid compact/legacy invitation round-trips and clear non-secret rejection messages; reject malformed base64, gzip, UTF-8 and oversized input.
- [ ] **DPCR-SEC-4.4** Add compact expansion/bomb test, under/at/over-limit fixtures and log-redaction assertions; no crash or unbounded allocation.
- [ ] **DPCR-SEC-4.5** In provider settings migration, detect embedded `apiKey` even if `read_key()` already yields `Some`; prefer current secure-store key, then scrub embedded key atomically.
- [ ] **DPCR-SEC-4.6** For secure-store-empty, write/verify the embedded key before scrubbing; on secure read/write error or sanitization failure, show error and retain recoverable bytes.
- [ ] **DPCR-SEC-4.7** Review and complete one-time migration/retirement of `claude-api-key.txt`; never silently reintroduce plaintext in release; document debug secret-store exception.
- [ ] **DPCR-SEC-4.8** Write tests for pre-existing key, differing legacy key, empty keychain, read/write/rename failure, keyless provider, corrupt settings, legacy filename and restart persistence.
- [ ] **DPCR-SEC-4.9** Verify synthetic tests never log secrets, use restrictive permissions, run keychain release workflow and focused protocol/provider suites on exact code head.

**Phase 4 exit:** S-1 and S-2 hold; no new incompatible invitation encoding needed.

## Phase 5 — History UI paging and async state ordering (DP-REVIEW-004,008)

- [ ] **DPCR-UI-5.1** Update `HandHistoryScreen` to group live/archived/legacy sessions and show all retained histories; no current-session history may mask older saved sessions.
- [ ] **DPCR-UI-5.2** Ensure per-session newest-first ordering and `slice(0, visibleCount)` or equivalent; initial 100 **newest** entries, later `Show older` loads genuinely older entries.
- [ ] **DPCR-UI-5.3** Fix React keys to compound identity and refine badges, saved counts, labels, selection and accessible status text for V2 provenance.
- [ ] **DPCR-UI-5.4** Replace existing reverse-paging expectations in `HandHistoryScreen.test.tsx`; add empty/100/101/205/two-session/legacy tests and verify bounded DOM.
- [ ] **DPCR-UI-5.5** Add shared or per-hook monotonic refresh guards for `useLobbySession` and `MainTableScreen`, used by both event-triggered fetches and polling.
- [ ] **DPCR-UI-5.6** Invalidate stale results at unmount/session change; keep a terminal session terminal, without preventing a genuinely new session identity from becoming active.
- [ ] **DPCR-UI-5.7** Do not reset reconnect/error/backoff states on discarded results; ensure no listener/timer leaks or extra polling loops.
- [ ] **DPCR-UI-5.8** Deferred-promise tests for A/B request inversions, success/failure interleave, terminal then stale connected, cleanup, replacement session, and continuing normal poll/updates.
- [ ] **DPCR-UI-5.9** Run frontend accessibility/interaction regressions for history, lobby and table; commit and record evidence.

**Phase 5 exit:** H-6 and U-1 hold, no existing reconnect UX regression.

## Phase 6 — Documentation, backlog and release-gate reconciliation (DP-REVIEW-009)

- [ ] **DPCR-DOC-6.1** Inspect `DP-UX-P1-001` ready state in source/tests and mark implemented/partial/open precisely, with test IDs.
- [ ] **DPCR-DOC-6.2** Inspect `DP-UX-P1-003` NPC profile dirty-state and slug validation and mark precise implementation/verification scope.
- [ ] **DPCR-DOC-6.3** Reconcile `DP-UX-P2-001` long history paging with corrected V2 grouping and test evidence.
- [ ] **DPCR-DOC-6.4** Add DP-REVIEW-001..009 references, specific fixed commits and tests to `docs/DESKTOP_POKER_CURRENT_BACKLOG.md`; do not erase history or source spec.
- [ ] **DPCR-DOC-6.5** Update readme/architecture/persistence/release notes as needed for V2 history schema, V1 read-only migration, different tournament grouping, profile lock and limitations.
- [ ] **DPCR-DOC-6.6** Keep `docs/PHYSICAL_LAN_RELEASE_VALIDATION.md` physical two-machine gate **NOT RUN/BLOCKED** until actual physical-machine evidence; do not substitute loopback CI.
- [ ] **DPCR-DOC-6.7** Keep Android mixed-runtime interop and live remote LLM-provider qualifications independently open/partial unless corresponding direct evidence exists.
- [ ] **DPCR-DOC-6.8** Audit logs/artifacts for confidential material and ensure no prohibited test fixtures/secrets are committed.

**Phase 6 exit:** The code and docs describe the same state; no unsupported PASS or release-ready claim.

## Phase 7 — Full integration, exact-head qualification and closure

- [ ] **DPCR-QA-7.1** Re-read current master/spec/TODO/backlog after all merges/commits and confirm no duplicate/conflicting implementations.
- [ ] **DPCR-QA-7.2** Run `cargo fmt --all -- --check`; `cargo clippy --workspace --all-targets --all-features -- -D warnings`; `cargo test --workspace --all-targets --all-features -- --test-threads=2` and record counts/ignored tests.
- [ ] **DPCR-QA-7.3** Run `npm ci`; `npm run format:check`; `npm run lint`; `npm run test`; `npm run build` under Node 24; record results.
- [ ] **DPCR-QA-7.4** Complete relevant dependency audits, contract tests and canonical JSON checks without changing network protocol unnecessarily.
- [ ] **DPCR-QA-7.5** Execute Linux real TCP join interruption/reconnect, history V1 migration, two-tournament full-game completion, final save + process restart, keychain and invitation security negative tests.
- [ ] **DPCR-QA-7.6** Execute `.github/workflows/ci.yml`, `runtime-full-game-validation.yml`, `runtime-validation.yml` and applicable reconnect/release-package/keychain workflows on the **current executable code SHA**; poll to terminal state and inspect linked artifacts/results.
- [ ] **DPCR-QA-7.7** Check persisted data does not lose hands, two profiles remain isolated, no phantom connected participants, no credential/log leak, terminal status is monotonic, and no regression in observer/private cards.
- [ ] **DPCR-QA-7.8** Record passing run URLs, actual SHA, proof artifacts, limitations, and excluded hardware/manual gates for every fixed finding.
- [ ] **DPCR-QA-7.9** Reconcile each checklist item and nine review statuses with independently supported evidence; do not tick an untested branch or block entire software remediation on unrelated future Android work.
- [ ] **DPCR-QA-7.10** Update final backlog and sign off the remediation as software-qualified only if all required code, migration, tests and exact-head CI pass; do not publish a GitHub Release without separately satisfying the intended release acceptance.

## Required outcome and evidence table

Fill these cells **after** verified implementation; blanks are intentional. Never invent hashes or run IDs.

| Review item | Implementation SHA(s) | Focused regression | Exact-head CI / runtime run | Result |
|---|---|---|---|---|
| DP-REVIEW-001 | — | — | — | OPEN |
| DP-REVIEW-002 | — | — | — | OPEN |
| DP-REVIEW-003 | — | — | — | OPEN |
| DP-REVIEW-004 | — | — | — | OPEN |
| DP-REVIEW-005 | — | — | — | OPEN |
| DP-REVIEW-006 | — | — | — | OPEN |
| DP-REVIEW-007 | — | — | — | OPEN |
| DP-REVIEW-008 | — | — | — | OPEN |
| DP-REVIEW-009 | — | — | — | OPEN |

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
