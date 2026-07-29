# Desktop Poker Runtime Hardening Fixes Reconciliation

Date: 2026-07-29  
Result: **COMPLETE**

## Evidence source identity

The retained publishers name more than one commit because documentation-only evidence and reconciliation commits landed while independent workflows were finishing. The reconciliation gate compared every named commit against the current tree across all product-source paths and required zero differences.

- Source-equivalent validated commit: `561c374e31414b3581355d8e83216d8aadee2616`
- Source-equivalent validated commit: `5662a2bbd7705c60267991686c26cb81da7491d5`
- Source-equivalent validated commit: `5f13e32ab5d5a3cda265c2cbbd0986c9c1e837c6`

## Implemented behavior

- Remote client actions use explicit typed outcomes.
- Timeout-advanced rejected actions commit and publish the advanced state before the rejection is returned.
- Wrong-player, stale-window, and invalid-size remote submissions reject visibly and publish zero transitions.
- Gameplay-state invariants normalize networking-only participant metadata before comparison.
- Command errors preserve stable typed codes without substring classification.
- Join-session failures preserve invalid-payload, network-timeout, disconnected-runtime, and unknown-command provenance instead of collapsing every failure into `INVALID_JOIN_PAYLOAD`.
- Host runtime health fields are synchronized across Rust, TypeScript, UI diagnostics, and shared contract fixtures.
- Client event polling preserves timeout-versus-disconnection semantics.
- Inbound and outbound frame-size limits are symmetric and regression-tested.
- Hostile peer and abuse coverage is indexed and backed by deterministic tests.
- Keyless local LLM providers do not depend on an OS keychain; API-provider secret deletion remains fail-closed.

## Key implementation commits

- `06c080471a4269eaca48f5a58d26f851b4f57375` — typed command error provenance
- `0bc5216575e1e7bafa9355596360afb361ee718b` — hostile-peer abuse matrix
- `0d813c2499c33287c7c513cdb9fff92b1d31e4ee` — normalized remote action invariants
- `c97a8be60b264ec3a158902747b1ec2a3acaefc1` — real remote timeout publication regression
- `14a7df4a2d980c72f5164a97638fd7dc7e32e422` — remote rejection visibility and zero publication
- `bd252a2e39363a3ad8e06b95b6ddde6bd0f7ce4b` — typed command errors and shared DTO contract coverage
- `e494ca46cc764dc78a90ee3f7200de70a8357726` — Rust HostRuntimeHealth shared fixture assertion
- `adbd7401a666ba94a978b03f9dd868cd3adcc1b6` — nested TypeScript HostRuntimeHealth assertion
- `df5733421b360d26011faff08663945d53e9aab8` — typed join-session error provenance
- `5bbeefdca40836a1d518103422aea904baf1d9f1` — keyless local providers avoid unavailable platform keychains
- `99daa6d9a5f1379c19ca7788601cbe965fe2447f` — canonical formatting for the keyless-provider regression test

## Retained validation evidence

- **General CI: PASS** — `docs/runtime-validation/ci-latest.json`; GitHub Actions run `30446064358`; validated source-equivalent commit `5f13e32ab5d5a3cda265c2cbbd0986c9c1e837c6`.
- **Release runtime: PASS** — `docs/runtime-validation/latest.json`; GitHub Actions run `30453595346`; validated source-equivalent commit `5662a2bbd7705c60267991686c26cb81da7491d5`.
- **Full gameplay: PASS** — `docs/runtime-validation/gameplay-latest.json`; GitHub Actions run `30446064609`; validated source-equivalent commit `5f13e32ab5d5a3cda265c2cbbd0986c9c1e837c6`.
- **Reconnect and host-loss: PASS** — `docs/runtime-validation/reconnect-failure-latest.json`; GitHub Actions run `30436380943`; validated source-equivalent commit `561c374e31414b3581355d8e83216d8aadee2616`.
- **Rule-based NPC tournament: PASS** — `docs/runtime-validation/rule-based-tournament-latest.json`; GitHub Actions run `30436381577`; validated source-equivalent commit `561c374e31414b3581355d8e83216d8aadee2616`.
- **Embedded model inference: PASS** — `docs/runtime-validation/embedded-model-latest.json`; GitHub Actions run `30436380826`; validated source-equivalent commit `561c374e31414b3581355d8e83216d8aadee2616`.
- **Full embedded NPC tournament: PASS** — `docs/runtime-validation/embedded-tournament-latest.json`; GitHub Actions run `30436380826`; validated source-equivalent commit `561c374e31414b3581355d8e83216d8aadee2616`.

## Baseline and transient validation notes

- The earlier rule-based NPC failure was retained as a pre-existing baseline artifact and was not silently reclassified as a regression. The final rule-based tournament rerun passed.
- Embedded-tournament runs cancelled by newer source pushes were treated as incomplete evidence, not as code success. Completion required a later non-cancelled PASS artifact.
- The final embedded tournament itself passed, but its first durable-evidence push conflicted with concurrent evidence publishers. The successful artifact was recovered and published without changing its payload.
- Validation exposed a real keyless-provider storage bug. Product source was fixed; no CI-only fallback or silent keychain bypass was added.
- An optional patch preflight caught a malformed generated expression before it could modify source; the corrected guarded run passed and the superseded failure log was removed.

## Scope note

The automated multi-instance validation uses separate real release processes, isolated profile directories, and real TCP connections on one Linux runner. A separate physical multi-machine LAN session remains an intentionally deferred manual field-validation item, not an unimplemented code requirement.

The TODO's historical DebugPanel path is `src/components/shell/DebugPanel.tsx`; the current implementation and tests live at `src/components/debug/DebugPanel.tsx` and `src/components/debug/DebugPanel.test.tsx`.

## Reconciliation conclusion

Every non-deferred task and acceptance criterion in `docs/DESKTOP_POKER_RUNTIME_HARDENING_FIXES_TODO_2026-07-28.md` is implemented and backed by committed tests or retained runtime evidence. Temporary Ralph-loop workflows, triggers, and patch scripts have been removed.
