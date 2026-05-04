# Codebase Walkthrough

## Top-Level Flow

The app starts very simply in `src/main.tsx` and `src/App.tsx`: React mounts a hash-router, then wraps the UI in `DesktopBootstrapProvider`. That provider immediately asks Tauri for the bootstrap contract and subscribes to `desktop://bootstrap` updates through `src/api/desktop.ts`. From there, `src/app/AppShell.tsx` is the main frontend router: it renders loading or bridge-failure states first, then builds the screen map from the Rust-provided bootstrap metadata instead of hardcoding product assumptions in the UI.

That frontend shell is intentionally thin. `src/app/DesktopShellProvider.tsx` owns local-only state like display name, host draft, remembered join payloads, ready toggles, and cached hand-history count. Persistence is browser-side and namespaced per instance via `src/app/persistence.ts` and `src/app/shell.ts`, which matches the project’s multi-instance requirement: each instance gets its own storage keys and window-state bucket.

## Frontend Surfaces

The important frontend screens are mostly presenters over Rust commands. `src/screens/HostTournamentSetupScreen.tsx` uses the backend only for LAN-IP resolution right now, then stages a host draft and a placeholder share surface. `src/screens/JoinTournamentScreen.tsx` is a little more concrete: it validates pasted or launch-supplied payloads with the Rust decoder, remembers recent payloads, and prepares the next handoff step. `src/screens/MainTableScreen.tsx` is the richest live surface: it fetches a `TableViewSnapshot`, renders local vs observer mode, submits table actions back to Rust, and persists settled hand history. `src/components/debug/DebugPanel.tsx` exposes the same pattern for debug-only inspection: bootstrap metadata, snapshot JSON, protocol log, current action window, and the multi-instance launch helper.

The key architectural point is that the frontend does not compute poker truth. It asks Rust for already-shaped views and sends user intent back down. That boundary is visible in the API file itself: `src/api/desktop.ts` mostly defines typed `invoke(...)` wrappers and shared view-model shapes.

## Rust Backend

The Tauri backend starts in `src-tauri/src/lib.rs`. It creates a `DesktopAppState`, emits the bootstrap event during setup, and registers a small command surface from `src-tauri/src/commands.rs`. Those commands are narrow: fetch bootstrap, validate join payloads, resolve LAN IP, fetch a table view, submit a table action, fetch debug state, and launch another debug client instance.

The main composition root is `src-tauri/src/app_state/mod.rs`. That file does three things that matter:

- It detects instance identity from `--instance-id` or `DESKTOP_POKER_INSTANCE_ID`, then derives storage/session/reconnect namespaces and a per-instance profile directory.
- It detects an optional launch payload from CLI or env and parses it through the protocol layer up front.
- It exposes a `DesktopTableRuntime` that backs the current table and debug screens.

The protocol layer is clearly separated in `src-tauri/src/protocol/mod.rs` and `src-tauri/src/protocol/models.rs`. This is where the Android-compatible message types, signed envelopes, encrypted private envelopes, join payload encoding/decoding, and canonical signing behavior live. The domain layer in `src-tauri/src/domain/mod.rs` defines the core immutable tournament and participant models plus validation rules. Hidden-information control is enforced by the projector in `src-tauri/src/domain/projector.rs`: it produces a public projection, per-player private projections, and an observer projection with no hole cards and no action authority. The tournament logic itself is in `src-tauri/src/tournament/mod.rs`, where the `TournamentController` owns ready checks, roster freeze, blind progression, action windows, timeout handling, hand advancement, and completion.

The networking and crypto stacks are real modules, not placeholders. `src-tauri/src/networking/mod.rs` and `src-tauri/src/networking/runtime.rs` implement raw TCP framing, host/client runtime types, connectable LAN-IP validation, join payload generation, and host-side snapshot/public-event/private-envelope handling. `src-tauri/src/crypto/mod.rs` wraps the Ed25519/X25519/ChaCha stack behind an internal provider abstraction.

## Most Important Current Nuance

The codebase is slightly ahead of the UI in some places and slightly behind in others. The backend has real protocol, crypto, networking, and tournament modules, but the current shell surfaces are not all driving those live runtime paths yet. The clearest proof is in `src-tauri/src/app_state/mod.rs`: `DesktopAppState` currently initializes `DesktopTableRuntime`, and that runtime builds a `demo_controller()`, starts a tournament immediately, drives to an “interesting” local turn, and auto-plays opponents. So the main table and debug surfaces are Rust-backed and authoritative, but they are currently powered by an in-process demo tournament harness rather than a live host/client session.

That same pattern shows up in the host and join screens. They already use real backend pieces where it matters most for correctness:

- host screen uses real LAN-IP resolution
- join screen uses the real Rust payload decoder/validator
- debug screen uses the real Tauri command surface and can spawn extra instances

But those screens still describe the next step as wiring into the live runtime, which means the repo contains both the actual networking stack and a shell/runtime layer that has not fully switched over to it yet.

## Suggested Next Walkthroughs

If you want the next walkthrough pass, the highest-value options are:

1. Trace one end-to-end slice, like join payload from paste to protocol decode to client connect.
2. Trace the poker path, from `TournamentController` state to projector output to table UI.
3. Audit the gap between the existing live networking runtime and the current host/join shell wiring.
