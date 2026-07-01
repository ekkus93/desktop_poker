# Fix 12 Responses — Final Protocol Ledger and Snapshot Ordering Cleanup
# Spec: DESKTOP_POKER_PROTOCOL_SEQUENCE_FIX12_SPEC.md
# TODO: DESKTOP_POKER_PROTOCOL_SEQUENCE_FIX12_TODO.md

---

1. Q: **P0.2 — Delay vs. cleanup helper**: The "delay installation" approach (clone the stream,
   do all snapshot validation, install only on success) is feasible and eliminates the need for a
   cleanup helper and its test. The alternative is an explicit
   `clear_command_stream_after_failed_reconnect` helper plus a test that requires constructing a
   `ClientCommandConnection` with a real or test `TcpStream`. Should the implementation use the
   **delay-installation approach** (simpler, no TcpStream test plumbing needed), or is there a
   specific reason to use the cleanup-helper approach?

   A:

---

2. Q: **P0.3 — `RequiredSequenceDecision` enum**: The TODO presents this consolidated enum/helper
   as optional and says only add it if it simplifies the production code (it probably does not
   simplify the `SNAPSHOT_EVENT` branch much beyond what `is_stale_server_sequence` directly
   provides). Should the implementation **skip the enum** and use `is_stale_server_sequence`
   inline in the snapshot branch, or is the consolidated helper preferred?

   A:

---

Fill in the `A:` lines above, then share the file or paste your answers back.
