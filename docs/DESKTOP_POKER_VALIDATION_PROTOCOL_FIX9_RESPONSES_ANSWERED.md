# Fix 9 Responses — Validation Docs and Public Event Sequence Hardening
# Spec: DESKTOP_POKER_VALIDATION_PROTOCOL_FIX9_SPEC.md
# TODO: DESKTOP_POKER_VALIDATION_PROTOCOL_FIX9_TODO.md

---

1. Q: **README "Getting started" block** (lines 183–191): This quick-start snippet contains
   `npm run lint` and `npm run test` but no `npm run format:check`. Should P0.1 also update
   this block to include `format:check`, or is this block intentionally a minimal "get up and
   running" flow and not a validation section?

   A: Yes. Update the README “Getting started” block to include `npm run format:check` if that block already includes `npm run lint` and `npm run test`.

Reason: once a quick-start block includes validation commands, it stops being only “how to launch the app.” Leaving out `format:check` would keep the exact validation drift Fix 9 is trying to remove.

Preferred README shape:

```bash
npm ci
npm run format:check
npm run lint
npm run test
npm run build
```

If the block is meant to be a minimal launch-only path, split it into two subsections instead:

```md
### Start the app

npm ci
npm run tauri dev

### Validate before committing

npm run format:check
npm run lint
npm run test
npm run build
cargo fmt --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-targets --all-features
cargo test -p poker-core
cargo tree -p poker-core
```

Do not keep `lint`/`test` in the block while omitting `format:check`.

---

2. Q: **lint-n-test SKILL.md `--all-targets` flag**: The SKILL.md currently uses
   `cargo test --workspace` (no `--all-targets --all-features`). Should this be updated to
   match the full validation standard (`cargo test --workspace --all-targets --all-features`)
   as part of P0.1, or should it stay lighter (workspace-only) since the skill is a quick
   local sanity check rather than a CI-level gate?

   A: Update it to the full validation standard: `cargo test --workspace --all-targets --all-features`.

The `lint-n-test` skill should be a reliable “this repo is clean” command set, not a lighter partial check that can miss non-default targets/features. If a faster local sanity check is useful, add it as an explicitly named optional shortcut, but the canonical validation path should match CI and README.

Preferred Rust validation block:

```bash
cargo fmt --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-targets --all-features
cargo test -p poker-core
cargo tree -p poker-core
```

This also avoids future agents using a weaker command and then claiming the repo is fully validated.

---

3. Q: **P0.3 test approach**: The spec offers two paths — (a) a full near-runtime test that
   feeds a malformed signed envelope through the `ClientRuntime` read loop, or (b) extract a
   small helper (`public_event_server_sequence_or_warn`) and test the helper directly. Given
   the existing test infrastructure uses real TCP, **which do you prefer**: the
   extracted-helper unit test (faster, easier to construct), or a near-runtime test that
   exercises the actual read-loop dispatch path?

   A: Prefer the extracted-helper unit test for Fix 9, provided the production read-loop path is required to call that helper.

Do not add a heavier real-TCP read-loop test for this pass unless it is already straightforward with existing helpers. The issue is narrow: a public event envelope without `server_sequence` must be treated as malformed, emit `ProtocolWarning`, and skip dispatch. A helper test can cover that deterministically without brittle TCP setup.

Required implementation shape:

```rust
fn public_event_server_sequence_or_warn(
    sender: &std::sync::mpsc::Sender<ClientRuntimeEvent>,
    warning_counts: &mut std::collections::BTreeMap<String, u64>,
    player_id: &str,
    server_sequence: Option<u64>,
) -> Option<u64> {
    match server_sequence {
        Some(sequence) => Some(sequence),
        None => {
            emit_protocol_warning(
                sender,
                warning_counts,
                player_id,
                "public event envelope missing server sequence",
            );
            None
        }
    }
}
```

Then the read-loop branch must use it like this:

```rust
let Some(server_sequence) = public_event_server_sequence_or_warn(
    &sender,
    &mut protocol_warning_counts,
    &player_id,
    envelope.server_sequence,
) else {
    continue;
};
```

The test should assert both behaviors:

```rust
#[test]
fn missing_public_event_server_sequence_emits_warning_and_returns_none() {
    let (sender, receiver) = std::sync::mpsc::channel();
    let mut warning_counts = std::collections::BTreeMap::new();

    let result = public_event_server_sequence_or_warn(
        &sender,
        &mut warning_counts,
        "player-1",
        None,
    );

    assert_eq!(result, None);

    let warning = receiver.try_recv().expect("expected protocol warning");
    assert!(matches!(
        warning,
        ClientRuntimeEvent::ProtocolWarning { ref player_id, ref reason, count }
            if player_id == "player-1"
                && reason.contains("missing server sequence")
                && count == 1
    ));
}

#[test]
fn present_public_event_server_sequence_returns_sequence_without_warning() {
    let (sender, receiver) = std::sync::mpsc::channel();
    let mut warning_counts = std::collections::BTreeMap::new();

    let result = public_event_server_sequence_or_warn(
        &sender,
        &mut warning_counts,
        "player-1",
        Some(42),
    );

    assert_eq!(result, Some(42));
    assert!(receiver.try_recv().is_err());
}
```

If the helper is not called by the real read-loop branch, the helper test is not sufficient. The production code must remove the `unwrap_or_default()` behavior entirely.

---

Fill in each `A:` line above, then share the file or paste your answers back.
