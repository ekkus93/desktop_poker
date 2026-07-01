# Fix 7 Responses — DESKTOP_POKER_CORE_EXTRACTION_FIX7_SPEC / _TODO

Fill in each `A:` line and share the file back when ready.

---

1. Q: **P0.2 — DebugPanel rendering style**: Should the new health counters be added using the **existing inline conditional pattern** (`{counter > 0 && <li>Label: {value}</li>}` for each field), or by **refactoring to the `hostHealthCounterRows` helper** approach suggested in the TODO? The spec says "adapt to current style" (which means inline), but the suggested code uses a helper function that returns an array and renders with `.map`.

   A: Use the existing inline conditional pattern for this pass. The goal of Fix 7 is to surface the missing counters with minimum churn, not to refactor `DebugPanel` rendering. Add each new counter next to the existing host runtime health counters using the same style already present in the component.

   Acceptable direction:

   ```tsx
   {hostRuntimeHealth.streamCloneErrorCount > 0 && (
     <li>Stream clone errors: {hostRuntimeHealth.streamCloneErrorCount}</li>
   )}
   {hostRuntimeHealth.clientRegistryErrorCount > 0 && (
     <li>Client registry errors: {hostRuntimeHealth.clientRegistryErrorCount}</li>
   )}
   {hostRuntimeHealth.reconnectMarkErrorCount > 0 && (
     <li>Reconnect mark errors: {hostRuntimeHealth.reconnectMarkErrorCount}</li>
   )}
   {hostRuntimeHealth.snapshotSyncErrorCount > 0 && (
     <li>Snapshot sync errors: {hostRuntimeHealth.snapshotSyncErrorCount}</li>
   )}
   ```

   Also update the TypeScript `HostRuntimeHealth` type so these fields are modeled explicitly. Do not add the helper/map refactor unless the existing component already has moved in that direction. A helper refactor is fine later, but it should not be bundled with this hardening cleanup.

---

2. Q: **P0.3 — helper naming**: The TODO's suggested code references `disconnect_client_or_record_health`, but the function already in `host_session.rs` is called `disconnect_client`. Should we **rename `disconnect_client` to `disconnect_client_or_record_health`** (closer to the spec's naming), or **keep the existing name and just call both `record_state_lock_error` + `disconnect_client`** at each lock-failure site?

   A: Keep the existing `disconnect_client` name. Do not rename it just to match the TODO snippet. Add a separate small helper such as `record_state_lock_error(...)`, then call `record_state_lock_error(...)` before `disconnect_client(...)` at each authoritative-state lock-failure site.

   Preferred pattern:

   ```rust
   fn record_state_lock_error(
       runtime_health: &Arc<Mutex<HostRuntimeHealth>>,
       context: impl Into<String>,
   ) {
       let context = context.into();
       update_health(runtime_health, |health| {
           health.state_lock_error_count += 1;
           health.record_error(context);
       });
   }
   ```

   Then at each lock-failure branch:

   ```rust
   record_state_lock_error(
       &runtime_health,
       format!("authoritative state lock poisoned while handling action for {player_id}"),
   );
   disconnect_client(&clients, &authoritative_state, &runtime_health, &player_id);
   break;
   ```

   Renaming `disconnect_client` would create extra import/test churn without improving behavior. The important requirement is that state-lock poisoning is recorded in `HostRuntimeHealth` before the disconnect/break path.

---

3. Q: **P0.5 — NPC test approach**: Which level of coverage is acceptable?
   - **Option A (preferred by spec)**: Full integration test — call `try_npc_action` with a real `HostServer` + corrupted hole cards. More proof, more setup.
   - **Option B (spec's fallback)**: Extract `validated_acting_hole_cards` as a named helper function (replacing the inline code at `action.rs:236–256`), test the helper directly. Simpler and still catches a deletion of the validation gate.
   - **Option C**: Both — a helper test for the validation logic, plus one near-integration test that calls into `try_npc_action` with a minimally constructed `TournamentState` (no full `HostServer` needed if `try_npc_action` can be called with simpler arguments).

   A: Prefer Option C if it is reasonably achievable in this codebase. Extract `validated_acting_hole_cards(...)` as a named helper and unit-test it directly, then add one near-integration test that exercises the real NPC action path far enough to prove that missing/invalid acting NPC hole cards cause no action submission and a structured internal error.

   Minimum acceptable implementation for Fix 7 is Option B plus a clear comment explaining why the full/near-integration test is impractical right now. However, do not skip the production helper extraction. The validation gate should be named and tested so future refactors do not accidentally restore the empty-hole-card fallback.

   Suggested helper shape:

   ```rust
   fn validated_acting_hole_cards<'a>(
       fresh_hand: &'a HandState,
       player_id: &str,
   ) -> Result<&'a [Card], String> {
       match fresh_hand.hole_cards_by_player_id.get(player_id) {
           Some(cards) if cards.len() == 2 => Ok(cards.as_slice()),
           Some(cards) => Err(format!(
               "NPC {player_id} has invalid hole-card count {}; expected 2; no action submitted",
               cards.len()
           )),
           None => Err(format!(
               "NPC {player_id} is missing hole cards; no action submitted"
           )),
       }
   }
   ```

   Then `try_npc_action` should convert `Err(message)` into `record_npc_internal_error(...)` and submit no action. The test should assert both missing cards and wrong card count.

---

4. Q: **P1.2 — `set_next_deck_for_test` on `PokerEngine`**: Should this be added now (wrapping the already-public `TournamentController::set_next_deck`) or deferred to whenever Android FFI work starts? It is not required by any current DoD item.

   A: Add it now, but keep it explicitly test-only or test-oriented. This is small, useful, and helps lock down deterministic `poker-core` behavior before Android/FFI work begins. Do not expose it as a normal gameplay API unless there is a production use case.

   Preferred implementation:

   ```rust
   #[cfg(any(test, feature = "test-support"))]
   impl PokerEngine {
       pub fn set_next_deck_for_test(
           &mut self,
           cards: Vec<Card>,
       ) -> Result<(), PokerCoreError> {
           self.controller
               .set_next_deck(cards)
               .map_err(|error| PokerCoreError::Engine(error.to_string()))
       }
   }
   ```

   If the project does not want a new Cargo feature yet, `#[cfg(test)]` is enough for now. If integration tests outside the crate need it, add a `test-support` feature in `crates/poker-core/Cargo.toml`:

   ```toml
   [features]
   default = []
   test-support = []
   ```

   Do not add Android FFI bindings for this method in Fix 7.
