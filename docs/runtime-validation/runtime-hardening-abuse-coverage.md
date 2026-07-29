# Runtime hardening abuse-test coverage

This index maps hostile or malformed peer behavior to deterministic tests in the normal Rust suite. A row marked **Open** is intentionally not represented as complete.

| Hostile input or condition | Coverage | Runnable test |
|---|---|---|
| Oversized frame prefix | Covered | `networking::runtime::tests::abuse::host_accept_loop_survives_oversized_truncated_and_malformed_join_frames` |
| Truncated frame body | Covered | `networking::runtime::tests::abuse::host_accept_loop_survives_oversized_truncated_and_malformed_join_frames` |
| Malformed JSON | Covered | `networking::runtime::tests::abuse::host_accept_loop_survives_oversized_truncated_and_malformed_join_frames` |
| Wrong first-message `messageType` | Covered | initial-request rejection tests under `networking::runtime::tests` |
| Invalid join token | Covered | join-request rejection tests under `networking::runtime::tests` |
| Duplicate player ID join | Covered | join identity/capacity tests under `networking::runtime::tests` |
| Already-connected reconnect | Covered | reconnect tests using `RECONNECT_ALREADY_CONNECTED` |
| Reconnect after host-side disconnect | Covered | `networking::runtime::tests::reconnect` and end-to-end reconnect tests |
| Resync after stale server sequence | Covered | `networking::runtime::tests::resync` |
| Unsupported post-connect request | Covered | connected-session protocol rejection tests |
| Bad signature | Covered | integrity and protocol-warning tests |
| Remote stale action-window ID | Covered | `networking::runtime::tests::action_outcomes::remote_stale_window_rejection_does_not_mutate_state` |
| Remote invalid raise amount | Covered | `networking::runtime::tests::action_outcomes::remote_invalid_raise_rejection_does_not_mutate_state` |

## Explicitly deferred

- Physical-LAN packet loss, router isolation, and cross-device firewall behavior remain release/manual validation concerns rather than deterministic protocol-unit tests.
- Resource-exhaustion behavior above the configured connection limits is covered by bounded-counter/unit tests; sustained distributed load testing is not part of normal CI.
