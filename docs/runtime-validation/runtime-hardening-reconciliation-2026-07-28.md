# Desktop Poker Runtime Hardening Reconciliation

Date: 2026-07-28  
Repository: `ekkus93/desktop_poker`  
Branch: `master`  
Result: **COMPLETE**

## Authoritative final results

| Gate | Run / revision | Result |
|---|---|---|
| General CI and browser geometry | run `30403803338`, commit `7a5d547ddb103fd1276f9ac258941eac1ea079f4` | PASS |
| Direct Linux release and multi-instance runtime | run `30403803495`, commit `7a5d547ddb103fd1276f9ac258941eac1ea079f4` | PASS |
| Full release tournament and persistence | run `30335336680` | PASS |
| Reconnect and host-loss matrix | run `30335336702` | PASS |
| Rule-based NPC tournament | run `30335336686` | PASS |
| Embedded local GGUF tournament | `docs/runtime-validation/embedded-tournament-latest.json` | PASS |

`docs/runtime-validation/ci-latest.json` is the authoritative current general-CI result. `docs/runtime-validation/latest.json`, `gameplay-latest.json`, `reconnect-failure-latest.json`, `rule-based-tournament-latest.json`, and `embedded-tournament-latest.json` retain the runtime evidence used for Phase 15.

## Final CI coverage

The passing CI gate executed:

- `npm ci` and npm audits;
- frontend Prettier verification;
- ESLint;
- frontend unit tests;
- TypeScript/Vite production build;
- Rust formatting;
- Clippy across the workspace, all targets, and all features with warnings denied;
- Rust workspace tests;
- direct `poker-core` tests;
- browser geometry validation.

## Phase reconciliation

- **Phases 1–3:** frame-size limits, explicit client shutdown, bounded pending joins, and bounded active-client slots are implemented and covered by tests.
- **Phases 4–6:** named operation deadlines, pending UI states, typed event polling, and code-driven reconnect rejection handling are implemented and tested.
- **Phases 7–8:** explicit action outcomes, timeout rollback, authoritative publish rules, and observable non-fatal lobby snapshot failures are implemented and tested.
- **Phases 9–10:** critical Tauri commands expose serialized `{code, message, recoverable}` errors; shared Rust/TypeScript DTO fixtures are exercised by both test suites. The v1 policy is explicit contract fixtures rather than generated bindings.
- **Phases 11–14:** storage writes fail non-fatally with visible warnings; all five LLM provider variants are documented and round-trip tested; malformed-peer coverage is part of normal tests; sanitized runtime-health warnings are visible outside debug-only UI while raw diagnostics remain in debug state.
- **Phase 15:** retained runtime evidence covers real TCP host/join, seats, ready state, tournament start, legal and rejected actions, observer transition, reconnect, host loss, malformed invitation handling, private-card isolation, final standings/history, and rule-based plus embedded-local NPC tournaments. Shutdown/thread and oversized-frame behavior are additionally covered by deterministic Rust tests.
- **Phase 16:** `.github/workflows/ci.yml` runs the important hardening tests automatically. Heavier release, reconnect, packaging, keychain, and NPC scenarios remain in dedicated workflows with machine-readable evidence under `docs/runtime-validation/`.

## Reconciliation decision

All tasks and acceptance criteria in `docs/DESKTOP_POKER_RUNTIME_HARDENING_CODE_REVIEW_TODO.md` are satisfied by committed code and evidence. The TODO can be marked complete.

The deferred two-machine physical-LAN gate remains open only in the separate release-readiness backlog. It was not silently waived and is not required to close this code-review hardening TODO.
