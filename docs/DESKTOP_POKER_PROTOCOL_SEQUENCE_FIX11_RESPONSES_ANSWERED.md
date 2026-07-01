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

   A: Yes. Fix 11 should harden lines 116 and 204 too.
      
      Those assignments are the same bug class as the line 316 `SNAPSHOT_EVENT` issue: snapshot-derived `Option<u64>` is being copied directly into `last_seen_server_sequence`. The fix should not be line-specific. The invariant should be:
      
      - any host-originated snapshot that is accepted as authoritative/resync state must have `server_sequence: Some(_)`;
      - missing snapshot `server_sequence` is malformed protocol input;
      - missing snapshot `server_sequence` must not clear or overwrite `last_seen_server_sequence`;
      - the client should emit/record a visible error or warning and avoid applying that malformed snapshot as the new ordering baseline.
      
      Keep the scope small by fixing only snapshot/public sequence validation, not by redesigning the runtime.
      
      Recommended implementation direction:
      
      ```rust
      fn snapshot_server_sequence_or_warn(
          sender: &std::sync::mpsc::Sender<ClientRuntimeEvent>,
          counts: &mut std::collections::BTreeMap<String, u64>,
          player_id: &str,
          server_sequence: Option<u64>,
      ) -> Option<u64> {
          match server_sequence {
              Some(sequence) => Some(sequence),
              None => {
                  emit_protocol_warning(
                      sender,
                      counts,
                      player_id,
                      "snapshot envelope missing server sequence",
                  );
                  None
              }
          }
      }
      ```
      
      Then replace each direct assignment like this:
      
      ```rust
      last_seen_server_sequence = snapshot_envelope.server_sequence;
      ```
      
      with validation first:
      
      ```rust
      let Some(snapshot_sequence) = snapshot_server_sequence_or_warn(
          &sender,
          &mut protocol_warning_counts,
          &player_id,
          snapshot_envelope.server_sequence,
      ) else {
          // Malformed snapshot/resync response. Do not mutate last_seen_server_sequence.
          continue; // or break/return SafeError if this path cannot safely continue
      };
      
      last_seen_server_sequence = Some(snapshot_sequence);
      ```
      
      Use the control flow that fits the surrounding path:
      
      - For a live `SNAPSHOT_EVENT` frame: emit `ProtocolWarning`, drop the frame, and continue reading.
      - For a resync response requested after stale-sequence detection: emit `SafeError` or `ProtocolWarning` and break/reconnect if continuing without a valid resync snapshot would leave the client in an unsafe state.
      - For initial join/reconnect snapshot setup: fail the connect/reconnect attempt rather than silently accepting a snapshot with no sequence.
      
      Also update the focused audit so this must return no hits:
      
      ```bash
      rg -n "last_seen_server_sequence = .*server_sequence" src-tauri/src/networking/runtime/client.rs
      ```
      
      A direct assignment from a validated local `u64` is fine:
      
      ```rust
      last_seen_server_sequence = Some(snapshot_sequence);
      ```
      
      Add tests for at least one reconnect/resync/snapshot path or a production-used helper that proves missing snapshot sequence preserves prior `last_seen_server_sequence` and emits a visible diagnostic. Do not leave lines 116 and 204 for a later fix; they are in scope because they are the same ordering-state corruption risk.
      

---

Fill in the `A:` line above, then share the file or paste your answer back.
