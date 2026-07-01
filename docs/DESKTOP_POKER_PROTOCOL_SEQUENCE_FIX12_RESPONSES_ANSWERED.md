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

   A: Use the **delay-installation approach**.

   There is no reason to add a cleanup helper or introduce `TcpStream` test plumbing if the stream can simply be installed after all reconnect snapshot validation succeeds. That is the safer and simpler shape because there is no transient state where `command_connection.stream` contains a cloned stream tied to a reconnect attempt whose snapshot later proves malformed.

   Preferred implementation shape:

   ```rust
   let next_command_stream = reconnect_stream
       .try_clone()
       .map_err(|error| NetworkingError::Protocol(format!(
           "failed to clone reconnect stream for command connection: {error}"
       )))?;

   let snapshot_envelope = read_snapshot_response(/* existing args */)?;

   let Some(snapshot_sequence) = snapshot_server_sequence_or_warn(
       &sender,
       &mut protocol_warning_counts,
       &player_id,
       snapshot_envelope.server_sequence,
   ) else {
       let _ = sender.send(ClientRuntimeEvent::SafeError {
           player_id: player_id.clone(),
           message: "reconnect snapshot missing server sequence".to_string(),
       });
       break;
   };

   // Validate all other required snapshot fields here too, before installing.
   // Only after the snapshot is accepted:
   reconnect_token = snapshot_envelope.payload.reconnect_token.clone();
   last_seen_server_sequence = Some(snapshot_sequence);
   command_connection.stream = Some(next_command_stream);
   ```

   The exact error handling should match the current code style, but the important ordering rule is: **do not store the cloned command stream until the reconnect/resync snapshot has passed required sequence validation and any other required snapshot validation.**

---

2. Q: **P0.3 — `RequiredSequenceDecision` enum**: The TODO presents this consolidated enum/helper
   as optional and says only add it if it simplifies the production code (it probably does not
   simplify the `SNAPSHOT_EVENT` branch much beyond what `is_stale_server_sequence` directly
   provides). Should the implementation **skip the enum** and use `is_stale_server_sequence`
   inline in the snapshot branch, or is the consolidated helper preferred?

   A: Skip the `RequiredSequenceDecision` enum for Fix 12 and use `is_stale_server_sequence(...)` inline in the snapshot branch.

   The enum is only worth adding if it clearly reduces production complexity. For this patch, explicit inline logic is easier for Claude Code to implement correctly and easier to review. Keep the behavior direct:

   ```rust
   let Some(snapshot_sequence) = snapshot_server_sequence_or_warn(
       &sender,
       &mut protocol_warning_counts,
       &player_id,
       envelope.server_sequence,
   ) else {
       // Malformed sequenced snapshot. Do not update last_seen_server_sequence.
       continue;
   };

   if is_stale_server_sequence(last_seen_server_sequence, Some(snapshot_sequence)) {
       emit_protocol_warning(
           &sender,
           &mut protocol_warning_counts,
           &player_id,
           "stale snapshot server sequence",
       );
       continue;
   }

   reconnect_token = envelope.payload.reconnect_token.clone();
   last_seen_server_sequence = Some(snapshot_sequence);

   if let Some(next_host_encryption_public_key) =
       envelope.payload.host_encryption_public_key.clone()
   {
       host_encryption_public_key = next_host_encryption_public_key;
   }

   let _ = sender.send(ClientRuntimeEvent::Snapshot(Box::new(envelope.payload)));
   ```

   Do not introduce a broader abstraction unless the current production code has multiple duplicated required-sequence/stale-sequence branches that become materially clearer with one helper. The acceptance criteria matter more than the abstraction: missing snapshot sequence warns/drops/preserves last-seen, stale snapshot sequence warns/drops/preserves last-seen, and valid fresh snapshot sequence updates last-seen only after validation.

---

Fill in the `A:` lines above, then share the file or paste your answers back.
