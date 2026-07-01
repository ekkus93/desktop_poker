# Responses — Fix 15: Final Event Delivery and Validation Closure
# Spec: docs/DESKTOP_POKER_EVENT_DELIVERY_FINAL_FIX15_SPEC.md
# TODO: docs/DESKTOP_POKER_EVENT_DELIVERY_FINAL_FIX15_TODO.md

1. Q: Cleanup lock blocks (P0.1 scope): The two `if let Ok(mut connection) = command_connection_for_thread.lock()` blocks already have Fix-14 "Best-effort cleanup" comments. P0.1 targets `sender.send(...)` calls, not lock blocks. Should the implementor leave those two lock lines exactly as-is (already addressed by Fix 14) and only touch `sender.send(...)` lines for this task?
   A: Yes. Leave those two cleanup lock blocks exactly as-is unless you discover a real regression while auditing. Fix 14 already handled them with explanatory best-effort cleanup comments. Fix 15 should focus on ignored `sender.send(...)` calls / `ClientRuntimeEvent` delivery paths, not churn lock-cleanup code that is already documented.

   The intent is to make event-channel delivery intent explicit, not to refactor every benign cleanup path. Do not change the cleanup lock blocks just to reduce grep output. Only touch them if the reconnect install invariant has regressed, for example if a required command-stream install is again using silent `if let Ok(...)` handling before snapshot emission.

2. Q: SafeError sends on clone/lock failure (spec vs P0.2): The spec says the helper must not be used for "command-stream installation failure," but P0.2's required shape shows `send_runtime_event_best_effort` used to deliver the `SafeError` notification when clone or lock fails (with `break` immediately after). Confirming: the intent is that the helper IS used for those SafeError notification sends — the `break` is what preserves the invariant, and the helper is only about event-channel delivery — correct?
   A: Correct. The helper may be used to send the `SafeError` notification for clone/lock failure. The important invariant is that command-stream installation failure is not treated as recoverable and the reconnect snapshot is not emitted afterward. In other words, this is acceptable:

   ```rust
   let cloned_stream = match stream.try_clone() {
       Ok(cloned_stream) => cloned_stream,
       Err(error) => {
           send_runtime_event_best_effort(
               &sender,
               ClientRuntimeEvent::SafeError {
                   player_id: player_id.clone(),
                   message: format!("failed to clone reconnect command stream: {error}"),
               },
           );
           break;
       }
   };
   ```

   What is not acceptable is using the helper as a way to log/notify and then continue into snapshot acceptance. The helper is only about best-effort event-channel delivery; it must not become a fallback that allows the runtime to proceed after a required command-stream install fails. After clone or lock failure, emit `SafeError` best-effort, then `break` / stop that runtime path before any accepted reconnect snapshot is exposed.

---
Filled in the A: lines above.
