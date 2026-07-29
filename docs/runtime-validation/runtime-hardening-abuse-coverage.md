# Runtime hardening abuse-test coverage

This index maps hostile or malformed peer behavior to deterministic tests in the normal Rust suite. Every **Covered** row names an exact runnable test.

| Hostile input or condition | Coverage | Runnable test |
|---|---|---|
| Oversized frame prefix | Covered | `networking::runtime::tests::abuse::host_accept_loop_survives_oversized_truncated_and_malformed_join_frames` |
| Truncated frame body | Covered | `networking::runtime::tests::abuse::host_accept_loop_survives_oversized_truncated_and_malformed_join_frames` |
| Malformed JSON | Covered | `networking::runtime::tests::abuse::host_accept_loop_survives_oversized_truncated_and_malformed_join_frames` |
| Wrong first-message `messageType` | Covered | `networking::runtime::tests::join::initial_request_rejects_wrong_message_type` |
| Invalid join token | Covered | `networking::runtime::tests::join::join_requests_reject_the_wrong_join_token` |
| Duplicate player ID join | Covered | `networking::runtime::tests::join::duplicate_player_id_join_is_rejected_without_replacing_identity` |
| Already-connected reconnect | Covered | `networking::runtime::tests::reconnect::reconnect_rejects_already_connected_participants` |
| Reconnect after host-side disconnect | Covered | `networking::runtime::tests::reconnect::reconnect_succeeds_only_with_original_keypair_and_valid_token` |
| Resync after stale server sequence | Covered | `networking::runtime::tests::misc::resync_after_a_sequence_gap_allows_followup_public_events_to_continue` |
| Unsupported post-connect request | Covered | `networking::runtime::tests::session::connected_session_rejects_unsupported_request_with_protocol_error` |
| Bad public-event signature | Covered | `networking::runtime::tests::protocol_warning::bad_public_signature_emits_protocol_warning_and_runtime_continues` |
| Remote stale action-window ID | Covered | `networking::runtime::tests::action_outcomes::remote_stale_window_rejection_does_not_mutate_state` |
| Remote invalid raise amount | Covered | `networking::runtime::tests::action_outcomes::remote_invalid_raise_rejection_does_not_mutate_state` |

## Explicitly deferred

- Physical-LAN packet loss, router isolation, and cross-device firewall behavior remain release/manual validation concerns rather than deterministic protocol-unit tests.
- Sustained distributed resource-exhaustion/load testing remains outside normal CI; configured connection limits and counters are covered by deterministic unit tests.
