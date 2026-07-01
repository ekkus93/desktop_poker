# Fix 10 Responses — Public Event Sequence Ordering Hardening
# Spec: DESKTOP_POKER_PROTOCOL_SEQUENCE_FIX10_SPEC.md
# TODO: DESKTOP_POKER_PROTOCOL_SEQUENCE_FIX10_TODO.md

---

1. Q: After Fix 10, `ProtocolError` messages will no longer trigger a resync (even if stale)
   and will no longer update `last_seen_server_sequence` — because the outer stale-check and
   last-seen-update blocks are removed from the `_` catch-all arm, and `ProtocolError` is not
   in the `matches!(envelope.message_type, ...)` list that now owns those operations. The spec
   calls `ProtocolError` "intentionally unsequenced." **Is this the intended behavior, or should
   `ProtocolError` continue to update `last_seen_server_sequence` when it carries one?**

   A: Yes, that is intended. Treat `ProtocolError` as intentionally unsequenced. It should not update `last_seen_server_sequence`, should not participate in stale-sequence detection, and should not trigger resync based on its optional `server_sequence` value.

   Rationale: `last_seen_server_sequence` should represent the client’s position in the authoritative live public-event stream. `ProtocolError` is a control/error notification, not a game-state transition. Letting it advance or clear ordering state would recreate the same class of bug Fix 10 is trying to remove: malformed or non-gameplay frames changing replay/resync ordering state.

   Implementation guidance:

   - Keep `ProtocolError` out of the live-public-event `matches!(...)` list.
   - Do not require `server_sequence` for `ProtocolError`.
   - Do not call `is_stale_server_sequence(...)` for `ProtocolError`.
   - Do not assign `last_seen_server_sequence = envelope.server_sequence` for `ProtocolError`.
   - Continue surfacing `ProtocolError` as `ClientRuntimeEvent::SafeError` or the existing safe error/debug event path.
   - If a future protocol design needs sequenced server error events, add a separate explicit message type such as `SequencedProtocolErrorEvent` or include it deliberately in the live-public-event set with tests. Do not make the current generic `ProtocolError` implicitly sequenced.

   Suggested test expectation:

   ```rust
   #[test]
   fn protocol_error_does_not_update_last_seen_server_sequence() {
       // Arrange last_seen_server_sequence = Some(10) and a ProtocolError envelope
       // carrying server_sequence = Some(99).
       // Act through the extracted sequencing helper or nearest read-loop helper.
       // Assert last_seen_server_sequence remains Some(10), no resync is requested,
       // and the error is surfaced through the safe error path.
   }
   ```

---

Fill in the `A:` line above, then share the file or paste your answer back.
