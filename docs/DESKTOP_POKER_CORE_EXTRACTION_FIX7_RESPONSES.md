# Fix 7 Responses — DESKTOP_POKER_CORE_EXTRACTION_FIX7_SPEC / _TODO

Fill in each `A:` line and share the file back when ready.

---

1. Q: **P0.2 — DebugPanel rendering style**: Should the new health counters be added using the **existing inline conditional pattern** (`{counter > 0 && <li>Label: {value}</li>}` for each field), or by **refactoring to the `hostHealthCounterRows` helper** approach suggested in the TODO? The spec says "adapt to current style" (which means inline), but the suggested code uses a helper function that returns an array and renders with `.map`.

   A:

---

2. Q: **P0.3 — helper naming**: The TODO's suggested code references `disconnect_client_or_record_health`, but the function already in `host_session.rs` is called `disconnect_client`. Should we **rename `disconnect_client` to `disconnect_client_or_record_health`** (closer to the spec's naming), or **keep the existing name and just call both `record_state_lock_error` + `disconnect_client`** at each lock-failure site?

   A:

---

3. Q: **P0.5 — NPC test approach**: Which level of coverage is acceptable?
   - **Option A (preferred by spec)**: Full integration test — call `try_npc_action` with a real `HostServer` + corrupted hole cards. More proof, more setup.
   - **Option B (spec's fallback)**: Extract `validated_acting_hole_cards` as a named helper function (replacing the inline code at `action.rs:236–256`), test the helper directly. Simpler and still catches a deletion of the validation gate.
   - **Option C**: Both — a helper test for the validation logic, plus one near-integration test that calls into `try_npc_action` with a minimally constructed `TournamentState` (no full `HostServer` needed if `try_npc_action` can be called with simpler arguments).

   A:

---

4. Q: **P1.2 — `set_next_deck_for_test` on `PokerEngine`**: Should this be added now (wrapping the already-public `TournamentController::set_next_deck`) or deferred to whenever Android FFI work starts? It is not required by any current DoD item.

   A:
