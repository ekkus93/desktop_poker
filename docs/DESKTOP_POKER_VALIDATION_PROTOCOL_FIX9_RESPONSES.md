# Fix 9 Responses — Validation Docs and Public Event Sequence Hardening
# Spec: DESKTOP_POKER_VALIDATION_PROTOCOL_FIX9_SPEC.md
# TODO: DESKTOP_POKER_VALIDATION_PROTOCOL_FIX9_TODO.md

---

1. Q: **README "Getting started" block** (lines 183–191): This quick-start snippet contains
   `npm run lint` and `npm run test` but no `npm run format:check`. Should P0.1 also update
   this block to include `format:check`, or is this block intentionally a minimal "get up and
   running" flow and not a validation section?

   A:

---

2. Q: **lint-n-test SKILL.md `--all-targets` flag**: The SKILL.md currently uses
   `cargo test --workspace` (no `--all-targets --all-features`). Should this be updated to
   match the full validation standard (`cargo test --workspace --all-targets --all-features`)
   as part of P0.1, or should it stay lighter (workspace-only) since the skill is a quick
   local sanity check rather than a CI-level gate?

   A:

---

3. Q: **P0.3 test approach**: The spec offers two paths — (a) a full near-runtime test that
   feeds a malformed signed envelope through the `ClientRuntime` read loop, or (b) extract a
   small helper (`public_event_server_sequence_or_warn`) and test the helper directly. Given
   the existing test infrastructure uses real TCP, **which do you prefer**: the
   extracted-helper unit test (faster, easier to construct), or a near-runtime test that
   exercises the actual read-loop dispatch path?

   A:

---

Fill in each `A:` line above, then share the file or paste your answers back.
