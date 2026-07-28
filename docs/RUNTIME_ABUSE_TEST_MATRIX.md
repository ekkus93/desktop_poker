# Runtime Abuse Test Matrix

This matrix maps the runtime-hardening abuse cases to committed automated coverage. The full Rust workspace test command is the authoritative execution gate.

| Abuse case | Primary automated coverage |
|---|---|
| Oversized frame length | `src-tauri/src/networking/framing.rs` — rejects the advertised length before body allocation |
| Truncated frame length/body | `src-tauri/src/networking/framing.rs` — truncated prefix and truncated body tests |
| Malformed frame JSON | `src-tauri/src/networking/framing.rs` — invalid JSON and type-mismatch tests |
| Oversized, truncated, and malformed initial join peers cannot kill the host | `src-tauri/src/networking/runtime/tests/abuse.rs` — reconnects a healthy client after all three bad peers |
| Missing `messageType` | `src-tauri/src/networking/runtime/tests/protocol_warning.rs` — warning emission and low-noise counting |
| Missing or invalid signature | `src-tauri/src/protocol/models/tests.rs` and runtime join/reconnect handler tests |
| Wrong table ID or session epoch | runtime join/reconnect/resync handler validation tests under `src-tauri/src/networking/runtime/tests/` |
| Stale/replayed counter or stale server sequence | `src-tauri/src/protocol/replay.rs`, runtime reconnect tests, and runtime resync tests |
| Decrypt/authenticated-metadata failure | crypto provider tests and encrypted private-envelope tests in `src-tauri/src/protocol/models/tests.rs` |
| Missing authoritative `serverSequence` | snapshot/client runtime protocol tests under `src-tauri/src/networking/runtime/tests/` |
| Reconnect with stale token | `src-tauri/src/networking/runtime/tests/reconnect.rs` |
| Reconnect while participant is still connected | `src-tauri/src/networking/runtime/tests/reconnect.rs` — retry and retry-exhaustion cases |
| Resync with future sequence | `src-tauri/src/networking/runtime/tests/resync.rs` — `resync_rejects_future_sequences` |
| Invalid action, wrong actor, stale action window, and timeout race | poker-core controller tests and `src-tauri/src/networking/runtime/tests/tournament.rs` |
| Lobby snapshot write failure | `src-tauri/src/networking/runtime/tests/tournament.rs` — mutation remains authoritative, client is removed/reconnectable, health increments |

Bad peers may be rejected or disconnected, but these tests require the host/client runtime to remain alive, bounded, and observable. Any newly accepted protocol message type or new command path should add its corresponding malformed-input and replay cases to this matrix.
