# Desktop Poker

[![CI](https://github.com/ekkus93/desktop_poker/actions/workflows/ci.yml/badge.svg)](https://github.com/ekkus93/desktop_poker/actions/workflows/ci.yml)

Desktop Poker is a Linux-first Tauri 2 desktop client/host for single-table Sit 'n Go No-Limit Texas Hold'em over real LAN TCP. The Rust backend owns networking, protocol compatibility, crypto, tournament orchestration, reconnect/resync, persistence, and state projection; the React + TypeScript frontend is a rendering shell.

## Current product status

The repository currently includes:

- A complete desktop host and join flow for a single-table Sit 'n Go over LAN TCP.
- A player-first frontend built around `Home`, `Host`, `Join`, `Lobby`, `Main Table`, `History`, `Tournament Complete`, and contextual recovery states.
- A Rust backend that owns poker rules, tournament orchestration, networking, reconnect and resync behavior, protocol compatibility, crypto, persistence, and state projection.
- Launch-payload handling for direct join flows using compact `pkr1_...` invitations.
- Debug-gated internal tools and multi-instance local testing support without exposing those flows as normal player UX.
- Cached local shell state, saved hand-history summaries, and per-instance storage and window-state isolation.
- Linux desktop bundle output through Tauri 2.
- Frontend and Rust test coverage for host, join, table, observer, reconnect, persistence, and transport behavior.

In practice, the app is already usable for local desktop-hosted poker sessions, while mixed Android/Desktop live sessions remain an explicit interop follow-up rather than a proven shipping claim.

## Current player flow

The current player-facing flow is:

1. Home: choose `Host Tournament` or `Join Tournament`
2. Host or Join setup: open a table or bring an invite and confirm the destination
3. Lobby: gather the table, check readiness, and start the first hand
4. Main Table: play the hand while the board, pot, and current decision stay front and center
5. History, Help, Recovery, or Complete: review secondary details only when you choose them or when the game needs them

Recovery and reconnect states appear only when needed, and debug or multi-instance tooling stays on a hidden debug-only route outside the normal player path.

## What the app is responsible for

### Frontend

- Presents the player flow and screen hierarchy.
- Renders host, join, lobby, table, history, completion, help, and recovery surfaces.
- Persists lightweight local shell state such as recent payloads, host draft values, and cached hand-history summaries.

### Rust backend

- Validates tournament configuration and join payloads.
- Runs the authoritative poker engine and tournament lifecycle.
- Owns real LAN TCP transport, reconnect and resync logic, protocol framing, signing and encryption helpers, and per-instance bootstrap metadata.
- Projects player-safe table views for the frontend.

### Internal tooling

- Supports debug review, payload handoff, and multi-instance development workflows.
- Remains intentionally separated from the normal player path and lives on a hidden debug-only route.

## Frozen implementation choices

- **Frontend stack:** React + TypeScript
- **Serialization path:** `serde` models plus one canonical JSON byte serializer path for signed bytes
- **Crypto stack:** `ed25519-dalek`, `x25519-dalek`, and `chacha20poly1305`
- **Transport framing:** length-prefixed JSON envelopes
- **Default LAN host port:** `43818` to match the current Android default
- **Per-instance storage:** profile namespace under a per-instance local data directory

## Tooling

### Required toolchains

- Rust stable toolchain
- Node.js 24 and npm
- Tauri CLI (installed through the project `devDependencies`)

### Linux prerequisites for Tauri

Install the GTK/WebKit and build packages required by Tauri 2 on Linux. On Debian or Ubuntu, this is typically:

```bash
sudo apt update
sudo apt install -y \
  build-essential \
  curl \
  wget \
  file \
  libxdo-dev \
  libssl-dev \
  libayatana-appindicator3-dev \
  librsvg2-dev \
  libwebkit2gtk-4.1-dev
```

If your distro uses different package names, install the WebKitGTK 4.1 and GTK 3 development packages that Tauri requires.

## Getting started

```bash
npm install
cargo fmt --manifest-path src-tauri/Cargo.toml
cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets --all-features -- -D warnings
cargo test --manifest-path src-tauri/Cargo.toml
npm run lint
npm run test
npm run test:geometry
npm run tauri dev
```

## Build the app

For a production desktop build:

```bash
npm install
npm run tauri build
```

This compiles the frontend and Rust backend, then produces release bundles under `src-tauri/target/release/bundle/`.

If you only want the release binary without the platform bundles:

```bash
cargo build --manifest-path src-tauri/Cargo.toml --release
```

## Run the app

For local development:

```bash
npm run tauri dev
```

For a compiled release binary:

```bash
./src-tauri/target/release/desktop-poker
```

You can also run a packaged Linux build directly, for example:

```bash
./src-tauri/target/release/bundle/appimage/Desktop\ Poker_0.1.0_amd64.AppImage
```

## Running multiple instances locally

The desktop app is intentionally designed for multiple concurrent instances. Use an explicit instance id so each launch gets its own storage namespace, session identity, and reconnect namespace:

```bash
DESKTOP_POKER_INSTANCE_ID=host-a npm run tauri dev
DESKTOP_POKER_INSTANCE_ID=client-b npm run tauri dev
```

You can also pass the instance id on the app command line:

```bash
npm run tauri dev -- -- --instance-id client-c
```

Production binaries can also be launched multiple times with distinct ids:

```bash
./src-tauri/target/release/desktop-poker --instance-id host-a
./src-tauri/target/release/desktop-poker --instance-id client-b
```

The desktop shell also persists per-instance display name, host draft, recent direct-join payloads, saved hand-history summaries, and Tauri window state so repeated local sessions do not stomp one another.

## Passing a join payload at launch

The app accepts a launch payload from either environment or CLI input so the join flow can open with a shared `pkr1_...` invitation already attached:

```bash
DESKTOP_POKER_JOIN_PAYLOAD='pkr1_...' npm run tauri dev
```

or

```bash
npm run tauri dev -- -- --join-payload 'pkr1_...'
```

The CLI form and env var are both surfaced through the Rust bootstrap state and consumed by the frontend join and recovery flows.

## Local multi-instance host/join flow

1. Launch a host instance with its own id, for example `DESKTOP_POKER_INSTANCE_ID=host-a npm run tauri dev`.
2. Copy the current `pkr1_...` payload from the host flow or from the hidden debug route in debug builds.
3. Launch a second instance with a different id and either paste the payload on the Join screen or pass it on the command line:

```bash
DESKTOP_POKER_INSTANCE_ID=client-b npm run tauri dev -- -- --join-payload 'pkr1_...'
```

Loopback (`127.0.0.1`) flows are covered by the in-repo runtime tests, and local LAN flows use the same payload path once a connectable host IP is available. In debug builds, the hidden debug route can copy the payload directly and launch another instance with the payload already attached.

## Architecture notes

See [docs/DESKTOP_ARCHITECTURE.md](docs/DESKTOP_ARCHITECTURE.md) for the Rust/frontend ownership boundary, protocol compatibility stance, and multi-instance rules.

## Android interop status

See [docs/ANDROID_INTEROP_AUDIT.md](docs/ANDROID_INTEROP_AUDIT.md) for the current Android/Desktop audit.

Current status:

- Core protocol version, join-payload shape, canonical JSON rules, reconnect metadata, and key-derivation behavior were compared against the Android source.
- Android-side crypto, networking, and tournament tests were run during the audit.
- Mixed-runtime Android/Desktop host-client sessions are still **audited but not yet proven** by live end-to-end runs in this repository.

## Release notes and current limitations

What is supported today:

- Single-table Sit 'n Go No-Limit Texas Hold'em hosted from the desktop app over LAN TCP.
- Direct join via a shared `pkr1_...` payload.
- Reconnect and resync behavior within the desktop runtime.
- Multi-instance local testing with isolated per-instance storage and session identity.
- Linux development and bundle output through Tauri 2.

What is intentionally limited right now:

- **Interop:** Android/Desktop interoperability is audited, but not fully proven until live mixed-runtime sessions succeed.
- **Discovery:** there is no room-code discovery or matchmaking flow yet. Direct `pkr1_...` payload join is the supported path.
- **Network scope:** the app is LAN-only and assumes a trusted host-authoritative table on the local network.
- **Production behavior:** release builds use the real TCP LAN runtime, while debug launch helpers stay hidden outside debug builds.
- **Assets:** the table uses generated card and felt treatments plus simple status badges so the MVP stays license-safe.
- **Sound:** sound is currently not included. Any future sound support should remain optional and off by default.

## Linux bundles

`npm run tauri build` produces Linux release bundles under `src-tauri/target/release/bundle/`.

Current bundle targets:

- `src-tauri/target/release/bundle/deb/Desktop Poker_0.1.0_amd64.deb`
- `src-tauri/target/release/bundle/rpm/Desktop Poker-0.1.0-1.x86_64.rpm`
- `src-tauri/target/release/bundle/appimage/Desktop Poker_0.1.0_amd64.AppImage`

These bundle names reflect the current package version and will change when the app version changes.
