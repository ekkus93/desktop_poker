# Desktop Architecture

## Ownership boundaries

### Rust-owned logic boundary

Rust is the source of truth for:

- LAN TCP transport
- protocol envelopes and canonical serialization
- crypto and key handling
- tournament/session orchestration
- reconnect/resync behavior
- persistence and per-instance storage namespaces
- authoritative state projection for public, private, and observer views

The frontend must never become the authoritative source of game state.

### Frontend-owned rendering boundary

The React + TypeScript frontend owns:

- routing and screen composition
- rendering state that Rust projects into UI-friendly shapes
- local interaction affordances such as forms, dialogs, and desktop layouts
- developer-facing debug views in debug builds only

The frontend may stage user input locally, but all game actions must flow through Rust-owned validation and authority.

## Current implementation choices

- **Frontend stack:** React + TypeScript
- **Rust JSON models:** `serde`
- **Canonical signing bytes:** one canonical JSON serializer path used for signatures
- **Crypto stack target:** `ed25519-dalek`, `x25519-dalek`, `chacha20poly1305`
- **TCP framing:** length-prefixed JSON envelopes
- **Default host port:** `43818`
- **Per-instance storage:** instance-scoped profile directory under local app data

## Durable hand history and provider secrets

- Rust stores durable history in the profile-scoped `hand-history.v2.json` file. Each live hand is keyed by `(tableId, sessionEpoch, handNumber)`; legacy V1 entries remain explicitly unattributed.
- The original `hand-history.json` is retained unchanged for recovery and downgrade compatibility. Older binaries read only V1 and do not display V2-only records; re-upgrading keeps the V2 file.
- V2 read/merge transactions take a per-profile operating-system file lock. Writes use unique temporary files, sync file contents, atomically replace the destination, and sync the containing directory where supported.
- Browser history is a migration/cache source, not evidence of a durable write. Completion success is shown only after the Tauri backend confirms persistence.
- Release provider keys use the operating-system secret store. Provider settings JSON contains non-secret configuration; debug builds use the documented local development store and do not provide release fallback behavior.

## Android compatibility goal

Desktop targets the current Android runtime semantics rather than stale markdown:

- protocol version `1`
- direct join payload flow as the canonical v1 join path
- compact `pkr1_` join payload support
- signed public events for steady-state gameplay
- snapshots for join, reconnect, and explicit resync
- room-code discovery is not treated as production protocol truth

## Multi-instance requirements

- Multiple desktop instances must be able to run on one machine.
- Instance identity must be separable via CLI arg or env var.
- Per-instance local state must not overwrite another instance's profile.
- Host port collisions must be surfaced explicitly rather than silently hidden.
