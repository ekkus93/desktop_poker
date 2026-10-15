# Desktop Poker — Code Review Remediation Specification

**Status:** Proposed / not implemented  
**Date:** 2026-10-15  
**Repository:** `https://github.com/ekkus93/desktop_poker`  
**Reviewed baseline:** `master` `fb7452e9ab74250bbc5b47283692ea66f35ac760`  
**Companion implementation checklist:** `docs/DESKTOP_POKER_CODE_REVIEW_REMEDIATION_TODO_2026-10-15.md`  
**Evidence source:** `DESKTOP_POKER_CODE_REVIEW_2026-10-15.md` (October 15 review, separate attachment)

> **Authority and freshness:** This document is a proposed remediation design for the verified snapshot, not proof that an issue is still present on a future `master`. Before implementation, inspect the exact current head and reconcile any intervening fixes; do not overwrite more recent work. Do not mark an item complete without corresponding tests and exact-head evidence.

## 1. Mission, scope, and non-goals

Fix the nine findings from the October 15, 2026 source review with minimal regression risk, preserving the existing Rust-authoritative game engine, signed/encrypted protocol, per-profile storage isolation, Tauri/React architecture, and existing live gameplay. Priority is **silent data loss and phantom connections first**, then security and UI correctness. Each change must be accompanied by focused automated regressions and existing CI qualification.

### In scope

| ID | Severity | Remediation | Primary owner paths |
|---|---|---|---|
| DP-REVIEW-001 | High | Transactional initial join/reconnect and failure cleanup | `src-tauri/src/networking/runtime/{host,handlers}.rs` |
| DP-REVIEW-002 | High | Tournament-scoped persisted hand identities | `src-tauri/src/history_storage.rs`, `src-tauri/src/app_state/{mod,projection}.rs`, `src/api/desktop.ts`, frontend persistence and screens |
| DP-REVIEW-003 | High | Durable history authoritative over stale cache | `src/app/persistence.ts`, Rust storage |
| DP-REVIEW-004 | Medium | Correct newest-first paging and grouping | `src/screens/HandHistoryScreen.tsx` |
| DP-REVIEW-005 | Medium | Strict compact/legacy invitation decode limits | `src-tauri/src/protocol/join_payload.rs` |
| DP-REVIEW-006 | Medium | Eliminate plaintext legacy provider keys after safe migration | `src-tauri/src/npc/provider_storage.rs` |
| DP-REVIEW-007 | Medium | Concurrent history transaction safety and unique temporary files | `src-tauri/src/history_storage.rs` |
| DP-REVIEW-008 | Medium/Low | Ignore out-of-order frontend snapshots | `src/screens/useLobbySession.ts`, `src/screens/MainTableScreen.tsx` |
| DP-REVIEW-009 | Low | Reconcile source, backlog, tests, and release documentation | `docs/DESKTOP_POKER_CURRENT_BACKLOG.md` and related docs |

### Explicit non-goals

- No rewrite of Hold'em evaluation, betting, cryptography, Tauri architecture, or working connection protocol.
- No Android/UniFFI feature implementation as part of this bug-fix set; preserve compatibility and track its existing gate separately.
- No claim that CI's two-instance, same-machine test qualifies two **physical** LAN hosts.
- No claim that all remote LLM providers are qualified because rule-based or embedded GGUF works.
- No broad user-interface redesign, telemetry service, network relay, cloud sync, or account system.
- No automatic GitHub push or master mutation merely from creating these planning documents.

## 2. Baseline, safety rules, and delivery policy

1. Before each development session, inspect `master` HEAD, branch/PR state, relevant running/exact-head CI, `docs/DESKTOP_POKER_CURRENT_BACKLOG.md`, this spec, and the TODO. Detect any later implementation to avoid duplicated or conflicting fixes.
2. The verified review baseline is `fb7452e9ab74250bbc5b47283692ea66f35ac760`. Capture the new baseline SHA when work actually begins and use *that* exact SHA for edits, evidence, and optimistic-concurrency checks.
3. Follow the repository owner's established workflow when authorized to work on GitHub: use **Ralph Bridge only** for GitHub and CI access, not the default GitHub tool; work directly on `master` rather than creating a PR/branch for each subtask. Inspect and reconcile existing work before new commits. Never force-push or overwrite a changed head.
4. Make small, independently testable commits grouped by coherent subsystem; avoid combining network, storage, and provider changes in one unreviewable diff. Run required tests after each group and exact-head CI after integration. A timeout/temporary tool failure is not evidence of an implementation defect; retry/poll, retain logs and report concrete limitations.
5. Preserve legacy persisted files. Never silently replace malformed data with empty state. Where a migration or disk write fails, show an actionable non-secret error and leave recoverable originals intact.
6. No test evidence may include live invitation secrets, reconnect tokens, provider API keys, full private hole cards, or raw personal settings. Use synthetic fixtures, redact logs, restrict permissions, and state exact artifact/SHA provenance.
7. Keep success claims narrow: distinguish code inspection, deterministic tests, running Tauri tests, release artifact tests, actual hardware tests, and manual UX/a11y checks.

## 3. System invariants (normative)

### Networking

- **N-1:** No participant has `Connected` authoritative state without a matching registered active transport, except for a tightly scoped, unpublished provisional phase that can always be rolled back.
- **N-2:** A join/reconnect request that fails at any stage (validation, snapshot build, write, timeout transition, stream clone, client registration, session spawn) cannot consume a permanent seat/client slot or strand reconnect state.
- **N-3:** A rollback never removes or downgrades a newer successful connection or a different player session. Concurrent joins for the same player are deterministic and single-winner.
- **N-4:** Connection cleanup maintains existing signing, encryption, token, expiry, replay, seat, observer, phase, and terminal-disconnect invariants. No remote socket write while holding broad authoritative or transition locks.

### History

- **H-1:** Every *new* persisted hand has a stable tournament identifier plus positive hand number; deduplicate only within a tournament, never globally on hand number.
- **H-2:** A saved hand is not lost when a different tournament starts with hand 1, when the application restarts, or when an older snapshot arrives.
- **H-3:** Durable V2 storage is authoritative for an existing hand identity. A stale browser cache cannot replace it; cache migration must be explicit, one-way and idempotent.
- **H-4:** Concurrent writers cannot drop previously committed hands. A failed write or corrupt file preserves all prior recoverable bytes. Acknowledged save means durable persistence succeeded, not just localStorage mutation.
- **H-5:** Imported old-format histories remain readable without falsely attributing them to a newly created tournament. Migration never invents unavailable historical table/session identity.
- **H-6:** Within a tournament, hand numbers are descending in the history UI; initial visible records are the *newest* records. Loading older records does not duplicate or reorder existing ones.

### Security/UI

- **S-1:** Decoding untrusted invitation text performs bounded work and memory allocation before parsing/acceptance, for compact gzip and legacy JSON formats.
- **S-2:** When API keys have been safely migrated to the configured secure store, no old plaintext key remains in the settings JSON; keychain failure must never destroy the only recoverable copy.
- **U-1:** Old asynchronous refresh results cannot roll the lobby/table UI back to an earlier request for the same session, nor reopen a terminated session. Request failures preserve truthful connectivity state.
- **Q-1:** Every resolved review item has a reproducible test or objectively reviewable non-automated evidence and updated backlog status.

## 4. DP-REVIEW-001 — Transactional join/reconnect admission

**Observed source:** `src-tauri/src/networking/runtime/handlers.rs:80-135,191-266` mutates `TournamentState` before the host registers the connection; `src-tauri/src/networking/runtime/host.rs:191-265` contains post-acceptance error returns without cleanup. The initial snapshot `write_json_frame` failure path also lacks compensation.

### Required implementation

1. Represent the attempt as an explicit `AdmissionAttempt`/guard or equivalent small state machine with unique attempt generation and original state. Distinguish **new join** from **reconnect**. `Prepared -> SnapshotBuilt -> SnapshotDelivered -> Registered -> Active`, with a single cleanup path for every failure.
2. Preserve all data needed for precise compensation. For a new join, remove only that uncommitted participant if it is still owned by the failed attempt; release its reserved capacity/slot and any related readiness/seat effects. For a reconnect, restore the pre-attempt disconnected/reconnect-eligible state, token expiry, and participant phase *only if the attempt is still current*.
3. Choose synchronization/commit order such that external readers cannot observe a permanent connected participant without an established registered stream. If staged state is temporarily needed to build an accurate snapshot, contain it and guarantee rollback. Define lock ordering with `transition_lock`, authoritative-state mutex, and client-registry mutex; never perform blocking TCP I/O holding those locks.
4. On snapshot construction failure, write failure, timeout reconfiguration failure, stream cloning failure, registry poison/failure, replacement conflict, and session-thread spawn failure: close/retire that transport, release all guards, roll back and record a categorized non-secret health error. Verify the `spawn_host_client_session` interface actually exposes launch errors; if not, refactor it to do so.
5. On successful registration, transfer ownership of active-client slot guard to the session and disable compensation only once the session is accountable for disconnect cleanup. Ensure any subsequent error invokes existing cleanup exactly once.
6. Handle simultaneous attempts from the same player ID and reconnect token deterministically: one wins, loser cannot undo the winner. Preserve existing authentication, replay/sequence checks, max pending joins, active client caps and reconnect expiry.
7. If genuine in-progress connections require a temporary state, it must not count as a usable connected seat, and must not leak private snapshots to other players.

### Tests and acceptance

- Unit/fault-injection tests cover post-insertion snapshot-build error, snapshot-send close/reset, read/write-timeout reconfiguration failure, stream clone, registry insertion, session spawn and poisoned-lock handling (via injectible seam; do **not** rely on fragile OS timing alone).
- Real TCP test: repeatedly disconnect immediately after transmitting the initial signed join request. Capacity returns; same authorized identity can retry; no connected phantom remains. Test both before seating and before ready state.
- Real TCP reconnect test: interrupt delivery of the reconnect snapshot. Previously disconnected participant remains reconnectable until its legitimate expiry, and a later retry succeeds. Verify token/key misuse remains rejected.
- Concurrent same-identity join/reconnect; no double registration, seat corruption, cleanup of winner, deadlock, permanent slot leak, or unauthorized cards.
- Existing `join.rs`, `reconnect.rs`, `reconnect_expiry.rs`, `host_shutdown.rs`, and end-to-end reconnect tests remain passing.

## 5. DP-REVIEW-002/003/007 — Versioned tournament-aware durable history

These three findings **must be designed and implemented together**. Fixing the frontend key alone does not fix Rust merging; changing Rust alone does not fix cache precedence or concurrent writes.

### 5.1 Existing behavior and selected data contract

- V1 on-disk: `hand-history.json` with `{updatedAtMs,entries:[{handNumber,...}]}`; entries are deduplicated by `handNumber` in both Rust and TypeScript.
- V2 on-disk: `hand-history.v2.json` (new file in the same per-profile directory); **preserve V1 as an immutable migration source and recovery asset**. The V2 document uses an explicit integer `schemaVersion: 2`, `updatedAtMs`, and `entries` with stable per-hand identity.
- New identity: `{tableId: string, sessionEpoch: positive integer, handNumber: positive integer}`. The tuple is authoritative. `tableId` and `sessionEpoch` already exist on host/client session-status DTOs and on the Rust state, but `TableViewSnapshot` currently exposes only `tableId`; extend its Rust and TypeScript contracts with `sessionEpoch`, including debug/test snapshots and API contract tests. **Never infer sessionEpoch by parsing tableId.**
- Each V2 entry retains existing summary/pot/winner/elimination/board fields. Add `provenance` (`live`, `importedDurableV1`, `importedCacheV1`) and optional display metadata (`tournamentName`, `sessionStartedAtMs`), treating names/timestamps as non-key, best-effort presentation metadata. If using a different struct layout, preserve the same invariants and document the JSON contract.
- Legacy V1 data lacks tournament provenance. Import it into a **stable, reserved legacy-origin group**, rather than assigning it to current live play. A legacy group may distinguish durable-file import from cache import, so conflicting entries can be retained without lying about their source. Unknown historical sessions cannot be reconstructed; the UI should label them as legacy/unattributed.
- Deduplication key for new entries: `(tableId, sessionEpoch, handNumber)`. For legacy imports: `(legacyOriginId, handNumber)` within their known source group. Do not collapse non-identical entries from different legacy origins. Identical import candidates may be collapsed only with an explicit deterministic migration rule.
- Reject invalid/empty new identity fields and non-positive/non-integral hand numbers; preserve valid V1 source bytes if migration encounters unsupported entries. Define hard caps/field validation consistent with existing DTOs to prevent runaway files from hanging UI.

Example **illustrative** V2 persisted entry (not an implemented API yet):

```json
{
  "schemaVersion": 2,
  "updatedAtMs": 1792100000000,
  "entries": [
    {
      "tableId": "table-instance-1792100000000",
      "sessionEpoch": 1792100000000,
      "handNumber": 1,
      "provenance": "live",
      "tournamentName": "Friday Night",
      "summary": "Player A wins",
      "potTotal": 200,
      "winningPlayers": ["Player A"],
      "eliminatedPlayers": [],
      "boardCards": []
    }
  ]
}
```

Legacy entries can use a separate tagged union with `{provenance:"importedDurableV1", legacyOriginId:"..."}` rather than forging `tableId`/`sessionEpoch`. Do not serialize both formats ambiguously.

### 5.2 Durable-store authority and migration

1. Prefer distinct versioned Tauri commands (`read_persisted_hand_history_v2`, `merge_persisted_hand_history_v2`) or carefully version the existing command payload and enforce schema checks. Rust must validate all incoming fields rather than trusting a frontend TypeScript type. Older input without identity must never silently merge into the current tournament.
2. First V2 read: if V2 exists and validates, return it; do **not** replace it with stale V1. If V2 is absent, atomically convert valid V1 into V2 with deterministic legacy IDs, provenance and a persisted migration marker/source fingerprint. Preserve the V1 file unmodified. If V1 is corrupt/unsupported, report an actionable error without deleting it or claiming empty history.
3. Browser cache is no longer the source of truth for an already-durable identity. Build an explicit **one-time cache import**: add genuinely missing identities only, and retain conflicting/ambiguous V1 records under a labeled legacy-cache origin; never overwrite durable V2 records on load. Persist a migration marker before clearing/retiring the old cache. If durable persistence fails, retain cache bytes and report an error; retry safely.
4. For a new live history entry, obtain identity from the **same snapshot** that contains the history (not from a stale UI host draft or unrelated status query). This applies to table polling, completion gate, history view, renderer cache and any saved count/status. Extend DTOs and callbacks end-to-end.
5. Replace prior global `handNumber` normalization in `src/app/persistence.ts` and `src-tauri/src/history_storage.rs` with identical compound-ID semantics. Merge explicit incoming authoritative live updates only into their matching hand; a passive cache read must never overwrite durable values.
6. Preserve the completion screen's **durable-save-before-success** behavior. The text `Final history saved.` must appear only after backend V2 persistence succeeded. Avoid using localStorage success as proxy for disk durability.
7. Treat `hand-history.v2.json` as profile-scoped and separate from other instance IDs. Keep a safe reversible migration and documentation of downgrade behavior: older app versions only see V1 snapshot; they must not be told they have V2 histories. Do not delete V2 when rolling back.

### 5.3 Concurrency, atomicity, and file integrity

1. Serialise read–merge–validate–replace with a per-profile transaction lock shared by **all** commands in the current process. Review all read paths for consistent visible committed snapshots.
2. If two processes can share a profile (`DESKTOP_POKER_INSTANCE_ID`), implement an OS-backed interprocess lock or explicitly enforce and test single-writer profile ownership. Do not silently assume one writer when runtime profiles are user-configurable. Never hold the lock across an unbounded network operation or UI callback.
3. Generate random/unique temporary filenames in the target directory with secure create-new semantics; preserve existing `write_all`, file `sync_all`, atomic rename, directory sync, and cleanup. Review Windows fallback/backup behavior; avoid deleting the only good copy on failure.
4. For lock acquisition, no implicit last-writer-wins loss. A failed lock or fsync is a failure presented to the caller; original on-disk data is unchanged. Add predictable failure injection for interrupted writes and poisoned/acquisition failures.
5. Prefer no-op writes for an identical incoming history payload, and avoid updating `updatedAtMs` on a read operation. Reopening History must not itself mutate an already migrated store.

### 5.4 Presentation and lifecycle

1. Hand History displays the current and saved tournaments without hiding prior sessions because a live table has entries. Group and label sessions (live name and ID; legacy label for unknown provenance), with deterministic newest-session-first ordering.
2. Inside a tournament, display `handNumber` descending; render only first 100 initially, then successive older pages (`slice(0,visibleCount)` on descending data). Session selection, page counters, React keys and accessibility labels use compound identity.
3. For history entries with no reliable across-session timestamp, use stable deterministic fallback ordering and mark provenance; do not fabricate settlement times.
4. Preserve read access to all saved histories across restart; show visible errors for corrupted V1/V2 storage instead of silently resetting, deleting or hiding the file.
5. If history persistence is configured only for local player profiles, preserve that boundary and do not accidentally synchronize private local histories across the network.

### Tests and acceptance

- Pure Rust tests: V1->V2 migration, idempotent restart and retry, corrupt V1/V2, unsupported schema, distinct tournaments both hand 1, duplicate same tournament/hand, stable and deterministic ordering, no-op read, atomic failure recovery, concurrent same-profile writers and disjoint updates.
- Pure TypeScript tests: cache-only V1 and V2 data; stale cached hand conflicts with durable hand; cache absent; same hand number in different tournaments; no false current-tournament assignment; no second-import duplicates.
- Integration: finish tournament A (hands 1–N), restart, finish B (hands 1–M), restart, see both without loss. Interrupt history write or force storage error: preserve both recoverable source data and honest completion messaging.
- Contract: `sessionEpoch` propagated through Rust projection, Tauri JSON and frontend types, including host/client, local/observer, debug fixtures; no private card leakage.
- Performance: use >=205 hand fixtures in at least two sessions and verify bounded visible DOM, stable paging, newest-first, all older entries reachable.

## 6. DP-REVIEW-004 — History paging correction

**Observed source:** `src/screens/HandHistoryScreen.tsx:25-34,132-145` calculates `firstVisibleHistoryIndex = length - visibleCount` even though persistent ordering is newest-first. Current `HandHistoryScreen.test.tsx` asserts the resulting oldest-page behavior, so the regression test **must change** rather than being preserved as a false oracle.

**Required behavior:** For each selected, descending sorted group, display first 100 newest; `Show 100 older hands` adds the next older 100; remaining batch correctly counts down; no duplicate React keys across tournaments; update screen-reader status to reflect actual record ordering and group. Add tests with reverse/mixed input order, exactly 100, 101, 205, and empty entries. Do this inside the V2 grouping work, or as an early isolated safe fix with follow-up tests.

## 7. DP-REVIEW-005 — Bounded join-payload parsing

**Observed source:** `src-tauri/src/protocol/join_payload.rs:67-88`; compact gzip is decoded and fully inflated using unbounded `read_to_string`.

**Selected conservative limits (adjust only with a measured compatibility rationale):**

| Input | Maximum |
|---|---:|
| Entire encoded compact invitation (`pkr1_` included) | 16 KiB |
| Compressed bytes following base64 decoding | 12 KiB |
| Inflated UTF-8 JSON bytes | 64 KiB |
| Legacy raw JSON invitation bytes | 64 KiB |

- Reject excess encoded length **before** base64 decode. Use a bounded `Read::take(MAX_INFLATED+1)` or equivalent for gzip and reject overflow before allocating an unrestricted string. Keep errors stable, non-secret and distinguish invalid base64, gzip, UTF-8, JSON, version and size violations.
- Enforce a compressed-byte bound even if a future caller bypasses encoded-length entry checks; apply the legacy bound before parsing. Preserve `validate_join_payload()` and current valid compact/legacy round-trips; optionally reject trailing gzip junk with an explicit policy.
- Add a gzip-bomb regression, invalid/truncated gzip, base64 overflow, invalid UTF-8, boundary +/-1 byte cases, and encoded round-trip fixtures. Do not log raw invitation/token on rejection. Limit checks are *local parsing hardening*; signed protocol validation remains authoritative.

## 8. DP-REVIEW-006 — Provider secret migration and cleanup

**Observed source:** `src-tauri/src/npc/provider_storage.rs:240-314`. In `load_provider_config`, `Ok(Some(key))` bypasses `apiKey` stripping in old combined JSON.

- After loading legacy combined JSON, detect whether `apiKey` exists regardless of keychain state. If a valid secure store key exists, prefer that key and **atomically rewrite** non-secret settings, with restrictive file permissions and fsync, removing `apiKey`. Never replace the existing secure key merely because legacy settings contain a different value.
- If secure store has no key, write and verify the embedded key before scrubbing. If write/read/verification fails, leave the old file intact, return a visible migration failure, and never claim cleanup succeeded. Never print the value in logs/errors.
- If rewriting sanitized settings fails, treat it as an explicit incomplete migration, not success; do not erase the sole recoverable secret or silently continue with a plaintext file in a release-ready state.
- Review the older `claude-api-key.txt` path and define non-destructive one-time migration/retirement: do not continue relying on an old plaintext key once secure storage has succeeded. For debug builds, document that `llm-provider-key.dat` is deliberately not OS-keychain security and ensure release does not fall back to it.
- Test existing keychain key + legacy JSON, empty store + legacy JSON, differing keys, secure write/read failure, scrub/rename failure, settings corruption, keyless providers, legacy plaintext file, and startup after restart. Confirm logs/CI fixtures contain no example secret outside synthetic tests.

## 9. DP-REVIEW-008 — Monotonic frontend refreshes

**Observed source:** `src/screens/useLobbySession.ts:163-220` and `src/screens/MainTableScreen.tsx:110-158`; timer and event fetches run independently and may settle out of order.

- Use monotonically increasing request generations for each screen **and session identity** (or preferably an authoritative per-session snapshot revision if already safely available). A response with a generation older than the latest applied request must not update state; consider discarding older dispatched requests even if the newer request has not returned, but ensure failure fallback remains truthful.
- A terminal state must not be undone by stale `Connected`/`Running` results for that session; a genuinely new session can intentionally reset terminal state using different identity. Do not reset error counters or slow-connection indicators on ignored/stale results.
- Apply the same gate to event-triggered refresh and timeout/poll code paths. Invalidate all in-flight results on unmount, route transition or session change; clean up event handlers/timers. Avoid extra unconditional polling or request storms.
- Tests with manually controlled deferred promises: event result A requested first / poll B later / resolve B then A; inverse resolution; latest rejection while old pending; terminal then stale running; unmount/cleanup; new session identity after termination. Verify no regression of reconnect/navigation behavior.

## 10. DP-REVIEW-009 — Backlog reconciliation

Review `docs/DESKTOP_POKER_CURRENT_BACKLOG.md` against code/tests. In particular, clarify whether the following are already implemented, partial, or missing:

- `DP-UX-P1-001` authoritative ready-state handling (`TournamentLobbyScreen.tsx` and focused ready-state tests).
- `DP-UX-P1-003` NPC profile unsaved edits and slug validation (`NpcProfilesScreen.tsx`, tests).
- `DP-UX-P2-001` bounded history rendering (existing 100-row strategy; correct reverse paging and V2 grouping still outstanding).
- Other backlog entries touched by the remediation (physical LAN, credential migration, Android contract, remote LLM evidence) must retain their true original scopes/statuses.

Do **not** mark broad UX features done solely because a helper or individual regression exists. Require targeted tests or documented manual evidence. Keep this spec and TODO authoritative for DP-REVIEW-001..009 and cross-reference them from the current backlog; avoid rewriting historical specs or erasing evidence.

## 11. Delivery phases and dependencies

| Phase | Required scope | Depends on | Exit gate |
|---|---|---|---|
| A: Baseline | Inspect current master, record tests, reproduce defects, audit changed head | None | Reproducers/known baseline documented |
| B: Connection correctness | DP-REVIEW-001 | A | Fault injection + interrupted TCP/reconnect tests green |
| C: History integrity | DP-REVIEW-002,003,007 as one coordinated schema/migration transaction | A | Multi-tournament, migration, race, restart tests green |
| D: Security | DP-REVIEW-005,006 | A | Size/secret failure matrices green |
| E: UI correctness | DP-REVIEW-004,008 | C for history display; A for refresh races | Correct paging, non-regressing snapshot tests green |
| F: Reconciliation | DP-REVIEW-009, upgrade docs and evidence | B–E | Every item traceably complete or honestly blocked |
| G: Qualification | Complete exact-head CI, runtime smoke, packaging; record outstanding external gates | B–F | No regressions and exact commit evidence |

**Parallelism:** Phase B and Phase D may proceed independently; Phase C's Rust store, DTOs, frontend persistence and migration must coordinate on **one** reviewed V2 contract. Phase E's table/lobby refresh can run alongside C; history UI grouping follows V2 DTO adoption.

## 12. Required qualification matrix

### Automated local/unit/contract (mandatory)

From repository root with Rust stable and Node.js 24 (and system libraries required for Tauri):

```bash
npm ci
npm run format:check
npm run lint
npm run test
npm run build
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-targets --all-features -- --test-threads=2
```

Run the focused Rust and Vitest suites after each changed area. The existing `.github/workflows/ci.yml` is the source of truth for exact command/environment parity. Record totals and failed/ignored tests; do not claim green based on a partial run. Dependency audit policy remains in CI.

### Automated integration/runtime (mandatory on a qualified Linux runner)

- Signed join, interrupted handshake, reconnect and host-stop tests under real TCP (both fail and success paths).
- Tauri completion saves durable V2 records across process restart, as well as read-only migration from V1.
- Full-game Linux release build and runtime validation: `.github/workflows/runtime-full-game-validation.yml`, `.github/workflows/runtime-validation.yml`, and the relevant reconnect / release-package flows on the **current executable-code SHA**. Confirm event/log artifact contents rather than trusting workflow title alone.
- Perform migrations using ephemeral, isolated profiles; intentionally seed V1 durable and localStorage fixtures, and confirm both original V1 preservation and V2 idempotency.
- Enforce no secret/token/private-card leakage in logs/evidence.

### Human/hardware gates (explicitly separate)

- `docs/PHYSICAL_LAN_RELEASE_VALIDATION.md` two physical machines: required for advertising physical LAN release readiness, **not** retroactively inferred from local two-process CI.
- Android/Desktop `ACTION_WINDOW_OPENED_EVENT` / `ACTION_REJECTED_EVENT` interoperability and remote LLM-provider matrix remain independently tracked. They are not completion requirements for the nine reviewed bugs, and should not be automatically marked PASS.
- Accessibility and UX manual review remains open unless performed with traceable evidence.

### Evidence checklist

For each DP-REVIEW ID record: baseline SHA; implementation commit SHA(s); exact changed files; test names and pass/fail counts; exact-head CI run URLs/status; required runtime artifacts; remaining caveats; final backlog entry. `PASS` requires the test itself to exist and pass. `BLOCKED` needs a specific dependency and evidence; merely being unable to poll CI is `PENDING`, not proof of success/failure.

## 13. Overall definition of done

This remediation is complete **only** when all nine DP-REVIEW issues have: implementation integrated into current `master` (when development has been authorized), focused regressions, accepted schema compatibility and safe migration, exact-head passing CI, applicable Linux runtime proof, and reconciled TODO/backlog status. There must be no known unresolved phantom admission, history collision, stale-cache overwrite, lost writer update, unbounded decompression, plaintext credential migration leak, reversed paging or stale UI regression in the tested paths.

The **software remediation** may be complete while physical-LAN hardware acceptance, Android interoperability, remote LLM qualification or manual accessibility remain explicitly unqualified; these follow their existing independent release gates. No published release or universal release-readiness claim is implied by closing this TODO.

## 14. Decisions, assumptions, and change control

- **Decision:** Preserve complete history across successive tournaments; do not reinterpret the product as “last tournament only” to avoid the bug.
- **Decision:** Stable identity uses authoritative `tableId`, `sessionEpoch` and hand number; no parsing IDs from names/timestamps and no new cryptographic/protocol identity unless proven necessary.
- **Decision:** Adopt side-by-side V2 file rather than destructively rewriting V1; migration is idempotent and recoverable.
- **Decision:** Treat the durable V2 store as canonical, browser cache as a recoverable import source only.
- **Decision:** Explicitly label unattributed legacy data; historical tournament identity cannot be reliably reconstructed.
- **Decision:** A separate two-physical-machine LAN gate remains required for full physical LAN release qualification.
- **Assumption to verify:** A single `DESKTOP_POKER_INSTANCE_ID` may be launched twice. If allowed, require cross-process history safety; if deliberately disallowed, enforce single-writer ownership and test its error handling.
- **Assumption to verify:** Full hand histories exposed by a Tauri snapshot have the authoritative session identity. Verify debug/mock states and remote-client projections before choosing final API field names.
- **Change control:** Any different schema or synchronization approach must be documented with tradeoffs, revised tests, migration compatibility analysis, and equivalent protection of N/H/S/U invariants before implementation.
