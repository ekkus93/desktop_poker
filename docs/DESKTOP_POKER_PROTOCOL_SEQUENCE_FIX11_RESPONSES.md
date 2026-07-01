# Fix 11 Responses — Finish Snapshot/Public Sequence Validation
# Spec: DESKTOP_POKER_PROTOCOL_SEQUENCE_FIX11_SPEC.md
# TODO: DESKTOP_POKER_PROTOCOL_SEQUENCE_FIX11_TODO.md

---

1. Q: **Lines 116 and 204 (reconnect and resync paths)**: These have
   `last_seen_server_sequence = snapshot_envelope.server_sequence` where
   `snapshot_envelope.server_sequence` is `Option<u64>` — the same bug class as the Fix 11
   target (line 316). The spec says Fix 11 must stay small and identifies only line 316 as the
   explicit problem. Should Fix 11 also harden lines 116 and 204 (reconnect and resync snapshot
   assignments), or should those be left for a later fix?

   A:

---

Fill in the `A:` line above, then share the file or paste your answer back.
