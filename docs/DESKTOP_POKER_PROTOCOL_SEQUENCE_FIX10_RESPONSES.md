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

   A:

---

Fill in the `A:` line above, then share the file or paste your answer back.
